# Ion

**C++ package manager, build orchestration, and linter — implemented in Rust.**

[![CI](https://github.com/cybergenii/ion/actions/workflows/ci.yml/badge.svg)](https://github.com/cybergenii/ion/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.3.0-green.svg)](https://github.com/cybergenii/ion)

## Overview

Ion is one CLI for both **packaging** and **quality**: you describe the project in `ion.toml`, run `ion install` to resolve dependencies (Ion registry, GitHub, ConanCenter, vcpkg, git, or paths), then `ion build`, `ion run`, and `ion test`. Use `ion check` for static analysis and `ion lsp` to drive diagnostics, fixes, and go-to-definition in an editor.

- **Always available without libclang:** tree-sitter–based checks, textual dataflow, and several heuristics.
- **With libclang installed:** additional semantic rules and richer LSP behavior (including go-to-definition).

Day-to-day commands most users run: `ion add`, `ion install`, `ion build`, `ion check`.

## Prerequisites

| What you are doing | Requirements |
|--------------------|--------------|
| **Using Ion** to build a C++ project | A C++ compiler and **CMake** on your PATH. The [installer](#installation) can bootstrap common tools on supported platforms via `ION_INSTALL_DEPS` (see comments in that section). |
| **Full lint + LSP semantics** | **libclang** (system package or LLVM install) so `ion check` and `ion lsp` can use the same AST pipeline as Conan/vcpkg-style C++ code. |
| **Hacking on Ion itself** | **Rust** (stable) and **Cargo**; see [Contributing](CONTRIBUTING.md). |

## Vision

Ion brings a Cargo-style workflow to C++: one manifest, one lockfile, one CLI for scaffolding, dependencies, builds, tests, and static analysis. It targets teams that want less CMake and scripting friction without giving up interoperability with existing ecosystems.

## Features

### Shipped in v0.3.x
- **End-to-end workflow**: Scaffold, resolve dependencies, build, run, test, clean, and lint from a single CLI
- **Project Scaffolding**: Executable, library, or header-only templates with sensible defaults
- **Manifest**: `ion.toml` (TOML) for package metadata and dependencies
- **CMake**: Generated `CMakeLists.txt` wired to resolved dependencies
- **Dependencies**: `add`, `install`, `remove`, `update`, `tree`, `outdated`
- **Multi-registry resolution**: Ion registry, GitHub releases, **ConanCenter**, **vcpkg** ports, git URLs, and local paths (see [Dependency sources](#dependency-sources))
- **Lockfile and cache**: `ion.lock` for reproducible resolution; cached downloads and extraction
- **Build**: `ion build`, `ion run`, `ion test`, `ion clean`
- **Linting**: `ion check` with `--fix`, `--watch`, `--list-rules`, `--rule`, JSON/SARIF output; semantic rules when libclang is available; `ion lsp` for editor integration (diagnostics, code actions, go-to-definition where supported)
- **Cross-platform**: Linux, macOS, and Windows

### Roadmap
- Deeper semantic analysis and fewer false positives in lint rules
- Stronger path-sensitive dataflow (leaks, use-after-free)
- LSP: richer hover and quick-fix coverage
- Broader rule coverage for modern C++ and resource safety

## Quick Start

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

## Usage

### Project management

```bash
ion new <project-name> [--std 20] [--template executable|library|header-only]
ion init [--std 20]
```

### Dependency management

Core commands:

```bash
ion add <spec>              # Add a dependency (see formats below)
ion add --dev <spec>        # Dev dependency
ion install                 # Download and resolve per ion.toml / ion.lock
ion remove <name>
ion update
ion outdated
ion tree
```

#### Dependency sources

Ion resolves packages from several backends. Use a **prefix** on `ion add`, or set the equivalent fields in `ion.toml`.

| Source | `ion add` example | `ion.toml` (inline table) |
|--------|-------------------|---------------------------|
| **Ion registry** | `ion add fmt` or `ion add fmt@10.2.1` | `fmt = "10.2.1"` |
| **GitHub** | `ion add github:fmtlib/fmt@10.2.1` | `fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.2.1" }` |
| **ConanCenter** | `ion add conan:fmt/10.2.1@` | `fmt = { conan = "fmt/10.2.1@" }` |
| **vcpkg** | `ion add vcpkg:fmt` | `fmt = { vcpkg = "fmt" }` |
| **Git** | `ion add git:https://example.com/lib.git@tag` | `mylib = { git = "...", tag = "..." }` |

**ConanCenter** references follow Conan’s usual `name/version@user/channel` style; for center packages the trailing `@` is common (e.g. `fmt/10.2.1@`). Ion talks to ConanCenter over HTTPS; you do not need the Conan CLI installed for this resolution path.

**vcpkg** dependencies use the **port name** (e.g. `fmt`, `openssl`). Ion uses the public vcpkg port index and baseline metadata to resolve versions and source archives; the classic `vcpkg` tool is not required for declaring the dependency in Ion.

After changing dependencies, run:

```bash
ion install
```

Registry URLs, mirrors, and Conan/vcpkg toggles are documented under [Global configuration](#global-configuration).

### Build and run

```bash
ion build [--build-type debug|release]
ion run [-- args]
ion test
ion clean
```

### Code quality

```bash
ion check
ion check --list-rules
ion check --rule modern/nullptr,memory/leak
ion check --fix
ion check --watch
ion lsp    # Language Server Protocol for editors
```

## Project structure

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

## Configuration

### `ion.toml`

Project manifest: metadata, dependencies, and build hints.

```toml
[package]
name = "my-project"
version = "0.1.0"
cpp-standard = "20"
description = "A modern C++ application"
authors = ["Your Name <you@example.com>"]
license = "MIT"
repository = "https://github.com/you/my-project"

[dependencies]
# Ion registry (version range or exact)
fmt = "10.1.1"
spdlog = "^1.12"
boost = { version = "1.83.0", features = ["system", "filesystem"] }

# ConanCenter and vcpkg (optional; use one source per package)
# logging = { conan = "spdlog/1.13.0@" }
# zlib = { vcpkg = "zlib" }

[dev-dependencies]
catch2 = "3.4.0"

[build]
compiler-flags = ["-Wall", "-Wextra", "-Wpedantic"]
linker-flags = []
features = ["threading"]
```

### Global configuration

`~/.config/ion/config.toml`:

```toml
[registry]
default = "ion"
url = "https://registry.ion-cpp.dev"
mirrors = ["https://mirror1.example.com", "https://mirror2.example.com"]

[registry.conan]
enabled = true
url = "https://conan.io/center"

[registry.vcpkg]
enabled = true

[cache]
directory = "~/.cache/ion"
max-size-mb = 2048

[build]
parallel-jobs = 8
ccache = true
```

## Roadmap

### Phase 1: Foundation
- [x] Project scaffolding (`new`, `init`)
- [x] Manifest parsing (`ion.toml`)
- [x] CMake generation
- [x] CLI interface

### Phase 2: Package manager
- [x] Lockfile generation and freshness checks
- [x] Multi-source registry adapters
- [x] Package download/caching and extraction
- [x] Dependency lifecycle commands (`add/remove/install/update`)
- [x] Build/run/test/clean command integration
- [x] CMake dependency wiring support

### Phase 3: Linting
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

### Phase 4: Advanced features
- [x] Initial LSP server plumbing (`ion lsp`)
- [x] Diagnostic conversion pipeline for editor integration
- [x] Smart-pointer heuristics (`memory/smart-get`, `memory/raw-from-smart`, `memory/move-after-use`, `memory/shared-cycle-hint`)
- [x] Expanded auto-fix (`modern/c-cast` → `static_cast` when safe)
- [x] Editor UX: full sync, unsaved buffers, diagnostic `code`, range-filtered code actions
- [x] Go-to-definition (`textDocument/definition`, libclang when available)

### Phase 5: Production (Months 13-18)
- [ ] Public beta
- [ ] Package ecosystem (200+ curated packages) — prerequisite: [registry, CI, and publishing architecture](docs/ECOSYSTEM_ARCHITECTURE.md)
- [ ] Enterprise features
- [ ] v1.0 launch

## Contributing

Contributions are welcome.

1. **Issues**: Report bugs with reproduction steps and environment details.
2. **Features**: Open a discussion or issue before large changes.
3. **Pull requests**: Keep changes focused; match existing style and tests.
4. **Documentation**: Improvements to this README and inline help are appreciated.

### Development setup

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

### Tests

```bash
cargo test
cargo test test_manifest_creation
cargo test -- --nocapture
```

## Comparison

Ion is not a drop-in replacement for every Conan or vcpkg workflow; it orchestrates downloads and CMake integration from multiple ecosystems. Rough positioning:

| Capability | Ion | Conan | vcpkg | CMake `FetchContent` |
|------------|-----|-------|-------|----------------------|
| Single manifest + lockfile for the project | Yes | Yes | Yes | Partial |
| Built-in static analysis / linter | Yes | No | No | No |
| Native integration with ConanCenter / vcpkg ports | Via adapters | Native | Native | Manual |
| Primary focus | CLI + CMake + lint | C/C++ packages & binary artifacts | Ports + toolchain integration | CMake-centric vendoring |

Use Ion when you want one tool for dependency resolution, generated CMake, and optional `ion check` / `ion lsp` in the same repo.

## Why Ion?

C++ projects often combine manual downloads, ad hoc CMake, and separate analysis tools. Ion standardizes:

- Declaring dependencies (including **ConanCenter** and **vcpkg** names) in `ion.toml`
- Resolving and caching artifacts with `ion install`
- Building and testing with `ion build` / `ion test`
- Catching issues early with `ion check` and editor integration via `ion lsp`

The CLI is implemented in Rust; the project is MIT-licensed.

## Architecture

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

**Scaling the package catalog** (registry service, tarball contract, maintainer CI, quality gates) is outlined in [docs/ECOSYSTEM_ARCHITECTURE.md](docs/ECOSYSTEM_ARCHITECTURE.md). The CLI in this repo implements the **consumer** side (`src/registry/ion.rs`); a large ecosystem also needs server-side publishing and automated package builds.

## License

Distributed under the [MIT License](LICENSE).

## Acknowledgments

Rust and Cargo; Conan and vcpkg for prior art in C++ packaging; clang and the LLVM project for tooling ideas.

## Links

- [Issues](https://github.com/cybergenii/ion/issues)
- [Discussions](https://github.com/cybergenii/ion/discussions)
- [Documentation](https://ion.cybergenii.com/docs)
- [Examples](https://github.com/cybergenii/ion-examples)
- [Contributing](CONTRIBUTING.md)
- [Package ecosystem architecture](docs/ECOSYSTEM_ARCHITECTURE.md) (roadmap)
- [Publish API (draft)](docs/PUBLISH_API.md)
- [CI templates for library authors](docs/templates/README.md)

Maintainer contact: [cybersgenii@gmail.com](mailto:cybersgenii@gmail.com) · [@cyber_genii](https://twitter.com/cyber_genii)
