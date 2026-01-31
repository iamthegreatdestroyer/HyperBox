//! Core type definitions for HyperBox.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

/// Unique identifier for a container.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContainerId(String);

impl ContainerId {
    /// Create a new container ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string().replace('-', "")[..12].to_string())
    }

    /// Create from an existing string.
    #[must_use]
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get short ID (first 12 characters).
    #[must_use]
    pub fn short(&self) -> &str {
        &self.0[..12.min(self.0.len())]
    }
}

impl Default for ContainerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ContainerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ContainerId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ContainerId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Container state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerState {
    /// Container is being created
    Creating,
    /// Container is created but not started
    Created,
    /// Container is running
    Running,
    /// Container is paused
    Paused,
    /// Container has stopped
    Stopped,
    /// Container is being removed
    Removing,
    /// Container has exited
    Exited,
    /// Container state is unknown
    Unknown,
}

impl std::fmt::Display for ContainerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Creating => write!(f, "creating"),
            Self::Created => write!(f, "created"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Stopped => write!(f, "stopped"),
            Self::Removing => write!(f, "removing"),
            Self::Exited => write!(f, "exited"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Image reference (e.g., "docker.io/library/alpine:3.18").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageRef {
    /// Registry (e.g., "docker.io")
    pub registry: String,
    /// Repository (e.g., "library/alpine")
    pub repository: String,
    /// Tag (e.g., "3.18") or digest
    pub tag: String,
}

impl ImageRef {
    /// Parse an image reference string.
    #[must_use]
    pub fn parse(image: &str) -> Self {
        // Simple parser - production would need full OCI reference parsing
        let parts: Vec<&str> = image.splitn(2, '/').collect();
        let (registry, rest) = if parts.len() == 2 && parts[0].contains('.') {
            (parts[0].to_string(), parts[1])
        } else {
            ("docker.io".to_string(), image)
        };

        let repo_parts: Vec<&str> = rest.splitn(2, ':').collect();
        let repository = if registry == "docker.io" && !repo_parts[0].contains('/') {
            format!("library/{}", repo_parts[0])
        } else {
            repo_parts[0].to_string()
        };

        let tag = repo_parts.get(1).unwrap_or(&"latest").to_string();

        Self {
            registry,
            repository,
            tag,
        }
    }

    /// Get the full image reference string.
    #[must_use]
    pub fn full_name(&self) -> String {
        format!("{}/{}:{}", self.registry, self.repository, self.tag)
    }
}

impl std::fmt::Display for ImageRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_name())
    }
}

impl From<&str> for ImageRef {
    fn from(s: &str) -> Self {
        Self::parse(s)
    }
}

/// Container specification for creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    /// Container name (optional)
    pub name: Option<String>,
    /// Image reference
    pub image: ImageRef,
    /// Command to run
    pub command: Vec<String>,
    /// Arguments to the command
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Working directory
    pub working_dir: Option<PathBuf>,
    /// User to run as (uid:gid)
    pub user: Option<String>,
    /// Volume mounts
    pub mounts: Vec<Mount>,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Resource limits
    pub resources: ResourceLimits,
    /// Labels
    pub labels: HashMap<String, String>,
    /// Restart policy
    pub restart_policy: RestartPolicy,
    /// Hostname
    pub hostname: Option<String>,
    /// Privileged mode
    pub privileged: bool,
    /// Read-only root filesystem
    pub read_only_rootfs: bool,
    /// TTY allocation
    pub tty: bool,
    /// Stdin open
    pub stdin_open: bool,
}

impl ContainerSpec {
    /// Create a new container spec builder.
    #[must_use]
    pub fn builder() -> ContainerSpecBuilder {
        ContainerSpecBuilder::default()
    }

    /// Compute a hash of the spec for caching/warm pool matching.
    #[must_use]
    pub fn hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let json = serde_json::to_string(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        hex::encode(hasher.finalize())[..16].to_string()
    }
}

/// Builder for [`ContainerSpec`].
#[derive(Debug, Default)]
pub struct ContainerSpecBuilder {
    spec: ContainerSpec,
}

impl Default for ContainerSpec {
    fn default() -> Self {
        Self {
            name: None,
            image: ImageRef::parse("alpine:latest"),
            command: Vec::new(),
            args: Vec::new(),
            env: HashMap::new(),
            working_dir: None,
            user: None,
            mounts: Vec::new(),
            ports: Vec::new(),
            resources: ResourceLimits::default(),
            labels: HashMap::new(),
            restart_policy: RestartPolicy::No,
            hostname: None,
            privileged: false,
            read_only_rootfs: false,
            tty: false,
            stdin_open: false,
        }
    }
}

impl ContainerSpecBuilder {
    /// Set the image.
    #[must_use]
    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.spec.image = ImageRef::parse(&image.into());
        self
    }

    /// Set the command.
    #[must_use]
    pub fn command(mut self, cmd: Vec<impl Into<String>>) -> Self {
        self.spec.command = cmd.into_iter().map(Into::into).collect();
        self
    }

    /// Set the name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.spec.name = Some(name.into());
        self
    }

    /// Add an environment variable.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.spec.env.insert(key.into(), value.into());
        self
    }

    /// Add a mount.
    #[must_use]
    pub fn mount(mut self, mount: Mount) -> Self {
        self.spec.mounts.push(mount);
        self
    }

    /// Add a port mapping.
    #[must_use]
    pub fn port(mut self, host: u16, container: u16) -> Self {
        self.spec.ports.push(PortMapping {
            host_port: host,
            container_port: container,
            protocol: Protocol::Tcp,
            host_ip: None,
        });
        self
    }

    /// Set resource limits.
    #[must_use]
    pub fn resources(mut self, resources: ResourceLimits) -> Self {
        self.spec.resources = resources;
        self
    }

    /// Enable TTY.
    #[must_use]
    pub fn tty(mut self, tty: bool) -> Self {
        self.spec.tty = tty;
        self
    }

    /// Build the spec.
    #[must_use]
    pub fn build(self) -> ContainerSpec {
        self.spec
    }
}

/// Volume mount configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    /// Source path on host
    pub source: PathBuf,
    /// Target path in container
    pub target: PathBuf,
    /// Read-only mount
    pub read_only: bool,
    /// Mount type
    pub mount_type: MountType,
}

/// Mount type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountType {
    /// Bind mount
    Bind,
    /// Volume mount
    Volume,
    /// Tmpfs mount
    Tmpfs,
}

/// Port mapping configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Host port
    pub host_port: u16,
    /// Container port
    pub container_port: u16,
    /// Protocol (tcp/udp)
    pub protocol: Protocol,
    /// Host IP to bind to
    pub host_ip: Option<String>,
}

/// Network protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    /// TCP
    Tcp,
    /// UDP
    Udp,
}

/// Resource limits for containers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU limit in millicores (1000 = 1 CPU)
    pub cpu_millicores: Option<u64>,
    /// Memory limit in bytes
    pub memory_bytes: Option<u64>,
    /// Memory swap limit in bytes
    pub memory_swap_bytes: Option<u64>,
    /// PIDs limit
    pub pids_limit: Option<u64>,
    /// IO read bytes per second
    pub io_read_bps: Option<u64>,
    /// IO write bytes per second
    pub io_write_bps: Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_millicores: None,
            memory_bytes: None,
            memory_swap_bytes: None,
            pids_limit: Some(4096),
            io_read_bps: None,
            io_write_bps: None,
        }
    }
}

/// Restart policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestartPolicy {
    /// Never restart
    No,
    /// Restart on failure
    OnFailure,
    /// Always restart
    Always,
    /// Restart unless stopped manually
    UnlessStopped,
}

/// Container statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    /// Container ID
    pub container_id: ContainerId,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// CPU stats
    pub cpu: CpuStats,
    /// Memory stats
    pub memory: MemoryStats,
    /// Network stats
    pub network: NetworkStats,
    /// Block IO stats
    pub block_io: BlockIoStats,
    /// PIDs count
    pub pids: u64,
}

/// CPU statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuStats {
    /// CPU usage percentage (0-100 per core)
    pub usage_percent: f64,
    /// Total CPU time in nanoseconds
    pub total_usage_ns: u64,
    /// System CPU time in nanoseconds
    pub system_usage_ns: u64,
    /// Number of CPUs
    pub num_cpus: u32,
}

/// Memory statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Used memory in bytes
    pub used_bytes: u64,
    /// Available memory in bytes
    pub available_bytes: u64,
    /// Memory limit in bytes
    pub limit_bytes: u64,
    /// Cache in bytes
    pub cache_bytes: u64,
    /// Memory usage percentage
    pub usage_percent: f64,
}

/// Network statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Bytes received
    pub rx_bytes: u64,
    /// Bytes transmitted
    pub tx_bytes: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Packets transmitted
    pub tx_packets: u64,
    /// Receive errors
    pub rx_errors: u64,
    /// Transmit errors
    pub tx_errors: u64,
}

/// Block IO statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockIoStats {
    /// Bytes read
    pub read_bytes: u64,
    /// Bytes written
    pub write_bytes: u64,
    /// Read operations
    pub read_ops: u64,
    /// Write operations
    pub write_ops: u64,
}

/// Exec specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecSpec {
    /// Command to execute
    pub command: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Working directory
    pub working_dir: Option<PathBuf>,
    /// User to run as
    pub user: Option<String>,
    /// Allocate TTY
    pub tty: bool,
    /// Attach stdin
    pub attach_stdin: bool,
    /// Attach stdout
    pub attach_stdout: bool,
    /// Attach stderr
    pub attach_stderr: bool,
    /// Privileged mode
    pub privileged: bool,
}

/// Exec result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    /// Exit code
    pub exit_code: i32,
    /// Stdout output
    pub stdout: String,
    /// Stderr output
    pub stderr: String,
}

/// Log options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogOptions {
    /// Show stdout
    pub stdout: bool,
    /// Show stderr
    pub stderr: bool,
    /// Follow logs
    pub follow: bool,
    /// Tail N lines
    pub tail: Option<usize>,
    /// Since timestamp
    pub since: Option<DateTime<Utc>>,
    /// Until timestamp
    pub until: Option<DateTime<Utc>>,
    /// Show timestamps
    pub timestamps: bool,
}

impl Default for LogOptions {
    fn default() -> Self {
        Self {
            stdout: true,
            stderr: true,
            follow: false,
            tail: None,
            since: None,
            until: None,
            timestamps: false,
        }
    }
}

/// Checkpoint identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(String);

impl CheckpointId {
    /// Create a new checkpoint ID.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get as string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CheckpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
