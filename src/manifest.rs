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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Dependency {
    Simple(String),
    Detailed(DetailedDependency),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetailedDependency {
    pub version: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub git: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    #[serde(default)]
    pub compiler_flags: Vec<String>,
    #[serde(default)]
    pub linker_flags: Vec<String>,
    #[serde(default)]
    pub features: Vec<String>,
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
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            build: None,
        }
    }
    
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Manifest = toml::from_str(&content)?;
        Ok(manifest)
    }
    
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
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
}

