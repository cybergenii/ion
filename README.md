# Ion ⚡

**A modern, fast, and user-friendly C++ package manager and linter written in Rust**

<!-- [![Build Status](https://github.com/cybergenii/ion/workflows/CI/badge.svg)](https://github.com/cybergenii/ion/actions) -->
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.3.0-green.svg)](https://github.com/cybergenii/ion)

## 🎯 Vision

Ion aims to bring the ease of use found in modern package managers (like Cargo, npm, pip) to the C++ ecosystem. No more wrestling with CMake configurations, dependency hell, or manual library management. With Ion, you can create a full C++ project, import dependencies, build/run/test it, and lint it from one CLI.

## ✨ Features

### Available Now (v0.3.0) ✅
- **End-to-End Workflow**: Scaffold, depend, build, run, test, clean, and lint a full C++ project from one tool
- **Project Scaffolding**: Create new C++ projects with best practices built-in
- **Multiple Templates**: Executable, library, or header-only project types
- **Modern Manifest**: Simple TOML configuration (`ion.toml`)
- **CMake Integration**: Automatic CMakeLists.txt generation
- **Dependency Management**: Add, install, remove, update, tree, and outdated commands
- **Multi-Source Registry Support**: Ion, GitHub, Conan, vcpkg, git, and local path dependencies
- **Lockfile + Cache**: Deterministic `ion.lock` resolution and cached package extraction
- **Build Pipeline Commands**: `ion build`, `ion run`, `ion test`, `ion clean`
- **Linting Commands**: `ion check`, `ion check --fix`, `ion check --watch`, `ion check --list-rules`, smart-pointer heuristics (`memory/smart-get`, `memory/raw-from-smart`, …)
- **Cross-Platform**: Works on Linux, macOS, and Windows
- **Beautiful CLI**: Colored output with helpful error messages

### In Progress / Coming Soon 🚧
- **Quality Focus**: Existing workflow is complete; current work is on deeper analysis precision and UX polish
- **Lint Rule Precision**: Reduce false positives with deeper semantic analysis
- **Advanced Dataflow**: Better path-sensitive leak/use-after-free reasoning
- **LSP Maturity**: Richer hover docs and stronger quick-fix workflows
- **Rule Coverage**: Expand modern C++ and resource-safety checks

## 🚀 Quick Start

### Installation

```bash
# From source (requires Rust)
git clone https://github.com/cybergenii/ion
cd ion
cargo install --path .

# One-line installer (latest release)
curl -fsSL https://ion.cybergenii.com/install.sh | sh

# Optional installer overrides
# ION_INSTALL_DIR="$HOME/.local/bin" ION_VERSION="0.3.0" ION_NO_MODIFY_PATH=1 \
#   curl -fsSL https://ion.cybergenii.com/install.sh | sh
#
# Dependency bootstrap controls (default installs missing deps):
# ION_INSTALL_DEPS=1   # default, auto-install missing cmake/compiler/make+ninja/libclang
# ION_INSTALL_DEPS=0   # do not auto-install dependencies
# ION_SKIP_DEPS=1      # skip dependency checks entirely

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

### Dependency Management

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

### Code Quality

```bash
# Check code for issues
ion check

# List available lint rule IDs
ion check --list-rules

# Run only selected rules (comma-separated)
ion check --rule modern/nullptr,memory/leak

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
repository = "https://github.com/cybergenii/my-project"

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

### Phase 1: Foundation ✅
- [x] Project scaffolding (`new`, `init`)
- [x] Manifest parsing (`ion.toml`)
- [x] CMake generation
- [x] CLI interface

### Phase 2: Package Manager ✅
- [x] Lockfile generation and freshness checks
- [x] Multi-source registry adapters
- [x] Package download/caching and extraction
- [x] Dependency lifecycle commands (`add/remove/install/update`)
- [x] Build/run/test/clean command integration
- [x] CMake dependency wiring support

### Phase 3: Linting 🚧 (In Progress)
- [x] `ion check` command
- [x] `ion check --fix`
- [x] `ion check --watch`
- [x] `ion check --list-rules`
- [x] Rule filtering via `--rule <id[,id...]>`
- [x] Text/JSON/SARIF reporting
- [x] Semantic context for AST rules (enclosing function, full-file source for cross-checks)
- [x] Tuned heuristics (double-free gating, nullptr/resource filters)
- [x] Function-scoped textual dataflow for `memory/leak` and `memory/use-after-free` (assignment clears freed state)
- [ ] Full path-sensitive / interprocedural analysis (CFG-backed, cross-function summaries)

### Phase 4: Advanced Features 🚧 (In Progress)
- [x] Initial LSP server plumbing (`ion lsp`)
- [x] Diagnostic conversion pipeline for editor integration
- [x] Smart-pointer heuristics (`memory/smart-get`, `memory/raw-from-smart`, `memory/move-after-use`, `memory/shared-cycle-hint`)
- [x] Expanded auto-fix (`modern/c-cast` → `static_cast` when safe)
- [x] Editor UX: full sync, unsaved buffers, diagnostic `code`, range-filtered code actions
- [ ] Symbol-aware navigation (go-to-def, etc.)

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
git clone https://github.com/cybergenii/ion
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
│   │   ├── install.rs    # Package installation
│   │   └── ...
│   ├── manifest.rs       # ion.toml parsing
│   ├── config.rs         # Configuration management
│   ├── resolver/         # Dependency resolution
│   ├── registry/         # Registry adapters and package retrieval
│   ├── cmake/            # CMake integration/generation
│   ├── linter/           # Static analysis + reporting/fixes
│   ├── analysis/         # CFG/dataflow analysis
│   └── lsp/              # Language Server Protocol integration
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

- **Issues**: [GitHub Issues](https://github.com/cybergenii/ion/issues)
- **Discussions**: [GitHub Discussions](https://github.com/cybergenii/ion/discussions)
- **Email**: cybersgenii@gmail.com
- **Twitter**: [@cybergenii](https://twitter.com/cyber_genii)

---

**Built with ❤️ and Rust** | [Documentation](https://ion.cybergenii.com/docs) | [Examples](https://github.com/cybergenii/ion-examples) | [Contributing](CONTRIBUTING.md)
