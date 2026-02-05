//! Container runtime trait definition.

use crate::error::Result;
use crate::types::*;
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};

/// Container runtime trait.
///
/// This trait defines the interface that all container runtimes must implement.
/// It provides a unified API for container lifecycle management across different
/// runtime implementations (crun, youki, runc, Firecracker).
///
/// # Performance Requirements
///
/// Implementations should target the following performance benchmarks:
/// - `create`: <30ms
/// - `start`: <20ms
/// - `stop`: <100ms (excluding timeout wait)
/// - `stats`: <5ms
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to allow concurrent access.
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Get the runtime name.
    fn name(&self) -> &'static str;

    /// Get the runtime version.
    async fn version(&self) -> Result<String>;

    /// Check if the runtime is available and properly configured.
    async fn is_available(&self) -> bool;

    /// Create a new container from the given specification.
    ///
    /// This creates the container but does not start it.
    /// The container will be in the "created" state.
    ///
    /// # Arguments
    ///
    /// * `spec` - Container specification
    ///
    /// # Returns
    ///
    /// The unique container ID.
    async fn create(&self, spec: ContainerSpec) -> Result<ContainerId>;

    /// Start a created container.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    async fn start(&self, id: &ContainerId) -> Result<()>;

    /// Stop a running container.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    /// * `timeout` - Time to wait for graceful shutdown before killing
    async fn stop(&self, id: &ContainerId, timeout: Duration) -> Result<()>;

    /// Kill a container with a signal.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    /// * `signal` - Signal to send (e.g., "SIGTERM", "SIGKILL")
    async fn kill(&self, id: &ContainerId, signal: &str) -> Result<()>;

    /// Remove a stopped container.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    async fn remove(&self, id: &ContainerId) -> Result<()>;

    /// Pause a running container.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    async fn pause(&self, id: &ContainerId) -> Result<()>;

    /// Resume a paused container.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    async fn resume(&self, id: &ContainerId) -> Result<()>;

    /// Execute a command in a running container.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    /// * `spec` - Exec specification
    ///
    /// # Returns
    ///
    /// The execution result including exit code and output.
    async fn exec(&self, id: &ContainerId, spec: ExecSpec) -> Result<ExecResult>;

    /// Get container state.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    ///
    /// # Returns
    ///
    /// Current container state.
    async fn state(&self, id: &ContainerId) -> Result<ContainerState>;

    /// Get container resource statistics.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    ///
    /// # Returns
    ///
    /// Current resource statistics.
    async fn stats(&self, id: &ContainerId) -> Result<ContainerStats>;

    /// Get container logs.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    /// * `opts` - Log options
    ///
    /// # Returns
    ///
    /// Async reader for log data.
    async fn logs(
        &self,
        id: &ContainerId,
        opts: LogOptions,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>>;

    /// Attach to a container's stdio.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    ///
    /// # Returns
    ///
    /// Tuple of (stdin writer, stdout reader, stderr reader).
    async fn attach(
        &self,
        id: &ContainerId,
    ) -> Result<(
        Box<dyn AsyncWrite + Send + Unpin>,
        Box<dyn AsyncRead + Send + Unpin>,
        Box<dyn AsyncRead + Send + Unpin>,
    )>;

    /// List all containers managed by this runtime.
    ///
    /// # Returns
    ///
    /// List of container IDs and their states.
    async fn list(&self) -> Result<Vec<(ContainerId, ContainerState)>>;

    /// Wait for a container to exit.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    ///
    /// # Returns
    ///
    /// Exit code of the container.
    async fn wait(&self, id: &ContainerId) -> Result<i32>;

    /// Checkpoint a running container (CRIU).
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    /// * `checkpoint_path` - Path to store checkpoint images
    ///
    /// # Returns
    ///
    /// Checkpoint ID for later restore.
    async fn checkpoint(&self, id: &ContainerId, checkpoint_path: &Path) -> Result<CheckpointId>;

    /// Restore a container from a checkpoint.
    ///
    /// # Arguments
    ///
    /// * `checkpoint_path` - Path to checkpoint images
    /// * `spec` - Container specification (for overrides)
    ///
    /// # Returns
    ///
    /// New container ID.
    async fn restore(&self, checkpoint_path: &Path, spec: ContainerSpec) -> Result<ContainerId>;

    /// Update container resource limits.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    /// * `resources` - New resource limits
    async fn update(&self, id: &ContainerId, resources: ResourceLimits) -> Result<()>;

    /// Get container processes.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID
    ///
    /// # Returns
    ///
    /// List of processes running in the container.
    async fn top(&self, id: &ContainerId) -> Result<Vec<ProcessInfo>>;

    /// Pull an image from a registry.
    ///
    /// # Arguments
    ///
    /// * `image` - Image reference to pull
    ///
    /// # Returns
    ///
    /// Ok(()) on successful pull.
    async fn pull_image(&self, image: &crate::types::ImageRef) -> Result<()>;

    /// Check if an image exists locally.
    ///
    /// # Arguments
    ///
    /// * `image` - Image reference to check
    ///
    /// # Returns
    ///
    /// True if the image exists locally.
    async fn image_exists(&self, image: &str) -> Result<bool>;

    /// List locally available images.
    ///
    /// # Returns
    ///
    /// List of image references.
    async fn list_images(&self) -> Result<Vec<ImageInfo>>;
}

/// Image information.
#[derive(Debug, Clone)]
pub struct ImageInfo {
    /// Image ID (digest)
    pub id: String,
    /// Image tags
    pub tags: Vec<String>,
    /// Size in bytes
    pub size: u64,
    /// Creation timestamp
    pub created: chrono::DateTime<chrono::Utc>,
}

/// Process information.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: u32,
    /// Parent process ID
    pub ppid: u32,
    /// CPU usage percentage
    pub cpu_percent: f64,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// Command
    pub command: String,
}

/// Extension trait for runtime-specific operations.
#[async_trait]
pub trait ContainerRuntimeExt: ContainerRuntime {
    /// Run a container (create + start + wait).
    async fn run(&self, spec: ContainerSpec) -> Result<(ContainerId, i32)> {
        let id = self.create(spec).await?;
        self.start(&id).await?;
        let exit_code = self.wait(&id).await?;
        Ok((id, exit_code))
    }

    /// Force remove a container (kill + remove).
    async fn force_remove(&self, id: &ContainerId) -> Result<()> {
        let _ = self.kill(id, "SIGKILL").await;
        self.remove(id).await
    }
}

// Blanket implementation
impl<T: ContainerRuntime> ContainerRuntimeExt for T {}
