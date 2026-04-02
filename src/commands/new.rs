use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::Path;

use crate::manifest::Manifest;

pub fn execute(name: &str, cpp_standard: &str, template: &str) -> Result<()> {
    println!(
        "{} new project `{}`...",
        "Creating".green().bold(),
        name.cyan()
    );

    // Validate project name
    if !is_valid_project_name(name) {
        anyhow::bail!(
            "Invalid project name. Use only alphanumeric characters, hyphens, and underscores."
        );
    }

    // Check if directory already exists
    if Path::new(name).exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    // Create project directory
    fs::create_dir(name).context("Failed to create project directory")?;

    // Create project structure
    create_project_structure(name, cpp_standard, template)?;

    println!("\n{} Created project '{}'", "✓".green().bold(), name.cyan());
    println!("\n{}", "Next steps:".bold());
    println!("  cd {}", name);
    println!("  ion build");
    println!("  ion run");

    Ok(())
}

fn is_valid_project_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        && !name.starts_with('-')
}

fn create_project_structure(name: &str, cpp_standard: &str, template: &str) -> Result<()> {
    let base_path = Path::new(name);

    // Create directories
    fs::create_dir_all(base_path.join("src"))?;
    fs::create_dir_all(base_path.join("include"))?;
    fs::create_dir_all(base_path.join("tests"))?;
    fs::create_dir_all(base_path.join("docs"))?;

    // Create ion.toml manifest
    let manifest = Manifest::new(name, cpp_standard);
    let manifest_content = toml::to_string_pretty(&manifest)?;
    fs::write(base_path.join("ion.toml"), manifest_content)?;

    // Create CMakeLists.txt
    let cmake_content = generate_cmake(name, cpp_standard, template);
    fs::write(base_path.join("CMakeLists.txt"), cmake_content)?;

    // Create source files based on template
    match template {
        "executable" => create_executable_template(base_path, name)?,
        "library" => create_library_template(base_path, name)?,
        "header-only" => create_header_only_template(base_path, name)?,
        _ => anyhow::bail!("Unknown template: {}", template),
    }

    // Create .gitignore
    let gitignore_content = generate_gitignore();
    fs::write(base_path.join(".gitignore"), gitignore_content)?;

    // Create README.md
    let readme_content = generate_readme(name);
    fs::write(base_path.join("README.md"), readme_content)?;

    Ok(())
}

fn generate_cmake(name: &str, cpp_standard: &str, template: &str) -> String {
    let project_type = match template {
        "executable" => "add_executable",
        _ => "add_library",
    };

    format!(
        r#"cmake_minimum_required(VERSION 3.15)
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

# Create {template}
{project_type}({name} ${{SOURCES}})

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
"#,
        name = name,
        cpp_standard = cpp_standard,
        template = template,
        project_type = project_type
    )
}

fn create_executable_template(base_path: &Path, _name: &str) -> Result<()> {
    let main_cpp = r#"#include <iostream>
#include <string>

int main(int argc, char* argv[]) {
    std::cout << "Hello from Ion!" << std::endl;
    
    if (argc > 1) {
        std::cout << "Arguments:" << std::endl;
        for (int i = 1; i < argc; ++i) {
            std::cout << "  " << i << ": " << argv[i] << std::endl;
        }
    }
    
    return 0;
}
"#;

    fs::write(base_path.join("src/main.cpp"), main_cpp)?;

    let test_cpp = r#"#include <cassert>
#include <iostream>

// Simple test example
void test_basic() {
    assert(1 + 1 == 2);
    std::cout << "✓ Basic test passed" << std::endl;
}

int main() {
    test_basic();
    std::cout << "All tests passed!" << std::endl;
    return 0;
}
"#
    .to_string();

    fs::write(base_path.join("tests/test_main.cpp"), test_cpp)?;

    Ok(())
}

fn create_library_template(base_path: &Path, name: &str) -> Result<()> {
    let header = format!(
        r#"#ifndef {guard}_H
#define {guard}_H

#include <string>

namespace {name} {{

class Example {{
public:
    Example();
    ~Example();
    
    std::string greet(const std::string& name);
}};

}} // namespace {name}

#endif // {guard}_H
"#,
        name = name,
        guard = name.to_uppercase()
    );

    fs::write(
        base_path.join("include").join(format!("{}.h", name)),
        header,
    )?;

    let source = format!(
        r#"#include "{name}.h"

namespace {name} {{

Example::Example() {{
}}

Example::~Example() {{
}}

std::string Example::greet(const std::string& name) {{
    return "Hello, " + name + "!";
}}

}} // namespace {name}
"#,
        name = name
    );

    fs::write(base_path.join("src").join(format!("{}.cpp", name)), source)?;

    Ok(())
}

fn create_header_only_template(base_path: &Path, name: &str) -> Result<()> {
    let header = format!(
        r#"#ifndef {guard}_H
#define {guard}_H

#include <string>

namespace {name} {{

template<typename T>
class Container {{
public:
    Container() = default;
    
    void add(const T& item) {{
        items_.push_back(item);
    }}
    
    size_t size() const {{
        return items_.size();
    }}
    
private:
    std::vector<T> items_;
}};

}} // namespace {name}

#endif // {guard}_H
"#,
        name = name,
        guard = name.to_uppercase()
    );

    fs::write(
        base_path.join("include").join(format!("{}.h", name)),
        header,
    )?;

    Ok(())
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
"#
    .to_string()
}

fn generate_readme(name: &str) -> String {
    format!(
        r#"# {name}

A C++ project created with Ion.

## Building

```bash
ion build
```

## Running

```bash
ion run
```

## Testing

```bash
ion test
```

## Adding Dependencies

```bash
ion add <package-name>
```

## Project Structure

```
{name}/
├── src/           # Source files
├── include/       # Header files
├── tests/         # Test files
├── docs/          # Documentation
├── ion.toml       # Ion manifest
└── CMakeLists.txt # CMake configuration
```

## License

MIT
"#,
        name = name
    )
}
