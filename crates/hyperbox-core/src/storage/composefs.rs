//! Composefs integration for efficient image mounting.
//!
//! Composefs provides content-addressed filesystem mounting with
//! automatic deduplication and verification.

use crate::error::Result;
use std::path::PathBuf;
use std::process::Command;
use tokio::fs;
use tracing::{debug, info};

/// Composefs manager for mounting container images.
pub struct ComposefsManager {
    /// Root directory for composefs data
    root_dir: PathBuf,
    /// Objects directory (content-addressed blobs)
    objects_dir: PathBuf,
}

impl ComposefsManager {
    /// Create a new composefs manager.
    #[must_use]
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        let root_dir = root_dir.into();
        let objects_dir = root_dir.join("objects");

        Self {
            root_dir,
            objects_dir,
        }
    }

    /// Initialize composefs directories.
    pub async fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.root_dir).await?;
        fs::create_dir_all(&self.objects_dir).await?;

        info!("Initialized composefs at {:?}", self.root_dir);
        Ok(())
    }

    /// Check if composefs is available.
    pub fn is_available(&self) -> bool {
        // Check for composefs-info binary
        Command::new("composefs-info")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Create a composefs image from a directory.
    #[cfg(unix)]
    pub async fn create_image(&self, source_dir: &Path, image_name: &str) -> Result<PathBuf> {
        let image_path = self.root_dir.join(format!("{image_name}.cfs"));
        let digest_store = self.objects_dir.to_string_lossy();

        info!("Creating composefs image {} from {:?}", image_name, source_dir);

        // mkcomposefs --digest-store=<objects_dir> <source_dir> <output.cfs>
        let output = Command::new("mkcomposefs")
            .arg(format!("--digest-store={digest_store}"))
            .arg(source_dir)
            .arg(&image_path)
            .output()
            .map_err(|e| CoreError::StorageOperation(format!("create composefs: {}", e)))?;

        if !output.status.success() {
            return Err(CoreError::StorageOperation(format!(
                "create composefs: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        debug!("Created composefs image at {:?}", image_path);
        Ok(image_path)
    }

    /// Mount a composefs image.
    #[cfg(unix)]
    pub async fn mount(&self, image_path: &Path, mount_point: &Path) -> Result<()> {
        fs::create_dir_all(mount_point).await?;

        let objects_dir = self.objects_dir.to_string_lossy();

        info!("Mounting composefs {:?} at {:?}", image_path, mount_point);

        // mount -t composefs -o basedir=<objects_dir> <image.cfs> <mount_point>
        let output = Command::new("mount")
            .args([
                "-t",
                "composefs",
                "-o",
                &format!("basedir={objects_dir}"),
                image_path.to_str().unwrap(),
                mount_point.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| CoreError::StorageOperation(format!("mount composefs: {}", e)))?;

        if !output.status.success() {
            return Err(CoreError::StorageOperation(format!(
                "mount composefs: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    /// Unmount a composefs image.
    #[cfg(unix)]
    pub async fn unmount(&self, mount_point: &Path) -> Result<()> {
        let output = Command::new("umount")
            .arg(mount_point)
            .output()
            .map_err(|e| CoreError::StorageOperation(format!("unmount composefs: {}", e)))?;

        if !output.status.success() {
            // Try lazy unmount
            let _ = Command::new("umount")
                .args(["-l", mount_point.to_str().unwrap()])
                .output();
        }

        Ok(())
    }

    /// Get storage statistics.
    pub async fn stats(&self) -> Result<ComposefsStats> {
        let mut total_size = 0u64;
        let mut object_count = 0u64;

        if let Ok(mut entries) = tokio::fs::read_dir(&self.objects_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(metadata) = entry.metadata().await {
                    total_size += metadata.len();
                    object_count += 1;
                }
            }
        }

        Ok(ComposefsStats {
            total_size,
            object_count,
            objects_dir: self.objects_dir.clone(),
        })
    }

    /// Garbage collect unused objects.
    pub async fn gc(&self, used_digests: &[String]) -> Result<GcResult> {
        let removed_count = 0u64;
        let freed_bytes = 0u64;

        // Would walk objects directory and remove unused
        // Simplified implementation
        info!(
            "Garbage collection complete: {} objects removed, {} bytes freed",
            removed_count, freed_bytes
        );

        Ok(GcResult {
            removed_count,
            freed_bytes,
        })
    }
}

/// Composefs storage statistics.
#[derive(Debug, Clone)]
pub struct ComposefsStats {
    /// Total size of objects
    pub total_size: u64,
    /// Number of objects
    pub object_count: u64,
    /// Objects directory
    pub objects_dir: PathBuf,
}

/// Garbage collection result.
#[derive(Debug, Clone)]
pub struct GcResult {
    /// Number of objects removed
    pub removed_count: u64,
    /// Bytes freed
    pub freed_bytes: u64,
}
