use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

use super::{DownloadResult, DependencySpec, PackageCache, PackageInfo, Registry};

/// Local filesystem adapter.
/// Resolves packages from a local path (e.g. for monorepo development or path overrides).
/// Supports `path = "../mylib"` dependency specs.
pub struct LocalRegistry;

impl LocalRegistry {
    pub fn new() -> Self {
        Self
    }

    /// Infer package metadata from a local directory's ion.toml
    fn read_local_info(path: &Path, name: &str) -> Result<PackageInfo> {
        let manifest_path = path.join("ion.toml");

        if manifest_path.exists() {
            let content = std::fs::read_to_string(&manifest_path)
                .with_context(|| format!("Failed to read ion.toml at {}", path.display()))?;
            let manifest: crate::manifest::Manifest = toml::from_str(&content)
                .with_context(|| "Failed to parse local ion.toml")?;

            Ok(PackageInfo {
                name: manifest.package.name.clone(),
                version: manifest.package.version.clone(),
                source: "local".to_string(),
                source_uri: format!("path+{}", path.canonicalize()?.display()),
                description: manifest.package.description.clone(),
                homepage: manifest.package.repository.clone(),
                license: manifest.package.license.clone(),
                cmake_targets: vec![format!("{}::{}", manifest.package.name, manifest.package.name)],
                dependencies: manifest
                    .dependencies
                    .iter()
                    .map(|(dep_name, dep)| {
                        let version_req = match dep {
                            crate::manifest::Dependency::Simple(v) => v.clone(),
                            crate::manifest::Dependency::Detailed(d) => d.version.clone(),
                        };
                        super::PackageDependency {
                            name: dep_name.clone(),
                            version_req,
                            optional: false,
                        }
                    })
                    .collect(),
                features: vec![],
            })
        } else {
            // Minimal info for a directory without ion.toml
            Ok(PackageInfo {
                name: name.to_string(),
                version: "0.0.0".to_string(),
                source: "local".to_string(),
                source_uri: format!("path+{}", path.canonicalize()?.display()),
                description: None,
                homepage: None,
                license: None,
                cmake_targets: vec![format!("{}::{}", name, name)],
                dependencies: vec![],
                features: vec![],
            })
        }
    }
}

#[async_trait]
impl Registry for LocalRegistry {
    fn name(&self) -> &str {
        "local"
    }

    async fn search(&self, _query: &str) -> Result<Vec<PackageInfo>> {
        // Local packages are not searchable
        Ok(vec![])
    }

    async fn resolve(&self, name: &str, _version_req: &str) -> Result<PackageInfo> {
        anyhow::bail!(
            "Local registry requires a path spec. Use: {{ path = \"../{}\" }}",
            name
        )
    }

    async fn download(&self, info: &PackageInfo, cache: &PackageCache) -> Result<DownloadResult> {
        // For local packages, we create a symlink / copy into the cache
        let local_path = info
            .source_uri
            .strip_prefix("path+")
            .unwrap_or(&info.source_uri);
        let src_path = PathBuf::from(local_path);

        if !src_path.exists() {
            anyhow::bail!(
                "Local dependency path does not exist: {}",
                src_path.display()
            );
        }

        let pkg_dir = cache.package_dir(info);
        let dest = pkg_dir.join("src");
        std::fs::create_dir_all(&dest)?;

        // Copy the local package to cache (so the cache path is consistent)
        copy_dir_all(&src_path, &dest)?;

        let meta = super::cache::CachedMeta {
            name: info.name.clone(),
            version: info.version.clone(),
            source: info.source_uri.clone(),
            // Local packages don't have a meaningful checksum - use mtime hash
            checksum: format!("local:{}", local_path),
        };
        std::fs::write(
            pkg_dir.join("meta.json"),
            serde_json::to_string_pretty(&meta)?,
        )?;

        let include_dirs = find_include_dirs(&dest);
        let lib_files = find_lib_files(&dest);

        Ok(DownloadResult {
            checksum: meta.checksum,
            extracted_path: dest,
            include_dirs,
            lib_files,
            cmake_targets: info.cmake_targets.clone(),
        })
    }

    fn can_handle(&self, spec: &DependencySpec) -> bool {
        matches!(spec, DependencySpec::Local { .. })
    }
}

impl Default for LocalRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively copy a directory
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in walkdir::WalkDir::new(src)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let rel = entry.path().strip_prefix(src)?;
        let dest_path = dst.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

fn find_include_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for candidate in ["include", "inc"] {
        let p = root.join(candidate);
        if p.is_dir() {
            dirs.push(p);
        }
    }
    if dirs.is_empty() {
        dirs.push(root.to_path_buf());
    }
    dirs
}

fn find_lib_files(root: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(root)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|ext| matches!(ext, "a" | "lib" | "so" | "dylib" | "dll"))
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
        .collect()
}
