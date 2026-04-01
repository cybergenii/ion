pub mod cache;
pub mod conan;
pub mod git;
pub mod github;
pub mod ion;
pub mod local;
pub mod vcpkg;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub use cache::PackageCache;

/// Information about an available package version from any registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    /// Human-readable source identifier (e.g. "ion", "github", "conan")
    pub source: String,
    /// Canonical source URI stored in the lockfile
    pub source_uri: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    /// CMake import targets this package provides (e.g. ["fmt::fmt"])
    pub cmake_targets: Vec<String>,
    /// Transitive dependencies
    pub dependencies: Vec<PackageDependency>,
    pub features: Vec<String>,
}

/// A dependency declared by a package in the registry index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDependency {
    pub name: String,
    pub version_req: String,
    pub optional: bool,
}

/// Result of downloading + unpacking a package.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    /// SHA-256 hex digest of the downloaded archive
    pub checksum: String,
    /// Path to the extracted package root on disk
    pub extracted_path: PathBuf,
    /// Include directories relative to extracted_path
    pub include_dirs: Vec<PathBuf>,
    /// Library files (static or shared) relative to extracted_path
    pub lib_files: Vec<PathBuf>,
    /// CMake targets this package provides
    pub cmake_targets: Vec<String>,
}

/// The unified interface all registry adapters must implement.
#[async_trait::async_trait]
pub trait Registry: Send + Sync {
    /// Human-readable name of this registry (e.g. "ion", "github")
    fn name(&self) -> &str;

    /// Search for packages matching a query string
    async fn search(&self, query: &str) -> Result<Vec<PackageInfo>>;

    /// Resolve a specific package name + version requirement.
    /// Returns the best matching PackageInfo, or an error.
    async fn resolve(&self, name: &str, version_req: &str) -> Result<PackageInfo>;

    /// Download and extract a specific package version.
    async fn download(&self, info: &PackageInfo, cache: &PackageCache) -> Result<DownloadResult>;

    /// Check if this registry can handle the given dependency spec.
    /// e.g. GitHub registry handles specs with `git = "..."`
    fn can_handle(&self, spec: &DependencySpec) -> bool;
}

/// The parsed form of a dependency as specified in ion.toml.
#[derive(Debug, Clone)]
pub enum DependencySpec {
    /// Ion native registry: `fmt = "10.2"` or `fmt = { version = "10.2" }`
    Ion {
        name: String,
        version_req: String,
        features: Vec<String>,
    },
    /// GitHub: `fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.2.1" }`
    Git {
        name: String,
        url: String,
        rev: GitRev,
    },
    /// ConanCenter: `fmt = { conan = "fmt/10.2.1@" }`
    Conan {
        reference: String,
    },
    /// vcpkg: `fmt = { vcpkg = "fmt" }`
    Vcpkg {
        port: String,
        features: Vec<String>,
    },
    /// Local path: `mylib = { path = "../mylib" }`
    Local {
        name: String,
        path: std::path::PathBuf,
    },
}

impl DependencySpec {
    /// Canonical name of this dependency (used as the key)
    pub fn name(&self) -> &str {
        match self {
            DependencySpec::Ion { name, .. } => name,
            DependencySpec::Git { name, .. } => name,
            DependencySpec::Conan { reference } => {
                reference.split('/').next().unwrap_or(reference)
            }
            DependencySpec::Vcpkg { port, .. } => port,
            DependencySpec::Local { name, .. } => name,
        }
    }
}

/// Git revision specifier
#[derive(Debug, Clone)]
pub enum GitRev {
    Tag(String),
    Branch(String),
    Commit(String),
}

/// The registry manager — tries registries in priority order.
pub struct RegistryManager {
    registries: Vec<Box<dyn Registry>>,
    pub cache: PackageCache,
}

impl RegistryManager {
    pub fn new(cache_dir: &Path) -> Result<Self> {
        let cache = PackageCache::new(cache_dir)?;
        Ok(Self {
            registries: Vec::new(),
            cache,
        })
    }

    /// Add a registry (higher priority = add first)
    pub fn add_registry(&mut self, registry: Box<dyn Registry>) {
        self.registries.push(registry);
    }

    /// Initialize default registries in priority order:
    /// Ion → GitHub → Conan → vcpkg
    pub fn with_defaults(cache_dir: &Path, github_token: Option<String>) -> Result<Self> {
        let mut mgr = Self::new(cache_dir)?;
        mgr.add_registry(Box::new(ion::IonRegistry::new()));
        mgr.add_registry(Box::new(github::GitHubRegistry::new(github_token)));
        mgr.add_registry(Box::new(conan::ConanRegistry::new()));
        mgr.add_registry(Box::new(vcpkg::VcpkgRegistry::new(cache_dir)?));
        mgr.add_registry(Box::new(git::GitRegistry::new(cache_dir)?));
        Ok(mgr)
    }

    /// Resolve a dependency spec using available registries
    pub async fn resolve_dep(&self, spec: &DependencySpec) -> Result<PackageInfo> {
        for registry in &self.registries {
            if registry.can_handle(spec) {
                let name = spec.name();
                let version_req = match spec {
                    DependencySpec::Ion { version_req, .. } => version_req.clone(),
                    _ => "*".to_string(),
                };
                return registry.resolve(name, &version_req).await;
            }
        }
        anyhow::bail!(
            "No registry can handle this dependency: {:?}",
            spec.name()
        )
    }

    /// Download a package, using cache if available
    pub async fn download(&self, info: &PackageInfo) -> Result<DownloadResult> {
        // Check cache first
        if let Some(cached) = self.cache.get_cached(info)? {
            return Ok(cached);
        }

        // Find the registry that owns this source
        for registry in &self.registries {
            if info.source_uri.starts_with(&format!("{}+", registry.name()))
                || info.source == registry.name()
            {
                let result = registry.download(info, &self.cache).await?;
                return Ok(result);
            }
        }

        // Fallback: try GitHub if source looks like a GitHub URL
        if info.source_uri.contains("github.com") {
            for registry in &self.registries {
                if registry.name() == "github" {
                    return registry.download(info, &self.cache).await;
                }
            }
        }

        anyhow::bail!("Cannot find a registry to download: {} {}", info.name, info.version)
    }

    pub fn cache(&self) -> &PackageCache {
        &self.cache
    }
}

// We need async_trait for the Registry trait
// Add it to Cargo.toml if not present
