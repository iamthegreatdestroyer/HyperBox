//! Project-centric error types.

use std::path::PathBuf;
use thiserror::Error;

/// Result type for project operations.
pub type Result<T> = std::result::Result<T, ProjectError>;

/// Project-related errors.
#[derive(Debug, Error)]
pub enum ProjectError {
    /// Project not found.
    #[error("Project not found: {0}")]
    NotFound(String),

    /// Project already exists.
    #[error("Project already exists: {0}")]
    AlreadyExists(String),

    /// Invalid project configuration.
    #[error("Invalid project configuration: {0}")]
    InvalidConfig(String),

    /// Project directory not found.
    #[error("Project directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    /// Failed to detect project type.
    #[error("Failed to detect project type in {path}")]
    DetectionFailed { path: PathBuf },

    /// Port allocation failed.
    #[error("Failed to allocate port: {reason}")]
    PortAllocation { reason: String },

    /// Container operation failed.
    #[error("Container operation failed: {operation} - {reason}")]
    ContainerOperation { operation: String, reason: String },

    /// Resource limit exceeded.
    #[error("Resource limit exceeded: {resource} (limit: {limit}, requested: {requested})")]
    ResourceLimitExceeded {
        resource: String,
        limit: u64,
        requested: u64,
    },

    /// File system error.
    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// TOML parsing error.
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// Core error.
    #[error("Core error: {0}")]
    Core(#[from] hyperbox_core::error::CoreError),

    /// Watcher error.
    #[error("File watcher error: {0}")]
    Watcher(String),

    /// YAML parsing error.
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    /// Configuration file error.
    #[error("Config error in {path}: {message}")]
    ConfigError { path: PathBuf, message: String },

    /// Project state error.
    #[error("Invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition {
        from: super::ProjectState,
        to: super::ProjectState,
    },

    /// Container not found in project.
    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    /// Failed to create container.
    #[error("Failed to create container '{container}': {reason}")]
    ContainerCreate { container: String, reason: String },

    /// Failed to start container.
    #[error("Failed to start container '{container}': {reason}")]
    ContainerStart { container: String, reason: String },

    /// Cyclic dependency detected in container definitions.
    #[error("Cyclic dependency detected in container depends_on")]
    CyclicDependency,
}
