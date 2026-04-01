use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use super::{DownloadResult, DependencySpec, PackageCache, PackageInfo, Registry};

/// vcpkg port index adapter.
/// Resolves packages from the vcpkg public registry using vcpkg's Git-based port index.
/// Supports `vcpkg = "pkg"` dependency specs.
pub struct VcpkgRegistry {
    /// Local cache directory where the vcpkg port index is cloned
    index_dir: PathBuf,
    client: reqwest::Client,
}

impl VcpkgRegistry {
    pub fn new(cache_dir: &Path) -> Result<Self> {
        let index_dir = cache_dir.join("vcpkg-index");
        let client = reqwest::Client::builder()
            .user_agent(concat!("ion/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { index_dir, client })
    }

    /// Ensure the vcpkg baseline registry is available locally.
    /// Uses the public vcpkg registry baseline:
    /// https://raw.githubusercontent.com/microsoft/vcpkg/master/versions/baseline.json
    fn ensure_index(&self) -> Result<()> {
        std::fs::create_dir_all(&self.index_dir)
            .context("Failed to create vcpkg index directory")?;
        Ok(())
    }

    async fn fetch_baseline(&self) -> Result<VcpkgBaseline> {
        let url = "https://raw.githubusercontent.com/microsoft/vcpkg/master/versions/baseline.json";
        let cache_path = self.index_dir.join("baseline.json");

        // Use cached version if recent (< 24 hours old)
        if cache_path.exists() {
            if let Ok(meta) = std::fs::metadata(&cache_path) {
                if let Ok(modified) = meta.modified() {
                    if modified.elapsed().map(|d| d.as_secs() < 86400).unwrap_or(false) {
                        let content = std::fs::read_to_string(&cache_path)?;
                        if let Ok(baseline) = serde_json::from_str(&content) {
                            return Ok(baseline);
                        }
                    }
                }
            }
        }

        // Fetch from GitHub
        let resp = self.client.get(url).send().await
            .context("Failed to fetch vcpkg baseline")?;
        let text = resp.text().await?;
        std::fs::write(&cache_path, &text)?;
        let baseline: VcpkgBaseline = serde_json::from_str(&text)
            .context("Failed to parse vcpkg baseline")?;
        Ok(baseline)
    }

    async fn fetch_port_versions(&self, name: &str) -> Result<Vec<String>> {
        // vcpkg versions are stored per-port at:
        // /versions/{first_char}-/{name}.json
        let first_char = name.chars().next().unwrap_or('a').to_lowercase().next().unwrap();
        let url = format!(
            "https://raw.githubusercontent.com/microsoft/vcpkg/master/versions/{}-/{}.json",
            first_char, name
        );
        let cache_path = self.index_dir.join(format!("{}.json", name));

        let text = if cache_path.exists() {
            std::fs::read_to_string(&cache_path)?
        } else {
            let resp = self.client.get(&url).send().await?;
            if !resp.status().is_success() {
                return Ok(vec![]);
            }
            let t = resp.text().await?;
            std::fs::write(&cache_path, &t)?;
            t
        };

        #[derive(Deserialize)]
        struct VcpkgVersions {
            versions: Vec<VcpkgVersionEntry>,
        }
        #[derive(Deserialize)]
        struct VcpkgVersionEntry {
            version: Option<String>,
            #[serde(rename = "version-semver")]
            version_semver: Option<String>,
            #[serde(rename = "version-string")]
            version_string: Option<String>,
        }

        let data: VcpkgVersions = serde_json::from_str(&text).unwrap_or(VcpkgVersions { versions: vec![] });
        Ok(data.versions.into_iter().filter_map(|v| {
            v.version_semver.or(v.version).or(v.version_string)
        }).collect())
    }
}

#[derive(Debug, Deserialize)]
struct VcpkgBaseline {
    default: std::collections::HashMap<String, VcpkgBaselineEntry>,
}

#[derive(Debug, Deserialize)]
struct VcpkgBaselineEntry {
    #[serde(rename = "baseline")]
    version: String,
    #[serde(rename = "port-version")]
    port_version: u32,
}

#[async_trait]
impl Registry for VcpkgRegistry {
    fn name(&self) -> &str {
        "vcpkg"
    }

    async fn search(&self, query: &str) -> Result<Vec<PackageInfo>> {
        self.ensure_index()?;
        let baseline = self.fetch_baseline().await?;

        let results: Vec<PackageInfo> = baseline.default
            .iter()
            .filter(|(name, _)| name.contains(query))
            .map(|(name, entry)| PackageInfo {
                name: name.clone(),
                version: entry.version.clone(),
                source: "vcpkg".to_string(),
                source_uri: format!("vcpkg+https://github.com/microsoft/vcpkg#{}", name),
                description: None,
                homepage: Some(format!("https://vcpkg.io/en/package/{}", name)),
                license: None,
                cmake_targets: infer_cmake_targets_vcpkg(name),
                dependencies: vec![],
                features: vec![],
            })
            .take(20)
            .collect();
        Ok(results)
    }

    async fn resolve(&self, name: &str, _version_req: &str) -> Result<PackageInfo> {
        self.ensure_index()?;
        let baseline = self.fetch_baseline().await.context("Failed to load vcpkg baseline")?;

        let entry = baseline.default.get(name).ok_or_else(|| {
            anyhow::anyhow!("Package '{}' not found in vcpkg registry", name)
        })?;

        Ok(PackageInfo {
            name: name.to_string(),
            version: entry.version.clone(),
            source: "vcpkg".to_string(),
            source_uri: format!("vcpkg+https://github.com/microsoft/vcpkg#{}", name),
            description: None,
            homepage: Some(format!("https://vcpkg.io/en/package/{}", name)),
            license: None,
            cmake_targets: infer_cmake_targets_vcpkg(name),
            dependencies: vec![],
            features: vec![],
        })
    }

    async fn download(&self, info: &PackageInfo, cache: &PackageCache) -> Result<DownloadResult> {
        // vcpkg packages are source ports, not pre-built binaries.
        // We use vcpkg's source archive mechanism:
        // The portfile specifies the upstream source tarball.
        // We fetch the portfile to get the archive URL, then download it.

        let first_char = info.name.chars().next().unwrap_or('a').to_lowercase().next().unwrap();
        let portfile_url = format!(
            "https://raw.githubusercontent.com/microsoft/vcpkg/master/ports/{}/portfile.cmake",
            info.name
        );

        let portfile_resp = self.client.get(&portfile_url).send().await
            .with_context(|| format!("Failed to fetch portfile for {}", info.name))?;

        if !portfile_resp.status().is_success() {
            anyhow::bail!("vcpkg port '{}' portfile not found", info.name);
        }

        let portfile = portfile_resp.text().await?;

        // Extract the URL and SHA512 from the portfile
        let archive_url = extract_cmake_arg(&portfile, "URL")
            .ok_or_else(|| anyhow::anyhow!("Could not find archive URL in vcpkg portfile for {}", info.name))?;

        let resp = self.client.get(&archive_url).send().await
            .with_context(|| format!("Failed to download vcpkg package {}", info.name))?;
        let bytes = resp.bytes().await?.to_vec();

        if archive_url.ends_with(".zip") {
            cache.store_zip(info, &bytes, None)
        } else {
            cache.store(info, &bytes, None)
        }
    }

    fn can_handle(&self, spec: &DependencySpec) -> bool {
        matches!(spec, DependencySpec::Vcpkg { .. })
    }
}

/// Extract a CMake function argument value from portfile content
fn extract_cmake_arg(content: &str, arg: &str) -> Option<String> {
    let pattern = format!("{} ", arg);
    content.lines()
        .find(|l| l.trim().starts_with(&pattern))?
        .trim()
        .strip_prefix(&pattern)?
        .trim()
        .trim_matches('"')
        .to_string()
        .into()
}

fn infer_cmake_targets_vcpkg(name: &str) -> Vec<String> {
    // vcpkg packages expose CMake targets following standard conventions
    match name.to_lowercase().as_str() {
        "fmt" => vec!["fmt::fmt".to_string()],
        "spdlog" => vec!["spdlog::spdlog".to_string()],
        "boost" => vec!["Boost::boost".to_string()],
        "nlohmann-json" => vec!["nlohmann_json::nlohmann_json".to_string()],
        "catch2" => vec!["Catch2::Catch2".to_string()],
        "gtest" => vec!["GTest::gtest".to_string(), "GTest::gtest_main".to_string()],
        "eigen3" => vec!["Eigen3::Eigen".to_string()],
        "zlib" => vec!["ZLIB::ZLIB".to_string()],
        "openssl" => vec!["OpenSSL::SSL".to_string(), "OpenSSL::Crypto".to_string()],
        "sqlite3" => vec!["SQLite::SQLite3".to_string()],
        "curl" => vec!["CURL::libcurl".to_string()],
        "protobuf" => vec!["protobuf::libprotobuf".to_string()],
        "grpc" => vec!["gRPC::grpc++".to_string()],
        _ => vec![format!("{}::{}", name, name)],
    }
}
