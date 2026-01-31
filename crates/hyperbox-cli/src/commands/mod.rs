//! CLI command definitions and implementations.

use clap::{Parser, Subcommand};

pub mod completion;
pub mod container;
pub mod image;
pub mod project;
pub mod system;

/// HyperBox - 20x Faster Container Development
#[derive(Parser)]
#[command(name = "hb")]
#[command(author = "HyperBox Contributors")]
#[command(version)]
#[command(about = "Project-centric container development, 20x faster than Docker")]
#[command(long_about = r#"
HyperBox is a revolutionary container management platform designed for developers.

Key Features:
  • Project-centric: Automatic project detection and configuration
  • Sub-linear startup: <100ms warm starts with CRIU checkpointing
  • Predictive pre-warming: Containers ready before you need them
  • Smart caching: Lazy layer loading with eStargz

Examples:
  hb project open .          Open current directory as project
  hb project start           Start project containers
  hb container list          List all containers
  hb image pull nginx        Pull an image
"#)]
pub struct Cli {
    /// Verbose output (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text", global = true)]
    pub output: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

/// Output format.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// Top-level commands.
#[derive(Subcommand)]
pub enum Commands {
    /// Manage projects
    #[command(alias = "p")]
    Project(project::ProjectCommand),

    /// Manage containers
    #[command(alias = "c")]
    Container(container::ContainerCommand),

    /// Manage images
    #[command(alias = "i")]
    Image(image::ImageCommand),

    /// System commands
    #[command(alias = "sys")]
    System(system::SystemCommand),

    /// Generate shell completions
    Completion(completion::CompletionCommand),
}
