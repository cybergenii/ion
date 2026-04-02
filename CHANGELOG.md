# Changelog

All notable changes to Ion will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure and CLI skeleton
- `ion new` command for creating new C++ projects
- `ion init` command for initializing existing directories
- Support for multiple project templates (executable, library, header-only)
- Automatic CMakeLists.txt generation
- Package manifest (ion.toml) structure
- Beautiful colored CLI output
- Cross-platform support (Linux, macOS, Windows)
- Comprehensive documentation and contribution guidelines
- CI/CD workflows for automated testing and releases

### Changed
- N/A

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- N/A

## [0.1.0] - 2024-10-16

### Added
- Initial release with basic project scaffolding functionality
- Project creation with `ion new`
- Project initialization with `ion init`
- TOML-based manifest file
- CMake integration
- Cross-platform support

---

## Release Notes

### [0.1.0] - Foundation Release

This is the first release of Ion, focusing on project scaffolding and the foundation for future features.

**Highlights:**
- 🎨 Beautiful CLI with colored output
- 📦 Easy project creation with sensible defaults
- 🔧 Automatic build system setup (CMake)
- 🌍 Cross-platform support (Linux, macOS, Windows)
- 📝 Clear and comprehensive documentation

**What's Next:**
- Package management functionality
- Dependency resolution
- Build system integration
- Code linting capabilities

**Installation:**
```bash
cargo install ionx
```

**Usage:**
```bash
ion new my-project
cd my-project
ion build
ion run
```

For more information, see the [README](README.md).

