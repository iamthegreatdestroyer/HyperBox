//! Daemon error types.

use thiserror::Error;

/// Daemon errors.
#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("State error: {0}")]
    State(String),

    #[error("Container error: {0}")]
    Container(String),

    #[error("Project error: {0}")]
    Project(String),

    #[error("Image error: {0}")]
    Image(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("gRPC error: {0}")]
    Grpc(String),

    #[error("Health check failed: {0}")]
    Health(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Core error: {0}")]
    Core(#[from] hyperbox_core::error::CoreError),

    #[error("Project error: {0}")]
    ProjectCrate(#[from] hyperbox_project::error::ProjectError),

    #[error("Optimization error: {0}")]
    Optimize(#[from] hyperbox_optimize::error::OptimizeError),
}

pub type Result<T> = std::result::Result<T, DaemonError>;
