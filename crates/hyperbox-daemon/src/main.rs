//! HyperBox Daemon - Background service for container management.
//!
//! The daemon provides:
//! - Unix socket API for CLI communication
//! - gRPC API for GUI and remote clients
//! - Container lifecycle management
//! - Project management
//! - Health monitoring and metrics

use anyhow::Result;
use std::path::PathBuf;
use tokio::signal;
use tracing::{info, warn};

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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logging();

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

fn init_logging() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

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
