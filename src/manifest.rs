use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub dependencies: HashMap<String, Dependency>,
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: HashMap<String, Dependency>,
    #[serde(default)]
    pub build: Option<Build>,
    #[serde(default)]
    pub features: HashMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(rename = "cpp-standard")]
    pub cpp_standard: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Dependency {
    Simple(String),
    Detailed(DetailedDependency),
}

impl Dependency {
    /// Get the version requirement string
    pub fn version_req(&self) -> &str {
        match self {
            Dependency::Simple(v) => v,
            Dependency::Detailed(d) => &d.version,
        }
    }

    /// Get enabled features
    pub fn features(&self) -> &[String] {
        match self {
            Dependency::Simple(_) => &[],
            Dependency::Detailed(d) => &d.features,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DetailedDependency {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub optional: bool,
    /// Git URL for git-based dependencies
    #[serde(default)]
    pub git: Option<String>,
    /// Git tag (for git deps)
    #[serde(default)]
    pub tag: Option<String>,
    /// Git branch (for git deps)
    #[serde(default)]
    pub branch: Option<String>,
    /// Git commit hash (for git deps)
    #[serde(default)]
    pub rev: Option<String>,
    /// ConanCenter reference (e.g. "fmt/10.2.1@")
    #[serde(default)]
    pub conan: Option<String>,
    /// vcpkg port name
    #[serde(default)]
    pub vcpkg: Option<String>,
    /// Local filesystem path
    #[serde(default)]
    pub path: Option<String>,
    /// Override which registry to use (e.g. "ion", "github")
    #[serde(default)]
    pub registry: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    #[serde(default)]
    pub compiler_flags: Vec<String>,
    #[serde(default)]
    pub linker_flags: Vec<String>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub sanitizers: Vec<String>,
}

impl Manifest {
    pub fn new(name: &str, cpp_standard: &str) -> Self {
        Self {
            package: Package {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                cpp_standard: cpp_standard.to_string(),
                description: None,
                authors: vec![],
                license: Some("MIT".to_string()),
                repository: None,
                homepage: None,
                keywords: vec![],
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            build: None,
            features: HashMap::new(),
        }
    }

    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Cannot read {}: {}", path, e))?;
        let manifest: Manifest = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Cannot parse {}: {}", path, e))?;
        Ok(manifest)
    }

    pub fn from_dir(dir: &std::path::Path) -> anyhow::Result<Self> {
        let path = dir.join("ion.toml");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Cannot read {}: {}", path.display(), e))?;
        let manifest: Manifest = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Cannot parse ion.toml: {}", e))?;
        Ok(manifest)
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize ion.toml: {}", e))?;
        std::fs::write(path, content)
            .map_err(|e| anyhow::anyhow!("Failed to write {}: {}", path, e))?;
        Ok(())
    }

    pub fn save_to_dir(&self, dir: &std::path::Path) -> anyhow::Result<()> {
        let path = dir.join("ion.toml");
        let content = toml::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize ion.toml: {}", e))?;
        std::fs::write(&path, content)
            .map_err(|e| anyhow::anyhow!("Failed to write {}: {}", path.display(), e))?;
        Ok(())
    }

    /// Add or update a runtime dependency
    pub fn add_dependency(&mut self, name: &str, version_req: &str) {
        self.dependencies.insert(
            name.to_string(),
            Dependency::Simple(version_req.to_string()),
        );
    }

    /// Add a detailed (multi-field) runtime dependency
    pub fn add_detailed_dependency(&mut self, name: &str, dep: DetailedDependency) {
        self.dependencies
            .insert(name.to_string(), Dependency::Detailed(dep));
    }

    /// Add or update a dev dependency
    pub fn add_dev_dependency(&mut self, name: &str, version_req: &str) {
        self.dev_dependencies.insert(
            name.to_string(),
            Dependency::Simple(version_req.to_string()),
        );
    }

    /// Remove a dependency (from either runtime or dev)
    pub fn remove_dependency(&mut self, name: &str) -> bool {
        let removed_runtime = self.dependencies.remove(name).is_some();
        let removed_dev = self.dev_dependencies.remove(name).is_some();
        removed_runtime || removed_dev
    }

    /// Check if a dependency exists
    pub fn has_dependency(&self, name: &str) -> bool {
        self.dependencies.contains_key(name) || self.dev_dependencies.contains_key(name)
    }

    /// Combined iterator over all dependencies
    pub fn all_dependencies(&self) -> impl Iterator<Item = (&String, &Dependency, bool)> {
        let runtime = self.dependencies.iter().map(|(n, d)| (n, d, false));
        let dev = self.dev_dependencies.iter().map(|(n, d)| (n, d, true));
        runtime.chain(dev)
    }

    /// Compute a stable hash of this manifest for lockfile freshness checks
    pub fn hash(&self) -> String {
        let content = toml::to_string(self).unwrap_or_default();
        crate::lockfile::hash_manifest(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_creation() {
        let manifest = Manifest::new("test-project", "20");
        assert_eq!(manifest.package.name, "test-project");
        assert_eq!(manifest.package.cpp_standard, "20");
        assert_eq!(manifest.package.version, "0.1.0");
    }

    #[test]
    fn test_manifest_serialization() {
        let manifest = Manifest::new("test-project", "17");
        let toml_str = toml::to_string_pretty(&manifest).unwrap();
        assert!(toml_str.contains("test-project"));
        assert!(toml_str.contains("cpp-standard = \"17\""));
    }

    #[test]
    fn test_add_remove_dependency() {
        let mut manifest = Manifest::new("test", "20");
        manifest.add_dependency("fmt", "^10.0");
        assert!(manifest.has_dependency("fmt"));
        assert_eq!(manifest.dependencies["fmt"].version_req(), "^10.0");

        manifest.remove_dependency("fmt");
        assert!(!manifest.has_dependency("fmt"));
    }

    #[test]
    fn test_detailed_dependency() {
        let mut manifest = Manifest::new("test", "20");
        manifest.add_detailed_dependency(
            "fmt",
            DetailedDependency {
                git: Some("https://github.com/fmtlib/fmt".to_string()),
                tag: Some("10.2.1".to_string()),
                ..Default::default()
            },
        );
        let toml_str = toml::to_string_pretty(&manifest).unwrap();
        assert!(toml_str.contains("fmtlib/fmt"));
    }
}
