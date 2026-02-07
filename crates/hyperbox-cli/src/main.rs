//! HyperBox CLI - The `hb` command.
//!
//! A fast, project-centric container management CLI.
//!
//! # Usage
//!
//! ```bash
//! hb project open .              # Open current directory as project
//! hb project start               # Start the current project
//! hb project stop                # Stop the current project
//! hb container list              # List all containers
//! hb image pull nginx            # Pull an image
//! hb docker run nginx            # Docker-compatible mode
//! ```

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod client;
mod commands;

use commands::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose);

    // Run command
    match cli.command {
        Commands::Project(cmd) => commands::project::run(cmd).await,
        Commands::Container(cmd) => commands::container::run(cmd).await,
        Commands::Image(cmd) => commands::image::run(cmd).await,
        Commands::System(cmd) => commands::system::run(cmd).await,
        Commands::Health(cmd) => commands::health::run(cmd).await,
        Commands::Completion(cmd) => commands::completion::run(cmd),
        Commands::Docker(cmd) => {
            cmd.execute(cli.output)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            Ok(())
        }
    }
}

fn init_logging(verbose: u8) {
    let level = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}
