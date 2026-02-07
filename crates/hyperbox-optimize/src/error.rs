//! Error types for optimization operations.

use std::path::PathBuf;
use thiserror::Error;

/// Result type for optimization operations.
pub type Result<T> = std::result::Result<T, OptimizeError>;

/// Errors that can occur during optimization operations.
#[derive(Debug, Error)]
pub enum OptimizeError {
    /// CRIU not available.
    #[error("CRIU not available: {reason}")]
    CriuNotAvailable { reason: String },

    /// Checkpoint operation failed.
    #[error("Checkpoint failed for container {container_id}: {reason}")]
    CheckpointFailed {
        container_id: String,
        reason: String,
    },

    /// Restore operation failed.
    #[error("Restore failed for container {container_id}: {reason}")]
    RestoreFailed {
        container_id: String,
        reason: String,
    },

    /// Checkpoint not found.
    #[error("Checkpoint not found: {path:?}")]
    CheckpointNotFound { path: PathBuf },

    /// Checkpoint expired.
    #[error("Checkpoint expired: {container_id}")]
    CheckpointExpired { container_id: String },

    /// Lazy loading failed.
    #[error("Lazy loading failed for layer {layer_id}: {reason}")]
    LazyLoadFailed { layer_id: String, reason: String },

    /// Layer not found.
    #[error("Layer not found: {layer_id}")]
    LayerNotFound { layer_id: String },

    /// Pre-warming failed.
    #[error("Pre-warming failed for image {image}: {reason}")]
    PrewarmFailed { image: String, reason: String },

    /// Prediction failed.
    #[error("Prediction failed: {reason}")]
    PredictionFailed { reason: String },

    /// Model training failed.
    #[error("Model training failed: {reason}")]
    ModelTrainingFailed { reason: String },

    /// Insufficient data.
    #[error(
        "Insufficient data for prediction: need at least {required} samples, have {available}"
    )]
    InsufficientData { required: usize, available: usize },

    /// Resource exhausted.
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },

    /// Deduplication operation failed.
    #[error("Deduplication failed: {reason}")]
    DedupFailed { reason: String },

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Core error.
    #[error("Core error: {0}")]
    Core(#[from] hyperbox_core::CoreError),
}
