use anyhow::{Context, Result};
use colored::*;
use std::collections::HashSet;
use std::env;

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;

/// ion tree — display the dependency tree
pub fn execute() -> Result<()> {
    let cwd = env::current_dir().context("Failed to get current directory")?;

    let manifest = Manifest::from_dir(&cwd).context("Failed to load ion.toml")?;

    // Try lockfile first (more accurate), fall back to manifest-only
    match Lockfile::load(&cwd)? {
        Some(lock) => print_lock_tree(&manifest, &lock),
        None => print_manifest_tree(&manifest),
    }
}

fn print_lock_tree(manifest: &Manifest, lock: &Lockfile) -> Result<()> {
    println!(
        "{} {} v{}",
        "📦".dimmed(),
        manifest.package.name.cyan().bold(),
        manifest.package.version.dimmed()
    );

    let lock_map = lock.as_map();
    let direct_deps: Vec<&str> = manifest.dependencies.keys().map(|k| k.as_str()).collect();
    let direct_dev_deps: Vec<&str> = manifest
        .dev_dependencies
        .keys()
        .map(|k| k.as_str())
        .collect();

    let mut seen = HashSet::new();

    // Runtime dependencies
    if !direct_deps.is_empty() {
        for (i, name) in direct_deps.iter().enumerate() {
            let is_last = i == direct_deps.len() - 1 && direct_dev_deps.is_empty();
            print_node(name, lock_map.get(name), &lock_map, &mut seen, "", is_last);
        }
    }

    // Dev dependencies (slightly dimmed)
    if !direct_dev_deps.is_empty() {
        println!("│");
        println!("└── {} (dev)", "dev-dependencies".dimmed());
        for (i, name) in direct_dev_deps.iter().enumerate() {
            let is_last = i == direct_dev_deps.len() - 1;
            print_node(
                name,
                lock_map.get(name),
                &lock_map,
                &mut seen,
                "    ",
                is_last,
            );
        }
    }

    println!();
    println!("  {} {} packages total", "→".cyan(), lock.packages.len());

    Ok(())
}

fn print_node(
    name: &str,
    pkg: Option<&&crate::lockfile::LockedPackage>,
    lock_map: &std::collections::HashMap<&str, &crate::lockfile::LockedPackage>,
    seen: &mut HashSet<String>,
    prefix: &str,
    is_last: bool,
) {
    let connector = if is_last { "└──" } else { "├──" };
    let child_prefix = if is_last { "    " } else { "│   " };

    if let Some(pkg) = pkg {
        let source = pkg.source.split('+').next().unwrap_or("ion").to_string();
        let dedup_marker = if seen.contains(&pkg.name) {
            " (*)".dimmed().to_string()
        } else {
            String::new()
        };

        println!(
            "{}{} {} {} {}{}",
            prefix,
            connector.dimmed(),
            pkg.name.cyan(),
            format!("v{}", pkg.version).dimmed(),
            format!("({})", source).dimmed(),
            dedup_marker,
        );

        if !seen.contains(&pkg.name) {
            seen.insert(pkg.name.clone());

            let deps: Vec<String> = pkg
                .dependencies
                .iter()
                .map(|d| d.split_whitespace().next().unwrap_or(d).to_string())
                .collect();

            for (i, dep_name) in deps.iter().enumerate() {
                let dep_is_last = i == deps.len() - 1;
                let next_prefix = format!("{}{}", prefix, child_prefix);
                print_node(
                    dep_name,
                    lock_map.get(dep_name.as_str()),
                    lock_map,
                    seen,
                    &next_prefix,
                    dep_is_last,
                );
            }
        }
    } else {
        println!(
            "{}{} {} {}",
            prefix,
            connector.dimmed(),
            name.cyan(),
            "(not installed)".red().dimmed()
        );
    }
}

fn print_manifest_tree(manifest: &Manifest) -> Result<()> {
    println!(
        "{} {} v{}",
        "📦".dimmed(),
        manifest.package.name.cyan().bold(),
        manifest.package.version.dimmed()
    );

    println!(
        "\n  {} No lockfile found. Run {} to install dependencies.",
        "ℹ".cyan(),
        "ion install".cyan()
    );
    println!();
    println!("  {} (from ion.toml, unresolved):", "Dependencies".bold());

    let all: Vec<_> = manifest.dependencies.keys().collect();
    for (i, name) in all.iter().enumerate() {
        let is_last = i == all.len() - 1;
        let connector = if is_last { "└──" } else { "├──" };
        println!(
            "  {} {} {}",
            connector.dimmed(),
            name.cyan(),
            manifest.dependencies[*name].version_req().dimmed()
        );
    }

    Ok(())
}
