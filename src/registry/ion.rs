use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{DownloadResult, DependencySpec, PackageCache, PackageDependency, PackageInfo, Registry};

/// Ion native registry client.
/// Communicates with https://registry.ion-cpp.dev (sparse index format).
///
/// Index format: GET /index/{name}.json → array of VersionEntry
pub struct IonRegistry {
    base_url: String,
    client: reqwest::Client,
}

impl IonRegistry {
    pub fn new() -> Self {
        Self {
            base_url: "https://registry.ion-cpp.dev".to_string(),
            client: reqwest::Client::builder()
                .user_agent(concat!("ion/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            base_url: url.into(),
            client: reqwest::Client::builder()
                .user_agent(concat!("ion/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    async fn fetch_index(&self, name: &str) -> Result<Vec<IonVersionEntry>> {
        let url = format!("{}/index/{}.json", self.base_url, name.to_lowercase());
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to reach Ion registry for package '{}'", name))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("Package '{}' not found in Ion registry", name);
        }

        let entries: Vec<IonVersionEntry> = resp
            .json()
            .await
            .with_context(|| format!("Failed to parse Ion registry response for '{}'", name))?;
        Ok(entries)
    }
}

/// A single version entry from the Ion registry index
#[derive(Debug, Deserialize, Serialize)]
struct IonVersionEntry {
    version: String,
    description: Option<String>,
    homepage: Option<String>,
    license: Option<String>,
    /// Download URL for the source tarball
    tarball: String,
    /// SHA-256 checksum of the tarball
    checksum: String,
    /// CMake import targets this version exports
    #[serde(default)]
    cmake_targets: Vec<String>,
    #[serde(default)]
    dependencies: Vec<IonDep>,
    #[serde(default)]
    features: Vec<String>,
    #[serde(default)]
    yanked: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct IonDep {
    name: String,
    version_req: String,
    #[serde(default)]
    optional: bool,
}

#[async_trait]
impl Registry for IonRegistry {
    fn name(&self) -> &str {
        "ion"
    }

    async fn search(&self, query: &str) -> Result<Vec<PackageInfo>> {
        let url = format!("{}/search?q={}", self.base_url, query);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(vec![]);
        }

        #[derive(Deserialize)]
        struct SearchResult {
            packages: Vec<IonSearchEntry>,
        }
        #[derive(Deserialize)]
        struct IonSearchEntry {
            name: String,
            latest_version: String,
            description: Option<String>,
        }

        let result: SearchResult = resp.json().await?;
        Ok(result
            .packages
            .into_iter()
            .map(|e| PackageInfo {
                name: e.name.clone(),
                version: e.latest_version,
                source: "ion".to_string(),
                source_uri: format!("ion+{}", self.base_url),
                description: e.description,
                homepage: None,
                license: None,
                cmake_targets: vec![],
                dependencies: vec![],
                features: vec![],
            })
            .collect())
    }

    async fn resolve(&self, name: &str, version_req: &str) -> Result<PackageInfo> {
        let entries = self.fetch_index(name).await?;

        // Filter out yanked versions
        let available: Vec<_> = entries.iter().filter(|e| !e.yanked).collect();
        if available.is_empty() {
            anyhow::bail!("No available versions for '{}' in Ion registry", name);
        }

        // Parse the version requirement
        let req = if version_req == "*" || version_req.is_empty() {
            semver::VersionReq::STAR
        } else {
            semver::VersionReq::parse(version_req)
                .with_context(|| format!("Invalid version requirement: {}", version_req))?
        };

        // Find the highest matching version
        let best = available
            .iter()
            .filter_map(|e| {
                semver::Version::parse(&e.version)
                    .ok()
                    .filter(|v| req.matches(v))
                    .map(|v| (v, *e))
            })
            .max_by_key(|(v, _)| v.clone())
            .map(|(_, e)| e);

        let entry = best.ok_or_else(|| {
            anyhow::anyhow!(
                "No version of '{}' satisfies '{}' in Ion registry",
                name,
                version_req
            )
        })?;

        Ok(PackageInfo {
            name: name.to_string(),
            version: entry.version.clone(),
            source: "ion".to_string(),
            source_uri: format!("ion+{}#{}", self.base_url, entry.checksum),
            description: entry.description.clone(),
            homepage: entry.homepage.clone(),
            license: entry.license.clone(),
            cmake_targets: entry.cmake_targets.clone(),
            dependencies: entry
                .dependencies
                .iter()
                .map(|d| PackageDependency {
                    name: d.name.clone(),
                    version_req: d.version_req.clone(),
                    optional: d.optional,
                })
                .collect(),
            features: entry.features.clone(),
        })
    }

    async fn download(&self, info: &PackageInfo, cache: &PackageCache) -> Result<DownloadResult> {
        // Extract tarball URL and expected checksum from source_uri
        // source_uri format: "ion+https://registry.ion-cpp.dev#sha256:abc123"
        let (tarball_url, expected_checksum) = {
            // Re-fetch the index to get the tarball URL for this exact version
            let entries = self.fetch_index(&info.name).await?;
            let entry = entries
                .iter()
                .find(|e| e.version == info.version)
                .ok_or_else(|| anyhow::anyhow!("Version {} not found in index", info.version))?;
            (entry.tarball.clone(), entry.checksum.clone())
        };

        let resp = self.client.get(&tarball_url).send().await
            .with_context(|| format!("Failed to download {}@{}", info.name, info.version))?;
        let bytes = resp.bytes().await?.to_vec();

        let checksum_with_prefix = if expected_checksum.starts_with("sha256:") {
            expected_checksum
        } else {
            format!("sha256:{}", expected_checksum)
        };

        cache.store(info, &bytes, Some(&checksum_with_prefix))
    }

    fn can_handle(&self, spec: &DependencySpec) -> bool {
        matches!(spec, DependencySpec::Ion { .. })
    }
}

impl Default for IonRegistry {
    fn default() -> Self {
        Self::new()
    }
}
