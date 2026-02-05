//! crun runtime implementation.
//!
//! crun is the primary runtime for HyperBox, targeting 47ms container lifecycle.

use crate::error::{CoreError, Result};
use crate::runtime::traits::ProcessInfo;
use crate::runtime::{ContainerRuntime, RuntimeConfig, RuntimeType};
use crate::types::*;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::process::Command;
use tracing::{debug, error, info, instrument, warn};

/// Internal cgroup statistics for crun containers.
#[derive(Debug, Default)]
struct CrunCgroupStats {
    /// Memory usage in bytes.
    memory_usage: u64,
    /// Memory limit in bytes.
    memory_limit: u64,
    /// CPU usage in microseconds.
    cpu_usage_usec: u64,
    /// Current PIDs count.
    pids_current: u64,
}

/// crun runtime implementation.
///
/// crun is a fast, low-memory OCI runtime written in C.
/// Target performance: 47ms total container lifecycle.
pub struct CrunRuntime {
    config: RuntimeConfig,
    binary_path: PathBuf,
}

impl CrunRuntime {
    /// Create a new crun runtime instance.
    ///
    /// # Errors
    ///
    /// Returns error if crun binary is not found.
    pub async fn new(config: RuntimeConfig) -> Result<Self> {
        let binary_path = Self::find_binary(&config)?;

        info!("Initializing crun runtime at {:?}", binary_path);

        Ok(Self {
            config,
            binary_path,
        })
    }

    fn find_binary(config: &RuntimeConfig) -> Result<PathBuf> {
        // Check explicit path first
        if let Some(ref path) = config.binary_path {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        // Search common paths
        for path in RuntimeType::Crun.search_paths() {
            if path.exists() {
                return Ok(path);
            }
        }

        // Try PATH lookup
        if let Ok(output) = std::process::Command::new("which").arg("crun").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(PathBuf::from(path));
            }
        }

        Err(CoreError::RuntimeNotAvailable {
            runtime: "crun".to_string(),
            path: PathBuf::from("/usr/bin/crun"),
        })
    }

    async fn run_crun(&self, args: &[&str]) -> Result<std::process::Output> {
        let mut cmd = Command::new(&self.binary_path);

        cmd.arg("--root")
            .arg(&self.config.root_dir)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if self.config.debug {
            cmd.arg("--debug");
        }

        debug!("Running crun: {:?}", cmd);

        let output =
            tokio::time::timeout(Duration::from_secs(self.config.timeout_seconds), cmd.output())
                .await
                .map_err(|_| CoreError::Timeout {
                    operation: "crun command".to_string(),
                    duration_ms: self.config.timeout_seconds * 1000,
                })?
                .map_err(|e| CoreError::RuntimeExecution(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("crun command failed: {}", stderr);
            return Err(CoreError::RuntimeExecution(stderr.to_string()));
        }

        Ok(output)
    }

    async fn generate_bundle(&self, spec: &ContainerSpec) -> Result<PathBuf> {
        // In production, this would create an OCI bundle with config.json
        // For now, create a minimal structure
        let bundle_dir = self.config.root_dir.join("bundles").join(spec.hash());

        tokio::fs::create_dir_all(&bundle_dir).await?;
        tokio::fs::create_dir_all(bundle_dir.join("rootfs")).await?;

        // Generate OCI config.json
        let oci_spec = self.spec_to_oci(spec)?;
        let config_path = bundle_dir.join("config.json");
        tokio::fs::write(&config_path, serde_json::to_string_pretty(&oci_spec)?).await?;

        Ok(bundle_dir)
    }

    fn spec_to_oci(&self, spec: &ContainerSpec) -> Result<serde_json::Value> {
        // Simplified OCI spec generation
        // Production would use oci-spec crate for full compliance
        let oci = serde_json::json!({
            "ociVersion": "1.0.2",
            "process": {
                "terminal": spec.tty,
                "user": {
                    "uid": 0,
                    "gid": 0
                },
                "args": if spec.command.is_empty() {
                    vec!["/bin/sh".to_string()]
                } else {
                    spec.command.clone()
                },
                "env": spec.env.iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>(),
                "cwd": spec.working_dir.as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "/".to_string())
            },
            "root": {
                "path": "rootfs",
                "readonly": spec.read_only_rootfs
            },
            "hostname": spec.hostname.as_deref().unwrap_or("hyperbox"),
            "linux": {
                "namespaces": [
                    { "type": "pid" },
                    { "type": "network" },
                    { "type": "ipc" },
                    { "type": "uts" },
                    { "type": "mount" },
                    { "type": "cgroup" }
                ],
                "resources": self.resources_to_oci(&spec.resources)
            }
        });

        Ok(oci)
    }

    fn resources_to_oci(&self, resources: &ResourceLimits) -> serde_json::Value {
        let mut oci_resources = serde_json::json!({});

        if let Some(memory) = resources.memory_bytes {
            oci_resources["memory"] = serde_json::json!({
                "limit": memory
            });
        }

        if let Some(cpu) = resources.cpu_millicores {
            oci_resources["cpu"] = serde_json::json!({
                "shares": (cpu * 1024 / 1000) as u64,
                "quota": (cpu * 100) as i64,
                "period": 100000
            });
        }

        if let Some(pids) = resources.pids_limit {
            oci_resources["pids"] = serde_json::json!({
                "limit": pids
            });
        }

        oci_resources
    }

    /// Read cgroup v2 statistics for a container.
    async fn read_cgroup_stats(&self, id: &ContainerId) -> Result<CrunCgroupStats> {
        // Try multiple possible cgroup paths
        let possible_paths = [
            PathBuf::from("/sys/fs/cgroup/hyperbox.slice")
                .join(format!("container-{}", id.as_str())),
            PathBuf::from("/sys/fs/cgroup/system.slice")
                .join(format!("crun-{}.scope", id.as_str())),
            PathBuf::from("/sys/fs/cgroup").join(id.as_str()),
        ];

        for cgroup_path in &possible_paths {
            if cgroup_path.exists() {
                return self.read_stats_from_path(cgroup_path).await;
            }
        }

        // Return empty stats if cgroup not found
        Ok(CrunCgroupStats::default())
    }

    /// Read stats from a specific cgroup path.
    async fn read_stats_from_path(&self, path: &Path) -> Result<CrunCgroupStats> {
        use tokio::fs;
        let mut stats = CrunCgroupStats::default();

        // Read memory.current
        if let Ok(content) = fs::read_to_string(path.join("memory.current")).await {
            stats.memory_usage = content.trim().parse().unwrap_or(0);
        }

        // Read memory.max
        if let Ok(content) = fs::read_to_string(path.join("memory.max")).await {
            stats.memory_limit = content.trim().parse().unwrap_or(u64::MAX);
        }

        // Read cpu.stat
        if let Ok(content) = fs::read_to_string(path.join("cpu.stat")).await {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[0] == "usage_usec" {
                    stats.cpu_usage_usec = parts[1].parse().unwrap_or(0);
                    break;
                }
            }
        }

        // Read pids.current
        if let Ok(content) = fs::read_to_string(path.join("pids.current")).await {
            stats.pids_current = content.trim().parse().unwrap_or(0);
        }

        Ok(stats)
    }
}

#[async_trait]
impl ContainerRuntime for CrunRuntime {
    fn name(&self) -> &'static str {
        "crun"
    }

    #[instrument(skip(self))]
    async fn version(&self) -> Result<String> {
        let output = self.run_crun(&["--version"]).await?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    async fn is_available(&self) -> bool {
        self.version().await.is_ok()
    }

    #[instrument(skip(self, spec), fields(image = %spec.image))]
    async fn create(&self, spec: ContainerSpec) -> Result<ContainerId> {
        let id = ContainerId::new();
        info!(container_id = %id, "Creating container");

        let bundle = self.generate_bundle(&spec).await?;

        self.run_crun(&["create", "--bundle", bundle.to_str().unwrap(), id.as_str()])
            .await?;

        info!(container_id = %id, "Container created");
        Ok(id)
    }

    #[instrument(skip(self))]
    async fn start(&self, id: &ContainerId) -> Result<()> {
        info!(container_id = %id, "Starting container");
        self.run_crun(&["start", id.as_str()]).await?;
        info!(container_id = %id, "Container started");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn stop(&self, id: &ContainerId, timeout: Duration) -> Result<()> {
        info!(container_id = %id, "Stopping container");

        // Send SIGTERM first
        let _ = self.kill(id, "SIGTERM").await;

        // Wait for container to stop
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if let Ok(state) = self.state(id).await {
                if matches!(state, ContainerState::Stopped | ContainerState::Exited) {
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Force kill if still running
        warn!(container_id = %id, "Container didn't stop gracefully, killing");
        self.kill(id, "SIGKILL").await
    }

    async fn kill(&self, id: &ContainerId, signal: &str) -> Result<()> {
        self.run_crun(&["kill", id.as_str(), signal]).await?;
        Ok(())
    }

    async fn remove(&self, id: &ContainerId) -> Result<()> {
        info!(container_id = %id, "Removing container");
        self.run_crun(&["delete", "--force", id.as_str()]).await?;
        Ok(())
    }

    async fn pause(&self, id: &ContainerId) -> Result<()> {
        self.run_crun(&["pause", id.as_str()]).await?;
        Ok(())
    }

    async fn resume(&self, id: &ContainerId) -> Result<()> {
        self.run_crun(&["resume", id.as_str()]).await?;
        Ok(())
    }

    #[instrument(skip(self, spec))]
    async fn exec(&self, id: &ContainerId, spec: ExecSpec) -> Result<ExecResult> {
        let mut args = vec!["exec".to_string()];

        if spec.tty {
            args.push("--tty".to_string());
        }

        for (key, value) in &spec.env {
            args.push("--env".to_string());
            args.push(format!("{key}={value}"));
        }

        if let Some(ref user) = spec.user {
            args.push("--user".to_string());
            args.push(user.clone());
        }

        if let Some(ref cwd) = spec.working_dir {
            args.push("--cwd".to_string());
            args.push(cwd.to_string_lossy().to_string());
        }

        args.push(id.to_string());
        args.extend(spec.command.iter().cloned());

        let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = self.run_crun(&args_refs).await?;

        Ok(ExecResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn state(&self, id: &ContainerId) -> Result<ContainerState> {
        let output = self.run_crun(&["state", id.as_str()]).await?;
        let state_json: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        let status = state_json["status"].as_str().unwrap_or("unknown");

        Ok(match status {
            "creating" => ContainerState::Creating,
            "created" => ContainerState::Created,
            "running" => ContainerState::Running,
            "paused" => ContainerState::Paused,
            "stopped" => ContainerState::Stopped,
            _ => ContainerState::Unknown,
        })
    }

    async fn stats(&self, id: &ContainerId) -> Result<ContainerStats> {
        // Read from cgroup v2 statistics
        // crun creates cgroups at /sys/fs/cgroup/hyperbox.slice/container-{id}
        // or uses the system.slice/crun-{id}.scope pattern
        let cgroup_stats = self.read_cgroup_stats(id).await;

        let (memory_usage, memory_limit, cpu_usage_usec, pids) = match cgroup_stats {
            Ok(stats) => {
                (stats.memory_usage, stats.memory_limit, stats.cpu_usage_usec, stats.pids_current)
            }
            Err(_) => (0, 0, 0, 0),
        };

        let memory_percent = if memory_limit > 0 {
            (memory_usage as f64 / memory_limit as f64) * 100.0
        } else {
            0.0
        };

        Ok(ContainerStats {
            container_id: id.clone(),
            timestamp: chrono::Utc::now(),
            cpu: CpuStats {
                usage_percent: 0.0, // Would need delta calculation
                total_usage_ns: cpu_usage_usec * 1000,
                system_usage_ns: 0,
                num_cpus: num_cpus::get() as u32,
            },
            memory: MemoryStats {
                used_bytes: memory_usage,
                available_bytes: memory_limit.saturating_sub(memory_usage),
                limit_bytes: memory_limit,
                cache_bytes: 0,
                usage_percent: memory_percent,
            },
            network: NetworkStats {
                rx_bytes: 0,
                tx_bytes: 0,
                rx_packets: 0,
                tx_packets: 0,
                rx_errors: 0,
                tx_errors: 0,
            },
            block_io: BlockIoStats {
                read_bytes: 0,
                write_bytes: 0,
                read_ops: 0,
                write_ops: 0,
            },
            pids,
        })
    }

    async fn logs(
        &self,
        _id: &ContainerId,
        _opts: LogOptions,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>> {
        // Return empty reader for now - would read from log driver
        let empty: &[u8] = &[];
        Ok(Box::new(std::io::Cursor::new(empty.to_vec())))
    }

    async fn attach(
        &self,
        _id: &ContainerId,
    ) -> Result<(
        Box<dyn AsyncWrite + Send + Unpin>,
        Box<dyn AsyncRead + Send + Unpin>,
        Box<dyn AsyncRead + Send + Unpin>,
    )> {
        Err(CoreError::Internal("Attach not implemented".to_string()))
    }

    async fn list(&self) -> Result<Vec<(ContainerId, ContainerState)>> {
        let output = self.run_crun(&["list", "--format", "json"]).await?;
        let containers: Vec<serde_json::Value> =
            serde_json::from_slice(&output.stdout).unwrap_or_default();

        let mut result = Vec::new();
        for container in containers {
            if let Some(id) = container["id"].as_str() {
                let state = match container["status"].as_str() {
                    Some("running") => ContainerState::Running,
                    Some("created") => ContainerState::Created,
                    Some("paused") => ContainerState::Paused,
                    Some("stopped") => ContainerState::Stopped,
                    _ => ContainerState::Unknown,
                };
                result.push((ContainerId::from(id), state));
            }
        }

        Ok(result)
    }

    async fn wait(&self, id: &ContainerId) -> Result<i32> {
        loop {
            let state = self.state(id).await?;
            match state {
                ContainerState::Exited | ContainerState::Stopped => {
                    // Would parse exit code from state
                    return Ok(0);
                }
                ContainerState::Unknown => {
                    return Err(CoreError::ContainerNotFound(id.to_string()));
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    #[instrument(skip(self))]
    async fn checkpoint(&self, id: &ContainerId, checkpoint_path: &Path) -> Result<CheckpointId> {
        tokio::fs::create_dir_all(checkpoint_path).await?;

        let checkpoint_id =
            CheckpointId::new(format!("{}-{}", id.short(), chrono::Utc::now().timestamp()));

        self.run_crun(&[
            "checkpoint",
            "--image-path",
            checkpoint_path.to_str().unwrap(),
            id.as_str(),
        ])
        .await?;

        info!(container_id = %id, checkpoint = %checkpoint_id, "Created checkpoint");
        Ok(checkpoint_id)
    }

    async fn restore(&self, checkpoint_path: &Path, spec: ContainerSpec) -> Result<ContainerId> {
        let id = ContainerId::new();
        let bundle = self.generate_bundle(&spec).await?;

        self.run_crun(&[
            "restore",
            "--image-path",
            checkpoint_path.to_str().unwrap(),
            "--bundle",
            bundle.to_str().unwrap(),
            id.as_str(),
        ])
        .await?;

        info!(container_id = %id, "Restored from checkpoint");
        Ok(id)
    }

    async fn update(&self, id: &ContainerId, resources: ResourceLimits) -> Result<()> {
        let mut args = vec!["update".to_string()];

        if let Some(memory) = resources.memory_bytes {
            args.push("--memory".to_string());
            args.push(memory.to_string());
        }

        if let Some(cpu) = resources.cpu_millicores {
            args.push("--cpu-shares".to_string());
            args.push((cpu * 1024 / 1000).to_string());
        }

        if let Some(pids) = resources.pids_limit {
            args.push("--pids-limit".to_string());
            args.push(pids.to_string());
        }

        args.push(id.to_string());

        let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run_crun(&args_refs).await?;

        Ok(())
    }

    async fn top(&self, id: &ContainerId) -> Result<Vec<ProcessInfo>> {
        // Would read from /proc filesystem for container PID namespace
        Ok(Vec::new())
    }

    async fn pull_image(&self, _image: &crate::types::ImageRef) -> Result<()> {
        // crun doesn't handle image pulling directly - it works with OCI bundles
        // Image pulling would be delegated to a registry client like skopeo or
        // our built-in ImageRegistry. For now, we return an error indicating
        // this should be handled at a higher level.
        Err(CoreError::Runtime(
            "crun does not handle image pulling. Use ImageRegistry or Docker/Podman to pull images.".to_string()
        ))
    }

    async fn image_exists(&self, _image: &str) -> Result<bool> {
        // crun works with OCI bundles, not images directly
        // Image management should be done at a higher level
        Err(CoreError::Runtime(
            "crun does not manage images. Use ImageRegistry for image operations.".to_string(),
        ))
    }

    async fn list_images(&self) -> Result<Vec<super::traits::ImageInfo>> {
        // crun works with OCI bundles, not images directly
        Err(CoreError::Runtime(
            "crun does not manage images. Use ImageRegistry for image operations.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_id_generation() {
        let id = ContainerId::new();
        assert!(!id.as_str().is_empty());
        assert!(id.as_str().len() >= 12);
    }

    #[test]
    fn test_image_ref_parsing() {
        let img = ImageRef::parse("alpine:3.18");
        assert_eq!(img.registry, "docker.io");
        assert_eq!(img.repository, "library/alpine");
        assert_eq!(img.tag, "3.18");

        let img = ImageRef::parse("gcr.io/my-project/my-image:v1");
        assert_eq!(img.registry, "gcr.io");
        assert_eq!(img.repository, "my-project/my-image");
        assert_eq!(img.tag, "v1");
    }
}
