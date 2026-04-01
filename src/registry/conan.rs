use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

use super::{DownloadResult, DependencySpec, PackageCache, PackageDependency, PackageInfo, Registry};

/// ConanCenter registry adapter.
/// Resolves C++ packages from https://conan.io/center using the v2 REST API.
/// Supports `conan = "pkg/version@"` dependency specs.
pub struct ConanRegistry {
    client: reqwest::Client,
}

impl ConanRegistry {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!("ion/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("Failed to build HTTP client");
        Self { client }
    }

    /// Parse a Conan package reference: "fmt/10.2.1" or "fmt/10.2.1@"
    fn parse_reference(reference: &str) -> (&str, &str) {
        let reference = reference.trim_end_matches('@');
        if let Some((name, version)) = reference.split_once('/') {
            (name, version)
        } else {
            (reference, "*")
        }
    }
}

/// ConanCenter v2 API package info
#[derive(Debug, Deserialize)]
struct ConanPackageInfo {
    name: String,
    latest: Option<String>,
    description: Option<String>,
    license: Option<String>,
    homepage: Option<String>,
    topics: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ConanVersionList {
    versions: Vec<String>,
}

#[async_trait]
impl Registry for ConanRegistry {
    fn name(&self) -> &str {
        "conan"
    }

    async fn search(&self, query: &str) -> Result<Vec<PackageInfo>> {
        let url = format!(
            "https://conan.io/center/api/ui/v1/packages?search={}",
            query
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(vec![]);
        }

        #[derive(Deserialize)]
        struct SearchResp {
            results: Vec<ConanPackageInfo>,
        }

        let result: SearchResp = resp.json().await.unwrap_or(SearchResp { results: vec![] });
        Ok(result.results.into_iter().map(|p| PackageInfo {
            version: p.latest.clone().unwrap_or_else(|| "*".to_string()),
            cmake_targets: infer_cmake_targets_for_conan(&p.name),
            name: p.name,
            source: "conan".to_string(),
            source_uri: "conan+https://conan.io/center".to_string(),
            description: p.description,
            homepage: p.homepage,
            license: p.license,
            dependencies: vec![],
            features: vec![],
        }).collect())
    }

    async fn resolve(&self, name: &str, version_req: &str) -> Result<PackageInfo> {
        // Fetch package metadata
        let meta_url = format!("https://conan.io/center/api/ui/v1/packages/{}", name);
        let meta_resp = self.client.get(&meta_url).send().await
            .with_context(|| format!("Failed to query ConanCenter for '{}'", name))?;

        if meta_resp.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("Package '{}' not found in ConanCenter", name);
        }

        let pkg_info: ConanPackageInfo = meta_resp.json().await
            .with_context(|| "Failed to parse ConanCenter package info")?;

        // Fetch available versions
        let ver_url = format!("https://conan.io/center/api/ui/v1/packages/{}/versions", name);
        let ver_resp = self.client.get(&ver_url).send().await?;
        let ver_list: ConanVersionList = ver_resp.json().await.unwrap_or(ConanVersionList {
            versions: pkg_info.latest.iter().cloned().collect(),
        });

        // Resolve version requirement
        let req = if version_req == "*" || version_req.is_empty() {
            semver::VersionReq::STAR
        } else {
            semver::VersionReq::parse(version_req).unwrap_or(semver::VersionReq::STAR)
        };

        let best = ver_list
            .versions
            .iter()
            .filter_map(|v| {
                semver::Version::parse(v)
                    .ok()
                    .filter(|sv| req.matches(sv))
                    .map(|sv| (sv, v.clone()))
            })
            .max_by_key(|(v, _)| v.clone())
            .map(|(_, v)| v);

        let version = best
            .or(pkg_info.latest.clone())
            .ok_or_else(|| anyhow::anyhow!("No version of '{}' satisfies '{}'", name, version_req))?;

        Ok(PackageInfo {
            name: name.to_string(),
            version: version.clone(),
            source: "conan".to_string(),
            source_uri: format!("conan+https://conan.io/center#{}:{}", name, version),
            description: pkg_info.description,
            homepage: pkg_info.homepage,
            license: pkg_info.license,
            cmake_targets: infer_cmake_targets_for_conan(name),
            dependencies: vec![],
            features: vec![],
        })
    }

    async fn download(&self, info: &PackageInfo, cache: &PackageCache) -> Result<DownloadResult> {
        // ConanCenter packages come as .tar.gz source archives
        // We download from the ConanCenter sources endpoint
        let url = format!(
            "https://conan.io/center/api/ui/v1/packages/{}/{}/sources",
            info.name, info.version
        );

        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to get download info for {}@{}", info.name, info.version))?;

        if !resp.status().is_success() {
            // Fallback: try GitHub if we know the upstream repo
            anyhow::bail!(
                "ConanCenter download failed for {}@{}. \
                 Try specifying the git source directly instead.",
                info.name, info.version
            );
        }

        #[derive(Deserialize)]
        struct SourceInfo { url: String, checksum: Option<String> }
        let source_info: SourceInfo = resp.json().await
            .with_context(|| "Failed to parse ConanCenter source info")?;

        let archive_resp = self.client.get(&source_info.url).send().await
            .with_context(|| format!("Failed to download archive for {}@{}", info.name, info.version))?;
        let bytes = archive_resp.bytes().await?.to_vec();

        let expected = source_info.checksum.as_deref();
        cache.store(info, &bytes, expected)
    }

    fn can_handle(&self, spec: &DependencySpec) -> bool {
        matches!(spec, DependencySpec::Conan { .. })
    }
}

impl Default for ConanRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn infer_cmake_targets_for_conan(name: &str) -> Vec<String> {
    // Conan packages typically export targets following CMake conventions
    match name.to_lowercase().as_str() {
        "fmt" => vec!["fmt::fmt".to_string()],
        "spdlog" => vec!["spdlog::spdlog".to_string()],
        "catch2" => vec!["Catch2::Catch2".to_string()],
        "boost" => vec!["Boost::boost".to_string()],
        "nlohmann_json" => vec!["nlohmann_json::nlohmann_json".to_string()],
        "openssl" => vec!["OpenSSL::SSL".to_string(), "OpenSSL::Crypto".to_string()],
        "zlib" => vec!["ZLIB::ZLIB".to_string()],
        "gtest" | "googletest" => vec!["GTest::gtest".to_string()],
        "eigen" => vec!["Eigen3::Eigen".to_string()],
        "protobuf" => vec!["protobuf::libprotobuf".to_string()],
        "grpc" => vec!["gRPC::grpc++".to_string()],
        _ => vec![format!("{}::{}", name, name)],
    }
}
