use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::path::Path;

use crate::config::Config;
use crate::lockfile::Lockfile;
use crate::manifest::{Dependency, DetailedDependency, Manifest};
use crate::registry::{DependencySpec, GitRev, RegistryManager};
use crate::resolver;

/// ion add <spec> [--dev]
///
/// Spec formats:
///   fmt                          → Ion registry, latest stable
///   fmt@10.2.1                   → Ion registry, exact version
///   fmt@"^10.0"                  → Ion registry, semver range
///   github:fmtlib/fmt@10.2.1     → GitHub releases
///   conan:fmt/10.2.1@            → ConanCenter
///   vcpkg:fmt                    → vcpkg
///   git:https://gitlab.com/...   → Arbitrary git
pub async fn execute(spec: &str, is_dev: bool) -> Result<()> {
    let cwd = env::current_dir().context("Failed to get current directory")?;

    // Find ion.toml
    let manifest_path = cwd.join("ion.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No ion.toml found. Run {} to initialize a project.",
            "ion init".cyan()
        );
    }

    let mut manifest = Manifest::from_dir(&cwd).context("Failed to load ion.toml")?;
    let config = Config::load().unwrap_or_default();

    // Parse the spec
    let (name, dep) = parse_add_spec(spec)?;

    println!(
        "{} {} {} to {}...",
        "Adding".green().bold(),
        name.cyan(),
        if is_dev { "(dev dependency)" } else { "" },
        "ion.toml".dimmed()
    );

    // Check if already present
    if manifest.has_dependency(&name) {
        println!(
            "  {} already exists. Updating version specification.",
            name.yellow()
        );
    }

    // Add to manifest
    if is_dev {
        manifest.dev_dependencies.insert(name.clone(), dep);
    } else {
        manifest.dependencies.insert(name.clone(), dep);
    }

    // Save manifest immediately
    manifest.save_to_dir(&cwd).context("Failed to save ion.toml")?;
    println!("  {} Updated ion.toml", "✓".green());

    // Run install to fetch and update lockfile
    println!("  {} Running install...", "→".cyan());
    super::install::execute_in_dir(&cwd).await?;

    println!("\n{} Added '{}'", "✓".green().bold(), name.cyan());
    Ok(())
}

/// Parse a user-supplied add spec into (name, Dependency)
fn parse_add_spec(spec: &str) -> Result<(String, Dependency)> {
    // Explicit registry prefix: "github:", "conan:", "vcpkg:", "git:"
    if let Some(rest) = spec.strip_prefix("github:") {
        // "github:fmtlib/fmt" or "github:fmtlib/fmt@10.2.1"
        let (repo, version) = split_at_version(rest);
        let name = repo.split('/').last().unwrap_or(repo).to_string();
        return Ok((name, Dependency::Detailed(DetailedDependency {
            git: Some(format!("https://github.com/{}", repo)),
            tag: if version.is_empty() { None } else { Some(ensure_v_prefix(version)) },
            ..Default::default()
        })));
    }

    if let Some(rest) = spec.strip_prefix("conan:") {
        // "conan:fmt/10.2.1"
        let name = rest.split('/').next().unwrap_or(rest).to_string();
        return Ok((name, Dependency::Detailed(DetailedDependency {
            conan: Some(rest.to_string()),
            ..Default::default()
        })));
    }

    if let Some(rest) = spec.strip_prefix("vcpkg:") {
        // "vcpkg:fmt"
        let (port, _) = split_at_version(rest);
        return Ok((port.to_string(), Dependency::Detailed(DetailedDependency {
            vcpkg: Some(port.to_string()),
            ..Default::default()
        })));
    }

    if let Some(rest) = spec.strip_prefix("git:") {
        // "git:https://gitlab.com/org/repo@branch-or-tag"
        let (url, rev) = split_at_version(rest);
        let name = url.split('/').last().unwrap_or(url)
            .trim_end_matches(".git")
            .to_string();
        return Ok((name, Dependency::Detailed(DetailedDependency {
            git: Some(url.to_string()),
            branch: if rev.is_empty() { None } else { Some(rev.to_string()) },
            ..Default::default()
        })));
    }

    // Default: Ion registry
    // "fmt" → latest, "fmt@10.2.1" → exact, "fmt@^10.0" → range
    let (name_part, version_part) = split_at_version(spec);
    let version_req = if version_part.is_empty() {
        "*".to_string()
    } else {
        crate::resolver::semver_utils::normalize_version_req(version_part)
    };

    Ok((name_part.to_string(), Dependency::Simple(version_req)))
}

fn split_at_version(s: &str) -> (&str, &str) {
    if let Some(pos) = s.find('@') {
        (&s[..pos], &s[pos + 1..])
    } else {
        (s, "")
    }
}

fn ensure_v_prefix(version: &str) -> String {
    if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{}", version)
    }
}
