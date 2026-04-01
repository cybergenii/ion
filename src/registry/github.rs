use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

use super::{DownloadResult, DependencySpec, PackageCache, PackageInfo, Registry};

/// GitHub Releases registry adapter.
/// Resolves packages from GitHub repos using the Releases API.
/// Supports `git = "https://github.com/owner/repo"` dependency specs.
pub struct GitHubRegistry {
    client: reqwest::Client,
}

impl GitHubRegistry {
    pub fn new(token: Option<String>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            "application/vnd.github+json".parse().unwrap(),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            "2022-11-28".parse().unwrap(),
        );
        if let Some(tok) = token {
            if let Ok(val) = format!("Bearer {}", tok).parse() {
                headers.insert(reqwest::header::AUTHORIZATION, val);
            }
        }

        let client = reqwest::Client::builder()
            .user_agent(concat!("ion/", env!("CARGO_PKG_VERSION")))
            .default_headers(headers)
            .build()
            .expect("Failed to build HTTP client");

        Self { client }
    }

    /// Parse `https://github.com/owner/repo` → ("owner", "repo")
    fn parse_github_url(url: &str) -> Option<(&str, &str)> {
        let url = url.trim_end_matches('/');
        let url = url.strip_prefix("https://github.com/")
            .or_else(|| url.strip_prefix("http://github.com/"))
            .or_else(|| url.strip_prefix("github.com/"))?;
        let mut parts = url.splitn(2, '/');
        let owner = parts.next()?;
        let repo = parts.next()?.trim_end_matches(".git");
        Some((owner, repo))
    }

    async fn list_releases(&self, owner: &str, repo: &str) -> Result<Vec<GitHubRelease>> {
        let url = format!("https://api.github.com/repos/{}/{}/releases", owner, repo);
        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to query GitHub releases for {}/{}", owner, repo))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("GitHub repo {}/{} not found", owner, repo);
        }
        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            anyhow::bail!("GitHub API rate limit exceeded. Set GITHUB_TOKEN to increase limits.");
        }

        let releases: Vec<GitHubRelease> = resp.json().await
            .with_context(|| "Failed to parse GitHub releases response")?;
        Ok(releases)
    }

    fn release_to_package_info(
        &self,
        name: &str,
        owner: &str,
        repo: &str,
        release: &GitHubRelease,
    ) -> PackageInfo {
        // Clean the version tag (strip 'v' prefix if present)
        let version = release.tag_name.trim_start_matches('v').to_string();

        PackageInfo {
            name: name.to_string(),
            version,
            source: "github".to_string(),
            source_uri: format!("github+https://github.com/{}/{}", owner, repo),
            description: release.body.clone().map(|b| {
                // Truncate long release notes
                if b.len() > 200 { format!("{}...", &b[..200]) } else { b }
            }),
            homepage: Some(format!("https://github.com/{}/{}", owner, repo)),
            license: None,
            cmake_targets: infer_cmake_targets(name),
            dependencies: vec![],
            features: vec![],
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    tarball_url: Option<String>,
    zipball_url: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    content_type: String,
}

#[async_trait]
impl Registry for GitHubRegistry {
    fn name(&self) -> &str {
        "github"
    }

    async fn search(&self, query: &str) -> Result<Vec<PackageInfo>> {
        let url = format!(
            "https://api.github.com/search/repositories?q={}+topic:ion-cpp-package&sort=stars",
            query
        );
        let resp = self.client.get(&url).send().await?;

        #[derive(Deserialize)]
        struct SearchResult {
            items: Vec<GHRepo>,
        }
        #[derive(Deserialize)]
        struct GHRepo {
            name: String,
            full_name: String,
            description: Option<String>,
        }

        let result: SearchResult = resp.json().await.unwrap_or(SearchResult { items: vec![] });
        Ok(result.items.into_iter().map(|r| {
            let (_owner, _repo) = r.full_name.split_once('/').unwrap_or(("", &r.full_name));
            PackageInfo {
                name: r.name.clone(),
                version: "latest".to_string(),
                source: "github".to_string(),
                source_uri: format!("github+https://github.com/{}", r.full_name),
                description: r.description,
                homepage: Some(format!("https://github.com/{}", r.full_name)),
                license: None,
                cmake_targets: infer_cmake_targets(&r.name),
                dependencies: vec![],
                features: vec![],
            }
        }).collect())
    }

    async fn resolve(&self, name: &str, version_req: &str) -> Result<PackageInfo> {
        // For GitHub registry, `name` should be "owner/repo" or just "repo"
        // Try to infer owner from common patterns
        let (owner, repo) = if name.contains('/') {
            let mut p = name.splitn(2, '/');
            (p.next().unwrap().to_string(), p.next().unwrap().to_string())
        } else {
            // Try common naming conventions (e.g. fmtlib/fmt for fmt)
            let owner = common_owner_for(name);
            (owner, name.to_string())
        };

        let releases = self.list_releases(&owner, &repo).await?;

        let stable: Vec<_> = releases
            .iter()
            .filter(|r| !r.draft && !r.prerelease)
            .collect();

        if stable.is_empty() {
            anyhow::bail!("No stable releases found for github:{}/{}", owner, repo);
        }

        // Parse version requirement
        let req = if version_req == "*" || version_req.is_empty() {
            semver::VersionReq::STAR
        } else {
            semver::VersionReq::parse(version_req).unwrap_or(semver::VersionReq::STAR)
        };

        let best = stable
            .iter()
            .filter_map(|r| {
                let v_str = r.tag_name.trim_start_matches('v');
                semver::Version::parse(v_str)
                    .ok()
                    .filter(|v| req.matches(v))
                    .map(|v| (v, *r))
            })
            .max_by_key(|(v, _)| v.clone())
            .map(|(_, r)| r);

        // Fall back to first release if no semver match
        let release = best.or_else(|| stable.first().copied()).ok_or_else(|| {
            anyhow::anyhow!("No matching release for github:{}/{} @ {}", owner, repo, version_req)
        })?;

        Ok(self.release_to_package_info(name, &owner, &repo, release))
    }

    async fn download(&self, info: &PackageInfo, cache: &PackageCache) -> Result<DownloadResult> {
        // Extract owner/repo from source_uri
        let repo_url = info.source_uri
            .strip_prefix("github+")
            .unwrap_or(&info.source_uri);
        let (owner, repo) = Self::parse_github_url(repo_url)
            .ok_or_else(|| anyhow::anyhow!("Invalid GitHub source URI: {}", info.source_uri))?;

        let releases = self.list_releases(owner, repo).await?;
        let release = releases
            .iter()
            .find(|r| r.tag_name.trim_start_matches('v') == info.version
                || r.tag_name == info.version)
            .ok_or_else(|| anyhow::anyhow!("Release {} not found for {}", info.version, info.name))?;

        // Prefer tarball over zipball
        let download_url = release.tarball_url.as_deref()
            .or(release.zipball_url.as_deref())
            .ok_or_else(|| anyhow::anyhow!("No download URL for release {}", info.version))?;

        let resp = self.client.get(download_url).send().await
            .with_context(|| format!("Failed to download {}@{}", info.name, info.version))?;
        let bytes = resp.bytes().await?.to_vec();

        if download_url.ends_with(".zip") {
            cache.store_zip(info, &bytes, None)
        } else {
            cache.store(info, &bytes, None)
        }
    }

    fn can_handle(&self, spec: &DependencySpec) -> bool {
        match spec {
            DependencySpec::Git { url, .. } => {
                url.contains("github.com")
            }
            _ => false,
        }
    }
}

/// Infer likely CMake targets from a package name
fn infer_cmake_targets(name: &str) -> Vec<String> {
    // Common well-known packages
    match name.to_lowercase().as_str() {
        "fmt" => vec!["fmt::fmt".to_string(), "fmt::fmt-header-only".to_string()],
        "spdlog" => vec!["spdlog::spdlog".to_string()],
        "catch2" => vec!["Catch2::Catch2".to_string(), "Catch2::Catch2WithMain".to_string()],
        "boost" => vec!["Boost::boost".to_string()],
        "nlohmann_json" | "nlohmann-json" | "json" => {
            vec!["nlohmann_json::nlohmann_json".to_string()]
        }
        "abseil" | "absl" => vec!["absl::base".to_string()],
        "googletest" | "gtest" => vec!["GTest::gtest".to_string(), "GTest::gtest_main".to_string()],
        "eigen" | "eigen3" => vec!["Eigen3::Eigen".to_string()],
        "zlib" => vec!["ZLIB::ZLIB".to_string()],
        "openssl" => vec!["OpenSSL::SSL".to_string(), "OpenSSL::Crypto".to_string()],
        _ => vec![format!("{}::{}", name, name)],
    }
}

/// Known owner mappings for popular packages
fn common_owner_for(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "fmt" => "fmtlib",
        "spdlog" => "gabime",
        "catch2" => "catchorg",
        "nlohmann_json" | "json" => "nlohmann",
        "googletest" | "gtest" => "google",
        "eigen" | "eigen3" => "libeigen",
        "abseil" | "absl" => "abseil",
        "zlib" => "madler",
        "cli11" => "CLIUtils",
        "cxxopts" => "jarro2783",
        "doctest" => "doctest",
        "tinyxml2" => "leethomason",
        "sqlite3" => "sqlite",
        "openssl" => "openssl",
        _ => name,
    }.to_string()
}
