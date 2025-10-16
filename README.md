# Ion ⚡

**A modern, fast, and user-friendly C++ package manager and linter written in Rust**

[![Build Status](https://github.com/yourusername/ion/workflows/CI/badge.svg)](https://github.com/yourusername/ion/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-green.svg)](https://github.com/yourusername/ion)

## 🎯 Vision

Ion aims to bring the ease of use found in modern package managers (like Cargo, npm, pip) to the C++ ecosystem. No more wrestling with CMake configurations, dependency hell, or manual library management. Ion handles it all.

## ✨ Features

### Available Now (v0.1.0) ✅
- **Project Scaffolding**: Create new C++ projects with best practices built-in
- **Multiple Templates**: Executable, library, or header-only project types
- **Modern Manifest**: Simple TOML configuration (`ion.toml`)
- **CMake Integration**: Automatic CMakeLists.txt generation
- **Cross-Platform**: Works on Linux, macOS, and Windows
- **Beautiful CLI**: Colored output with helpful error messages

### Coming Soon 🚧
- **Package Management**: Install, update, and remove dependencies with a single command
- **Dependency Resolution**: Smart version constraint solving with PubGrub algorithm
- **Package Registry**: Centralized registry for C++ packages
- **Build System**: Fast, incremental builds with caching
- **Code Linting**: Advanced memory safety and code quality checks
- **IDE Integration**: Language Server Protocol (LSP) support
- **Watch Mode**: Real-time code checking as you type

## 🚀 Quick Start

### Installation

```bash
# From source (requires Rust)
git clone https://github.com/yourusername/ion
cd ion
cargo install --path .

# Or use cargo directly
cargo install ion
```

### Create Your First Project

```bash
# Create a new executable project
ion new my-app

# Create a library
ion new my-lib --template library

# Navigate to your project
cd my-app

# Build and run
ion build
ion run
```

### Initialize Existing Project

```bash
# In your existing C++ project directory
ion init

# This creates ion.toml and sets up the project structure
```

## 📖 Usage

### Project Management

```bash
# Create new project
ion new <project-name> [--std 20] [--template executable|library|header-only]

# Initialize existing directory
ion init [--std 20]
```

### Dependency Management (Coming Soon)

```bash
# Add a dependency
ion add fmt

# Add a development dependency
ion add --dev catch2

# Install all dependencies
ion install

# Remove a dependency
ion remove fmt

# Update dependencies
ion update

# Show outdated packages
ion outdated

# Display dependency tree
ion tree
```

### Build & Run

```bash
# Build the project
ion build [--build-type debug|release]

# Build and run
ion run [-- args]

# Run tests
ion test

# Clean build artifacts
ion clean
```

### Code Quality (Coming Soon)

```bash
# Check code for issues
ion check

# Check with auto-fix
ion check --fix

# Watch mode for real-time feedback
ion check --watch
```

## 📝 Project Structure

When you create a new project with Ion, you get:

```
my-project/
├── src/              # Source files (.cpp)
├── include/          # Header files (.h, .hpp)
├── tests/            # Test files
├── docs/             # Documentation
├── ion.toml          # Ion manifest (dependencies, config)
├── CMakeLists.txt    # Generated CMake configuration
├── .gitignore        # Sensible defaults for C++ projects
└── README.md         # Project documentation
```

## ⚙️ Configuration

### ion.toml

The `ion.toml` file is the heart of your Ion project:

```toml
[package]
name = "my-project"
version = "0.1.0"
cpp-standard = "20"
description = "A modern C++ application"
authors = ["Your Name <your.email@example.com>"]
license = "MIT"
repository = "https://github.com/yourusername/my-project"

[dependencies]
fmt = "10.1.1"
spdlog = "^1.12"
boost = { version = "1.83.0", features = ["system", "filesystem"] }

[dev-dependencies]
catch2 = "3.4.0"
benchmark = "1.8.0"

[build]
compiler-flags = ["-Wall", "-Wextra", "-Wpedantic"]
linker-flags = []
features = ["threading"]
```

### Global Configuration

Create `~/.config/ion/config.toml` for global settings:

```toml
[registry]
url = "https://registry.ion-cpp.dev"
mirrors = ["https://mirror1.example.com", "https://mirror2.example.com"]

[cache]
directory = "~/.cache/ion"
max-size-mb = 2048

[build]
parallel-jobs = 8
ccache = true
```

## 🗺️ Roadmap

### Phase 1: Foundation ✅ (Current)
- [x] Project scaffolding (`new`, `init`)
- [x] Manifest parsing (`ion.toml`)
- [x] CMake generation
- [x] CLI interface

### Phase 2: Package Manager 🚧 (Months 3-6)
- [ ] Dependency resolution (PubGrub algorithm)
- [ ] Package registry client
- [ ] Package downloading & caching
- [ ] Package installation & organization
- [ ] Build system integration
- [ ] Cross-platform support

### Phase 3: Linting (Months 7-9)
- [ ] Basic memory leak detection
- [ ] Use-after-free detection
- [ ] Null pointer dereference checking
- [ ] Resource leak detection
- [ ] Modern C++ suggestions

### Phase 4: Advanced Features (Months 10-12)
- [ ] Smart pointer analysis
- [ ] Lifetime analysis
- [ ] LSP integration
- [ ] Auto-fix capabilities
- [ ] Watch mode

### Phase 5: Production (Months 13-18)
- [ ] Public beta
- [ ] Package ecosystem (200+ packages)
- [ ] Enterprise features
- [ ] v1.0 launch

## 🤝 Contributing

We welcome contributions! Here's how you can help:

1. **Report Bugs**: Open an issue with a clear description
2. **Suggest Features**: Share your ideas for improvements
3. **Submit PRs**: Fix bugs or implement features
4. **Write Documentation**: Help improve our docs
5. **Spread the Word**: Star the repo, share with friends

### Development Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/ion
cd ion

# Build the project
cargo build

# Run tests
cargo test

# Run the CLI locally
cargo run -- new test-project

# Run with release optimizations
cargo build --release
./target/release/ion --help
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_manifest_creation

# Run with output
cargo test -- --nocapture
```

## 📊 Comparison with Other Tools

| Feature | Ion | Conan | vcpkg | CMake + FetchContent |
|---------|-----|-------|-------|---------------------|
| **Easy to Use** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| **Speed** | ⚡ Blazing Fast | 🐌 Slow | 🐌 Slow | 🚗 Moderate |
| **Dependency Resolution** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| **Built-in Linter** | ✅ Yes | ❌ No | ❌ No | ❌ No |
| **Modern CLI** | ✅ Yes | ⚠️ Complex | ⚠️ Complex | ❌ No |
| **Cross-Platform** | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |

## 💡 Why Ion?

### Problem: C++ Dependency Management is Painful

```bash
# The old way (manual, error-prone)
1. Search for library documentation
2. Download source code or binaries
3. Configure build system manually
4. Handle platform-specific differences
5. Manage version conflicts manually
6. Update each dependency individually
```

### Solution: Ion Makes it Simple

```bash
# The Ion way (simple, fast)
ion add fmt spdlog boost
ion build
# Done! ✨
```

### Why Choose Ion?

1. **Blazing Fast**: Written in Rust for maximum performance
2. **Modern UX**: Beautiful CLI with helpful error messages
3. **Smart**: Advanced dependency resolution prevents conflicts
4. **Safe**: Built-in linter catches memory bugs early
5. **Complete**: Package manager + build tool + linter in one
6. **Open**: MIT licensed, community-driven

## 🏗️ Architecture

Ion is built with a modular architecture:

```
ion/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── commands/         # Command implementations
│   │   ├── new.rs        # Project creation
│   │   ├── init.rs       # Project initialization
│   │   ├── install.rs    # Package installation (TODO)
│   │   └── ...
│   ├── manifest.rs       # ion.toml parsing
│   ├── config.rs         # Configuration management
│   ├── resolver/         # Dependency resolution (TODO)
│   ├── registry/         # Package registry client (TODO)
│   ├── builder/          # Build system (TODO)
│   └── linter/           # Code analysis (TODO)
└── tests/                # Integration tests
```

## 📜 License

Ion is licensed under the [MIT License](LICENSE).

## 🙏 Acknowledgments

- **Rust Community**: For the amazing tools and ecosystem
- **Cargo**: Inspiration for the user experience
- **Conan & vcpkg**: Pioneering C++ package management
- **clang-tidy**: Inspiration for code analysis

## 📞 Contact & Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/ion/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/ion/discussions)
- **Email**: your.email@example.com
- **Twitter**: [@yourusername](https://twitter.com/yourusername)

---

**Built with ❤️ and Rust** | [Documentation](https://ion-cpp.dev/docs) | [Examples](https://github.com/yourusername/ion-examples) | [Contributing](CONTRIBUTING.md)
