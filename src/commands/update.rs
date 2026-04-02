use anyhow::{Context, Result};
use colored::*;
use std::env;

use crate::config::Config;
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::registry::RegistryManager;
use crate::resolver;

/// ion update [package] — update one or all dependencies to their latest matching version
pub async fn execute(package: Option<&str>) -> Result<()> {
    let cwd = env::current_dir().context("Failed to get current directory")?;

    let manifest_path = cwd.join("ion.toml");
    if !manifest_path.exists() {
        anyhow::bail!("No ion.toml found in current directory.");
    }

    let manifest = Manifest::from_dir(&cwd).context("Failed to load ion.toml")?;
    let config = Config::load().unwrap_or_default();

    if let Some(name) = package {
        println!("{} {}...", "Updating".green().bold(), name.cyan());
        if !manifest.has_dependency(name) {
            anyhow::bail!("'{}' is not a dependency of this project.", name);
        }
    } else {
        println!("{}", "Updating all dependencies...".green().bold());
    }

    let existing_lock = Lockfile::load(&cwd)?;
    let registry = RegistryManager::with_defaults(&config.cache.directory, config.github_token())?;

    // Force re-resolution (ignore lockfile)
    let resolved = resolver::resolve(
        &manifest, &registry, None, // ignore existing lock → force fresh resolution
        true,
    )
    .await?;

    // Compare with existing lock to show what changed
    if let Some(lock) = &existing_lock {
        let old_map = lock.as_map();
        let mut changes = Vec::new();

        for node in &resolved.packages {
            if let Some(pkg) = package {
                if node.name != pkg {
                    continue;
                }
            }
            if let Some(old) = old_map.get(node.name.as_str()) {
                if old.version != node.version {
                    changes.push((node.name.clone(), old.version.clone(), node.version.clone()));
                }
            } else {
                changes.push((node.name.clone(), "".to_string(), node.version.clone()));
            }
        }

        if changes.is_empty() {
            println!("{}", "  Everything is already up to date.".dimmed());
            return Ok(());
        }

        println!("\n  {} packages to update:", changes.len());
        for (name, old, new) in &changes {
            if old.is_empty() {
                println!("    {} {} → {}", name.cyan(), "new".green(), new.green());
            } else {
                println!("    {} {} → {}", name.cyan(), old.red(), new.green());
            }
        }
        println!();
    }

    // Re-run install with force update
    super::install::execute_in_dir(&cwd).await?;

    println!("\n{} Update complete", "✓".green().bold());
    Ok(())
}
