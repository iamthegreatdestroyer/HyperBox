//! HyperBox Daemon - Background service for container management.
//!
//! The daemon provides:
//! - Unix socket API for CLI communication
//! - gRPC API for GUI and remote clients
//! - Container lifecycle management
//! - Project management
//! - Health monitoring and metrics

use anyhow::Result;
use clap::Parser;
use tokio::signal;
use tracing::info;

mod api;
mod config;
mod error;
mod grpc;
mod health;
mod ipc;
mod lifecycle;
mod state;

use config::DaemonConfig;
use state::DaemonState;

/// HyperBox Daemon - Background service for container management
#[derive(Parser, Debug)]
#[command(name = "hyperboxd")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<std::path::PathBuf>,

    /// Run in foreground (don't daemonize)
    #[arg(short, long)]
    foreground: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Show configuration and exit
    #[arg(long)]
    show_config: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle --show-config before anything else
    if args.show_config {
        let config = DaemonConfig::load()?;
        println!("{}", toml::to_string_pretty(&config)?);
        return Ok(());
    }

    // Initialize logging with the specified level
    init_logging(&args.log_level);

    info!("Starting HyperBox daemon v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = DaemonConfig::load()?;
    info!("Configuration loaded from {:?}", config.config_path);

    // Initialize daemon state
    let state = DaemonState::new(config.clone()).await?;

    // Start services
    let api_handle = tokio::spawn(api::serve(state.clone(), config.api_socket.clone()));
    let grpc_handle = tokio::spawn(grpc::serve(state.clone(), config.grpc_addr.clone()));
    let health_handle = tokio::spawn(health::monitor(state.clone()));
    let lifecycle_handle = tokio::spawn(lifecycle::manager(state.clone()));

    info!("HyperBox daemon started");
    info!("  API socket: {:?}", config.api_socket);
    info!("  gRPC address: {}", config.grpc_addr);

    // Wait for shutdown signal
    shutdown_signal().await;

    info!("Shutdown signal received, stopping services...");

    // Graceful shutdown
    api_handle.abort();
    grpc_handle.abort();
    health_handle.abort();
    lifecycle_handle.abort();

    // Save state
    state.save().await?;

    info!("HyperBox daemon stopped");

    Ok(())
}

fn init_logging(level: &str) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
