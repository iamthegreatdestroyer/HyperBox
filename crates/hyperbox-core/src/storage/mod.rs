//! Storage layer for container images and layers.
//!
//! Provides composefs integration, layer management, and image caching.

pub mod composefs;
pub mod layers;
pub mod registry;

pub use composefs::ComposefsManager;
pub use layers::LayerStore;
pub use registry::ImageRegistry;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Image manifest (OCI Image Manifest).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageManifest {
    /// Schema version
    pub schema_version: u32,
    /// Media type
    pub media_type: String,
    /// Config descriptor
    pub config: Descriptor,
    /// Layer descriptors
    pub layers: Vec<Descriptor>,
    /// Annotations
    #[serde(default)]
    pub annotations: std::collections::HashMap<String, String>,
}

/// Content descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Descriptor {
    /// Media type
    pub media_type: String,
    /// Content digest
    pub digest: String,
    /// Size in bytes
    pub size: u64,
    /// Annotations
    #[serde(default)]
    pub annotations: std::collections::HashMap<String, String>,
}

/// Image configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Architecture
    pub architecture: String,
    /// OS
    pub os: String,
    /// Created timestamp
    pub created: Option<String>,
    /// Author
    pub author: Option<String>,
    /// Container configuration
    pub config: Option<ContainerConfig>,
    /// Root filesystem
    pub rootfs: RootFs,
    /// History
    pub history: Vec<HistoryEntry>,
}

/// Container configuration from image.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// User
    #[serde(rename = "User")]
    pub user: Option<String>,
    /// Exposed ports
    #[serde(rename = "ExposedPorts")]
    pub exposed_ports: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// Environment variables
    #[serde(rename = "Env")]
    pub env: Option<Vec<String>>,
    /// Entrypoint
    #[serde(rename = "Entrypoint")]
    pub entrypoint: Option<Vec<String>>,
    /// Command
    #[serde(rename = "Cmd")]
    pub cmd: Option<Vec<String>>,
    /// Volumes
    #[serde(rename = "Volumes")]
    pub volumes: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// Working directory
    #[serde(rename = "WorkingDir")]
    pub working_dir: Option<String>,
    /// Labels
    #[serde(rename = "Labels")]
    pub labels: Option<std::collections::HashMap<String, String>>,
    /// Stop signal
    #[serde(rename = "StopSignal")]
    pub stop_signal: Option<String>,
}

/// Root filesystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootFs {
    /// Type (should be "layers")
    #[serde(rename = "type")]
    pub fs_type: String,
    /// Diff IDs (layer digests)
    pub diff_ids: Vec<String>,
}

/// History entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Created timestamp
    pub created: Option<String>,
    /// Created by command
    pub created_by: Option<String>,
    /// Empty layer
    pub empty_layer: Option<bool>,
    /// Comment
    pub comment: Option<String>,
}

/// Storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Root directory for storage
    pub root_dir: PathBuf,
    /// Image directory
    pub images_dir: PathBuf,
    /// Layers directory
    pub layers_dir: PathBuf,
    /// Containers directory
    pub containers_dir: PathBuf,
    /// Use composefs for mounting
    pub use_composefs: bool,
    /// Enable layer deduplication
    pub deduplicate: bool,
    /// Max cache size in bytes
    pub max_cache_size: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let root_dir = if cfg!(unix) {
            PathBuf::from("/var/lib/hyperbox")
        } else {
            PathBuf::from(r"C:\ProgramData\hyperbox")
        };

        Self {
            images_dir: root_dir.join("images"),
            layers_dir: root_dir.join("layers"),
            containers_dir: root_dir.join("containers"),
            root_dir,
            use_composefs: true,
            deduplicate: true,
            max_cache_size: 10 * 1024 * 1024 * 1024, // 10GB
        }
    }
}
