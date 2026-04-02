use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::Config;
use crate::manifest::Manifest;

#[derive(Debug, Clone, PartialEq)]
pub enum BuildType {
    Debug,
    Release,
}

impl BuildType {
    pub fn cmake_value(&self) -> &str {
        match self {
            BuildType::Debug => "Debug",
            BuildType::Release => "Release",
        }
    }

    pub fn dir_name(&self) -> &str {
        match self {
            BuildType::Debug => "debug",
            BuildType::Release => "release",
        }
    }
}

/// ion build [--release]
pub fn execute(build_type: BuildType) -> Result<PathBuf> {
    let cwd = env::current_dir().context("Failed to get current directory")?;
    execute_in_dir(&cwd, build_type)
}

pub fn execute_in_dir(project_root: &Path, build_type: BuildType) -> Result<PathBuf> {
    let manifest_path = project_root.join("ion.toml");
    if !manifest_path.exists() {
        anyhow::bail!("No ion.toml found. Run 'ion init' to initialize a project.");
    }

    let manifest = Manifest::from_dir(project_root).context("Failed to load ion.toml")?;
    let config = Config::load().unwrap_or_default();

    // Detect CMake
    let cmake = find_cmake()?;
    println!(
        "{} {} with CMake...",
        "Building".green().bold(),
        build_type.cmake_value().cyan()
    );

    // Detect C++ compiler
    let compiler = detect_compiler(&config.build.compiler);
    if let Some(c) = &compiler {
        println!("  {} Using compiler: {}", "→".cyan(), c.dimmed());
    }

    // Build directory
    let build_dir = project_root.join("build").join(build_type.dir_name());
    std::fs::create_dir_all(&build_dir).context("Failed to create build directory")?;

    // Configure step
    println!("  {} Configuring...", "→".cyan());
    let mut configure_args = vec![
        "-S".to_string(),
        project_root.to_string_lossy().to_string(),
        "-B".to_string(),
        build_dir.to_string_lossy().to_string(),
        format!("-DCMAKE_BUILD_TYPE={}", build_type.cmake_value()),
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON".to_string(),
    ];

    if let Some(compiler) = &compiler {
        configure_args.push(format!("-DCMAKE_CXX_COMPILER={}", compiler));
    }

    if config.build.ccache {
        configure_args.push("-DCMAKE_CXX_COMPILER_LAUNCHER=ccache".to_string());
    }

    // Add user's extra cmake flags
    configure_args.extend(config.build.cmake_flags.iter().cloned());

    let configure_status = Command::new(&cmake)
        .args(&configure_args)
        .current_dir(project_root)
        .status()
        .with_context(|| format!("Failed to run cmake configure: {}", cmake))?;

    if !configure_status.success() {
        anyhow::bail!("CMake configuration failed. Check the output above for errors.");
    }

    // Build step
    let jobs = config.parallel_jobs();
    println!("  {} Compiling ({} jobs)...", "→".cyan(), jobs);

    let build_status = Command::new(&cmake)
        .args([
            "--build",
            build_dir.to_str().unwrap(),
            "--parallel",
            &jobs.to_string(),
            "--config",
            build_type.cmake_value(),
        ])
        .current_dir(project_root)
        .status()
        .with_context(|| "Failed to run cmake build")?;

    if !build_status.success() {
        anyhow::bail!("Build failed. Check compiler errors above.");
    }

    // Find the output binary
    let binary_path = find_binary(&build_dir, &manifest.package.name);

    println!(
        "\n{} Build successful {}",
        "✓".green().bold(),
        format!("[{}]", build_type.cmake_value()).dimmed()
    );

    if let Some(ref bin) = binary_path {
        println!("  Output: {}", bin.display().to_string().cyan());
    }

    Ok(binary_path.unwrap_or(build_dir))
}

/// Find cmake in PATH
fn find_cmake() -> Result<String> {
    for candidate in ["cmake", "cmake3"] {
        if which_exists(candidate) {
            return Ok(candidate.to_string());
        }
    }

    // Common install locations
    for path in [
        "/usr/bin/cmake",
        "/usr/local/bin/cmake",
        "/opt/homebrew/bin/cmake",
    ] {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    anyhow::bail!(
        "{} CMake not found. Install it with:\n\
         \n\
         Ubuntu/Debian:  sudo apt install cmake\n\
         macOS:          brew install cmake\n\
         Windows:        winget install Kitware.CMake\n\
         \n\
         Or download from https://cmake.org/download/",
        "Error:".red().bold()
    )
}

/// Detect the best available C++ compiler
fn detect_compiler(preferred: &Option<String>) -> Option<String> {
    if let Some(c) = preferred {
        return Some(c.clone());
    }

    // Try in preference order
    for candidate in ["clang++", "g++", "c++"] {
        if which_exists(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn which_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Find the built binary in the build directory
fn find_binary(build_dir: &Path, name: &str) -> Option<PathBuf> {
    // Look for an executable with the project name
    let candidates = [
        build_dir.join(name),
        build_dir.join(format!("{}.exe", name)),
        build_dir.join("Debug").join(name),
        build_dir.join("Release").join(name),
        build_dir.join("Debug").join(format!("{}.exe", name)),
        build_dir.join("Release").join(format!("{}.exe", name)),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }

    // Search recursively
    walkdir::WalkDir::new(build_dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| {
            let p = e.path();
            p.file_stem().and_then(|s| s.to_str()) == Some(name) && p.is_file() && is_executable(p)
        })
        .map(|e| e.into_path())
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("exe"))
        .unwrap_or(true)
}
