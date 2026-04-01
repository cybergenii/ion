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

use commands::{new, init, add, remove, install, update, build, run, test, clean, outdated, tree};
use build::BuildType;

#[derive(Parser)]
#[command(name = "ion")]
#[command(version = "0.2.0")]
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

        None => {
            println!(
                "{}",
                "Ion — Modern C++ Package Manager".bright_cyan().bold()
            );
            println!();
            println!("  {}  {}", "ion new <name>".green(), "Create a new project");
            println!("  {}  {}", "ion add <pkg>".green(), "Add a dependency");
            println!("  {}  {}", "ion install".green(), "Install all dependencies");
            println!("  {}  {}", "ion build".green(), "Build the project");
            println!("  {}  {}", "ion run".green(), "Build and run");
            println!("  {}  {}", "ion tree".green(), "Show dependency tree");
            println!("  {}  {}", "ion outdated".green(), "Check for updates");
            println!();
            println!("Run {} for all commands", "ion --help".cyan());
        }
    }

    Ok(())
}
