use anyhow::{Result, Context};
use colored::*;
use std::fs;
use std::path::Path;

use crate::manifest::Manifest;

pub fn execute(cpp_standard: &str) -> Result<()> {
    println!("{} Ion project in current directory...", "Initializing".green().bold());
    
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let project_name = current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .context("Failed to get directory name")?;
    
    // Check if ion.toml already exists
    if Path::new("ion.toml").exists() {
        anyhow::bail!("ion.toml already exists in this directory");
    }
    
    // Create directories if they don't exist
    create_directories()?;
    
    // Create ion.toml manifest
    let manifest = Manifest::new(project_name, cpp_standard);
    let manifest_content = toml::to_string_pretty(&manifest)?;
    fs::write("ion.toml", manifest_content)?;
    
    // Create CMakeLists.txt if it doesn't exist
    if !Path::new("CMakeLists.txt").exists() {
        let cmake_content = generate_cmake(project_name, cpp_standard);
        fs::write("CMakeLists.txt", cmake_content)?;
    }
    
    // Create .gitignore if it doesn't exist
    if !Path::new(".gitignore").exists() {
        let gitignore_content = generate_gitignore();
        fs::write(".gitignore", gitignore_content)?;
    }
    
    println!("\n{} Initialized Ion project '{}'", "✓".green().bold(), project_name.cyan());
    println!("\n{}", "Next steps:".bold());
    println!("  ion add <package>    # Add dependencies");
    println!("  ion build            # Build the project");
    
    Ok(())
}

fn create_directories() -> Result<()> {
    for dir in &["src", "include", "tests", "docs"] {
        if !Path::new(dir).exists() {
            fs::create_dir(dir).context(format!("Failed to create {} directory", dir))?;
        }
    }
    Ok(())
}

fn generate_cmake(name: &str, cpp_standard: &str) -> String {
    format!(r#"cmake_minimum_required(VERSION 3.15)
project({name} VERSION 0.1.0)

# Set C++ standard
set(CMAKE_CXX_STANDARD {cpp_standard})
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Export compile commands for tools
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Include directories
include_directories(${{PROJECT_SOURCE_DIR}}/include)

# Source files
file(GLOB_RECURSE SOURCES "${{PROJECT_SOURCE_DIR}}/src/*.cpp")

# Create executable (modify as needed)
add_executable({name} ${{SOURCES}})

# Link libraries (managed by Ion)
# Ion will generate this section automatically

# Testing
enable_testing()
file(GLOB_RECURSE TEST_SOURCES "${{PROJECT_SOURCE_DIR}}/tests/*.cpp")
if(TEST_SOURCES)
    add_executable({name}_tests ${{TEST_SOURCES}})
    target_link_libraries({name}_tests {name})
    add_test(NAME {name}_tests COMMAND {name}_tests)
endif()
"#, name = name, cpp_standard = cpp_standard)
}

fn generate_gitignore() -> String {
    r#"# Build artifacts
build/
*.o
*.a
*.so
*.dylib
*.dll
*.exe

# Ion
.ion/
ion.lock

# CMake
CMakeCache.txt
CMakeFiles/
cmake_install.cmake
compile_commands.json

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db
"#.to_string()
}

