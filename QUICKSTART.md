# Ion Quick Start Guide

Welcome to Ion! This guide will get you up and running in 5 minutes.

## Installation

### Option 1: Install from Source (Recommended for Development)

```bash
cd /home/cybergenii/Desktop/codes/ion
cargo install --path .
```

This installs the `ion` binary to `~/.cargo/bin/` (make sure it's in your PATH).

### Option 2: Use Directly from Build

```bash
cd /home/cybergenii/Desktop/codes/ion
cargo build --release
# The binary is at: ./target/release/ion
```

Add an alias to your shell config:
```bash
echo 'alias ion="/home/cybergenii/Desktop/codes/ion/target/release/ion"' >> ~/.bashrc
source ~/.bashrc
```

## First Steps

### 1. Verify Installation

```bash
ion --version
# Output: ion 0.1.0

ion --help
# Shows all available commands
```

### 2. Create Your First Project

```bash
# Create a new C++ executable project
ion new my-first-app

# Navigate to the project
cd my-first-app

# Check what was created
ls -la
# Output:
# - src/main.cpp       (your main source file)
# - include/           (header files)
# - tests/             (test files)
# - ion.toml           (project manifest)
# - CMakeLists.txt     (build configuration)
# - README.md          (project documentation)
```

### 3. Explore the Project Structure

```bash
# View the manifest
cat ion.toml

# View the main source file
cat src/main.cpp

# View the CMake configuration
cat CMakeLists.txt
```

### 4. Build Your Project (Coming Soon)

Currently, you can use CMake directly:

```bash
# Create build directory
mkdir build && cd build

# Configure with CMake
cmake ..

# Build
cmake --build .

# Run
./my-first-app
```

Soon, you'll be able to just run:
```bash
ion build
ion run
```

## Project Templates

Ion supports different project templates:

### Executable (Default)
```bash
ion new my-app --template executable
```
Creates a project with a `main.cpp` file.

### Library
```bash
ion new my-lib --template library
```
Creates a library project with headers and implementation files.

### Header-Only Library
```bash
ion new my-header-lib --template header-only
```
Creates a header-only library project.

## C++ Standards

Specify the C++ standard version:

```bash
ion new my-app --std 17    # C++17
ion new my-app --std 20    # C++20 (default)
ion new my-app --std 23    # C++23
```

## Initialize Existing Projects

Already have a C++ project? Initialize it with Ion:

```bash
cd my-existing-project
ion init

# Now you have:
# - ion.toml           (manifest)
# - CMakeLists.txt     (if not present)
# - src/, include/, tests/ directories
```

## What's Next?

### Currently Available (v0.1.0) ✅
- ✅ Project creation (`ion new`)
- ✅ Project initialization (`ion init`)
- ✅ Multiple templates
- ✅ CMake generation

### Coming Soon 🚧

#### Phase 2: Package Manager (Months 3-6)
```bash
# Add dependencies (COMING SOON)
ion add fmt spdlog boost

# Install all dependencies (COMING SOON)
ion install

# Build and run (COMING SOON)
ion build
ion run
```

#### Phase 3: Linter (Months 7-9)
```bash
# Check code for issues (COMING SOON)
ion check

# Auto-fix issues (COMING SOON)
ion check --fix

# Watch mode (COMING SOON)
ion check --watch
```

## Examples

### Create a C++20 Project
```bash
ion new modern-cpp-app --std 20
cd modern-cpp-app
```

### Create a Library
```bash
ion new awesome-lib --template library
cd awesome-lib
```

### Initialize Existing Project
```bash
cd ~/my-old-project
ion init --std 17
```

## Customizing Your Project

### Edit ion.toml

```toml
[package]
name = "my-project"
version = "0.1.0"
cpp-standard = "20"
description = "My awesome C++ project"
authors = ["Your Name <your.email@example.com>"]
license = "MIT"

# Coming soon: Dependencies
[dependencies]
# fmt = "10.1.1"
# spdlog = "^1.12"

# Coming soon: Dev dependencies (for testing)
[dev-dependencies]
# catch2 = "3.4.0"
```

### Customize Build Settings

Edit `CMakeLists.txt` to add custom flags, libraries, or configurations.

## Development Workflow

Current workflow:
```bash
# 1. Create project
ion new my-app

# 2. Navigate to project
cd my-app

# 3. Write code
vim src/main.cpp

# 4. Build with CMake
mkdir build && cd build
cmake ..
cmake --build .

# 5. Run
./my-app
```

Future workflow (coming soon):
```bash
# 1. Create project
ion new my-app && cd my-app

# 2. Add dependencies
ion add fmt spdlog

# 3. Write code
vim src/main.cpp

# 4. Build and run
ion run

# 5. Check code quality
ion check --fix
```

## Tips & Tricks

### Bash/Zsh Completion (Coming Soon)
```bash
# Generate completion script
ion completions bash > ~/.local/share/bash-completion/completions/ion
```

### Use Different Build Types
When using CMake manually:
```bash
# Debug build (default)
cmake -DCMAKE_BUILD_TYPE=Debug ..

# Release build (optimized)
cmake -DCMAKE_BUILD_TYPE=Release ..
```

### Project Naming Conventions
- Use lowercase with hyphens: `my-project`
- Use lowercase with underscores: `my_project`
- Avoid spaces and special characters

## Troubleshooting

### Ion Command Not Found
```bash
# Make sure ~/.cargo/bin is in your PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Build Fails
```bash
# Make sure you have a C++ compiler installed
g++ --version   # GCC
clang++ --version   # Clang
```

### Need Help?
```bash
# Show help for any command
ion --help
ion new --help
ion init --help
```

## Contributing

Want to help build Ion? Check out:
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines
- [README.md](README.md) - Full documentation
- [GitHub Issues](https://github.com/yourusername/ion/issues) - Report bugs or request features

## Learning Resources

### Rust (for contributors)
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)

### C++ (for users)
- [cppreference.com](https://en.cppreference.com/)
- [Modern C++ Guide](https://github.com/AnthonyCalandra/modern-cpp-features)

## Stay Updated

- **GitHub**: Watch the repository for updates
- **Changelog**: See [CHANGELOG.md](CHANGELOG.md) for release notes
- **Roadmap**: See [README.md](README.md#roadmap) for future plans

---

**Ready to build something awesome? Let's go! 🚀**

```bash
ion new my-awesome-project && cd my-awesome-project
```

