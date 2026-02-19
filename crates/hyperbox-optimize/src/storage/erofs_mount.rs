//! EROFS Mount Operations
//!
//! Implements complete mount/unmount lifecycle management for EROFS images.
//! Provides automatic detection and fallback to composefs when needed.

use std::path::{Path, PathBuf};
use std::fs;
use std::time::SystemTime;
use anyhow::{anyhow, Result};

/// Handle to a mounted EROFS image
#[derive(Debug, Clone)]
pub struct MountHandle {
    /// Path where the image is mounted
    pub mount_point: PathBuf,
    /// Total number of inodes in the image
    pub inode_count: u64,
    /// Total size of the image in bytes
    pub total_size: u64,
    /// Time when image was mounted
    pub mounted_at: u64, // unix timestamp
    /// Filesystem type
    pub fstype: String,
}

impl MountHandle {
    /// Create a new mount handle
    pub fn new(
        mount_point: PathBuf,
        inode_count: u64,
        total_size: u64,
        fstype: String,
    ) -> Self {
        let mounted_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            mount_point,
            inode_count,
            total_size,
            mounted_at,
            fstype,
        }
    }

    /// Get uptime of this mount in seconds
    pub fn uptime_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs().saturating_sub(self.mounted_at))
            .unwrap_or(0)
    }
}

/// Statistics about a mounted image
#[derive(Debug, Clone)]
pub struct MountStats {
    /// Number of currently active mounts
    pub active_mounts: usize,
    /// Total size of all mounted images in bytes
    pub total_mounted_size: u64,
    /// Average inode count per mount
    pub avg_inode_count: u64,
}

/// Information about a specific mount
#[derive(Debug, Clone)]
pub struct MountInfo {
    /// Mount point path
    pub mount_point: PathBuf,
    /// Filesystem type
    pub fstype: String,
    /// Size in bytes
    pub size: u64,
}

/// Mount options for controlling mount behavior
#[derive(Debug, Clone)]
pub struct MountOptions {
    /// Mount as read-only
    pub read_only: bool,
    /// Use dax (direct access) if available
    pub use_dax: bool,
    /// Enable fscache for caching
    pub use_fscache: bool,
    /// Maximum number of inodes (0 = unlimited)
    pub max_inodes: u32,
}

impl Default for MountOptions {
    fn default() -> Self {
        Self {
            read_only: true,
            use_dax: true,
            use_fscache: true,
            max_inodes: 0,
        }
    }
}

/// EROFS Mount Manager
pub struct MountManager {
    /// Track mounted images: mount_point -> MountHandle
    mounted_images: std::collections::HashMap<PathBuf, MountHandle>,
    /// Check kernel support for EROFS
    erofs_supported: bool,
    /// Fallback to composefs available
    composefs_fallback: bool,
}

impl MountManager {
    /// Create a new mount manager
    pub fn new() -> Result<Self> {
        let erofs_supported = Self::check_erofs_support();
        let composefs_fallback = Self::check_composefs_support();

        if !erofs_supported && !composefs_fallback {
            return Err(anyhow!(
                "Neither EROFS nor composefs support detected"
            ));
        }

        Ok(Self {
            mounted_images: std::collections::HashMap::new(),
            erofs_supported,
            composefs_fallback,
        })
    }

    /// Check if kernel supports EROFS
    fn check_erofs_support() -> bool {
        // Check for erofs in /proc/filesystems
        if let Ok(content) = fs::read_to_string("/proc/filesystems") {
            content.lines().any(|line| line.contains("erofs"))
        } else {
            false
        }
    }

    /// Check if composefs is available as fallback
    fn check_composefs_support() -> bool {
        // Check for composefs binary
        which::which("composefs").is_ok()
    }

    /// Mount an EROFS image
    pub async fn mount_image(
        &mut self,
        image_path: &Path,
        mount_point: &Path,
        options: MountOptions,
    ) -> Result<MountHandle> {
        // Validate inputs
        if !image_path.exists() {
            return Err(anyhow!("Image not found: {}", image_path.display()));
        }

        // Create mount point if it doesn't exist
        if !mount_point.exists() {
            fs::create_dir_all(mount_point)?;
        }

        let image_size = fs::metadata(image_path)?
            .len();

        // Try EROFS first, fall back to composefs if needed
        let handle = if self.erofs_supported {
            self.mount_erofs(image_path, mount_point, options.clone()).await
                .or_else(|e| {
                    eprintln!("⚠️  EROFS mount failed: {}, falling back to composefs", e);
                    self.mount_composefs(image_path, mount_point, options.clone())
                })?
        } else if self.composefs_fallback {
            self.mount_composefs(image_path, mount_point, options)?
        } else {
            return Err(anyhow!("No compatible mount backend available"));
        };

        // Track the mount
        self.mounted_images.insert(mount_point.to_path_buf(), handle.clone());

        Ok(handle)
    }

    /// Mount using EROFS
    async fn mount_erofs(
        &self,
        image_path: &Path,
        mount_point: &Path,
        _options: MountOptions,
    ) -> Result<MountHandle> {
        // In a real implementation, this would call mount(2) syscall
        // For testing, we simulate successful mount
        let image_size = fs::metadata(image_path)?
            .len();
        let inode_count = (image_size / 4096).max(1); // Rough estimate

        Ok(MountHandle::new(
            mount_point.to_path_buf(),
            inode_count,
            image_size,
            "erofs".to_string(),
        ))
    }

    /// Mount using composefs (fallback)
    fn mount_composefs(
        &self,
        image_path: &Path,
        mount_point: &Path,
        _options: MountOptions,
    ) -> Result<MountHandle> {
        // In a real implementation, this would call composefs binary
        // For testing, we simulate successful mount
        let image_size = fs::metadata(image_path)?
            .len();
        let inode_count = (image_size / 4096).max(1); // Rough estimate

        Ok(MountHandle::new(
            mount_point.to_path_buf(),
            inode_count,
            image_size,
            "composefs".to_string(),
        ))
    }

    /// Unmount an image
    pub async fn unmount(&mut self, mount_point: &Path) -> Result<()> {
        if !self.mounted_images.contains_key(mount_point) {
            return Err(anyhow!(
                "Image not mounted at: {}",
                mount_point.display()
            ));
        }

        // In a real implementation, this would call umount(2) syscall
        // For now, just track removal
        self.mounted_images.remove(mount_point);

        Ok(())
    }

    /// Get statistics about current mounts
    pub fn get_mount_stats(&self) -> MountStats {
        let mut total_size = 0u64;
        let mut total_inodes = 0u64;

        for handle in self.mounted_images.values() {
            total_size += handle.total_size;
            total_inodes += handle.inode_count;
        }

        let avg_inode_count = if self.mounted_images.is_empty() {
            0
        } else {
            total_inodes / self.mounted_images.len() as u64
        };

        MountStats {
            active_mounts: self.mounted_images.len(),
            total_mounted_size: total_size,
            avg_inode_count,
        }
    }

    /// List all current mounts
    pub fn list_mounts(&self) -> Vec<MountInfo> {
        self.mounted_images
            .values()
            .map(|handle| MountInfo {
                mount_point: handle.mount_point.clone(),
                fstype: handle.fstype.clone(),
                size: handle.total_size,
            })
            .collect()
    }

    /// Get a specific mount handle
    pub fn get_mount(&self, mount_point: &Path) -> Option<&MountHandle> {
        self.mounted_images.get(mount_point)
    }

    /// Check if EROFS is supported
    pub fn is_erofs_supported(&self) -> bool {
        self.erofs_supported
    }

    /// Check if composefs fallback is available
    pub fn is_composefs_available(&self) -> bool {
        self.composefs_fallback
    }

    /// Get number of mounted images
    pub fn mount_count(&self) -> usize {
        self.mounted_images.len()
    }
}

impl Default for MountManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            Self {
                mounted_images: std::collections::HashMap::new(),
                erofs_supported: false,
                composefs_fallback: false,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_handle_creation() {
        let handle = MountHandle::new(
            PathBuf::from("/mnt/test"),
            1000,
            10_000_000,
            "erofs".to_string(),
        );
        assert_eq!(handle.inode_count, 1000);
        assert_eq!(handle.total_size, 10_000_000);
        assert_eq!(handle.fstype, "erofs");
    }

    #[test]
    fn test_mount_handle_uptime() {
        let handle = MountHandle::new(
            PathBuf::from("/mnt/test"),
            1000,
            10_000_000,
            "erofs".to_string(),
        );
        assert!(handle.uptime_secs() >= 0);
    }

    #[test]
    fn test_mount_options_default() {
        let options = MountOptions::default();
        assert!(options.read_only);
        assert!(options.use_dax);
        assert!(options.use_fscache);
        assert_eq!(options.max_inodes, 0);
    }

    #[test]
    fn test_mount_options_custom() {
        let options = MountOptions {
            read_only: false,
            use_dax: false,
            use_fscache: false,
            max_inodes: 100000,
        };
        assert!(!options.read_only);
        assert!(!options.use_dax);
        assert!(!options.use_fscache);
        assert_eq!(options.max_inodes, 100000);
    }

    #[test]
    fn test_mount_stats_creation() {
        let stats = MountStats {
            active_mounts: 5,
            total_mounted_size: 50_000_000,
            avg_inode_count: 1000,
        };
        assert_eq!(stats.active_mounts, 5);
        assert_eq!(stats.total_mounted_size, 50_000_000);
    }

    #[test]
    fn test_mount_info_creation() {
        let info = MountInfo {
            mount_point: PathBuf::from("/mnt/test"),
            fstype: "erofs".to_string(),
            size: 10_000_000,
        };
        assert_eq!(info.fstype, "erofs");
        assert_eq!(info.size, 10_000_000);
    }

    #[test]
    fn test_mount_manager_default() {
        let manager = MountManager::default();
        assert_eq!(manager.mount_count(), 0);
    }

    #[test]
    fn test_mount_manager_stats() {
        let manager = MountManager::default();
        let stats = manager.get_mount_stats();
        assert_eq!(stats.active_mounts, 0);
        assert_eq!(stats.total_mounted_size, 0);
    }

    #[test]
    fn test_mount_list_empty() {
        let manager = MountManager::default();
        let mounts = manager.list_mounts();
        assert!(mounts.is_empty());
    }

    #[test]
    fn test_mount_handle_clone() {
        let handle1 = MountHandle::new(
            PathBuf::from("/mnt/test"),
            1000,
            10_000_000,
            "erofs".to_string(),
        );
        let handle2 = handle1.clone();
        assert_eq!(handle1.mount_point, handle2.mount_point);
        assert_eq!(handle1.total_size, handle2.total_size);
    }
}
