use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::process::Command;

use super::build::{execute_in_dir as build_in_dir, BuildType};
use crate::manifest::Manifest;

/// ion run [-- args...] — build then execute the project binary
pub fn execute(args: &[String], release: bool) -> Result<()> {
    let cwd = env::current_dir().context("Failed to get current directory")?;

    let manifest = Manifest::from_dir(&cwd).context("Failed to load ion.toml")?;

    let build_type = if release {
        BuildType::Release
    } else {
        BuildType::Debug
    };

    // Build first
    let binary_path = build_in_dir(&cwd, build_type)?;

    // Find the executable
    let executable = if binary_path.is_file() {
        binary_path
    } else {
        anyhow::bail!(
            "Could not find the built executable for '{}'. \
             Make sure CMakeLists.txt defines an add_executable target with the project name.",
            manifest.package.name
        )
    };

    println!(
        "\n{} {}{}",
        "Running".green().bold(),
        executable
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .cyan(),
        if args.is_empty() {
            String::new()
        } else {
            format!(" {}", args.join(" ")).dimmed().to_string()
        }
    );
    println!("{}", "─".repeat(50).dimmed());

    let mut cmd = Command::new(&executable);
    cmd.args(args);
    cmd.current_dir(&cwd);

    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute {}", executable.display()))?;

    println!("{}", "─".repeat(50).dimmed());

    if status.success() {
        println!("{}", "Process exited successfully.".green().dimmed());
    } else {
        let code = status.code().unwrap_or(-1);
        anyhow::bail!("Process exited with code {}", code);
    }

    Ok(())
}
