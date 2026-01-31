//! Daemon configuration.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

/// Daemon configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Path to this config file
    #[serde(skip)]
    pub config_path: PathBuf,

    /// Unix socket path for API
    pub api_socket: PathBuf,

    /// gRPC server address
    pub grpc_addr: SocketAddr,

    /// Data directory
    pub data_dir: PathBuf,

    /// Log level
    pub log_level: String,

    /// Container runtime configuration
    pub runtime: RuntimeConfig,

    /// Storage configuration
    pub storage: StorageConfig,

    /// Network configuration
    pub network: NetworkConfig,

    /// Optimization configuration
    pub optimization: OptimizationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Default container runtime
    pub default_runtime: String,

    /// Path to crun
    pub crun_path: PathBuf,

    /// Path to runc
    pub runc_path: PathBuf,

    /// Enable rootless mode
    pub rootless: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage driver
    pub driver: String,

    /// Images directory
    pub images_dir: PathBuf,

    /// Containers directory
    pub containers_dir: PathBuf,

    /// Volumes directory
    pub volumes_dir: PathBuf,

    /// Enable composefs
    pub composefs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Default network driver
    pub driver: String,

    /// Bridge network name
    pub bridge_name: String,

    /// CNI config directory
    pub cni_config_dir: PathBuf,

    /// CNI plugin directory
    pub cni_plugin_dir: PathBuf,

    /// Port range start
    pub port_range_start: u16,

    /// Port range end
    pub port_range_end: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    /// Enable CRIU checkpointing
    pub enable_criu: bool,

    /// CRIU path
    pub criu_path: PathBuf,

    /// Checkpoints directory
    pub checkpoints_dir: PathBuf,

    /// Checkpoint TTL in seconds
    pub checkpoint_ttl_seconds: u64,

    /// Enable lazy loading (eStargz)
    pub enable_lazy_loading: bool,

    /// Enable predictive pre-warming
    pub enable_prewarm: bool,

    /// Maximum pre-warmed containers
    pub max_prewarmed: usize,

    /// Pre-warm threshold (0.0-1.0)
    pub prewarm_threshold: f64,

    /// Pre-warm lookahead in seconds
    pub prewarm_lookahead_seconds: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let data_dir = default_data_dir();

        Self {
            config_path: default_config_path(),
            api_socket: default_socket_path(),
            grpc_addr: "127.0.0.1:50051".parse().unwrap(),
            data_dir: data_dir.clone(),
            log_level: "info".to_string(),
            runtime: RuntimeConfig {
                default_runtime: "crun".to_string(),
                crun_path: PathBuf::from("/usr/bin/crun"),
                runc_path: PathBuf::from("/usr/bin/runc"),
                rootless: false,
            },
            storage: StorageConfig {
                driver: "composefs".to_string(),
                images_dir: data_dir.join("images"),
                containers_dir: data_dir.join("containers"),
                volumes_dir: data_dir.join("volumes"),
                composefs: true,
            },
            network: NetworkConfig {
                driver: "bridge".to_string(),
                bridge_name: "hyperbox0".to_string(),
                cni_config_dir: PathBuf::from("/etc/cni/net.d"),
                cni_plugin_dir: PathBuf::from("/usr/lib/cni"),
                port_range_start: 32768,
                port_range_end: 60999,
            },
            optimization: OptimizationConfig {
                enable_criu: true,
                criu_path: PathBuf::from("/usr/sbin/criu"),
                checkpoints_dir: data_dir.join("checkpoints"),
                checkpoint_ttl_seconds: 86400, // 24 hours
                enable_lazy_loading: true,
                enable_prewarm: true,
                max_prewarmed: 10,
                prewarm_threshold: 0.7,
                prewarm_lookahead_seconds: 300,
            },
        }
    }
}

impl DaemonConfig {
    /// Load configuration from file or use defaults.
    pub fn load() -> Result<Self> {
        let config_path = default_config_path();

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            let mut config: DaemonConfig = toml::from_str(&content)
                .context("Failed to parse config file")?;
            config.config_path = config_path;
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save configuration to file.
    pub fn save(&self) -> Result<()> {
        let config_dir = self.config_path.parent()
            .context("Invalid config path")?;

        std::fs::create_dir_all(config_dir)
            .context("Failed to create config directory")?;

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(&self.config_path, content)
            .context("Failed to write config file")?;

        Ok(())
    }

    /// Ensure all required directories exist.
    pub fn ensure_directories(&self) -> Result<()> {
        let dirs = [
            &self.data_dir,
            &self.storage.images_dir,
            &self.storage.containers_dir,
            &self.storage.volumes_dir,
            &self.optimization.checkpoints_dir,
        ];

        for dir in dirs {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create directory: {:?}", dir))?;
        }

        Ok(())
    }
}

fn default_data_dir() -> PathBuf {
    if cfg!(target_os = "linux") {
        PathBuf::from("/var/lib/hyperbox")
    } else if cfg!(target_os = "macos") {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("hyperbox")
    } else {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("C:\\ProgramData"))
            .join("hyperbox")
    }
}

fn default_config_path() -> PathBuf {
    if cfg!(target_os = "linux") {
        PathBuf::from("/etc/hyperbox/daemon.toml")
    } else if cfg!(target_os = "macos") {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("hyperbox")
            .join("daemon.toml")
    } else {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("C:\\ProgramData"))
            .join("hyperbox")
            .join("daemon.toml")
    }
}

fn default_socket_path() -> PathBuf {
    if cfg!(target_os = "linux") {
        PathBuf::from("/run/hyperbox/hyperbox.sock")
    } else if cfg!(target_os = "macos") {
        dirs::runtime_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("hyperbox.sock")
    } else {
        // Windows uses named pipes, but we'll use a file path for now
        PathBuf::from("\\\\.\\pipe\\hyperbox")
    }
}
