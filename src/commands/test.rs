use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::process::Command;

use super::build::{execute_in_dir as build_in_dir, BuildType};
use crate::config::Config;

/// ion test — build then run ctest
pub fn execute() -> Result<()> {
    let cwd = env::current_dir().context("Failed to get current directory")?;
    let config = Config::load().unwrap_or_default();

    println!("{}", "Running tests...".green().bold());

    // Build in debug mode
    build_in_dir(&cwd, BuildType::Debug)?;

    let build_dir = cwd.join("build").join("debug");
    let jobs = config.parallel_jobs();

    println!("\n  {} Running ctest...", "→".cyan());

    let status = Command::new("ctest")
        .args([
            "--test-dir",
            build_dir.to_str().unwrap(),
            "--output-on-failure",
            "--parallel",
            &jobs.to_string(),
            "--verbose",
        ])
        .current_dir(&cwd)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("\n{} All tests passed", "✓".green().bold());
            Ok(())
        }
        Ok(s) => {
            anyhow::bail!("Tests failed with exit code {}", s.code().unwrap_or(-1))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!(
                "ctest not found. Install CMake to get ctest:\n\
                 Ubuntu/Debian: sudo apt install cmake\n\
                 macOS:         brew install cmake"
            )
        }
        Err(e) => Err(e).context("Failed to run ctest"),
    }
}
