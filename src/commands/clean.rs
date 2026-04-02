use anyhow::{Context, Result};
use colored::*;
use std::env;

/// ion clean [--all] — remove build artifacts and optionally the Ion cache
pub fn execute(all: bool) -> Result<()> {
    let cwd = env::current_dir().context("Failed to get current directory")?;

    println!("{}", "Cleaning build artifacts...".green().bold());

    // Remove build directory
    let build_dir = cwd.join("build");
    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir).context("Failed to remove build directory")?;
        println!("  {} Removed {}", "✓".green(), "build/".dimmed());
    } else {
        println!("  {} No build directory found.", "→".cyan());
    }

    if all {
        // Remove .ion/ local cache (Ion-managed CMake configs local to project)
        let ion_dir = cwd.join(".ion");
        if ion_dir.exists() {
            std::fs::remove_dir_all(&ion_dir).context("Failed to remove .ion directory")?;
            println!("  {} Removed {}", "✓".green(), ".ion/".dimmed());
        }

        println!(
            "\n  {} To also clear the global package cache, run:",
            "ℹ".cyan()
        );
        println!("     rm -rf ~/.cache/ion/packages");
    }

    println!("\n{} Clean complete", "✓".green().bold());
    Ok(())
}
