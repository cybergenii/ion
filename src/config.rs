use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub registry: RegistryConfig,
    pub cache: CacheConfig,
    pub build: BuildConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub url: String,
    #[serde(default)]
    pub mirrors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheConfig {
    pub directory: PathBuf,
    #[serde(default = "default_cache_size")]
    pub max_size_mb: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_parallel_jobs")]
    pub parallel_jobs: usize,
    #[serde(default)]
    pub ccache: bool,
}

fn default_cache_size() -> u64 {
    1024 // 1GB
}

fn default_parallel_jobs() -> usize {
    num_cpus::get()
}

impl Default for Config {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ion");
            
        Self {
            registry: RegistryConfig {
                url: "https://registry.ion-cpp.dev".to_string(),
                mirrors: vec![],
            },
            cache: CacheConfig {
                directory: cache_dir,
                max_size_mb: default_cache_size(),
            },
            build: BuildConfig {
                parallel_jobs: default_parallel_jobs(),
                ccache: false,
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Try to load from ~/.config/ion/config.toml
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("ion").join("config.toml");
            if config_path.exists() {
                let content = std::fs::read_to_string(config_path)?;
                let config: Config = toml::from_str(&content)?;
                return Ok(config);
            }
        }
        
        // Return default config if not found
        Ok(Config::default())
    }
}

