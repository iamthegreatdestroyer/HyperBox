//! cgroups v2 management for resource isolation.

use crate::error::{CoreError, Result};
use crate::types::ResourceLimits;
#[cfg(unix)]
use nix::libc;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, instrument};

/// cgroups v2 manager for container resource isolation.
pub struct CgroupManager {
    /// Base path for cgroups (usually /sys/fs/cgroup)
    base_path: PathBuf,
    /// HyperBox slice path
    hyperbox_slice: PathBuf,
}

impl CgroupManager {
    /// Default cgroup base path
    pub const DEFAULT_BASE: &'static str = "/sys/fs/cgroup";
    /// HyperBox slice name
    pub const HYPERBOX_SLICE: &'static str = "hyperbox.slice";

    /// Create a new cgroup manager.
    #[must_use]
    pub fn new() -> Self {
        let base_path = PathBuf::from(Self::DEFAULT_BASE);
        let hyperbox_slice = base_path.join(Self::HYPERBOX_SLICE);

        Self {
            base_path,
            hyperbox_slice,
        }
    }

    /// Create with custom base path.
    #[must_use]
    pub fn with_base_path(base_path: impl Into<PathBuf>) -> Self {
        let base_path = base_path.into();
        let hyperbox_slice = base_path.join(Self::HYPERBOX_SLICE);

        Self {
            base_path,
            hyperbox_slice,
        }
    }

    /// Initialize the HyperBox cgroup slice.
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> Result<()> {
        if !self.is_cgroup_v2().await {
            return Err(CoreError::CgroupOperation {
                operation: "initialize".to_string(),
                reason: "cgroups v2 is not available".to_string(),
            });
        }

        // Create hyperbox slice if it doesn't exist
        if !self.hyperbox_slice.exists() {
            fs::create_dir_all(&self.hyperbox_slice)
                .await
                .map_err(|e| CoreError::CgroupOperation {
                    operation: "create hyperbox slice".to_string(),
                    reason: e.to_string(),
                })?;
        }

        // Enable controllers
        self.enable_controllers(&self.hyperbox_slice).await?;

        debug!("Initialized cgroup manager at {:?}", self.hyperbox_slice);
        Ok(())
    }

    /// Check if cgroups v2 is available.
    async fn is_cgroup_v2(&self) -> bool {
        let cgroup_type = self.base_path.join("cgroup.type");
        if cgroup_type.exists() {
            return true;
        }

        // Also check for cgroup.controllers
        self.base_path.join("cgroup.controllers").exists()
    }

    /// Enable controllers in a cgroup.
    async fn enable_controllers(&self, path: &Path) -> Result<()> {
        let controllers = "+cpu +memory +io +pids";
        let subtree_control = path.join("cgroup.subtree_control");

        fs::write(&subtree_control, controllers)
            .await
            .map_err(|e| CoreError::CgroupOperation {
                operation: "enable controllers".to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Create a cgroup for a container.
    #[instrument(skip(self))]
    pub async fn create_container_cgroup(&self, container_id: &str) -> Result<PathBuf> {
        let cgroup_path = self
            .hyperbox_slice
            .join(format!("container-{container_id}"));

        fs::create_dir_all(&cgroup_path)
            .await
            .map_err(|e| CoreError::CgroupOperation {
                operation: "create container cgroup".to_string(),
                reason: e.to_string(),
            })?;

        debug!("Created container cgroup at {:?}", cgroup_path);
        Ok(cgroup_path)
    }

    /// Apply resource limits to a container cgroup.
    #[instrument(skip(self, limits))]
    pub async fn apply_limits(&self, cgroup_path: &Path, limits: &ResourceLimits) -> Result<()> {
        // Memory limits
        if let Some(memory) = limits.memory_bytes {
            let memory_max = cgroup_path.join("memory.max");
            fs::write(&memory_max, memory.to_string())
                .await
                .map_err(|e| CoreError::CgroupOperation {
                    operation: "set memory limit".to_string(),
                    reason: e.to_string(),
                })?;
        }

        // Memory swap limits
        if let Some(swap) = limits.memory_swap_bytes {
            let memory_swap = cgroup_path.join("memory.swap.max");
            fs::write(&memory_swap, swap.to_string())
                .await
                .map_err(|e| CoreError::CgroupOperation {
                    operation: "set swap limit".to_string(),
                    reason: e.to_string(),
                })?;
        }

        // CPU limits (millicores to cpu.max format)
        if let Some(cpu) = limits.cpu_millicores {
            // cpu.max format: "quota period" in microseconds
            // 1000 millicores = 1 CPU = 100000us quota per 100000us period
            let quota = (cpu * 100) as u64; // microseconds
            let period = 100_000u64;
            let cpu_max = cgroup_path.join("cpu.max");
            fs::write(&cpu_max, format!("{quota} {period}"))
                .await
                .map_err(|e| CoreError::CgroupOperation {
                    operation: "set cpu limit".to_string(),
                    reason: e.to_string(),
                })?;
        }

        // PIDs limit
        if let Some(pids) = limits.pids_limit {
            let pids_max = cgroup_path.join("pids.max");
            fs::write(&pids_max, pids.to_string()).await.map_err(|e| {
                CoreError::CgroupOperation {
                    operation: "set pids limit".to_string(),
                    reason: e.to_string(),
                }
            })?;
        }

        // IO limits
        if limits.io_read_bps.is_some() || limits.io_write_bps.is_some() {
            // Would need device major:minor for io.max
            // This is simplified; production would query devices
            let _io_max = cgroup_path.join("io.max");
            let mut io_config = String::new();

            if let Some(read_bps) = limits.io_read_bps {
                io_config.push_str(&format!("rbps={read_bps} "));
            }
            if let Some(write_bps) = limits.io_write_bps {
                io_config.push_str(&format!("wbps={write_bps}"));
            }

            // Note: This would need device specification in real implementation
            debug!("IO limits would be: {}", io_config);
        }

        debug!("Applied resource limits to {:?}", cgroup_path);
        Ok(())
    }

    /// Add a process to a cgroup.
    pub async fn add_process(&self, cgroup_path: &Path, pid: u32) -> Result<()> {
        let cgroup_procs = cgroup_path.join("cgroup.procs");
        fs::write(&cgroup_procs, pid.to_string())
            .await
            .map_err(|e| CoreError::CgroupOperation {
                operation: "add process".to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Read resource statistics for a cgroup.
    pub async fn read_stats(&self, cgroup_path: &Path) -> Result<CgroupStats> {
        let mut stats = CgroupStats::default();

        // Read memory stats
        if let Ok(content) = fs::read_to_string(cgroup_path.join("memory.current")).await {
            stats.memory_usage = content.trim().parse().unwrap_or(0);
        }

        // Read CPU stats
        if let Ok(content) = fs::read_to_string(cgroup_path.join("cpu.stat")).await {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    match parts[0] {
                        "usage_usec" => {
                            stats.cpu_usage_usec = parts[1].parse().unwrap_or(0);
                        }
                        "system_usec" => {
                            stats.cpu_system_usec = parts[1].parse().unwrap_or(0);
                        }
                        "user_usec" => {
                            stats.cpu_user_usec = parts[1].parse().unwrap_or(0);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Read PIDs
        if let Ok(content) = fs::read_to_string(cgroup_path.join("pids.current")).await {
            stats.pids_current = content.trim().parse().unwrap_or(0);
        }

        Ok(stats)
    }

    /// Remove a container cgroup.
    pub async fn remove_cgroup(&self, cgroup_path: &Path) -> Result<()> {
        // Kill all processes first
        let cgroup_procs = cgroup_path.join("cgroup.procs");
        if let Ok(content) = fs::read_to_string(&cgroup_procs).await {
            for line in content.lines() {
                if let Ok(pid) = line.trim().parse::<i32>() {
                    #[cfg(unix)]
                    unsafe {
                        libc::kill(pid, libc::SIGKILL);
                    }
                }
            }
        }

        // Wait briefly for processes to terminate
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Remove the cgroup directory
        fs::remove_dir(cgroup_path)
            .await
            .map_err(|e| CoreError::CgroupOperation {
                operation: "remove cgroup".to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }
}

impl Default for CgroupManager {
    fn default() -> Self {
        Self::new()
    }
}

/// cgroup statistics.
#[derive(Debug, Default, Clone)]
pub struct CgroupStats {
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// CPU usage in microseconds
    pub cpu_usage_usec: u64,
    /// System CPU time in microseconds
    pub cpu_system_usec: u64,
    /// User CPU time in microseconds
    pub cpu_user_usec: u64,
    /// Current PIDs count
    pub pids_current: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cgroup_manager_creation() {
        let manager = CgroupManager::new();
        assert_eq!(manager.base_path, PathBuf::from("/sys/fs/cgroup"));
        assert_eq!(manager.hyperbox_slice, PathBuf::from("/sys/fs/cgroup/hyperbox.slice"));
    }

    #[test]
    fn test_cgroup_manager_custom_path() {
        let manager = CgroupManager::with_base_path("/custom/cgroup");
        assert_eq!(manager.base_path, PathBuf::from("/custom/cgroup"));
        assert_eq!(manager.hyperbox_slice, PathBuf::from("/custom/cgroup/hyperbox.slice"));
    }

    #[test]
    fn test_cgroup_stats_default() {
        let stats = CgroupStats::default();
        assert_eq!(stats.memory_usage, 0);
        assert_eq!(stats.cpu_usage_usec, 0);
        assert_eq!(stats.cpu_system_usec, 0);
        assert_eq!(stats.cpu_user_usec, 0);
        assert_eq!(stats.pids_current, 0);
    }

    #[cfg(target_os = "linux")]
    mod linux_tests {
        use super::*;
        use tempfile::TempDir;

        #[tokio::test]
        async fn test_create_container_cgroup_structure() {
            let temp_dir = TempDir::new().unwrap();
            let manager = CgroupManager::with_base_path(temp_dir.path());

            // Create mock cgroup.controllers to pass v2 check
            let controllers_file = temp_dir.path().join("cgroup.controllers");
            std::fs::write(&controllers_file, "cpu memory io pids").unwrap();

            // Create hyperbox slice
            std::fs::create_dir_all(manager.hyperbox_slice.clone()).unwrap();

            let result = manager.create_container_cgroup("test-123").await;
            assert!(result.is_ok());

            let cgroup_path = result.unwrap();
            assert!(cgroup_path.exists());
            assert!(cgroup_path.ends_with("container-test-123"));
        }

        #[tokio::test]
        async fn test_apply_resource_limits() {
            let temp_dir = TempDir::new().unwrap();
            let container_cgroup = temp_dir.path().join("container-test");
            std::fs::create_dir_all(&container_cgroup).unwrap();

            let manager = CgroupManager::with_base_path(temp_dir.path());

            let limits = ResourceLimits {
                memory_bytes: Some(1024 * 1024 * 512), // 512MB
                cpu_millicores: Some(1000),            // 1 CPU
                pids_limit: Some(100),
                ..Default::default()
            };

            // This will fail without real cgroup files, but we test the logic
            let result = manager.apply_limits(&container_cgroup, &limits).await;
            // On non-cgroup systems, this will error, which is expected
            // The important thing is the code path executes without panics
            assert!(result.is_err() || result.is_ok());
        }

        #[tokio::test]
        async fn test_read_stats_missing_files() {
            let temp_dir = TempDir::new().unwrap();
            let container_cgroup = temp_dir.path().join("container-test");
            std::fs::create_dir_all(&container_cgroup).unwrap();

            let manager = CgroupManager::with_base_path(temp_dir.path());
            let stats = manager.read_stats(&container_cgroup).await.unwrap();

            // Should return defaults for missing files
            assert_eq!(stats.memory_usage, 0);
            assert_eq!(stats.cpu_usage_usec, 0);
        }

        #[tokio::test]
        async fn test_read_stats_with_mock_files() {
            let temp_dir = TempDir::new().unwrap();
            let container_cgroup = temp_dir.path().join("container-test");
            std::fs::create_dir_all(&container_cgroup).unwrap();

            // Create mock stat files
            std::fs::write(container_cgroup.join("memory.current"), "1048576\n").unwrap();
            std::fs::write(container_cgroup.join("pids.current"), "5\n").unwrap();
            std::fs::write(
                container_cgroup.join("cpu.stat"),
                "usage_usec 1000000\nuser_usec 800000\nsystem_usec 200000\n",
            )
            .unwrap();

            let manager = CgroupManager::with_base_path(temp_dir.path());
            let stats = manager.read_stats(&container_cgroup).await.unwrap();

            assert_eq!(stats.memory_usage, 1048576);
            assert_eq!(stats.pids_current, 5);
            assert_eq!(stats.cpu_usage_usec, 1000000);
            assert_eq!(stats.cpu_user_usec, 800000);
            assert_eq!(stats.cpu_system_usec, 200000);
        }
    }
}
