//! Error types for HyperBox Core.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias using [`CoreError`].
pub type Result<T> = std::result::Result<T, CoreError>;

/// Core error types for container runtime operations.
#[derive(Error, Debug)]
pub enum CoreError {
    /// Container not found
    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    /// Container already exists
    #[error("Container already exists: {0}")]
    ContainerAlreadyExists(String),

    /// Container is not running
    #[error("Container is not running: {0}")]
    ContainerNotRunning(String),

    /// Container is already running
    #[error("Container is already running: {0}")]
    ContainerAlreadyRunning(String),

    /// Image not found
    #[error("Image not found: {0}")]
    ImageNotFound(String),

    /// Runtime not available
    #[error("Runtime not available: {runtime}. Install path: {path:?}")]
    RuntimeNotAvailable { runtime: String, path: PathBuf },

    /// Runtime execution failed
    #[error("Runtime execution failed: {0}")]
    RuntimeExecution(String),

    /// Checkpoint operation failed
    #[error("Checkpoint operation failed: {0}")]
    CheckpointFailed(String),

    /// Restore operation failed
    #[error("Restore operation failed: {0}")]
    RestoreFailed(String),

    /// Cgroup operation failed
    #[error("Cgroup operation failed: {operation} - {reason}")]
    CgroupOperation { operation: String, reason: String },

    /// Namespace operation failed
    #[error("Namespace operation failed: {namespace_type} - {reason}")]
    NamespaceOperation {
        namespace_type: String,
        reason: String,
    },

    /// Network operation failed
    #[error("Network operation failed: {0}")]
    NetworkOperation(String),

    /// Network configuration error
    #[error("Network configuration error: {0}")]
    NetworkConfiguration(String),

    /// Port allocation failed
    #[error("Port allocation failed: port {port}")]
    PortAllocationFailed { port: u16 },

    /// Storage operation failed
    #[error("Storage operation failed: {0}")]
    StorageOperation(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Permission denied
    #[error("Permission denied: {operation}. Requires: {required}")]
    PermissionDenied { operation: String, required: String },

    /// Resource exhausted
    #[error("Resource exhausted: {resource}. Limit: {limit}, Requested: {requested}")]
    ResourceExhausted {
        resource: String,
        limit: String,
        requested: String,
    },

    /// Timeout
    #[error("Operation timed out after {duration_ms}ms: {operation}")]
    Timeout { operation: String, duration_ms: u64 },

    /// Invalid specification
    #[error("Invalid specification: {field} - {reason}")]
    InvalidSpec { field: String, reason: String },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl CoreError {
    /// Check if error is retryable
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Timeout { .. } | Self::ResourceExhausted { .. } | Self::Io(_))
    }

    /// Check if error is a not found error
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::ContainerNotFound(_) | Self::ImageNotFound(_))
    }
}
