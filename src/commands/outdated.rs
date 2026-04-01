use anyhow::{Context, Result};
use colored::*;
use comfy_table::{Cell, Color, Table, Attribute};
use std::env;

use crate::config::Config;
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::registry::RegistryManager;

/// ion outdated — show which packages have newer versions available
pub async fn execute() -> Result<()> {
    let cwd = env::current_dir().context("Failed to get current directory")?;

    let lock = Lockfile::load(&cwd)?.ok_or_else(|| {
        anyhow::anyhow!(
            "No ion.lock found. Run {} first.",
            "ion install".cyan()
        )
    })?;

    if lock.packages.is_empty() {
        println!("{}", "No dependencies installed.".dimmed());
        return Ok(());
    }

    println!("{}", "Checking for updates...".green().bold());

    let config = Config::load().unwrap_or_default();
    let registry = RegistryManager::with_defaults(
        &config.cache.directory,
        config.github_token(),
    )?;

    let mut table = Table::new();
    table.set_header(vec![
        Cell::new("Package").add_attribute(Attribute::Bold),
        Cell::new("Current").add_attribute(Attribute::Bold),
        Cell::new("Available").add_attribute(Attribute::Bold),
        Cell::new("Source").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
    ]);

    let mut any_outdated = false;

    for pkg in &lock.packages {
        let spec = crate::registry::DependencySpec::Ion {
            name: pkg.name.clone(),
            version_req: "*".to_string(),
            features: vec![],
        };

        let (latest_version, status_cell) = match registry.resolve_dep(&spec).await {
            Ok(info) => {
                let latest = info.version.clone();
                let is_outdated = is_newer(&latest, &pkg.version);

                if is_outdated {
                    any_outdated = true;
                    (
                        latest.clone(),
                        Cell::new("↑ Update available").fg(Color::Yellow),
                    )
                } else {
                    (latest, Cell::new("✓ Up to date").fg(Color::Green))
                }
            }
            Err(_) => {
                (
                    "?".to_string(),
                    Cell::new("? Unknown").fg(Color::DarkGrey),
                )
            }
        };

        let source_display = pkg.source
            .split('+')
            .next()
            .unwrap_or(&pkg.source)
            .to_string();

        table.add_row(vec![
            Cell::new(&pkg.name).fg(Color::Cyan),
            Cell::new(&pkg.version),
            Cell::new(&latest_version),
            Cell::new(&source_display).fg(Color::DarkGrey),
            status_cell,
        ]);
    }

    println!("\n{}", table);

    if any_outdated {
        println!("\nRun {} to update all packages.", "ion update".cyan());
    } else {
        println!("\n{} All packages are up to date.", "✓".green());
    }

    Ok(())
}

/// Returns true if `latest` is a newer semver than `current`
fn is_newer(latest: &str, current: &str) -> bool {
    match (
        semver::Version::parse(latest.trim_start_matches('v')),
        semver::Version::parse(current.trim_start_matches('v')),
    ) {
        (Ok(l), Ok(c)) => l > c,
        _ => false,
    }
}
