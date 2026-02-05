//! Project configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Project configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project type/framework
    pub project_type: ProjectType,
    /// Container configurations
    pub containers: Vec<ContainerDef>,
    /// Shared volumes
    pub volumes: Vec<VolumeDef>,
    /// Network configuration
    pub network: NetworkDef,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Build configuration
    pub build: BuildConfig,
    /// Development mode settings
    pub dev: DevConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            project_type: ProjectType::Generic,
            containers: Vec::new(),
            volumes: Vec::new(),
            network: NetworkDef::default(),
            environment: HashMap::new(),
            build: BuildConfig::default(),
            dev: DevConfig::default(),
        }
    }
}

/// Project type enumeration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    /// Generic project
    Generic,
    /// Node.js project
    Node,
    /// Python project
    Python,
    /// Rust project
    Rust,
    /// Go project
    Go,
    /// Java/JVM project
    Java,
    /// .NET project
    DotNet,
    /// Ruby project
    Ruby,
    /// PHP project
    Php,
    /// Docker Compose project
    Compose,
    /// Kubernetes project
    Kubernetes,
}

impl Default for ProjectType {
    fn default() -> Self {
        Self::Generic
    }
}

/// Container definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerDef {
    /// Container name
    pub name: String,
    /// Image reference
    pub image: String,
    /// Dockerfile path (for building)
    pub dockerfile: Option<PathBuf>,
    /// Build context
    pub context: Option<PathBuf>,
    /// Command override
    pub command: Option<Vec<String>>,
    /// Port mappings
    pub ports: Vec<PortDef>,
    /// Volume mounts
    pub volumes: Vec<String>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Depends on other containers
    pub depends_on: Vec<String>,
    /// Health check
    pub healthcheck: Option<HealthCheck>,
    /// Resource limits
    pub resources: Option<ResourceDef>,
}

/// Port definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDef {
    /// Container port
    pub container: u16,
    /// Host port (None for auto-allocation)
    pub host: Option<u16>,
    /// Protocol (tcp/udp)
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

fn default_protocol() -> String {
    "tcp".to_string()
}

/// Volume definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeDef {
    /// Volume name
    pub name: String,
    /// Source path (for bind mounts)
    pub source: Option<PathBuf>,
    /// Driver options
    pub driver_opts: HashMap<String, String>,
}

/// Network definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDef {
    /// Network name
    pub name: Option<String>,
    /// Enable isolation
    pub isolated: bool,
    /// Enable IPv6
    pub ipv6: bool,
    /// Custom subnet
    pub subnet: Option<String>,
}

impl Default for NetworkDef {
    fn default() -> Self {
        Self {
            name: None,
            isolated: true,
            ipv6: false,
            subnet: None,
        }
    }
}

/// Health check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Check command/test
    pub test: Vec<String>,
    /// Interval between checks (e.g., "30s")
    pub interval: String,
    /// Timeout for check (e.g., "10s")
    pub timeout: String,
    /// Retries before unhealthy
    pub retries: u32,
    /// Start period (e.g., "5s")
    pub start_period: Option<String>,
}

/// Resource limits definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDef {
    /// CPU limit (e.g., "0.5" for half a CPU)
    pub cpu_limit: Option<String>,
    /// Memory limit (e.g., "512m")
    pub memory_limit: Option<String>,
    /// CPU reservation
    pub cpu_reservation: Option<String>,
    /// Memory reservation
    pub memory_reservation: Option<String>,
}

/// Build configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Enable BuildKit
    pub buildkit: bool,
    /// Build arguments
    pub args: HashMap<String, String>,
    /// Target stage (multi-stage builds)
    pub target: Option<String>,
    /// Cache from images
    pub cache_from: Vec<String>,
    /// Platform
    pub platform: Option<String>,
}

/// Development mode configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevConfig {
    /// Enable hot reload
    pub hot_reload: bool,
    /// Paths to watch
    pub watch_paths: Vec<PathBuf>,
    /// Paths to ignore
    pub ignore_paths: Vec<String>,
    /// Sync mode (bind/copy)
    pub sync_mode: SyncMode,
}

impl Default for DevConfig {
    fn default() -> Self {
        Self {
            hot_reload: true,
            watch_paths: vec![PathBuf::from(".")],
            ignore_paths: vec![
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
                "**/target/**".to_string(),
                "**/__pycache__/**".to_string(),
            ],
            sync_mode: SyncMode::Bind,
        }
    }
}

/// File sync mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncMode {
    /// Bind mount (fastest, native)
    Bind,
    /// Copy files (better for cross-platform)
    Copy,
    /// Use mutagen or similar sync tool
    Mutagen,
}

impl Default for SyncMode {
    fn default() -> Self {
        Self::Bind
    }
}

/// HyperBox project file (hyperbox.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperboxConfig {
    /// Project metadata
    pub project: ProjectMeta,
    /// Container definitions
    #[serde(default)]
    pub containers: Vec<ContainerDef>,
    /// Volume definitions
    #[serde(default)]
    pub volumes: Vec<VolumeDef>,
    /// Network configuration
    #[serde(default)]
    pub network: NetworkDef,
    /// Build configuration
    #[serde(default)]
    pub build: BuildConfig,
    /// Dev configuration
    #[serde(default)]
    pub dev: DevConfig,
}

/// Project metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    /// Project name
    pub name: String,
    /// Project version
    pub version: Option<String>,
    /// Description
    pub description: Option<String>,
}
