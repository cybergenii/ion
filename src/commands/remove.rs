use anyhow::{Context, Result};
use colored::*;
use std::env;

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::resolver::graph::DependencyGraph;

/// ion remove <package> — remove a dependency from ion.toml and ion.lock
pub async fn execute(name: &str, purge_cache: bool) -> Result<()> {
    let cwd = env::current_dir().context("Failed to get current directory")?;

    let manifest_path = cwd.join("ion.toml");
    if !manifest_path.exists() {
        anyhow::bail!("No ion.toml found in current directory.");
    }

    let mut manifest = Manifest::from_dir(&cwd).context("Failed to load ion.toml")?;

    // Check the dep exists
    if !manifest.has_dependency(name) {
        anyhow::bail!("'{}' is not a dependency in this project.", name.yellow());
    }

    // Load lockfile to check for dependents
    if let Some(lock) = Lockfile::load(&cwd)? {
        let mut graph = DependencyGraph::new();
        for pkg in &lock.packages {
            let deps: Vec<String> = pkg
                .dependencies
                .iter()
                .map(|d| d.split_whitespace().next().unwrap_or(d).to_string())
                .collect();
            graph.add_node(pkg.name.clone(), pkg.version.clone(), deps);
        }

        let affected = graph.check_removal(name);
        if !affected.is_empty() {
            println!(
                "{} The following packages depend on '{}':",
                "⚠".yellow().bold(),
                name.yellow()
            );
            for dep in &affected {
                println!("    - {}", dep.red());
            }
            println!();
            println!(
                "These packages will no longer have their '{}' dependency satisfied.",
                name.yellow()
            );
            println!("If this is intentional, their versions may need to be updated.");
            println!();
        }
    }

    println!(
        "{} '{}' from dependencies...",
        "Removing".red().bold(),
        name.cyan()
    );

    // Remove from manifest
    let was_removed = manifest.remove_dependency(name);
    if !was_removed {
        anyhow::bail!("Failed to remove '{}' from manifest", name);
    }

    manifest
        .save_to_dir(&cwd)
        .context("Failed to save ion.toml")?;
    println!("  {} Updated ion.toml", "✓".green());

    // Update lockfile
    if let Some(mut lock) = Lockfile::load(&cwd)? {
        lock.remove(name);
        let manifest_hash = manifest.hash();
        lock.manifest_hash = manifest_hash;
        lock.save(&cwd).context("Failed to update ion.lock")?;
        println!("  {} Updated ion.lock", "✓".green());
    }

    // Purge from cache if requested
    if purge_cache {
        let config = crate::config::Config::load().unwrap_or_default();
        // Find the package in cache and evict
        let cache = crate::registry::PackageCache::new(&config.cache.directory)?;
        // We don't know the exact version at this point, so list and find
        let cached = cache.list_cached()?;
        for entry in cached {
            if entry.name == name {
                let fake_info = crate::registry::PackageInfo {
                    name: entry.name.clone(),
                    version: entry.version.clone(),
                    source: entry.source.clone(),
                    source_uri: entry.source.clone(),
                    description: None,
                    homepage: None,
                    license: None,
                    cmake_targets: vec![],
                    dependencies: vec![],
                    features: vec![],
                };
                cache.evict(&fake_info)?;
                println!(
                    "  {} Removed {} from cache",
                    "✓".green(),
                    entry.version.dimmed()
                );
            }
        }

        // Also remove the generated cmake config
        let cmake_dir = config.cache.directory.join("cmake");
        let config_file = cmake_dir.join(format!("{}Config.cmake", name));
        let _ = std::fs::remove_file(&config_file);
        let version_file = cmake_dir.join(format!("{}ConfigVersion.cmake", name));
        let _ = std::fs::remove_file(&version_file);
    }

    // Re-run install to fix up CMakeLists.txt with remaining deps
    super::install::execute_in_dir(&cwd).await?;

    println!("\n{} Removed '{}'", "✓".green().bold(), name.cyan());
    Ok(())
}
