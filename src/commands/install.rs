use anyhow::{Context, Result};
use colored::*;
use std::env;

use crate::cmake::CmakeGenerator;
use crate::config::Config;
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::registry::RegistryManager;
use crate::resolver;

/// ion install — resolve all dependencies, download, and update CMakeLists.txt
pub async fn execute() -> Result<()> {
    let cwd = env::current_dir().context("Failed to get current directory")?;
    execute_in_dir(&cwd).await
}

pub async fn execute_in_dir(project_root: &std::path::Path) -> Result<()> {
    println!("{}", "Installing dependencies...".green().bold());

    let manifest_path = project_root.join("ion.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No ion.toml found. Run {} to initialize a project.",
            "ion init".cyan()
        );
    }

    let manifest = Manifest::from_dir(project_root).context("Failed to load ion.toml")?;
    let config = Config::load().unwrap_or_default();

    if manifest.dependencies.is_empty() && manifest.dev_dependencies.is_empty() {
        println!("{}", "  No dependencies defined in ion.toml.".dimmed());
        return Ok(());
    }

    // Load existing lockfile
    let existing_lock = Lockfile::load(project_root)?;

    // Set up registry manager
    let registry = RegistryManager::with_defaults(&config.cache.directory, config.github_token())?;

    // Resolve dependencies
    let resolved = resolver::resolve(
        &manifest,
        &registry,
        existing_lock.as_ref(),
        false, // don't force update unless explicitly requested
    )
    .await?;

    if resolved.packages.is_empty() {
        println!("{}", "  All dependencies already satisfied.".dimmed());
        return Ok(());
    }

    println!(
        "  {} Resolved {} packages",
        "→".cyan(),
        resolved.packages.len()
    );

    // Download packages that are not yet cached
    let mut locked_packages = Vec::new();
    let ion_cmake_dir = config.cache.directory.join("cmake");

    for node in &resolved.packages {
        let pkg_info = crate::registry::PackageInfo {
            name: node.name.clone(),
            version: node.version.clone(),
            source: detect_source_type(&node.source_uri),
            source_uri: node.source_uri.clone(),
            description: None,
            homepage: None,
            license: None,
            cmake_targets: node.cmake_targets.clone(),
            dependencies: vec![],
            features: vec![],
        };

        // Check cache first
        let download = if let Some(cached) = registry.cache.get_cached(&pkg_info)? {
            println!(
                "  {} {} {} {}",
                "✓".green(),
                pkg_info.name.cyan(),
                pkg_info.version.dimmed(),
                "(cached)".dimmed()
            );
            cached
        } else {
            println!(
                "  {} {} {}",
                "↓".yellow(),
                pkg_info.name.cyan(),
                pkg_info.version.dimmed()
            );
            registry.download(&pkg_info).await.with_context(|| {
                format!("Failed to download {}@{}", pkg_info.name, pkg_info.version)
            })?
        };

        // Generate CMake config file for this package
        let locked = crate::lockfile::LockedPackage {
            name: pkg_info.name.clone(),
            version: pkg_info.version.clone(),
            source: pkg_info.source_uri.clone(),
            checksum: download.checksum.clone(),
            cmake_targets: pkg_info.cmake_targets.clone(),
            dependencies: node.direct_deps.clone(),
            dev_only: false,
            features: vec![],
        };

        crate::cmake::generate_config_file(&locked, &download, &ion_cmake_dir)
            .with_context(|| format!("Failed to generate CMake config for {}", pkg_info.name))?;

        locked_packages.push(locked);
    }

    // Write lockfile
    let manifest_hash = manifest.hash();
    let mut lockfile = Lockfile::new(manifest_hash);
    for pkg in &locked_packages {
        lockfile.upsert(pkg.clone());
    }
    lockfile
        .save(project_root)
        .context("Failed to write ion.lock")?;
    println!("  {} Wrote {}", "✓".green(), "ion.lock".dimmed());

    // Update CMakeLists.txt
    let cmake_gen = CmakeGenerator::new(project_root);
    if project_root.join("CMakeLists.txt").exists() {
        cmake_gen
            .update(&locked_packages, &ion_cmake_dir)
            .context("Failed to update CMakeLists.txt")?;
        println!("  {} Patched {}", "✓".green(), "CMakeLists.txt".dimmed());
    } else {
        println!(
            "  {} No CMakeLists.txt found — skipping CMake integration.",
            "⚠".yellow()
        );
    }

    println!(
        "\n{} {} dependencies installed successfully",
        "✓".green().bold(),
        locked_packages.len()
    );

    Ok(())
}

/// Determine source type from URI prefix
fn detect_source_type(uri: &str) -> String {
    if uri.starts_with("ion+") {
        "ion".to_string()
    } else if uri.starts_with("github+") {
        "github".to_string()
    } else if uri.starts_with("conan+") {
        "conan".to_string()
    } else if uri.starts_with("vcpkg+") {
        "vcpkg".to_string()
    } else if uri.starts_with("git+") {
        "git".to_string()
    } else if uri.starts_with("path+") {
        "local".to_string()
    } else {
        "unknown".to_string()
    }
}
