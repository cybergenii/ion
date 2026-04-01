use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub registry: RegistryConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub build: BuildConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Default registry: "ion", "github", "conan", "vcpkg"
    #[serde(default = "default_registry")]
    pub default: String,
    /// Ion registry URL
    #[serde(default = "default_registry_url")]
    pub url: String,
    /// Mirror URLs (tried if primary is unavailable)
    #[serde(default)]
    pub mirrors: Vec<String>,
    #[serde(default)]
    pub github: GitHubConfig,
    #[serde(default)]
    pub conan: ConanConfig,
    #[serde(default)]
    pub vcpkg: VcpkgConfig,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            default: default_registry(),
            url: default_registry_url(),
            mirrors: vec![],
            github: GitHubConfig::default(),
            conan: ConanConfig::default(),
            vcpkg: VcpkgConfig::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GitHubConfig {
    /// Optional GitHub personal access token to avoid rate limits.
    /// Also read from GITHUB_TOKEN environment variable.
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConanConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_conan_url")]
    pub url: String,
}

impl Default for ConanConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            url: default_conan_url(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VcpkgConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for VcpkgConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheConfig {
    pub directory: PathBuf,
    #[serde(default = "default_cache_size")]
    pub max_size_mb: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            directory: default_cache_dir(),
            max_size_mb: default_cache_size(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuildConfig {
    /// 0 = use all CPU cores
    #[serde(default)]
    pub parallel_jobs: usize,
    #[serde(default)]
    pub ccache: bool,
    /// Preferred compiler: "clang++", "g++", "cl" — auto-detected if empty
    #[serde(default)]
    pub compiler: Option<String>,
    /// Extra CMake configure flags
    #[serde(default)]
    pub cmake_flags: Vec<String>,
}

fn default_registry() -> String { "ion".to_string() }
fn default_registry_url() -> String { "https://registry.ion-cpp.dev".to_string() }
fn default_conan_url() -> String { "https://conan.io/center".to_string() }
fn default_cache_size() -> u64 { 4096 }
fn default_true() -> bool { true }

fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ion")
}

impl Config {
    /// Load config from ~/.config/ion/config.toml, falling back to defaults.
    pub fn load() -> Result<Self> {
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("ion").join("config.toml");
            if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)
                    .context("Failed to read Ion config file")?;
                let config: Config = toml::from_str(&content)
                    .context("Failed to parse Ion config file")?;
                return Ok(config);
            }
        }
        Ok(Config::default())
    }

    /// Save config to ~/.config/ion/config.toml
    pub fn save(&self) -> Result<()> {
        let config_dir = dirs::config_dir()
            .context("Cannot find config directory")?
            .join("ion");
        std::fs::create_dir_all(&config_dir)
            .context("Failed to create Ion config directory")?;
        let config_path = config_dir.join("config.toml");
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        std::fs::write(&config_path, content)
            .context("Failed to write Ion config file")?;
        Ok(())
    }

    /// Resolve the GitHub token: config file > GITHUB_TOKEN env var
    pub fn github_token(&self) -> Option<String> {
        self.registry.github.token.clone()
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
    }

    /// Resolve the number of parallel build jobs
    pub fn parallel_jobs(&self) -> usize {
        if self.build.parallel_jobs == 0 {
            num_cpus::get()
        } else {
            self.build.parallel_jobs
        }
    }
}
