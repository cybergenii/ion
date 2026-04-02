# Contributing to Ion

This document describes how to contribute code, tests, and documentation. For **using** Ion on your own C++ projects, see the [README](README.md).

## Ways to contribute

1. **Bugs** — Search [Issues](https://github.com/cybergenii/ion/issues) first, then open a new issue with OS, Rust version, Ion version, minimal steps to reproduce, and logs.
2. **Features** — Open a discussion or issue describing the problem and proposed usage before large changes.
3. **Pull requests** — Fork, branch from `main`, keep changes focused, add tests, and run the checks below before opening a PR.
4. **Docs** — README, `CONTRIBUTING.md`, and inline help; small typo fixes are welcome.

## Development setup

### Prerequisites

| Tool | Purpose |
|------|---------|
| **Rust** (stable; same as [CI](.github/workflows/ci.yml)) | Build Ion |
| **Git** | Version control |
| **C++ toolchain + CMake** (optional but useful) | Manually exercise `ion new` / `ion build` against sample projects |

### Clone and build

```bash
git clone https://github.com/your-username/ion
cd ion
git remote add upstream https://github.com/cybergenii/ion

cargo build
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all
```

### Before opening a PR

Run the same checks CI runs: `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt` (no diff). Update docs if your change affects user-visible behavior.

## Development guidelines

### Code Style

We follow standard Rust conventions:

```rust
// Good: Clear, descriptive names
pub fn create_project(name: &str, config: &Config) -> Result<Project> {
    // Implementation
}

// Good: Document public APIs
/// Creates a new C++ project with the specified configuration.
///
/// # Arguments
/// * `name` - The project name
/// * `config` - Project configuration
///
/// # Returns
/// A Result containing the created Project or an error
pub fn create_project(name: &str, config: &Config) -> Result<Project> {
    // Implementation
}

// Good: Use descriptive error messages
if !is_valid_name(name) {
    anyhow::bail!("Invalid project name: '{}'. Use only alphanumeric characters, hyphens, and underscores.", name);
}
```

### Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
feat: add support for private registries
fix: resolve dependency resolution deadlock
docs: update installation instructions
test: add integration tests for 'ion new'
refactor: simplify manifest parsing logic
perf: optimize package download parallelization
chore: update dependencies
```

### Testing

Write tests for all new features and bug fixes:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_name_validation() {
        assert!(is_valid_project_name("my-project"));
        assert!(is_valid_project_name("my_project"));
        assert!(!is_valid_project_name("-invalid"));
        assert!(!is_valid_project_name("invalid!"));
    }

    #[test]
    fn test_manifest_creation() {
        let manifest = Manifest::new("test", "20");
        assert_eq!(manifest.package.name, "test");
        assert_eq!(manifest.package.cpp_standard, "20");
    }
}
```

Run tests before submitting:

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_project_name_validation

# Run with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration
```

### Error Handling

Use `anyhow` for application errors and `thiserror` for library errors:

```rust
use anyhow::{Result, Context};

pub fn read_manifest(path: &str) -> Result<Manifest> {
    let content = std::fs::read_to_string(path)
        .context(format!("Failed to read manifest file: {}", path))?;
    
    let manifest = toml::from_str(&content)
        .context("Failed to parse manifest")?;
    
    Ok(manifest)
}
```

### Documentation

Document all public APIs:

```rust
/// Represents a package dependency.
///
/// # Examples
///
/// ```
/// use ion::manifest::Dependency;
///
/// let dep = Dependency::Simple("1.0.0".to_string());
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub enum Dependency {
    /// A simple version string
    Simple(String),
    /// Detailed dependency configuration
    Detailed(DetailedDependency),
}
```

## Pull request process

1. **Documentation** — Update README or user-facing help if behavior changes.
2. **CHANGELOG** — Add a user-visible entry under [CHANGELOG.md](CHANGELOG.md) when appropriate.
3. **Tests** — `cargo test` must pass; add tests for new behavior.
4. **Lint** — `cargo clippy -- -D warnings` (same as CI).
5. **Format** — `cargo fmt` with no uncommitted formatting diffs.
6. **Review** — Maintainer approval before merge.

### PR checklist

- [ ] Tests added or updated where relevant
- [ ] Docs / CHANGELOG updated if users are affected
- [ ] `cargo test` and `cargo clippy -- -D warnings` pass
- [ ] `cargo fmt` applied

## Project layout

```
ion/
├── src/
│   ├── main.rs           # CLI entry
│   ├── commands/         # ion subcommands
│   ├── manifest.rs       # ion.toml
│   ├── config.rs         # Global config
│   ├── resolver/         # Dependency resolution
│   ├── registry/         # Ion, GitHub, Conan, vcpkg, git, …
│   ├── cmake/            # CMake generation
│   ├── linter/           # ion check
│   ├── analysis/         # Dataflow / CFG helpers
│   └── lsp/              # ion lsp
├── .github/workflows/    # CI (test, fmt, clippy)
└── CHANGELOG.md
```

## Debugging tips

### Enable Debug Logging

```bash
# Set environment variable
RUST_LOG=debug cargo run -- new test-project

# Or in code
use env_logger;

fn main() {
    env_logger::init();
    // Your code
}
```

### Run with Verbose Output

```bash
cargo run -- new test-project -v
```

### Use the Debugger

VSCode launch.json:
```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug Ion",
            "cargo": {
                "args": ["build", "--bin=ion"],
                "filter": {
                    "name": "ion",
                    "kind": "bin"
                }
            },
            "args": ["new", "test-project"],
            "cwd": "${workspaceFolder}"
        }
    ]
}
```

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [clap](https://docs.rs/clap/), [tokio](https://docs.rs/tokio/) (where used)
- [Roadmap](README.md#roadmap)

## Questions

- [GitHub Discussions](https://github.com/cybergenii/ion/discussions)
- [Issues](https://github.com/cybergenii/ion/issues)
- Maintainer: [cybersgenii@gmail.com](mailto:cybersgenii@gmail.com)

## Code of conduct

Be respectful and inclusive. Disagreement should focus on technical merit.

## Recognition

Contributors are listed in [CONTRIBUTORS.md](CONTRIBUTORS.md) and release notes where applicable.

Thank you for contributing.

