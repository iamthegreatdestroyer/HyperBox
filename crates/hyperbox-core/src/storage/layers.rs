//! Image layer management.
//!
//! Provides layer caching, deduplication, and overlay filesystem support.

use crate::error::{CoreError, Result};
use dashmap::DashMap;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;
use tokio::fs;
use tracing::{debug, info};

/// Layer store for managing image layers.
pub struct LayerStore {
    /// Root directory for layers
    root_dir: PathBuf,
    /// Layer metadata cache
    layers: DashMap<String, LayerInfo>,
    /// Content-addressed storage
    cas_dir: PathBuf,
}

/// Layer information.
#[derive(Debug, Clone)]
pub struct LayerInfo {
    /// Layer digest (sha256:...)
    pub digest: String,
    /// Diff ID (uncompressed digest)
    pub diff_id: String,
    /// Size in bytes
    pub size: u64,
    /// Compressed size
    pub compressed_size: u64,
    /// Path to extracted layer
    pub path: PathBuf,
    /// Media type
    pub media_type: String,
    /// Reference count
    pub ref_count: u32,
}

impl LayerStore {
    /// Create a new layer store.
    #[must_use]
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        let root_dir = root_dir.into();
        let cas_dir = root_dir.join("blobs");

        Self {
            root_dir,
            cas_dir,
            layers: DashMap::new(),
        }
    }

    /// Initialize the layer store.
    pub async fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.root_dir).await?;
        fs::create_dir_all(&self.cas_dir).await?;
        fs::create_dir_all(self.root_dir.join("diff")).await?;
        fs::create_dir_all(self.root_dir.join("merged")).await?;

        info!("Initialized layer store at {:?}", self.root_dir);
        Ok(())
    }

    /// Get a layer by digest.
    #[must_use]
    pub fn get(&self, digest: &str) -> Option<LayerInfo> {
        self.layers.get(digest).map(|r| r.value().clone())
    }

    /// Check if a layer exists.
    #[must_use]
    pub fn has(&self, digest: &str) -> bool {
        self.layers.contains_key(digest)
    }

    /// Store a layer from a tar archive.
    pub async fn store_layer(&self, reader: impl Read, media_type: &str) -> Result<LayerInfo> {
        // Calculate digest while reading
        let mut hasher = Sha256::new();
        let mut data = Vec::new();
        let mut reader = reader;

        std::io::copy(&mut reader, &mut data)
            .map_err(|e| CoreError::StorageOperation(format!("read layer: {}", e)))?;

        hasher.update(&data);
        let hash = hasher.finalize();
        let digest = format!("sha256:{:x}", hash);

        // Check if already exists
        if let Some(info) = self.get(&digest) {
            debug!("Layer {} already exists", digest);
            return Ok(info);
        }

        // Store blob
        let blob_path = self.cas_dir.join(&digest);
        fs::write(&blob_path, &data).await?;

        // Extract layer
        let diff_dir = self.root_dir.join("diff").join(&digest);
        fs::create_dir_all(&diff_dir).await?;

        // Would extract tar here
        // For now, just store the info
        let info = LayerInfo {
            digest: digest.clone(),
            diff_id: digest.clone(), // Would be different for compressed layers
            size: data.len() as u64,
            compressed_size: data.len() as u64,
            path: diff_dir,
            media_type: media_type.to_string(),
            ref_count: 1,
        };

        self.layers.insert(digest, info.clone());
        Ok(info)
    }

    /// Extract a layer tar to a directory.
    pub async fn extract_layer(&self, digest: &str, target: &Path) -> Result<()> {
        let blob_path = self.cas_dir.join(digest);

        if !blob_path.exists() {
            return Err(CoreError::StorageOperation(format!(
                "extract layer: Layer {} not found",
                digest
            )));
        }

        fs::create_dir_all(target).await?;

        // Read the layer blob
        let data = fs::read(&blob_path).await?;

        // Decompress if gzipped
        let decompressed = if data.starts_with(&[0x1f, 0x8b]) {
            // GZip magic bytes
            let mut decoder = GzDecoder::new(&data[..]);
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| CoreError::StorageOperation(format!("decompress layer: {}", e)))?;
            decompressed
        } else {
            data
        };

        // Extract tar archive
        let mut archive = Archive::new(&decompressed[..]);

        archive
            .unpack(target)
            .map_err(|e| CoreError::StorageOperation(format!("extract tar: {}", e)))?;

        debug!("Extracted layer {} to {:?}", digest, target);
        Ok(())
    }

    /// Create an overlay mount for a container.
    #[cfg(unix)]
    pub async fn mount_overlay(
        &self,
        layer_digests: &[String],
        container_id: &str,
    ) -> Result<PathBuf> {
        let merged_dir = self.root_dir.join("merged").join(container_id);
        let work_dir = self.root_dir.join("work").join(container_id);
        let upper_dir = self.root_dir.join("upper").join(container_id);

        fs::create_dir_all(&merged_dir).await?;
        fs::create_dir_all(&work_dir).await?;
        fs::create_dir_all(&upper_dir).await?;

        // Build lower dirs string
        let lower_dirs: Vec<String> = layer_digests
            .iter()
            .filter_map(|d| {
                self.get(d)
                    .map(|info| info.path.to_string_lossy().to_string())
            })
            .collect();

        if lower_dirs.is_empty() {
            return Err(CoreError::StorageOperation("mount overlay: No layers found".to_string()));
        }

        let lowerdir = lower_dirs.join(":");

        info!("Mounting overlay for container {}", container_id);

        // mount -t overlay overlay -o lowerdir=...,upperdir=...,workdir=... merged
        let output = std::process::Command::new("mount")
            .args([
                "-t",
                "overlay",
                "overlay",
                "-o",
                &format!(
                    "lowerdir={},upperdir={},workdir={}",
                    lowerdir,
                    upper_dir.to_string_lossy(),
                    work_dir.to_string_lossy()
                ),
                merged_dir.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| CoreError::StorageOperation(format!("mount overlay: {}", e)))?;

        if !output.status.success() {
            return Err(CoreError::StorageOperation(format!(
                "mount overlay: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(merged_dir)
    }

    /// Unmount an overlay.
    #[cfg(unix)]
    pub async fn unmount_overlay(&self, container_id: &str) -> Result<()> {
        let merged_dir = self.root_dir.join("merged").join(container_id);

        if merged_dir.exists() {
            let _ = std::process::Command::new("umount")
                .arg(&merged_dir)
                .output();
        }

        // Cleanup directories
        let _ = fs::remove_dir_all(self.root_dir.join("work").join(container_id)).await;
        let _ = fs::remove_dir_all(self.root_dir.join("upper").join(container_id)).await;
        let _ = fs::remove_dir_all(&merged_dir).await;

        Ok(())
    }

    /// Increment reference count for a layer.
    pub fn add_ref(&self, digest: &str) {
        if let Some(mut layer) = self.layers.get_mut(digest) {
            layer.ref_count += 1;
        }
    }

    /// Decrement reference count for a layer.
    pub fn release(&self, digest: &str) -> bool {
        if let Some(mut layer) = self.layers.get_mut(digest) {
            layer.ref_count = layer.ref_count.saturating_sub(1);
            return layer.ref_count == 0;
        }
        false
    }

    /// Remove a layer (if ref count is zero).
    pub async fn remove(&self, digest: &str) -> Result<bool> {
        if let Some((_, info)) = self.layers.remove(digest) {
            if info.ref_count == 0 {
                let _ = fs::remove_file(self.cas_dir.join(digest)).await;
                let _ = fs::remove_dir_all(&info.path).await;
                return Ok(true);
            }
            // Re-insert if still referenced
            self.layers.insert(digest.to_string(), info);
        }
        Ok(false)
    }

    /// Get total storage used.
    pub fn total_size(&self) -> u64 {
        self.layers.iter().map(|l| l.value().size).sum()
    }

    /// List all layers.
    pub fn list(&self) -> Vec<LayerInfo> {
        self.layers.iter().map(|r| r.value().clone()).collect()
    }
}
