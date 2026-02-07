//! Composefs integration for content-addressed image mounting with deduplication.
//!
//! Composefs provides content-addressed filesystem mounting with automatic
//! deduplication, integrity verification, and efficient storage through
//! a two-level directory structure similar to Git's object store.
//!
//! ## Content-Addressed Object Store
//!
//! Objects are stored by their SHA-256 digest in a two-level directory:
//! ```text
//! objects/
//!   ab/
//!     cdef1234567890...  (sha256:abcdef1234567890...)
//!   12/
//!     34567890abcdef...  (sha256:1234567890abcdef...)
//! ```
//!
//! ## Deduplication
//!
//! Files with identical content share the same object in the store,
//! regardless of how many layers reference them. This saves significant
//! disk space for container images with shared base layers.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use hyperbox_core::storage::ComposefsManager;
//!
//! # async fn example() -> hyperbox_core::error::Result<()> {
//! let manager = ComposefsManager::new("/var/lib/hyperbox/composefs");
//! manager.initialize().await?;
//!
//! // Store content-addressed objects
//! let digest = manager.store_object(b"hello world").await?;
//! assert!(manager.has_object(&digest).await);
//!
//! // Verify integrity
//! assert!(manager.verify_object(&digest).await?);
//! # Ok(())
//! # }
//! ```

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

// =============================================================================
// Core Manager
// =============================================================================

/// Composefs manager for content-addressed storage and image mounting.
///
/// Manages a content-addressed object store where files are stored
/// by their SHA-256 digest. Provides deduplication, integrity verification,
/// garbage collection, and (on Linux) composefs image mounting.
pub struct ComposefsManager {
    /// Root directory for composefs data.
    root_dir: PathBuf,
    /// Objects directory (content-addressed blobs).
    objects_dir: PathBuf,
    /// Temporary directory for atomic writes.
    temp_dir: PathBuf,
}

impl ComposefsManager {
    /// Create a new composefs manager.
    #[must_use]
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        let root_dir = root_dir.into();
        let objects_dir = root_dir.join("objects");
        let temp_dir = root_dir.join("tmp");

        Self {
            root_dir,
            objects_dir,
            temp_dir,
        }
    }

    /// Initialize composefs directories.
    pub async fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.root_dir).await?;
        fs::create_dir_all(&self.objects_dir).await?;
        fs::create_dir_all(&self.temp_dir).await?;

        info!("Initialized composefs at {:?}", self.root_dir);
        Ok(())
    }

    /// Check if composefs tools are available on this system.
    pub fn is_available(&self) -> bool {
        std::process::Command::new("composefs-info")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get the root directory.
    #[must_use]
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Get the objects directory.
    #[must_use]
    pub fn objects_dir(&self) -> &Path {
        &self.objects_dir
    }

    // =========================================================================
    // Content-Addressed Object Store
    // =========================================================================

    /// Compute the SHA-256 digest of data.
    ///
    /// Returns a string in the format `sha256:<hex>`.
    #[must_use]
    fn compute_digest(data: &[u8]) -> String {
        let hash = Sha256::digest(data);
        format!("sha256:{}", hex::encode(hash))
    }

    /// Get the filesystem path for an object by its digest.
    ///
    /// Uses a two-level directory structure: `objects/ab/cdef1234...`
    /// where the first two hex characters form the prefix directory.
    #[must_use]
    pub fn object_path(&self, digest: &str) -> PathBuf {
        let hash = digest.strip_prefix("sha256:").unwrap_or(digest);
        if hash.len() < 3 {
            return self.objects_dir.join(hash);
        }
        let (prefix, rest) = hash.split_at(2);
        self.objects_dir.join(prefix).join(rest)
    }

    /// Store data and return its content digest.
    ///
    /// If an object with the same digest already exists, this is a no-op
    /// (content-addressed deduplication). Writes are atomic (temp + rename).
    ///
    /// # Returns
    ///
    /// The SHA-256 digest of the stored object (format: `sha256:abcdef...`).
    pub async fn store_object(&self, data: &[u8]) -> Result<String> {
        let digest = Self::compute_digest(data);
        self.store_object_inner(data, &digest).await?;
        Ok(digest)
    }

    /// Store data with a pre-computed digest. Returns `true` if newly stored,
    /// `false` if already existed (dedup hit).
    async fn store_object_inner(&self, data: &[u8], digest: &str) -> Result<bool> {
        let path = self.object_path(digest);

        // Already exists — dedup hit
        if path.exists() {
            debug!("Object {} already exists (dedup hit)", digest);
            return Ok(false);
        }

        // Create parent directory
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Atomic write: write to temp file, then rename
        let temp_path = self.temp_dir.join(format!(
            "obj-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().as_simple()
        ));

        if let Err(e) = fs::write(&temp_path, data).await {
            let _ = fs::remove_file(&temp_path).await;
            return Err(CoreError::StorageOperation(format!("write temp object: {e}")));
        }

        // Rename is atomic on the same filesystem
        match fs::rename(&temp_path, &path).await {
            Ok(()) => {
                debug!("Stored object {} ({} bytes)", digest, data.len());
                Ok(true)
            }
            Err(_) => {
                // Race condition: another writer may have stored it
                let _ = fs::remove_file(&temp_path).await;
                if path.exists() {
                    debug!("Object {} stored by concurrent writer", digest);
                    Ok(false)
                } else {
                    Err(CoreError::StorageOperation(format!(
                        "store object {digest}: rename failed"
                    )))
                }
            }
        }
    }

    /// Retrieve an object by its digest.
    ///
    /// # Errors
    ///
    /// Returns `StorageOperation` if the object does not exist or read fails.
    pub async fn get_object(&self, digest: &str) -> Result<Vec<u8>> {
        let path = self.object_path(digest);
        fs::read(&path)
            .await
            .map_err(|e| CoreError::StorageOperation(format!("get object {digest}: {e}")))
    }

    /// Check if an object exists in the store.
    pub async fn has_object(&self, digest: &str) -> bool {
        self.object_path(digest).exists()
    }

    /// Verify the integrity of a stored object.
    ///
    /// Reads the object, re-computes its SHA-256 digest, and compares
    /// it to the expected digest.
    ///
    /// # Returns
    ///
    /// `true` if the object is intact, `false` if corrupted.
    pub async fn verify_object(&self, digest: &str) -> Result<bool> {
        let data = self.get_object(digest).await?;
        let computed = Self::compute_digest(&data);
        Ok(computed == digest)
    }

    /// Remove an object from the store.
    ///
    /// Also removes the parent prefix directory if it becomes empty.
    pub async fn remove_object(&self, digest: &str) -> Result<()> {
        let path = self.object_path(digest);
        fs::remove_file(&path)
            .await
            .map_err(|e| CoreError::StorageOperation(format!("remove object {digest}: {e}")))?;

        // Clean up empty parent prefix directory
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir(parent).await; // Ignore error if not empty
        }

        debug!("Removed object {}", digest);
        Ok(())
    }

    /// Get the size in bytes of a stored object.
    pub async fn object_size(&self, digest: &str) -> Result<u64> {
        let path = self.object_path(digest);
        let metadata = fs::metadata(&path)
            .await
            .map_err(|e| CoreError::StorageOperation(format!("object size {digest}: {e}")))?;
        Ok(metadata.len())
    }

    // =========================================================================
    // Deduplication
    // =========================================================================

    /// Deduplicate all files in a directory into the content-addressed store.
    ///
    /// Scans the directory recursively, hashes each file, and stores unique
    /// content in the object store. Returns detailed deduplication statistics.
    ///
    /// Files with identical content across different paths share a single
    /// object in the store, saving disk space proportional to the redundancy.
    pub async fn deduplicate_directory(&self, dir: &Path) -> Result<DedupResult> {
        let mut files_stored = 0u64;
        let mut files_deduped = 0u64;
        let mut total_bytes = 0u64;
        let mut saved_bytes = 0u64;
        let mut digests = Vec::new();

        let entries: Vec<_> = WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        for entry in entries {
            let data = match fs::read(entry.path()).await {
                Ok(d) => d,
                Err(e) => {
                    warn!("Failed to read {:?} during dedup: {}", entry.path(), e);
                    continue;
                }
            };

            let size = data.len() as u64;
            total_bytes += size;

            let digest = Self::compute_digest(&data);
            let is_new = self.store_object_inner(&data, &digest).await?;

            if is_new {
                files_stored += 1;
            } else {
                files_deduped += 1;
                saved_bytes += size;
            }

            digests.push(digest);
        }

        info!(
            "Dedup: {} files ({}) — {} stored, {} deduped, {} saved",
            files_stored + files_deduped,
            format_bytes(total_bytes),
            files_stored,
            files_deduped,
            format_bytes(saved_bytes),
        );

        Ok(DedupResult {
            files_processed: files_stored + files_deduped,
            files_stored,
            files_deduplicated: files_deduped,
            total_bytes,
            saved_bytes,
            digests,
        })
    }

    // =========================================================================
    // Garbage Collection
    // =========================================================================

    /// Garbage-collect unreferenced objects from the store.
    ///
    /// Walks the entire object store and removes any object whose digest
    /// is not in the `used_digests` set. Also cleans up empty prefix
    /// directories left behind.
    ///
    /// # Arguments
    ///
    /// * `used_digests` — Digests of objects that are currently referenced.
    ///   Accepts both `sha256:abcdef...` and bare `abcdef...` formats.
    pub async fn gc(&self, used_digests: &[String]) -> Result<GcResult> {
        // Normalize all digests to sha256: prefix format
        let used: HashSet<String> = used_digests
            .iter()
            .map(|d| {
                if d.starts_with("sha256:") {
                    d.clone()
                } else {
                    format!("sha256:{d}")
                }
            })
            .collect();

        let mut removed_count = 0u64;
        let mut freed_bytes = 0u64;
        let mut empty_dirs = Vec::new();

        // Walk prefix directories (objects/ab/, objects/cd/, ...)
        let Ok(mut prefix_entries) = fs::read_dir(&self.objects_dir).await else {
            return Ok(GcResult {
                removed_count: 0,
                freed_bytes: 0,
            });
        };

        while let Ok(Some(prefix_entry)) = prefix_entries.next_entry().await {
            let is_dir = prefix_entry
                .file_type()
                .await
                .map(|ft| ft.is_dir())
                .unwrap_or(false);
            if !is_dir {
                continue;
            }

            let prefix_str = prefix_entry.file_name();
            let prefix = prefix_str.to_string_lossy();
            let mut dir_has_objects = false;

            let Ok(mut obj_entries) = fs::read_dir(prefix_entry.path()).await else {
                continue;
            };

            while let Ok(Some(obj_entry)) = obj_entries.next_entry().await {
                let file_name = obj_entry.file_name();
                let digest = format!("sha256:{}{}", prefix, file_name.to_string_lossy());

                if used.contains(&digest) {
                    dir_has_objects = true;
                } else {
                    if let Ok(metadata) = obj_entry.metadata().await {
                        freed_bytes += metadata.len();
                    }
                    if let Err(e) = fs::remove_file(obj_entry.path()).await {
                        warn!("GC: failed to remove {digest}: {e}");
                        dir_has_objects = true;
                        continue;
                    }
                    debug!("GC removed {}", digest);
                    removed_count += 1;
                }
            }

            if !dir_has_objects {
                empty_dirs.push(prefix_entry.path());
            }
        }

        // Clean up empty prefix directories
        for dir in &empty_dirs {
            let _ = fs::remove_dir(dir).await;
        }

        info!("GC complete: {} removed, {} freed", removed_count, format_bytes(freed_bytes));

        Ok(GcResult {
            removed_count,
            freed_bytes,
        })
    }

    // =========================================================================
    // Store Verification
    // =========================================================================

    /// Verify integrity of every object in the store.
    ///
    /// Re-hashes each object and compares against its expected digest.
    /// Returns a summary including a list of any corrupted object digests.
    pub async fn verify_store(&self) -> Result<VerifyResult> {
        let mut total = 0u64;
        let mut valid = 0u64;
        let mut corrupted = Vec::new();

        let Ok(mut prefix_entries) = fs::read_dir(&self.objects_dir).await else {
            return Ok(VerifyResult {
                total_objects: 0,
                valid_objects: 0,
                corrupted_objects: vec![],
            });
        };

        while let Ok(Some(prefix_entry)) = prefix_entries.next_entry().await {
            let is_dir = prefix_entry
                .file_type()
                .await
                .map(|ft| ft.is_dir())
                .unwrap_or(false);
            if !is_dir {
                continue;
            }

            let prefix = prefix_entry.file_name();
            let prefix_str = prefix.to_string_lossy();

            let Ok(mut obj_entries) = fs::read_dir(prefix_entry.path()).await else {
                continue;
            };

            while let Ok(Some(obj_entry)) = obj_entries.next_entry().await {
                total += 1;
                let file_name = obj_entry.file_name();
                let digest = format!("sha256:{}{}", prefix_str, file_name.to_string_lossy());

                match self.verify_object(&digest).await {
                    Ok(true) => valid += 1,
                    Ok(false) => {
                        warn!("Corrupted object: {}", digest);
                        corrupted.push(digest);
                    }
                    Err(e) => {
                        warn!("Failed to verify {}: {}", digest, e);
                        corrupted.push(digest);
                    }
                }
            }
        }

        info!("Verified {} objects: {} valid, {} corrupted", total, valid, corrupted.len());

        Ok(VerifyResult {
            total_objects: total,
            valid_objects: valid,
            corrupted_objects: corrupted,
        })
    }

    // =========================================================================
    // Composefs Image Operations (Linux only)
    // =========================================================================

    /// Create a composefs image from a directory.
    ///
    /// Requires `mkcomposefs` to be installed. Files are stored in the
    /// content-addressed object store for deduplication.
    #[cfg(unix)]
    pub async fn create_image(&self, source_dir: &Path, image_name: &str) -> Result<PathBuf> {
        let image_path = self.root_dir.join(format!("{image_name}.cfs"));

        info!("Creating composefs image {} from {:?}", image_name, source_dir);

        let output = std::process::Command::new("mkcomposefs")
            .arg(format!("--digest-store={}", self.objects_dir.display()))
            .arg(source_dir)
            .arg(&image_path)
            .output()
            .map_err(|e| CoreError::StorageOperation(format!("create composefs: {e}")))?;

        if !output.status.success() {
            return Err(CoreError::StorageOperation(format!(
                "mkcomposefs failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        debug!("Created composefs image at {:?}", image_path);
        Ok(image_path)
    }

    /// Mount a composefs image at a mount point.
    ///
    /// Requires root privileges and the `composefs` kernel module.
    #[cfg(unix)]
    pub async fn mount(&self, image_path: &Path, mount_point: &Path) -> Result<()> {
        fs::create_dir_all(mount_point).await?;

        info!("Mounting composefs {:?} at {:?}", image_path, mount_point);

        let output = std::process::Command::new("mount")
            .arg("-t")
            .arg("composefs")
            .arg("-o")
            .arg(format!("basedir={}", self.objects_dir.display()))
            .arg(image_path)
            .arg(mount_point)
            .output()
            .map_err(|e| CoreError::StorageOperation(format!("mount composefs: {e}")))?;

        if !output.status.success() {
            return Err(CoreError::StorageOperation(format!(
                "mount composefs: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    /// Unmount a composefs image. Falls back to lazy unmount on failure.
    #[cfg(unix)]
    pub async fn unmount(&self, mount_point: &Path) -> Result<()> {
        let output = std::process::Command::new("umount")
            .arg(mount_point)
            .output()
            .map_err(|e| CoreError::StorageOperation(format!("unmount composefs: {e}")))?;

        if !output.status.success() {
            // Try lazy unmount as fallback
            let _ = std::process::Command::new("umount")
                .arg("-l")
                .arg(mount_point)
                .output();
        }

        Ok(())
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    /// Get storage statistics for the object store.
    ///
    /// Walks the objects directory and aggregates size, count,
    /// and prefix directory information.
    pub async fn stats(&self) -> Result<ComposefsStats> {
        let mut total_size = 0u64;
        let mut object_count = 0u64;
        let mut prefix_count = 0u64;

        if let Ok(mut prefix_entries) = fs::read_dir(&self.objects_dir).await {
            while let Ok(Some(prefix_entry)) = prefix_entries.next_entry().await {
                let is_dir = prefix_entry
                    .file_type()
                    .await
                    .map(|ft| ft.is_dir())
                    .unwrap_or(false);
                if !is_dir {
                    continue;
                }
                prefix_count += 1;

                if let Ok(mut obj_entries) = fs::read_dir(prefix_entry.path()).await {
                    while let Ok(Some(obj_entry)) = obj_entries.next_entry().await {
                        if let Ok(metadata) = obj_entry.metadata().await {
                            total_size += metadata.len();
                            object_count += 1;
                        }
                    }
                }
            }
        }

        Ok(ComposefsStats {
            total_size,
            object_count,
            prefix_directories: prefix_count,
            objects_dir: self.objects_dir.clone(),
        })
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Format a byte count as a human-readable string.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

// =============================================================================
// Types
// =============================================================================

/// Composefs storage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposefsStats {
    /// Total size of all objects in bytes.
    pub total_size: u64,
    /// Number of unique objects in the store.
    pub object_count: u64,
    /// Number of prefix directories (hash buckets).
    pub prefix_directories: u64,
    /// Path to the objects directory.
    pub objects_dir: PathBuf,
}

/// Result of deduplicating a directory into the object store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupResult {
    /// Total files processed.
    pub files_processed: u64,
    /// Files newly stored (unique content).
    pub files_stored: u64,
    /// Files already present in the store (duplicates).
    pub files_deduplicated: u64,
    /// Total bytes across all processed files.
    pub total_bytes: u64,
    /// Bytes saved through deduplication.
    pub saved_bytes: u64,
    /// Digests of all processed files (in processing order).
    pub digests: Vec<String>,
}

impl DedupResult {
    /// Deduplication ratio: 0.0 = no savings, 1.0 = all duplicates.
    #[must_use]
    pub fn dedup_ratio(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        self.saved_bytes as f64 / self.total_bytes as f64
    }
}

/// Garbage collection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcResult {
    /// Number of objects removed.
    pub removed_count: u64,
    /// Total bytes freed.
    pub freed_bytes: u64,
}

/// Result of verifying the integrity of the object store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    /// Total objects checked.
    pub total_objects: u64,
    /// Objects that passed integrity verification.
    pub valid_objects: u64,
    /// Digests of objects that failed verification.
    pub corrupted_objects: Vec<String>,
}

impl VerifyResult {
    /// Returns `true` if all objects passed verification.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.corrupted_objects.is_empty()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_manager() -> (ComposefsManager, TempDir) {
        let dir = TempDir::new().unwrap();
        let manager = ComposefsManager::new(dir.path());
        (manager, dir)
    }

    // --- Digest computation ---

    #[test]
    fn test_compute_digest_format() {
        let digest = ComposefsManager::compute_digest(b"hello world");
        assert!(digest.starts_with("sha256:"));
        // sha256: (7) + 64 hex chars = 71
        assert_eq!(digest.len(), 71);
    }

    #[test]
    fn test_compute_digest_deterministic() {
        let d1 = ComposefsManager::compute_digest(b"test data");
        let d2 = ComposefsManager::compute_digest(b"test data");
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_compute_digest_unique() {
        let d1 = ComposefsManager::compute_digest(b"data1");
        let d2 = ComposefsManager::compute_digest(b"data2");
        assert_ne!(d1, d2);
    }

    #[test]
    fn test_compute_digest_empty() {
        let digest = ComposefsManager::compute_digest(b"");
        assert!(digest.starts_with("sha256:"));
        assert_eq!(digest.len(), 71);
    }

    // --- Object path ---

    #[test]
    fn test_object_path_two_level() {
        let (manager, _dir) = test_manager();
        let path = manager.object_path("sha256:abcdef1234567890");
        let path_str = path.to_string_lossy();
        // Should contain objects/ab/cdef1234567890
        assert!(path_str.contains("objects"));
        assert!(path_str.ends_with("cdef1234567890"));
    }

    #[test]
    fn test_object_path_strips_prefix() {
        let (manager, _dir) = test_manager();
        let with_prefix = manager.object_path("sha256:abcdef1234567890");
        let without_prefix = manager.object_path("abcdef1234567890");
        assert_eq!(with_prefix, without_prefix);
    }

    #[test]
    fn test_object_path_short_hash() {
        let (manager, _dir) = test_manager();
        // Edge case: very short hash
        let path = manager.object_path("ab");
        assert!(path.to_string_lossy().contains("ab"));
    }

    // --- Initialize ---

    #[tokio::test]
    async fn test_initialize_creates_dirs() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();
        assert!(manager.root_dir.exists());
        assert!(manager.objects_dir.exists());
        assert!(manager.temp_dir.exists());
    }

    #[tokio::test]
    async fn test_initialize_idempotent() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();
        manager.initialize().await.unwrap(); // Should not fail
    }

    // --- Store and retrieve ---

    #[tokio::test]
    async fn test_store_and_get() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let data = b"hello world";
        let digest = manager.store_object(data).await.unwrap();

        let retrieved = manager.get_object(&digest).await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_store_deduplication() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let data = b"duplicate content here";
        let d1 = manager.store_object(data).await.unwrap();
        let d2 = manager.store_object(data).await.unwrap();

        // Same digest
        assert_eq!(d1, d2);

        // Only one object on disk
        let stats = manager.stats().await.unwrap();
        assert_eq!(stats.object_count, 1);
    }

    #[tokio::test]
    async fn test_store_multiple_unique() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        manager.store_object(b"object-1").await.unwrap();
        manager.store_object(b"object-2").await.unwrap();
        manager.store_object(b"object-3").await.unwrap();

        let stats = manager.stats().await.unwrap();
        assert_eq!(stats.object_count, 3);
    }

    // --- Has / verify / remove ---

    #[tokio::test]
    async fn test_has_object() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let digest = manager.store_object(b"exists").await.unwrap();
        assert!(manager.has_object(&digest).await);
        assert!(!manager.has_object("sha256:0000000000").await);
    }

    #[tokio::test]
    async fn test_verify_object_intact() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let digest = manager.store_object(b"verify me").await.unwrap();
        assert!(manager.verify_object(&digest).await.unwrap());
    }

    #[tokio::test]
    async fn test_remove_object() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let digest = manager.store_object(b"remove me").await.unwrap();
        assert!(manager.has_object(&digest).await);

        manager.remove_object(&digest).await.unwrap();
        assert!(!manager.has_object(&digest).await);
    }

    #[tokio::test]
    async fn test_object_size() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let data = b"twelve bytes";
        let digest = manager.store_object(data).await.unwrap();
        let size = manager.object_size(&digest).await.unwrap();
        assert_eq!(size, data.len() as u64);
    }

    // --- Stats ---

    #[tokio::test]
    async fn test_stats_empty() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let stats = manager.stats().await.unwrap();
        assert_eq!(stats.object_count, 0);
        assert_eq!(stats.total_size, 0);
        assert_eq!(stats.prefix_directories, 0);
    }

    #[tokio::test]
    async fn test_stats_with_objects() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        manager.store_object(b"obj-1").await.unwrap();
        manager.store_object(b"obj-2").await.unwrap();
        manager.store_object(b"obj-1").await.unwrap(); // dup

        let stats = manager.stats().await.unwrap();
        assert_eq!(stats.object_count, 2);
        assert!(stats.total_size > 0);
        assert!(stats.prefix_directories > 0);
    }

    // --- Garbage collection ---

    #[tokio::test]
    async fn test_gc_removes_unreferenced() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let d1 = manager.store_object(b"keep-this").await.unwrap();
        let d2 = manager.store_object(b"remove-this").await.unwrap();

        let result = manager.gc(&[d1.clone()]).await.unwrap();
        assert_eq!(result.removed_count, 1);
        assert!(result.freed_bytes > 0);

        assert!(manager.has_object(&d1).await);
        assert!(!manager.has_object(&d2).await);
    }

    #[tokio::test]
    async fn test_gc_keeps_all_referenced() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let d1 = manager.store_object(b"keep-1").await.unwrap();
        let d2 = manager.store_object(b"keep-2").await.unwrap();

        let result = manager.gc(&[d1.clone(), d2.clone()]).await.unwrap();
        assert_eq!(result.removed_count, 0);

        assert!(manager.has_object(&d1).await);
        assert!(manager.has_object(&d2).await);
    }

    #[tokio::test]
    async fn test_gc_empty_store() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let result = manager.gc(&[]).await.unwrap();
        assert_eq!(result.removed_count, 0);
        assert_eq!(result.freed_bytes, 0);
    }

    #[tokio::test]
    async fn test_gc_bare_digest_format() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let digest = manager.store_object(b"keep").await.unwrap();
        // Pass bare digest without sha256: prefix
        let bare = digest.strip_prefix("sha256:").unwrap().to_string();

        let result = manager.gc(&[bare]).await.unwrap();
        assert_eq!(result.removed_count, 0);
        assert!(manager.has_object(&digest).await);
    }

    // --- Store verification ---

    #[tokio::test]
    async fn test_verify_store_healthy() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        manager.store_object(b"obj-1").await.unwrap();
        manager.store_object(b"obj-2").await.unwrap();

        let result = manager.verify_store().await.unwrap();
        assert_eq!(result.total_objects, 2);
        assert_eq!(result.valid_objects, 2);
        assert!(result.is_healthy());
    }

    #[tokio::test]
    async fn test_verify_store_empty() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let result = manager.verify_store().await.unwrap();
        assert_eq!(result.total_objects, 0);
        assert!(result.is_healthy());
    }

    // --- Directory deduplication ---

    #[tokio::test]
    async fn test_deduplicate_directory() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        // Create source directory with some files (including duplicate)
        let source = TempDir::new().unwrap();
        fs::write(source.path().join("a.txt"), b"content-A")
            .await
            .unwrap();
        fs::write(source.path().join("b.txt"), b"content-B")
            .await
            .unwrap();
        fs::write(source.path().join("c.txt"), b"content-A") // dup of a.txt
            .await
            .unwrap();

        let result = manager.deduplicate_directory(source.path()).await.unwrap();

        assert_eq!(result.files_processed, 3);
        assert_eq!(result.files_stored, 2);
        assert_eq!(result.files_deduplicated, 1);
        assert!(result.saved_bytes > 0);
        assert!(result.dedup_ratio() > 0.0);
        assert_eq!(result.digests.len(), 3);
    }

    #[tokio::test]
    async fn test_deduplicate_all_unique() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let source = TempDir::new().unwrap();
        fs::write(source.path().join("x.txt"), b"unique-x")
            .await
            .unwrap();
        fs::write(source.path().join("y.txt"), b"unique-y")
            .await
            .unwrap();

        let result = manager.deduplicate_directory(source.path()).await.unwrap();

        assert_eq!(result.files_stored, 2);
        assert_eq!(result.files_deduplicated, 0);
        assert_eq!(result.saved_bytes, 0);
        assert!((result.dedup_ratio()).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_deduplicate_empty_dir() {
        let (manager, _dir) = test_manager();
        manager.initialize().await.unwrap();

        let source = TempDir::new().unwrap();
        let result = manager.deduplicate_directory(source.path()).await.unwrap();

        assert_eq!(result.files_processed, 0);
        assert_eq!(result.files_stored, 0);
    }

    // --- DedupResult ---

    #[test]
    fn test_dedup_ratio() {
        let result = DedupResult {
            files_processed: 4,
            files_stored: 2,
            files_deduplicated: 2,
            total_bytes: 1000,
            saved_bytes: 500,
            digests: vec![],
        };
        assert!((result.dedup_ratio() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_dedup_ratio_zero_bytes() {
        let result = DedupResult {
            files_processed: 0,
            files_stored: 0,
            files_deduplicated: 0,
            total_bytes: 0,
            saved_bytes: 0,
            digests: vec![],
        };
        assert!(result.dedup_ratio().abs() < f64::EPSILON);
    }

    // --- VerifyResult ---

    #[test]
    fn test_verify_result_healthy() {
        let result = VerifyResult {
            total_objects: 10,
            valid_objects: 10,
            corrupted_objects: vec![],
        };
        assert!(result.is_healthy());
    }

    #[test]
    fn test_verify_result_corrupted() {
        let result = VerifyResult {
            total_objects: 10,
            valid_objects: 9,
            corrupted_objects: vec!["sha256:bad".to_string()],
        };
        assert!(!result.is_healthy());
    }

    // --- format_bytes ---

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.00 GB");
    }

    // --- Serialization ---

    #[test]
    fn test_composefs_stats_roundtrip() {
        let stats = ComposefsStats {
            total_size: 1024,
            object_count: 10,
            prefix_directories: 5,
            objects_dir: PathBuf::from("/tmp/objects"),
        };
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: ComposefsStats = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_size, 1024);
        assert_eq!(deserialized.object_count, 10);
        assert_eq!(deserialized.prefix_directories, 5);
    }

    #[test]
    fn test_gc_result_serialize() {
        let result = GcResult {
            removed_count: 5,
            freed_bytes: 10240,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: GcResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.removed_count, 5);
        assert_eq!(deserialized.freed_bytes, 10240);
    }

    // --- Manager basics ---

    #[test]
    fn test_manager_new() {
        let dir = TempDir::new().unwrap();
        let manager = ComposefsManager::new(dir.path());
        assert_eq!(manager.root_dir(), dir.path());
        assert_eq!(manager.objects_dir(), dir.path().join("objects"));
    }
}
