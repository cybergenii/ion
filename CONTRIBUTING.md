# Contributing to Ion

Thank you for your interest in contributing to Ion! This document provides guidelines and instructions for contributing.

## 🌟 Ways to Contribute

### 1. Report Bugs
- Check if the bug has already been reported in [Issues](https://github.com/yourusername/ion/issues)
- Use the bug report template
- Include as much detail as possible (OS, Rust version, Ion version, steps to reproduce)
- Attach relevant logs and error messages

### 2. Suggest Features
- Check if the feature has already been suggested
- Use the feature request template
- Clearly describe the problem your feature would solve
- Provide examples of how the feature would be used

### 3. Submit Pull Requests
- Fork the repository
- Create a feature branch (`git checkout -b feature/amazing-feature`)
- Make your changes
- Write tests for your changes
- Run tests and ensure they pass
- Commit with clear messages
- Push to your fork
- Open a Pull Request

### 4. Improve Documentation
- Fix typos or unclear explanations
- Add examples
- Improve README or API documentation
- Write tutorials or blog posts

### 5. Help Others
- Answer questions in Issues or Discussions
- Review Pull Requests
- Share your Ion projects

## 🛠️ Development Setup

### Prerequisites
- Rust 1.70 or later
- Git
- A C++ compiler (for testing generated projects)

### Setup Steps

```bash
# Clone your fork
git clone https://github.com/your-username/ion
cd ion

# Add upstream remote
git remote add upstream https://github.com/yourusername/ion

# Install dependencies
cargo build

# Run tests
cargo test

# Run clippy for linting
cargo clippy --all-targets --all-features

# Run formatter
cargo fmt --all
```

## 📋 Development Guidelines

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

## 🔍 Pull Request Process

1. **Update Documentation**: Ensure README and docs reflect your changes
2. **Update CHANGELOG**: Add your changes to CHANGELOG.md
3. **Run Tests**: All tests must pass
4. **Run Clippy**: No warnings allowed
5. **Run Formatter**: Code must be formatted with `rustfmt`
6. **Update Version**: Follow semver for version bumps
7. **Get Reviews**: Wait for at least one maintainer approval

### PR Checklist

- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Commit messages follow conventions

## 🏗️ Project Structure

Understanding the codebase:

```
ion/
├── src/
│   ├── main.rs           # CLI entry point, argument parsing
│   ├── commands/         # Command implementations
│   │   ├── mod.rs        # Module exports
│   │   ├── new.rs        # 'ion new' command
│   │   ├── init.rs       # 'ion init' command
│   │   └── ...           # Other commands
│   ├── manifest.rs       # ion.toml parsing and management
│   ├── config.rs         # Configuration management
│   ├── resolver/         # Dependency resolution (TODO)
│   ├── registry/         # Package registry client (TODO)
│   ├── builder/          # Build system (TODO)
│   └── linter/           # Code analysis (TODO)
├── tests/                # Integration tests
├── .github/
│   └── workflows/        # CI/CD workflows
├── docs/                 # Documentation
└── examples/             # Example projects
```

## 🐛 Debugging Tips

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

## 📚 Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [clap Documentation](https://docs.rs/clap/)
- [tokio Documentation](https://docs.rs/tokio/)
- [Ion Roadmap](README.md#roadmap)

## ❓ Questions?

- Open a [Discussion](https://github.com/yourusername/ion/discussions)
- Join our [Discord/Slack] (coming soon)
- Email: your.email@example.com

## 📜 Code of Conduct

Be respectful, inclusive, and professional. We're all here to build something great together.

## 🎉 Recognition

Contributors will be:
- Listed in CONTRIBUTORS.md
- Mentioned in release notes
- Given credit in documentation

Thank you for contributing to Ion! 🚀

