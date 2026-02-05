//! Docker/Bollard-based container runtime implementation.
//!
//! This module provides a container runtime backed by Docker Engine via the Bollard crate.
//! It serves as the primary runtime on Windows and as an alternative on Linux when
//! Docker is available.
//!
//! # Architecture
//!
//! This is a cross-domain synthesis combining:
//! - OCI container standards (crun interface)
//! - Docker Engine API (Bollard client)
//! - HyperBox performance optimizations
//!
//! The result is a runtime that works everywhere Docker runs while maintaining
//! the same interface as native runtimes like crun.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, StatsOptions, StopContainerOptions, WaitContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::{CreateImageOptions, ListImagesOptions};
use bollard::Docker;
use futures::StreamExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{debug, error, info, instrument, warn};

use crate::error::{CoreError, Result};
use crate::types::{
    BlockIoStats, CheckpointId, ContainerId, ContainerSpec, ContainerState, ContainerStats,
    CpuStats, ExecResult, ExecSpec, ImageRef, LogOptions, MemoryStats, NetworkStats,
    ResourceLimits,
};

use super::traits::{ContainerRuntime, ProcessInfo};

/// Docker-based container runtime using Bollard.
///
/// This runtime connects to the Docker daemon via:
/// - Unix socket on Linux: `/var/run/docker.sock`
/// - Named pipe on Windows: `//./pipe/docker_engine`
///
/// # Performance
///
/// While Docker adds some overhead compared to native runtimes like crun,
/// the Bollard implementation provides:
/// - Consistent behavior across platforms
/// - Access to Docker's image caching
/// - Integration with Docker Compose workflows
pub struct DockerRuntime {
    /// Bollard Docker client
    client: Docker,
    /// Prefix for container names to avoid conflicts
    name_prefix: String,
    /// Default stop timeout in seconds
    stop_timeout: i64,
}

impl DockerRuntime {
    /// Creates a new Docker runtime, connecting to the local Docker daemon.
    ///
    /// # Errors
    ///
    /// Returns an error if Docker daemon is not accessible.
    pub fn new() -> Result<Self> {
        let client = Docker::connect_with_local_defaults()
            .map_err(|e| CoreError::Runtime(format!("Failed to connect to Docker: {}", e)))?;

        Ok(Self {
            client,
            name_prefix: "hb-".to_string(),
            stop_timeout: 10,
        })
    }

    /// Creates a Docker runtime with a specific connection.
    #[cfg(unix)]
    pub fn with_connection(socket_path: &str) -> Result<Self> {
        let client = Docker::connect_with_unix(socket_path, 120, bollard::API_DEFAULT_VERSION)
            .map_err(|e| CoreError::Runtime(format!("Failed to connect to Docker: {}", e)))?;

        Ok(Self {
            client,
            name_prefix: "hb-".to_string(),
            stop_timeout: 10,
        })
    }

    /// Creates a Docker runtime with a specific connection.
    #[cfg(windows)]
    pub fn with_connection(pipe_path: &str) -> Result<Self> {
        let client = Docker::connect_with_named_pipe(pipe_path, 120, bollard::API_DEFAULT_VERSION)
            .map_err(|e| CoreError::Runtime(format!("Failed to connect to Docker: {}", e)))?;

        Ok(Self {
            client,
            name_prefix: "hb-".to_string(),
            stop_timeout: 10,
        })
    }

    /// Generate a container name from HyperBox container ID.
    fn container_name(&self, id: &ContainerId) -> String {
        format!("{}{}", self.name_prefix, id.short())
    }

    /// Parse HyperBox container ID from Docker container name.
    fn parse_container_id(&self, name: &str) -> Option<ContainerId> {
        let clean = name.trim_start_matches('/');
        if clean.starts_with(&self.name_prefix) {
            let id_part = clean.trim_start_matches(&self.name_prefix);
            Some(ContainerId::from(id_part))
        } else {
            None
        }
    }

    /// Convert HyperBox container spec to Docker config.
    fn spec_to_docker_config(&self, spec: &ContainerSpec) -> Config<String> {
        let mut env = Vec::new();
        for (key, value) in &spec.env {
            env.push(format!("{}={}", key, value));
        }

        let mut volumes = HashMap::new();
        for mount in &spec.mounts {
            volumes.insert(mount.target.display().to_string(), HashMap::<(), ()>::new());
        }

        let exposed_ports: Option<HashMap<String, HashMap<(), ()>>> = if spec.ports.is_empty() {
            None
        } else {
            let mut ports = HashMap::new();
            for port in &spec.ports {
                ports.insert(format!("{}/tcp", port.container_port), HashMap::new());
            }
            Some(ports)
        };

        let host_config = bollard::service::HostConfig {
            memory: spec.resources.memory_bytes.map(|m| m as i64),
            cpu_shares: spec
                .resources
                .cpu_millicores
                .map(|c| (c * 1024 / 1000) as i64),
            pids_limit: spec.resources.pids_limit.map(|p| p as i64),
            binds: Some(
                spec.mounts
                    .iter()
                    .map(|m| format!("{}:{}", m.source.display(), m.target.display()))
                    .collect(),
            ),
            port_bindings: if spec.ports.is_empty() {
                None
            } else {
                let mut bindings = HashMap::new();
                for port in &spec.ports {
                    bindings.insert(
                        format!("{}/tcp", port.container_port),
                        Some(vec![bollard::service::PortBinding {
                            host_ip: Some(port.host_ip.clone().unwrap_or_default()),
                            host_port: Some(port.host_port.to_string()),
                        }]),
                    );
                }
                Some(bindings)
            },
            ..Default::default()
        };

        Config {
            image: Some(spec.image.to_string()),
            cmd: if spec.command.is_empty() {
                None
            } else {
                Some(spec.command.clone())
            },
            env: if env.is_empty() { None } else { Some(env) },
            working_dir: spec.working_dir.as_ref().map(|p| p.display().to_string()),
            user: spec.user.clone(),
            exposed_ports,
            host_config: Some(host_config),
            labels: if spec.labels.is_empty() {
                None
            } else {
                Some(spec.labels.clone())
            },
            ..Default::default()
        }
    }

    /// Convert Docker container state to HyperBox state.
    fn docker_state_to_hyperbox(state: &str, running: bool, paused: bool) -> ContainerState {
        if paused {
            ContainerState::Paused
        } else if running {
            ContainerState::Running
        } else {
            match state.to_lowercase().as_str() {
                "created" => ContainerState::Created,
                "running" => ContainerState::Running,
                "paused" => ContainerState::Paused,
                "restarting" => ContainerState::Running,
                "removing" => ContainerState::Stopped,
                "exited" => ContainerState::Exited,
                "dead" => ContainerState::Stopped,
                _ => ContainerState::Unknown,
            }
        }
    }
}

#[async_trait]
impl ContainerRuntime for DockerRuntime {
    fn name(&self) -> &'static str {
        "docker"
    }

    async fn version(&self) -> Result<String> {
        let version = self
            .client
            .version()
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to get Docker version: {}", e)))?;
        Ok(format!(
            "Docker {} (API {})",
            version.version.unwrap_or_default(),
            version.api_version.unwrap_or_default()
        ))
    }

    async fn is_available(&self) -> bool {
        self.client.ping().await.is_ok()
    }

    #[instrument(skip(self, spec))]
    async fn create(&self, spec: ContainerSpec) -> Result<ContainerId> {
        let id = ContainerId::new();
        let container_name = self.container_name(&id);
        let config = self.spec_to_docker_config(&spec);

        debug!(
            container_id = %id,
            image = %spec.image,
            "Creating Docker container"
        );

        // Pull image if needed
        let image_name = spec.image.to_string();
        if !self.image_exists(&image_name).await.unwrap_or(false) {
            info!(image = %image_name, "Pulling image");
            self.pull_image(&spec.image).await?;
        }

        // Create container
        let options = CreateContainerOptions {
            name: container_name,
            platform: None,
        };

        self.client
            .create_container(Some(options), config)
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to create container: {}", e)))?;

        info!(container_id = %id, "Container created successfully");
        Ok(id)
    }

    #[instrument(skip(self))]
    async fn start(&self, id: &ContainerId) -> Result<()> {
        let container_name = self.container_name(id);

        self.client
            .start_container(&container_name, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to start container: {}", e)))?;

        info!(container_id = %id, "Container started");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn stop(&self, id: &ContainerId, timeout: Duration) -> Result<()> {
        let container_name = self.container_name(id);
        let timeout_secs = timeout.as_secs() as i64;

        let options = StopContainerOptions { t: timeout_secs };

        self.client
            .stop_container(&container_name, Some(options))
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to stop container: {}", e)))?;

        info!(container_id = %id, "Container stopped");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn kill(&self, id: &ContainerId, signal: &str) -> Result<()> {
        let container_name = self.container_name(id);

        self.client
            .kill_container(
                &container_name,
                Some(bollard::container::KillContainerOptions {
                    signal: signal.to_string(),
                }),
            )
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to kill container: {}", e)))?;

        info!(container_id = %id, signal = signal, "Container killed");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove(&self, id: &ContainerId) -> Result<()> {
        let container_name = self.container_name(id);

        let options = RemoveContainerOptions {
            force: false,
            v: true, // Remove volumes
            ..Default::default()
        };

        self.client
            .remove_container(&container_name, Some(options))
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to remove container: {}", e)))?;

        info!(container_id = %id, "Container removed");
        Ok(())
    }

    async fn pause(&self, id: &ContainerId) -> Result<()> {
        let container_name = self.container_name(id);

        self.client
            .pause_container(&container_name)
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to pause container: {}", e)))?;

        info!(container_id = %id, "Container paused");
        Ok(())
    }

    async fn resume(&self, id: &ContainerId) -> Result<()> {
        let container_name = self.container_name(id);

        self.client
            .unpause_container(&container_name)
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to resume container: {}", e)))?;

        info!(container_id = %id, "Container resumed");
        Ok(())
    }

    #[instrument(skip(self, spec))]
    async fn exec(&self, id: &ContainerId, spec: ExecSpec) -> Result<ExecResult> {
        let container_name = self.container_name(id);

        let exec_options = CreateExecOptions {
            cmd: Some(spec.command.clone()),
            env: if spec.env.is_empty() {
                None
            } else {
                Some(
                    spec.env
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect(),
                )
            },
            working_dir: spec
                .working_dir
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            user: spec.user.clone(),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(spec.tty),
            ..Default::default()
        };

        let exec = self
            .client
            .create_exec(&container_name, exec_options)
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to create exec: {}", e)))?;

        let start_result = self
            .client
            .start_exec(&exec.id, None)
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to start exec: {}", e)))?;

        // Collect output
        let mut stdout = String::new();
        let mut stderr = String::new();

        if let StartExecResults::Attached { mut output, .. } = start_result {
            while let Some(chunk) = output.next().await {
                match chunk {
                    Ok(bollard::container::LogOutput::StdOut { message }) => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    Ok(bollard::container::LogOutput::StdErr { message }) => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    Err(e) => {
                        warn!("exec output error: {}", e);
                    }
                    _ => {}
                }
            }
        }

        // Get exit code
        let inspect = self
            .client
            .inspect_exec(&exec.id)
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to inspect exec: {}", e)))?;

        Ok(ExecResult {
            exit_code: inspect.exit_code.unwrap_or(-1) as i32,
            stdout,
            stderr,
        })
    }

    async fn state(&self, id: &ContainerId) -> Result<ContainerState> {
        let container_name = self.container_name(id);

        let inspect = self
            .client
            .inspect_container(&container_name, None)
            .await
            .map_err(|e| {
                if e.to_string().contains("No such container") {
                    CoreError::ContainerNotFound(id.to_string())
                } else {
                    CoreError::Runtime(format!("Failed to inspect container: {}", e))
                }
            })?;

        let state = inspect.state.as_ref();
        let status = state
            .and_then(|s| s.status.as_ref())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let running = state.and_then(|s| s.running).unwrap_or(false);
        let paused = state.and_then(|s| s.paused).unwrap_or(false);

        Ok(Self::docker_state_to_hyperbox(&status, running, paused))
    }

    async fn stats(&self, id: &ContainerId) -> Result<ContainerStats> {
        let container_name = self.container_name(id);

        let options = StatsOptions {
            stream: false,
            one_shot: true,
        };

        let mut stats_stream = self.client.stats(&container_name, Some(options));

        if let Some(Ok(stats)) = stats_stream.next().await {
            let cpu_delta =
                stats.cpu_stats.cpu_usage.total_usage - stats.precpu_stats.cpu_usage.total_usage;
            let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0)
                - stats.precpu_stats.system_cpu_usage.unwrap_or(0);
            let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;

            let cpu_percent = if system_delta > 0 {
                (cpu_delta as f64 / system_delta as f64) * num_cpus * 100.0
            } else {
                0.0
            };

            let memory_usage = stats.memory_stats.usage.unwrap_or(0);
            let memory_limit = stats.memory_stats.limit.unwrap_or(1);
            let memory_percent = (memory_usage as f64 / memory_limit as f64) * 100.0;

            // Network stats
            let (rx_bytes, tx_bytes, rx_packets, tx_packets) = stats
                .networks
                .as_ref()
                .map(|networks| {
                    networks
                        .values()
                        .fold((0u64, 0u64, 0u64, 0u64), |acc, net| {
                            (
                                acc.0 + net.rx_bytes,
                                acc.1 + net.tx_bytes,
                                acc.2 + net.rx_packets,
                                acc.3 + net.tx_packets,
                            )
                        })
                })
                .unwrap_or((0, 0, 0, 0));

            // Block I/O stats
            let (read_bytes, write_bytes) = stats
                .blkio_stats
                .io_service_bytes_recursive
                .as_ref()
                .map(|io| {
                    io.iter()
                        .fold((0u64, 0u64), |acc, stat| match stat.op.as_str() {
                            "read" | "Read" => (acc.0 + stat.value, acc.1),
                            "write" | "Write" => (acc.0, acc.1 + stat.value),
                            _ => acc,
                        })
                })
                .unwrap_or((0, 0));

            // Get cache from memory stats - match on V1/V2 cgroups
            let cache_bytes = stats
                .memory_stats
                .stats
                .as_ref()
                .map(|s| match s {
                    bollard::container::MemoryStatsStats::V1(v1) => v1.cache,
                    bollard::container::MemoryStatsStats::V2(v2) => v2.inactive_file,
                })
                .unwrap_or(0);

            return Ok(ContainerStats {
                container_id: id.clone(),
                timestamp: chrono::Utc::now(),
                cpu: CpuStats {
                    usage_percent: cpu_percent,
                    total_usage_ns: stats.cpu_stats.cpu_usage.total_usage,
                    system_usage_ns: stats.cpu_stats.system_cpu_usage.unwrap_or(0),
                    num_cpus: num_cpus as u32,
                },
                memory: MemoryStats {
                    used_bytes: memory_usage,
                    available_bytes: memory_limit.saturating_sub(memory_usage),
                    limit_bytes: memory_limit,
                    cache_bytes,
                    usage_percent: memory_percent,
                },
                network: NetworkStats {
                    rx_bytes,
                    tx_bytes,
                    rx_packets,
                    tx_packets,
                    rx_errors: 0,
                    tx_errors: 0,
                },
                block_io: BlockIoStats {
                    read_bytes,
                    write_bytes,
                    read_ops: 0,
                    write_ops: 0,
                },
                pids: stats.pids_stats.current.unwrap_or(0),
            });
        }

        Err(CoreError::Runtime("Failed to get container stats".to_string()))
    }

    async fn logs(
        &self,
        id: &ContainerId,
        opts: LogOptions,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>> {
        let container_name = self.container_name(id);

        let options = LogsOptions::<String> {
            follow: opts.follow,
            stdout: opts.stdout,
            stderr: opts.stderr,
            since: opts.since.map(|t| t.timestamp()).unwrap_or(0),
            until: opts.until.map(|t| t.timestamp()).unwrap_or(0),
            timestamps: opts.timestamps,
            tail: opts
                .tail
                .map(|n| n.to_string())
                .unwrap_or_else(|| "all".to_string()),
        };

        let stream = self.client.logs(&container_name, Some(options));

        // Convert the stream to an AsyncRead
        let reader = tokio_util::io::StreamReader::new(stream.map(|result| {
            result
                .map(|output| {
                    let bytes = match output {
                        bollard::container::LogOutput::StdOut { message } => message,
                        bollard::container::LogOutput::StdErr { message } => message,
                        bollard::container::LogOutput::StdIn { message } => message,
                        bollard::container::LogOutput::Console { message } => message,
                    };
                    Ok::<_, std::io::Error>(bytes)
                })
                .unwrap_or_else(|e| {
                    Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
                })
        }));

        Ok(Box::new(reader))
    }

    async fn attach(
        &self,
        _id: &ContainerId,
    ) -> Result<(
        Box<dyn AsyncWrite + Send + Unpin>,
        Box<dyn AsyncRead + Send + Unpin>,
        Box<dyn AsyncRead + Send + Unpin>,
    )> {
        // TODO: Implement container attach with stdin/stdout/stderr
        Err(CoreError::Internal("Attach not yet implemented".to_string()))
    }

    async fn list(&self) -> Result<Vec<(ContainerId, ContainerState)>> {
        let options = ListContainersOptions::<String> {
            all: true,
            filters: {
                let mut filters = HashMap::new();
                filters.insert("name".to_string(), vec![format!("{}*", self.name_prefix)]);
                filters
            },
            ..Default::default()
        };

        let containers = self
            .client
            .list_containers(Some(options))
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to list containers: {}", e)))?;

        let mut result = Vec::new();
        for container in containers {
            if let Some(names) = container.names {
                for name in names {
                    if let Some(id) = self.parse_container_id(&name) {
                        let state = Self::docker_state_to_hyperbox(
                            &container.state.clone().unwrap_or_default(),
                            container.state.as_deref() == Some("running"),
                            false,
                        );
                        result.push((id, state));
                        break;
                    }
                }
            }
        }

        Ok(result)
    }

    async fn wait(&self, id: &ContainerId) -> Result<i32> {
        let container_name = self.container_name(id);

        let options = WaitContainerOptions {
            condition: "not-running",
        };

        let mut stream = self.client.wait_container(&container_name, Some(options));

        if let Some(result) = stream.next().await {
            let response = result.map_err(|e| CoreError::Runtime(format!("Wait failed: {}", e)))?;
            return Ok(response.status_code as i32);
        }

        Err(CoreError::Runtime("Wait stream ended unexpectedly".to_string()))
    }

    async fn checkpoint(&self, _id: &ContainerId, _checkpoint_path: &Path) -> Result<CheckpointId> {
        // Docker checkpoint requires experimental mode
        Err(CoreError::Internal("Checkpoint requires Docker experimental mode".to_string()))
    }

    async fn restore(&self, _checkpoint_path: &Path, _spec: ContainerSpec) -> Result<ContainerId> {
        Err(CoreError::Internal("Restore requires Docker experimental mode".to_string()))
    }

    async fn update(&self, id: &ContainerId, resources: ResourceLimits) -> Result<()> {
        let container_name = self.container_name(id);

        let update_options = bollard::container::UpdateContainerOptions::<String> {
            memory: resources.memory_bytes.map(|m| m as i64),
            cpu_shares: resources.cpu_millicores.map(|c| (c * 1024 / 1000) as isize),
            pids_limit: resources.pids_limit.map(|p| p as i64),
            ..Default::default()
        };

        self.client
            .update_container(&container_name, update_options)
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to update container: {}", e)))?;

        Ok(())
    }

    async fn top(&self, id: &ContainerId) -> Result<Vec<ProcessInfo>> {
        let container_name = self.container_name(id);

        let top = self
            .client
            .top_processes(&container_name, Some(bollard::container::TopOptions { ps_args: "aux" }))
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to get top: {}", e)))?;

        let mut processes = Vec::new();

        if let (Some(titles), Some(procs)) = (top.titles, top.processes) {
            let pid_idx = titles.iter().position(|t| t.to_uppercase() == "PID");
            let cmd_idx = titles
                .iter()
                .position(|t| t.to_uppercase() == "CMD" || t.to_uppercase() == "COMMAND");

            for proc in procs {
                let pid = pid_idx
                    .and_then(|i| proc.get(i))
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(0);
                let cmd = cmd_idx
                    .and_then(|i| proc.get(i))
                    .cloned()
                    .unwrap_or_default();

                processes.push(ProcessInfo {
                    pid,
                    ppid: 0,
                    cpu_percent: 0.0,
                    memory_bytes: 0,
                    command: cmd,
                });
            }
        }

        Ok(processes)
    }

    async fn pull_image(&self, image: &ImageRef) -> Result<()> {
        let options = CreateImageOptions {
            from_image: format!("{}/{}", image.registry, image.repository),
            tag: image.tag.clone(),
            ..Default::default()
        };

        info!(image = %image, "Pulling image");

        let mut stream = self.client.create_image(Some(options), None, None);

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        debug!(status = %status, "Image pull progress");
                    }
                    if let Some(err) = info.error {
                        error!(error = %err, "Image pull error");
                        return Err(CoreError::Runtime(format!("Pull failed: {}", err)));
                    }
                }
                Err(e) => {
                    return Err(CoreError::Runtime(format!("Pull failed: {}", e)));
                }
            }
        }

        info!(image = %image, "Image pulled successfully");
        Ok(())
    }

    async fn image_exists(&self, image: &str) -> Result<bool> {
        let options = ListImagesOptions::<String> {
            all: false,
            filters: {
                let mut filters = HashMap::new();
                filters.insert("reference".to_string(), vec![image.to_string()]);
                filters
            },
            ..Default::default()
        };

        match self.client.list_images(Some(options)).await {
            Ok(images) => Ok(!images.is_empty()),
            Err(e) => Err(CoreError::Runtime(format!("Failed to list images: {}", e))),
        }
    }

    async fn list_images(&self) -> Result<Vec<super::traits::ImageInfo>> {
        let options = ListImagesOptions::<String> {
            all: false,
            ..Default::default()
        };

        let images = self
            .client
            .list_images(Some(options))
            .await
            .map_err(|e| CoreError::Runtime(format!("Failed to list images: {}", e)))?;

        Ok(images
            .into_iter()
            .map(|img| super::traits::ImageInfo {
                id: img.id,
                tags: img.repo_tags,
                size: img.size as u64,
                created: chrono::DateTime::from_timestamp(img.created, 0)
                    .unwrap_or_else(|| chrono::Utc::now()),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_conversion() {
        assert!(matches!(
            DockerRuntime::docker_state_to_hyperbox("running", true, false),
            ContainerState::Running
        ));
        assert!(matches!(
            DockerRuntime::docker_state_to_hyperbox("paused", false, true),
            ContainerState::Paused
        ));
        assert!(matches!(
            DockerRuntime::docker_state_to_hyperbox("exited", false, false),
            ContainerState::Exited
        ));
    }
}
