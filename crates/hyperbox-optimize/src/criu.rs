//! CRIU (Checkpoint/Restore In Userspace) integration.
//!
//! Enables <100ms warm container starts by checkpointing running
//! containers and restoring them on demand.

use crate::error::{OptimizeError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::fs;
use tracing::{debug, info, warn};

/// Maximum checkpoint age before it's considered stale.
const MAX_CHECKPOINT_AGE: Duration = Duration::from_secs(3600 * 24); // 24 hours

/// CRIU checkpoint manager.
pub struct CriuManager {
    /// Directory for storing checkpoints.
    checkpoint_dir: PathBuf,
    /// Whether CRIU is available.
    available: AtomicBool,
    /// CRIU binary path.
    criu_path: Option<PathBuf>,
}

/// Checkpoint metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Container ID.
    pub container_id: String,
    /// Image name.
    pub image: String,
    /// Checkpoint path.
    pub path: PathBuf,
    /// Creation time.
    pub created_at: DateTime<Utc>,
    /// Size in bytes.
    pub size: u64,
    /// Whether checkpoint includes TCP connections.
    pub includes_tcp: bool,
    /// Whether checkpoint includes file locks.
    pub includes_file_locks: bool,
}

/// CRIU options.
#[derive(Debug, Clone, Default)]
pub struct CriuOptions {
    /// Leave container running after checkpoint.
    pub leave_running: bool,
    /// Include TCP connections in checkpoint.
    pub tcp_established: bool,
    /// Include file locks.
    pub file_locks: bool,
    /// External mount points.
    pub external_mounts: Vec<(PathBuf, PathBuf)>,
    /// Pre-dump for incremental checkpoints.
    pub pre_dump: bool,
}

impl CriuManager {
    /// Create a new CRIU manager.
    pub fn new(checkpoint_dir: impl Into<PathBuf>) -> Self {
        let checkpoint_dir = checkpoint_dir.into();

        Self {
            checkpoint_dir,
            available: AtomicBool::new(false),
            criu_path: None,
        }
    }

    /// Initialize the CRIU manager.
    pub async fn initialize(&mut self) -> Result<()> {
        // Create checkpoint directory
        fs::create_dir_all(&self.checkpoint_dir).await?;

        // Check for CRIU availability
        if let Some(path) = Self::find_criu() {
            self.criu_path = Some(path.clone());

            // Verify CRIU works
            let output = Command::new(&path).arg("check").output();

            match output {
                Ok(output) if output.status.success() => {
                    self.available.store(true, Ordering::Relaxed);
                    info!("CRIU available at {:?}", path);
                }
                Ok(output) => {
                    warn!("CRIU check failed: {}", String::from_utf8_lossy(&output.stderr));
                }
                Err(e) => {
                    warn!("Failed to run CRIU: {}", e);
                }
            }
        } else {
            warn!("CRIU not found in PATH");
        }

        // Clean up stale checkpoints
        self.cleanup_stale().await?;

        Ok(())
    }

    /// Find CRIU binary in PATH.
    fn find_criu() -> Option<PathBuf> {
        // Check common locations
        let paths = [
            "/usr/sbin/criu",
            "/usr/bin/criu",
            "/usr/local/sbin/criu",
            "/usr/local/bin/criu",
        ];

        for path in paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        // Check PATH
        which::which("criu").ok()
    }

    /// Check if CRIU is available.
    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::Relaxed)
    }

    /// Checkpoint a running container.
    pub async fn checkpoint(
        &self,
        container_id: &str,
        image: &str,
        pid: u32,
        options: &CriuOptions,
    ) -> Result<Checkpoint> {
        if !self.is_available() {
            return Err(OptimizeError::CriuNotAvailable {
                reason: "CRIU not installed or not functional".to_string(),
            });
        }

        let criu = self.criu_path.as_ref().unwrap();
        let checkpoint_path = self.checkpoint_dir.join(container_id);

        // Create checkpoint directory
        fs::create_dir_all(&checkpoint_path).await?;

        let start = Instant::now();

        // Build CRIU command
        let mut cmd = Command::new(criu);
        cmd.arg("dump")
            .arg("-t")
            .arg(pid.to_string())
            .arg("-D")
            .arg(&checkpoint_path)
            .arg("-o")
            .arg("dump.log")
            .arg("--shell-job");

        if options.leave_running {
            cmd.arg("--leave-running");
        }

        if options.tcp_established {
            cmd.arg("--tcp-established");
        }

        if options.file_locks {
            cmd.arg("--file-locks");
        }

        for (src, dst) in &options.external_mounts {
            cmd.arg("--ext-mount-map")
                .arg(format!("{}:{}", src.display(), dst.display()));
        }

        let output = cmd.output().map_err(|e| OptimizeError::CheckpointFailed {
            container_id: container_id.to_string(),
            reason: e.to_string(),
        })?;

        if !output.status.success() {
            return Err(OptimizeError::CheckpointFailed {
                container_id: container_id.to_string(),
                reason: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        let duration = start.elapsed();
        debug!("Checkpoint created in {:?}", duration);

        // Calculate checkpoint size
        let size = Self::directory_size(&checkpoint_path).await?;

        // Save metadata
        let checkpoint = Checkpoint {
            container_id: container_id.to_string(),
            image: image.to_string(),
            path: checkpoint_path.clone(),
            created_at: Utc::now(),
            size,
            includes_tcp: options.tcp_established,
            includes_file_locks: options.file_locks,
        };

        let metadata_path = checkpoint_path.join("checkpoint.json");
        let metadata = serde_json::to_string_pretty(&checkpoint)?;
        fs::write(metadata_path, metadata).await?;

        info!(
            "Created checkpoint for {} ({} MB in {:?})",
            container_id,
            size / (1024 * 1024),
            duration
        );

        Ok(checkpoint)
    }

    /// Restore a container from checkpoint.
    pub async fn restore(&self, checkpoint: &Checkpoint) -> Result<u32> {
        if !self.is_available() {
            return Err(OptimizeError::CriuNotAvailable {
                reason: "CRIU not installed or not functional".to_string(),
            });
        }

        if !checkpoint.path.exists() {
            return Err(OptimizeError::CheckpointNotFound {
                path: checkpoint.path.clone(),
            });
        }

        // Check if checkpoint is too old
        let age = Utc::now().signed_duration_since(checkpoint.created_at);
        if age.to_std().unwrap_or(Duration::MAX) > MAX_CHECKPOINT_AGE {
            return Err(OptimizeError::CheckpointExpired {
                container_id: checkpoint.container_id.clone(),
            });
        }

        let criu = self.criu_path.as_ref().unwrap();
        let start = Instant::now();

        let mut cmd = Command::new(criu);
        cmd.arg("restore")
            .arg("-D")
            .arg(&checkpoint.path)
            .arg("-o")
            .arg("restore.log")
            .arg("--shell-job")
            .arg("-d");

        if checkpoint.includes_tcp {
            cmd.arg("--tcp-established");
        }

        if checkpoint.includes_file_locks {
            cmd.arg("--file-locks");
        }

        let output = cmd.output().map_err(|e| OptimizeError::RestoreFailed {
            container_id: checkpoint.container_id.clone(),
            reason: e.to_string(),
        })?;

        if !output.status.success() {
            return Err(OptimizeError::RestoreFailed {
                container_id: checkpoint.container_id.clone(),
                reason: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        let duration = start.elapsed();

        // Parse PID from output
        let pid = Self::parse_restored_pid(&checkpoint.path).await?;

        info!("Restored container {} (PID {}) in {:?}", checkpoint.container_id, pid, duration);

        Ok(pid)
    }

    /// Parse restored process PID.
    async fn parse_restored_pid(checkpoint_path: &Path) -> Result<u32> {
        let restore_log = checkpoint_path.join("restore.log");

        if restore_log.exists() {
            let content = fs::read_to_string(&restore_log).await?;
            // Parse PID from CRIU restore log
            for line in content.lines().rev() {
                if line.contains("Restored") && line.contains("PID") {
                    if let Some(pid_str) = line.split_whitespace().last() {
                        if let Ok(pid) = pid_str.parse() {
                            return Ok(pid);
                        }
                    }
                }
            }
        }

        // Default to reading from pidfile
        let pidfile = checkpoint_path.join("restored.pid");
        if pidfile.exists() {
            let content = fs::read_to_string(&pidfile).await?;
            if let Ok(pid) = content.trim().parse() {
                return Ok(pid);
            }
        }

        Err(OptimizeError::RestoreFailed {
            container_id: String::new(),
            reason: "Could not determine restored PID".to_string(),
        })
    }

    /// Get checkpoint for a container.
    pub async fn get_checkpoint(&self, container_id: &str) -> Result<Option<Checkpoint>> {
        let checkpoint_path = self.checkpoint_dir.join(container_id);
        let metadata_path = checkpoint_path.join("checkpoint.json");

        if !metadata_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&metadata_path).await?;
        let checkpoint: Checkpoint = serde_json::from_str(&content)?;

        Ok(Some(checkpoint))
    }

    /// List all checkpoints.
    pub async fn list_checkpoints(&self) -> Result<Vec<Checkpoint>> {
        let mut checkpoints = Vec::new();
        let mut entries = fs::read_dir(&self.checkpoint_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let metadata_path = path.join("checkpoint.json");
                if metadata_path.exists() {
                    let content = fs::read_to_string(&metadata_path).await?;
                    if let Ok(checkpoint) = serde_json::from_str(&content) {
                        checkpoints.push(checkpoint);
                    }
                }
            }
        }

        Ok(checkpoints)
    }

    /// Delete a checkpoint.
    pub async fn delete_checkpoint(&self, container_id: &str) -> Result<()> {
        let checkpoint_path = self.checkpoint_dir.join(container_id);

        if checkpoint_path.exists() {
            fs::remove_dir_all(&checkpoint_path).await?;
            info!("Deleted checkpoint for {}", container_id);
        }

        Ok(())
    }

    /// Clean up stale checkpoints.
    async fn cleanup_stale(&self) -> Result<()> {
        let checkpoints = self.list_checkpoints().await?;
        let now = Utc::now();
        let mut cleaned = 0;

        for checkpoint in checkpoints {
            let age = now.signed_duration_since(checkpoint.created_at);
            if age.to_std().unwrap_or(Duration::MAX) > MAX_CHECKPOINT_AGE {
                if let Err(e) = self.delete_checkpoint(&checkpoint.container_id).await {
                    warn!("Failed to clean up checkpoint {}: {}", checkpoint.container_id, e);
                } else {
                    cleaned += 1;
                }
            }
        }

        if cleaned > 0 {
            info!("Cleaned up {} stale checkpoints", cleaned);
        }

        Ok(())
    }

    /// Calculate directory size.
    async fn directory_size(path: &Path) -> Result<u64> {
        let mut size = 0u64;
        let mut entries = fs::read_dir(path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                size += metadata.len();
            } else if metadata.is_dir() {
                size += Box::pin(Self::directory_size(&entry.path())).await?;
            }
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_criu_manager_creation() {
        let dir = tempdir().unwrap();
        let manager = CriuManager::new(dir.path());

        // Before initialization, CRIU is not marked as available
        assert!(!manager.is_available());
    }

    #[tokio::test]
    async fn test_criu_manager_initialize() {
        let dir = tempdir().unwrap();
        let mut manager = CriuManager::new(dir.path());

        // Initialize should succeed even if CRIU isn't installed
        let result = manager.initialize().await;
        assert!(result.is_ok());

        // Checkpoint directory should be created
        assert!(dir.path().exists());
    }

    #[tokio::test]
    async fn test_checkpoint_metadata_serialization() {
        let checkpoint = Checkpoint {
            container_id: "test-container-123".to_string(),
            image: "alpine:latest".to_string(),
            path: PathBuf::from("/tmp/checkpoints/test"),
            created_at: Utc::now(),
            size: 1024 * 1024,
            includes_tcp: false,
            includes_file_locks: false,
        };

        let json = serde_json::to_string(&checkpoint).unwrap();
        let deserialized: Checkpoint = serde_json::from_str(&json).unwrap();

        assert_eq!(checkpoint.container_id, deserialized.container_id);
        assert_eq!(checkpoint.image, deserialized.image);
        assert_eq!(checkpoint.size, deserialized.size);
    }

    #[tokio::test]
    async fn test_criu_options_default() {
        let options = CriuOptions::default();

        assert!(!options.leave_running);
        assert!(!options.tcp_established);
        assert!(!options.file_locks);
        assert!(options.external_mounts.is_empty());
        assert!(!options.pre_dump);
    }

    #[tokio::test]
    async fn test_list_checkpoints_empty() {
        let dir = tempdir().unwrap();
        let mut manager = CriuManager::new(dir.path());
        manager.initialize().await.unwrap();

        let checkpoints = manager.list_checkpoints().await.unwrap();
        assert!(checkpoints.is_empty());
    }

    #[tokio::test]
    async fn test_get_checkpoint_not_found() {
        let dir = tempdir().unwrap();
        let mut manager = CriuManager::new(dir.path());
        manager.initialize().await.unwrap();

        let result = manager.get_checkpoint("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_checkpoint_nonexistent() {
        let dir = tempdir().unwrap();
        let mut manager = CriuManager::new(dir.path());
        manager.initialize().await.unwrap();

        // Should not error when deleting nonexistent checkpoint
        let result = manager.delete_checkpoint("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_directory_size_empty() {
        let dir = tempdir().unwrap();
        let size = CriuManager::directory_size(dir.path()).await.unwrap();
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn test_directory_size_with_files() {
        let dir = tempdir().unwrap();

        // Create test files
        fs::write(dir.path().join("file1.txt"), "hello")
            .await
            .unwrap();
        fs::write(dir.path().join("file2.txt"), "world!")
            .await
            .unwrap();

        let size = CriuManager::directory_size(dir.path()).await.unwrap();
        assert_eq!(size, 11); // "hello" (5) + "world!" (6)
    }

    #[tokio::test]
    async fn test_find_criu_returns_option() {
        // This test just verifies the function runs without panicking
        let _result = CriuManager::find_criu();
    }

    #[tokio::test]
    async fn test_checkpoint_without_criu_fails() {
        let dir = tempdir().unwrap();
        let manager = CriuManager::new(dir.path());

        let options = CriuOptions::default();
        let result = manager
            .checkpoint("test", "alpine:latest", 1234, &options)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            OptimizeError::CriuNotAvailable { .. } => (),
            _ => panic!("Expected CriuNotAvailable error"),
        }
    }

    #[tokio::test]
    async fn test_restore_without_criu_fails() {
        let dir = tempdir().unwrap();
        let manager = CriuManager::new(dir.path());

        let checkpoint = Checkpoint {
            container_id: "test".to_string(),
            image: "alpine:latest".to_string(),
            path: dir.path().to_path_buf(),
            created_at: Utc::now(),
            size: 0,
            includes_tcp: false,
            includes_file_locks: false,
        };

        let result = manager.restore(&checkpoint).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_checkpoint_age_validation() {
        let old_checkpoint = Checkpoint {
            container_id: "old".to_string(),
            image: "alpine:latest".to_string(),
            path: PathBuf::from("/tmp/old"),
            created_at: Utc::now() - chrono::Duration::hours(48),
            size: 0,
            includes_tcp: false,
            includes_file_locks: false,
        };

        let age = Utc::now().signed_duration_since(old_checkpoint.created_at);
        assert!(age.to_std().unwrap() > MAX_CHECKPOINT_AGE);
    }
}
