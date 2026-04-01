//! Ion CLI binary — many modules expose helpers for tests and future commands; keep `-D warnings` builds clean.
#![allow(dead_code)]
#![allow(clippy::large_enum_variant)]

use clap::{Parser, Subcommand};
use colored::*;
use anyhow::Result;

mod commands;
mod config;
mod manifest;
mod lockfile;
mod registry;
mod resolver;
mod cmake;
mod linter;
mod analysis;
mod lsp;

use commands::{new, init, add, remove, install, update, build, run, test, clean, outdated, tree, check, lsp as lsp_cmd};
use build::BuildType;

#[derive(Parser)]
#[command(name = "ion")]
#[command(version = "0.3.0")]
#[command(about = "Modern C++ package manager", long_about = None)]
#[command(author = "Ion Contributors")]
#[command(
    after_help = "Examples:\n  ion new my-app\n  ion add fmtlib/fmt\n  ion build\n  ion run"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new C++ project
    New {
        /// Name of the project
        name: String,

        /// C++ standard to use (11, 14, 17, 20, 23)
        #[arg(long, default_value = "20")]
        std: String,

        /// Project template (executable, library, header-only)
        #[arg(long, default_value = "executable")]
        template: String,
    },

    /// Initialize an existing directory as an Ion project
    Init {
        /// C++ standard to use (11, 14, 17, 20, 23)
        #[arg(long, default_value = "20")]
        std: String,
    },

    /// Add a dependency to this project
    ///
    /// Spec formats:
    ///   fmt                        (Ion registry, latest)
    ///   fmt@10.2.1                 (Ion registry, exact)
    ///   github:fmtlib/fmt@10.2.1   (GitHub releases)
    ///   conan:fmt/10.2.1@          (ConanCenter)
    ///   vcpkg:fmt                  (vcpkg)
    ///   git:https://host/repo@tag  (Arbitrary git)
    Add {
        /// Package spec (see format above)
        spec: String,

        /// Add as a development dependency
        #[arg(long, short)]
        dev: bool,
    },

    /// Remove a dependency from this project
    Remove {
        /// Package name to remove
        name: String,

        /// Also delete the package from the global cache
        #[arg(long)]
        purge: bool,
    },

    /// Install all dependencies from ion.toml
    Install,

    /// Update one or all dependencies to latest matching versions
    Update {
        /// Name of a specific package to update (optional)
        package: Option<String>,
    },

    /// Build the project
    Build {
        /// Build in release mode (optimized)
        #[arg(long, short)]
        release: bool,
    },

    /// Build and run the project
    Run {
        /// Build in release mode
        #[arg(long, short)]
        release: bool,

        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Build and run tests via ctest
    Test,

    /// Remove build artifacts
    Clean {
        /// Also remove .ion/ local cache directory
        #[arg(long)]
        all: bool,
    },

    /// Show outdated dependencies
    Outdated,

    /// Display dependency tree
    Tree,

    /// Run static analysis and lint checks
    Check {
        /// Apply machine-fixable diagnostics
        #[arg(long)]
        fix: bool,
        /// Re-run analysis on source changes
        #[arg(long)]
        watch: bool,
        /// Output format
        #[arg(long, default_value = "text")]
        format: String,
        /// Comma-separated rule ids (e.g. modern/nullptr,memory/leak)
        #[arg(long)]
        rule: Option<String>,
        /// Print known rule ids and exit
        #[arg(long)]
        list_rules: bool,
        /// Disable colored output
        #[arg(long)]
        no_color: bool,
    },

    /// Start the Ion Language Server Protocol endpoint
    Lsp,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::New { name, std, template }) => {
            new::execute(&name, &std, &template)?;
        }
        Some(Commands::Init { std }) => {
            init::execute(&std)?;
        }

        Some(Commands::Add { spec, dev }) => {
            add::execute(&spec, dev).await?;
        }
        Some(Commands::Remove { name, purge }) => {
            remove::execute(&name, purge).await?;
        }
        Some(Commands::Install) => {
            install::execute().await?;
        }
        Some(Commands::Update { package }) => {
            update::execute(package.as_deref()).await?;
        }

        Some(Commands::Build { release }) => {
            let t = if release { BuildType::Release } else { BuildType::Debug };
            build::execute(t)?;
        }
        Some(Commands::Run { release, args }) => {
            run::execute(&args, release)?;
        }
        Some(Commands::Test) => {
            test::execute()?;
        }
        Some(Commands::Clean { all }) => {
            clean::execute(all)?;
        }

        Some(Commands::Outdated) => {
            outdated::execute().await?;
        }
        Some(Commands::Tree) => {
            tree::execute()?;
        }
        Some(Commands::Check {
            fix,
            watch,
            format,
            rule,
            list_rules,
            no_color,
        }) => {
            check::execute(
                fix,
                watch,
                &format,
                rule,
                list_rules,
                no_color,
            )
            .await?;
        }
        Some(Commands::Lsp) => {
            lsp_cmd::run().await?;
        }

        None => {
            println!(
                "{}",
                "Ion — Modern C++ Package Manager".bright_cyan().bold()
            );
            println!();
            println!("  {}  Create a new project", "ion new <name>".green());
            println!("  {}  Add a dependency", "ion add <pkg>".green());
            println!("  {}  Install all dependencies", "ion install".green());
            println!("  {}  Build the project", "ion build".green());
            println!("  {}  Build and run", "ion run".green());
            println!("  {}  Run C++ linting checks", "ion check".green());
            println!("  {}  Show dependency tree", "ion tree".green());
            println!("  {}  Check for updates", "ion outdated".green());
            println!();
            println!("Run {} for all commands", "ion --help".cyan());
        }
    }

    Ok(())
}
