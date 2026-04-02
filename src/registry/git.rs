use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{DependencySpec, DownloadResult, GitRev, PackageCache, PackageInfo, Registry};

/// Arbitrary Git repository adapter.
/// Supports any git URL (GitHub, GitLab, Gitea, Bitbucket, self-hosted, etc.)
/// that is NOT handled by the more specific registries.
pub struct GitRegistry {
    clone_dir: PathBuf,
    client: reqwest::Client,
}

impl GitRegistry {
    pub fn new(cache_dir: &Path) -> Result<Self> {
        let clone_dir = cache_dir.join("git-repos");
        std::fs::create_dir_all(&clone_dir).context("Failed to create git clone directory")?;
        let client = reqwest::Client::builder()
            .user_agent(concat!("ion/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { clone_dir, client })
    }

    fn repo_key(url: &str) -> String {
        url.replace("://", "_").replace(['/', ':', '.', '@'], "_")
    }

    /// Clone or update a git repository
    fn ensure_repo(&self, url: &str) -> Result<PathBuf> {
        let repo_dir = self.clone_dir.join(Self::repo_key(url));

        if repo_dir.exists() {
            // Repository exists, fetch latest
            let status = Command::new("git")
                .args(["fetch", "--all", "--tags", "--prune"])
                .current_dir(&repo_dir)
                .status()
                .context("Failed to run git fetch")?;
            if !status.success() {
                // Non-fatal: we can work with what we have
                eprintln!("Warning: git fetch failed for {}", url);
            }
        } else {
            // Clone the repository
            let status = Command::new("git")
                .args([
                    "clone",
                    "--filter=blob:none",
                    url,
                    repo_dir.to_str().unwrap(),
                ])
                .status()
                .with_context(|| format!("Failed to clone git repository: {}", url))?;

            if !status.success() {
                anyhow::bail!("git clone failed for {}", url);
            }
        }

        Ok(repo_dir)
    }

    /// List tags in a git repo in descending semver order
    fn list_tags(repo_dir: &Path) -> Result<Vec<String>> {
        let output = Command::new("git")
            .args(["tag", "--list", "--sort=-v:refname"])
            .current_dir(repo_dir)
            .output()
            .context("Failed to run git tag")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let tags: Vec<String> = stdout
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        Ok(tags)
    }

    /// Read ion.toml from a specific ref in the repo
    fn read_ion_toml(repo_dir: &Path, git_ref: &str) -> Option<crate::manifest::Manifest> {
        let output = Command::new("git")
            .args(["show", &format!("{}:ion.toml", git_ref)])
            .current_dir(repo_dir)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let content = String::from_utf8_lossy(&output.stdout);
        toml::from_str(&content).ok()
    }

    /// Archive a specific ref from the repo into a tarball
    fn archive_ref(repo_dir: &Path, git_ref: &str, dest: &Path) -> Result<PathBuf> {
        let archive_path = dest.join(format!("{}.tar.gz", git_ref.replace('/', "_")));
        let status = Command::new("git")
            .args([
                "archive",
                "--format=tar.gz",
                "--output",
                archive_path.to_str().unwrap(),
                git_ref,
            ])
            .current_dir(repo_dir)
            .status()
            .context("Failed to run git archive")?;

        if !status.success() {
            anyhow::bail!("git archive failed for ref {}", git_ref);
        }
        Ok(archive_path)
    }
}

#[async_trait]
impl Registry for GitRegistry {
    fn name(&self) -> &str {
        "git"
    }

    async fn search(&self, _query: &str) -> Result<Vec<PackageInfo>> {
        // Git registry does not support search (requires exact URL)
        Ok(vec![])
    }

    async fn resolve(&self, _name: &str, _version_req: &str) -> Result<PackageInfo> {
        // For arbitrary git, `name` IS the URL (stored in spec)
        anyhow::bail!(
            "Git registry requires full spec. Use: {{ git = \"<url>\", tag = \"<version>\" }}"
        )
    }

    async fn download(&self, info: &PackageInfo, cache: &PackageCache) -> Result<DownloadResult> {
        // source_uri format: "git+https://example.com/owner/repo#tag"
        let url = info
            .source_uri
            .strip_prefix("git+")
            .unwrap_or(&info.source_uri);
        let (url, git_ref) = if let Some((u, r)) = url.split_once('#') {
            (u, r.to_string())
        } else {
            (url, format!("v{}", info.version))
        };

        let repo_dir = self.ensure_repo(url)?;

        // Archive the specific ref
        let tmp_dir = tempfile::TempDir::new()?;
        let archive_path = Self::archive_ref(&repo_dir, &git_ref, tmp_dir.path())?;

        let bytes = std::fs::read(&archive_path)?;
        cache.store(info, &bytes, None)
    }

    fn can_handle(&self, spec: &DependencySpec) -> bool {
        match spec {
            DependencySpec::Git { url, .. } => {
                // Handle any git URL not claimed by GitHub adapter
                !url.contains("github.com")
            }
            _ => false,
        }
    }
}

/// Resolve a DependencySpec::Git to a PackageInfo
pub async fn resolve_git_spec(
    name: &str,
    url: &str,
    rev: &GitRev,
    cache_dir: &Path,
) -> Result<PackageInfo> {
    let registry = GitRegistry::new(cache_dir)?;
    let repo_dir = registry.ensure_repo(url)?;

    let git_ref = match rev {
        GitRev::Tag(t) => t.clone(),
        GitRev::Branch(b) => b.clone(),
        GitRev::Commit(c) => c.clone(),
    };

    // Try to read ion.toml from this ref for metadata
    let version = match rev {
        GitRev::Tag(t) => t.trim_start_matches('v').to_string(),
        GitRev::Branch(b) => b.clone(),
        GitRev::Commit(c) => c[..8.min(c.len())].to_string(),
    };

    // Get the HEAD commit hash for this ref (for the lockfile)
    let commit_output = Command::new("git")
        .args(["rev-parse", &git_ref])
        .current_dir(&repo_dir)
        .output()
        .context("Failed to resolve git ref")?;
    let _commit = String::from_utf8_lossy(&commit_output.stdout)
        .trim()
        .to_string();

    Ok(PackageInfo {
        name: name.to_string(),
        version,
        source: "git".to_string(),
        source_uri: format!("git+{}#{}", url, git_ref),
        description: None,
        homepage: Some(url.to_string()),
        license: None,
        cmake_targets: vec![format!("{}::{}", name, name)],
        dependencies: vec![],
        features: vec![],
    })
}
