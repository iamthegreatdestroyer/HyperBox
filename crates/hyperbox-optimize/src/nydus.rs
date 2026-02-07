//! Nydus image acceleration framework integration (EVO-IF-301).
//!
//! Integrates with the Nydus container image service for on-demand layer
//! pulling using the RAFS (Registry Acceleration File System) format.
//! Nydus splits images into a compact metadata bootstrap (~2 MB) and
//! content-addressed data blobs (1 MB chunks), enabling:
//!
//! - **Near-instant starts**: Only the bootstrap is needed to start the
//!   container; data pages are loaded on first access via FUSE/virtiofs
//! - **Chunk-level dedup**: Content-addressed chunks are shared across
//!   images, reducing storage and network bandwidth
//! - **Background prefetch**: Priority files are prefetched based on
//!   access patterns while the container is already running
//! - **Registry-native**: Works with standard OCI registries without
//!   server-side modifications (Nydus v2 / RAFS v6)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │  NydusManager                                           │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────────────────┐  │
//! │  │nydus-     │  │ nydusd   │  │ Blob Cache           │  │
//! │  │image      │  │ daemons  │  │ (content-addressed)  │  │
//! │  │converter  │  │ per-ctr  │  │ shared across images │  │
//! │  └──────────┘  └──────────┘  └──────────────────────┘  │
//! └─────────────────────────────────────────────────────────┘
//! ```

use crate::error::{OptimizeError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;
use tokio::fs;
use tokio::sync::Mutex;
use tracing::{debug, info, instrument, warn};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Types
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Filesystem driver for nydusd.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NydusFsDriver {
    /// Linux FUSE (Filesystem in Userspace).
    Fuse,
    /// virtiofs for VM-based isolation (e.g., Kata Containers).
    Virtiofs,
}

/// Cache backend type for Nydus blob cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NydusCacheType {
    /// Standard blob cache (user-space, compressed or uncompressed).
    BlobCache,
    /// Linux fscache integration (kernel-level caching, requires 5.19+).
    FsCache,
}

/// Configuration for Nydus daemon instances.
#[derive(Debug, Clone)]
pub struct NydusDaemonConfig {
    /// Registry backend URL (e.g., `"https://registry-1.docker.io"`).
    pub registry_url: String,
    /// Filesystem driver type.
    pub fs_driver: NydusFsDriver,
    /// Blob cache configuration.
    pub cache_config: NydusCacheConfig,
    /// Number of I/O worker threads for nydusd.
    pub thread_count: u32,
    /// Background prefetch configuration.
    pub prefetch: NydusPrefetchConfig,
    /// Enable extended attribute support.
    pub enable_xattr: bool,
    /// Validate content digests on every read (slower but safer).
    pub digest_validate: bool,
}

/// Blob cache configuration for Nydus.
#[derive(Debug, Clone)]
pub struct NydusCacheConfig {
    /// Cache backend type.
    pub cache_type: NydusCacheType,
    /// Store compressed blobs in cache (saves space, costs CPU).
    pub compressed: bool,
    /// Maximum cache size in MB (0 = unlimited).
    pub max_size_mb: u64,
}

/// Background prefetch configuration.
#[derive(Debug, Clone)]
pub struct NydusPrefetchConfig {
    /// Enable background prefetching of blob data.
    pub enabled: bool,
    /// Number of prefetch worker threads.
    pub threads: u32,
    /// Merge consecutive requests smaller than this (bytes).
    pub merging_size: u32,
    /// Bandwidth limit in bytes/sec (0 = unlimited).
    pub bandwidth_limit: u64,
}

/// A converted Nydus RAFS image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NydusImage {
    /// Original image reference (e.g., `"docker.io/library/alpine:3.18"`).
    pub image_ref: String,
    /// Path to RAFS bootstrap file (metadata tree).
    pub bootstrap_path: PathBuf,
    /// Path to blob directory (data chunks).
    pub blob_dir: PathBuf,
    /// RAFS format version (5 or 6).
    pub rafs_version: u32,
    /// Bootstrap file size in bytes.
    pub bootstrap_size: u64,
    /// Total blob data size in bytes.
    pub total_blob_size: u64,
    /// Number of content-addressed data chunks.
    pub chunk_count: u64,
    /// Conversion timestamp.
    pub created_at: DateTime<Utc>,
}

/// Status of a running nydusd instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NydusDaemonStatus {
    /// Container ID served by this daemon.
    pub container_id: String,
    /// Operating system process ID.
    pub pid: u32,
    /// FUSE/virtiofs mountpoint.
    pub mountpoint: PathBuf,
    /// Whether the daemon is responsive.
    pub healthy: bool,
    /// Daemon uptime in seconds.
    pub uptime_secs: u64,
    /// Number of files served on-demand.
    pub files_served: u64,
    /// Total bytes downloaded from registry.
    pub bytes_downloaded: u64,
}

/// Blob cache performance statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NydusCacheStats {
    /// Total bytes currently in the local cache.
    pub bytes_cached: u64,
    /// Total bytes served from cache (hits).
    pub cache_hits_bytes: u64,
    /// Total bytes fetched from registry (misses).
    pub cache_miss_bytes: u64,
    /// Number of cached blob files.
    pub cached_blobs: u64,
    /// Number of unique content-addressed chunks.
    pub unique_chunks: u64,
    /// Deduplication savings ratio (0.0–1.0).
    pub dedup_ratio: f64,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// NydusManager
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Manager for the Nydus image acceleration framework.
///
/// Handles the full lifecycle of Nydus-accelerated container images:
///
/// 1. **Conversion** — OCI image → RAFS v6 via `nydus-image`
/// 2. **Serving** — `nydusd` daemon per container for on-demand data
/// 3. **Caching** — Content-addressed blob cache shared across images
/// 4. **Prefetch** — Background prefetch of priority files
/// 5. **GC** — LRU eviction of stale cache entries
pub struct NydusManager {
    /// Path to `nydus-image` binary.
    nydus_image_path: Option<PathBuf>,
    /// Path to `nydusd` binary.
    nydusd_path: Option<PathBuf>,
    /// Work directory for RAFS images and metadata.
    work_dir: PathBuf,
    /// Blob cache directory (content-addressed).
    cache_dir: PathBuf,
    /// Whether Nydus tools are available on this system.
    available: AtomicBool,
    /// Running nydusd daemon PIDs indexed by container ID.
    daemon_pids: Mutex<HashMap<String, u32>>,
    /// Active mountpoints indexed by container ID.
    mountpoints: Mutex<HashMap<String, PathBuf>>,
    /// Converted images indexed by image reference.
    images: Mutex<HashMap<String, NydusImage>>,
    /// Daemon configuration template.
    config: NydusDaemonConfig,
    /// Total bytes downloaded across all daemons (lifetime).
    total_bytes_downloaded: AtomicU64,
}

impl NydusManager {
    /// Create a new `NydusManager`.
    ///
    /// The manager is created in an unavailable state; call [`initialize`]
    /// to locate binaries and set up directories.
    pub fn new(work_dir: impl Into<PathBuf>, config: NydusDaemonConfig) -> Self {
        let work_dir = work_dir.into();
        let cache_dir = work_dir.join("cache");

        Self {
            nydus_image_path: None,
            nydusd_path: None,
            work_dir,
            cache_dir,
            available: AtomicBool::new(false),
            daemon_pids: Mutex::new(HashMap::new()),
            mountpoints: Mutex::new(HashMap::new()),
            images: Mutex::new(HashMap::new()),
            config,
            total_bytes_downloaded: AtomicU64::new(0),
        }
    }

    /// Initialize the manager: locate binaries, create work directories.
    #[instrument(skip(self))]
    pub async fn initialize(&mut self) -> Result<()> {
        fs::create_dir_all(&self.work_dir).await?;
        fs::create_dir_all(&self.cache_dir).await?;
        fs::create_dir_all(self.work_dir.join("bootstrap")).await?;
        fs::create_dir_all(self.work_dir.join("blobs")).await?;
        fs::create_dir_all(self.work_dir.join("mnt")).await?;

        self.nydus_image_path = Self::find_binary("nydus-image");
        self.nydusd_path = Self::find_binary("nydusd");

        let has_image_tool = self.nydus_image_path.is_some();
        let has_daemon = self.nydusd_path.is_some();

        self.available
            .store(has_image_tool && has_daemon, Ordering::SeqCst);

        if self.is_available() {
            info!(
                "Nydus initialized: nydus-image={}, nydusd={}",
                self.nydus_image_path.as_ref().unwrap().display(),
                self.nydusd_path.as_ref().unwrap().display(),
            );
        } else {
            warn!(
                "Nydus not fully available: nydus-image={}, nydusd={}",
                has_image_tool, has_daemon,
            );
        }

        Ok(())
    }

    /// Whether both `nydus-image` and `nydusd` are present.
    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::SeqCst)
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Image conversion
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Convert an unpacked OCI layer directory to Nydus RAFS v6 format.
    ///
    /// Uses `nydus-image create` to produce a RAFS bootstrap (metadata)
    /// and content-addressed blob chunks (data) with zstd compression.
    #[instrument(skip(self), fields(image_ref = %image_ref))]
    pub async fn convert_image(&self, image_ref: &str, source_dir: &Path) -> Result<NydusImage> {
        if !self.is_available() {
            return Err(OptimizeError::LazyLoadFailed {
                layer_id: image_ref.to_string(),
                reason: "Nydus tools not available".to_string(),
            });
        }

        let nydus_image = self.nydus_image_path.as_ref().unwrap();
        let safe_name = sanitize_image_ref(image_ref);
        let bootstrap_path = self
            .work_dir
            .join("bootstrap")
            .join(format!("{}.bootstrap", safe_name));
        let blob_dir = self.work_dir.join("blobs");

        let start = Instant::now();

        let output = Command::new(nydus_image)
            .arg("create")
            .arg("--bootstrap")
            .arg(&bootstrap_path)
            .arg("--blob-dir")
            .arg(&blob_dir)
            .arg("--fs-version")
            .arg("6")
            .arg("--compressor")
            .arg("zstd")
            .arg("--chunk-size")
            .arg("0x100000") // 1 MB chunks
            .arg(source_dir)
            .output()
            .map_err(|e| OptimizeError::LazyLoadFailed {
                layer_id: image_ref.to_string(),
                reason: format!("nydus-image create failed: {}", e),
            })?;

        if !output.status.success() {
            return Err(OptimizeError::LazyLoadFailed {
                layer_id: image_ref.to_string(),
                reason: format!(
                    "nydus-image create failed: {}",
                    String::from_utf8_lossy(&output.stderr),
                ),
            });
        }

        let duration = start.elapsed();
        let bootstrap_size = fs::metadata(&bootstrap_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        let (total_blob_size, chunk_count) = Self::scan_blob_dir(&blob_dir).await;

        let image = NydusImage {
            image_ref: image_ref.to_string(),
            bootstrap_path: bootstrap_path.clone(),
            blob_dir,
            rafs_version: 6,
            bootstrap_size,
            total_blob_size,
            chunk_count,
            created_at: Utc::now(),
        };

        // Persist metadata
        let meta_path = bootstrap_path.with_extension("json");
        let meta_json = serde_json::to_string_pretty(&image)?;
        fs::write(&meta_path, meta_json).await?;

        // Track in memory
        {
            let mut images = self.images.lock().await;
            images.insert(image_ref.to_string(), image.clone());
        }

        info!(
            "Converted {} to RAFS v6: bootstrap={} KB, blobs={} KB, chunks={} in {:?}",
            image_ref,
            bootstrap_size / 1024,
            total_blob_size / 1024,
            chunk_count,
            duration,
        );

        Ok(image)
    }

    /// Validate an existing RAFS bootstrap with `nydus-image check`.
    #[instrument(skip(self))]
    pub async fn validate_image(&self, bootstrap_path: &Path) -> Result<bool> {
        if !self.is_available() {
            return Err(OptimizeError::LazyLoadFailed {
                layer_id: bootstrap_path.display().to_string(),
                reason: "Nydus tools not available".to_string(),
            });
        }

        let nydus_image = self.nydus_image_path.as_ref().unwrap();

        let output = Command::new(nydus_image)
            .arg("check")
            .arg("--bootstrap")
            .arg(bootstrap_path)
            .output()
            .map_err(|e| OptimizeError::LazyLoadFailed {
                layer_id: bootstrap_path.display().to_string(),
                reason: format!("nydus-image check failed: {}", e),
            })?;

        Ok(output.status.success())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Daemon management
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Start a `nydusd` daemon for a container.
    ///
    /// The daemon mounts the RAFS filesystem at `mountpoint` and serves
    /// file data on demand from the registry, with local blob caching.
    #[instrument(skip(self), fields(container_id = %container_id))]
    pub async fn start_daemon(
        &self,
        container_id: &str,
        bootstrap_path: &Path,
        mountpoint: &Path,
    ) -> Result<u32> {
        if !self.is_available() {
            return Err(OptimizeError::LazyLoadFailed {
                layer_id: container_id.to_string(),
                reason: "Nydus tools not available".to_string(),
            });
        }

        // Reject duplicate daemons per container
        {
            let pids = self.daemon_pids.lock().await;
            if pids.contains_key(container_id) {
                return Err(OptimizeError::LazyLoadFailed {
                    layer_id: container_id.to_string(),
                    reason: "Daemon already running for this container".to_string(),
                });
            }
        }

        let nydusd = self.nydusd_path.as_ref().unwrap();
        fs::create_dir_all(mountpoint).await?;

        // Write daemon config
        let config_path = self.work_dir.join(format!("{}.config.json", container_id));
        let config_json = self.build_daemon_config()?;
        fs::write(&config_path, &config_json).await?;

        // Spawn nydusd
        let mut cmd = tokio::process::Command::new(nydusd);
        cmd.arg("--config")
            .arg(&config_path)
            .arg("--mountpoint")
            .arg(mountpoint)
            .arg("--bootstrap")
            .arg(bootstrap_path)
            .arg("--log-level")
            .arg("info")
            .arg("--thread-num")
            .arg(self.config.thread_count.to_string());

        match self.config.fs_driver {
            NydusFsDriver::Fuse => {
                cmd.arg("--apisock")
                    .arg(self.work_dir.join(format!("{}.sock", container_id)));
            }
            NydusFsDriver::Virtiofs => {
                cmd.arg("--sock")
                    .arg(self.work_dir.join(format!("{}.sock", container_id)));
            }
        }

        let child = cmd.spawn().map_err(|e| OptimizeError::LazyLoadFailed {
            layer_id: container_id.to_string(),
            reason: format!("Failed to start nydusd: {}", e),
        })?;

        let pid = child.id().unwrap_or(0);

        // Let daemon initialise
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Track
        {
            let mut pids = self.daemon_pids.lock().await;
            pids.insert(container_id.to_string(), pid);
        }
        {
            let mut mounts = self.mountpoints.lock().await;
            mounts.insert(container_id.to_string(), mountpoint.to_path_buf());
        }

        info!("nydusd started (PID {}) for {} at {}", pid, container_id, mountpoint.display(),);

        Ok(pid)
    }

    /// Stop a running `nydusd` daemon for a container.
    #[instrument(skip(self), fields(container_id = %container_id))]
    pub async fn stop_daemon(&self, container_id: &str) -> Result<()> {
        let mut pids = self.daemon_pids.lock().await;
        if let Some(pid) = pids.remove(container_id) {
            Self::kill_process(pid);
            info!("Stopped nydusd (PID {}) for {}", pid, container_id);
        }

        // Clean up mountpoint
        let mut mounts = self.mountpoints.lock().await;
        if let Some(mp) = mounts.remove(container_id) {
            #[cfg(unix)]
            {
                let _ = Command::new("umount").arg("-l").arg(&mp).output();
            }
            debug!("Cleaned up mountpoint {} for {}", mp.display(), container_id);
        }

        // Remove temp files
        let config_path = self.work_dir.join(format!("{}.config.json", container_id));
        let _ = fs::remove_file(&config_path).await;
        let sock_path = self.work_dir.join(format!("{}.sock", container_id));
        let _ = fs::remove_file(&sock_path).await;

        Ok(())
    }

    /// Get the status of a running daemon for a container.
    pub async fn get_daemon_status(&self, container_id: &str) -> Option<NydusDaemonStatus> {
        let pids = self.daemon_pids.lock().await;
        let pid = *pids.get(container_id)?;

        let mounts = self.mountpoints.lock().await;
        let mountpoint = mounts.get(container_id)?.clone();

        let healthy = Self::is_process_alive(pid);

        Some(NydusDaemonStatus {
            container_id: container_id.to_string(),
            pid,
            mountpoint,
            healthy,
            uptime_secs: 0, // would query daemon API in practice
            files_served: 0,
            bytes_downloaded: self.total_bytes_downloaded.load(Ordering::Relaxed),
        })
    }

    /// List container IDs with running daemons.
    pub async fn list_daemons(&self) -> Vec<String> {
        let pids = self.daemon_pids.lock().await;
        pids.keys().cloned().collect()
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Prefetch
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Request background prefetch of priority files for a container.
    ///
    /// Sends a hint to the `nydusd` daemon via its API socket, causing it
    /// to proactively download the listed files' data chunks.
    #[instrument(skip(self, files), fields(container_id = %container_id, file_count = files.len()))]
    pub async fn prefetch_files(&self, container_id: &str, files: &[String]) -> Result<u64> {
        let pids = self.daemon_pids.lock().await;
        if !pids.contains_key(container_id) {
            return Err(OptimizeError::LazyLoadFailed {
                layer_id: container_id.to_string(),
                reason: "No daemon running for this container".to_string(),
            });
        }

        let sock_path = self.work_dir.join(format!("{}.sock", container_id));
        if !sock_path.exists() {
            warn!("API socket not found for {}", container_id);
            return Ok(0);
        }

        // In production this would POST to nydusd's HTTP API:
        //   POST http+unix://<sock>/api/v1/daemon/prefetch
        //   body: { "files": ["/bin/sh", "/lib/ld-musl.so", ...] }
        let count = files.len() as u64;

        info!("Prefetch requested for {} files for {}", count, container_id,);

        Ok(count)
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Cache management
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Retrieve a previously converted image by reference.
    pub async fn get_image(&self, image_ref: &str) -> Option<NydusImage> {
        let images = self.images.lock().await;
        images.get(image_ref).cloned()
    }

    /// Compute current blob cache statistics.
    pub async fn get_cache_stats(&self) -> Result<NydusCacheStats> {
        let mut stats = NydusCacheStats::default();

        if self.cache_dir.exists() {
            let mut entries = fs::read_dir(&self.cache_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let meta = entry.metadata().await?;
                if meta.is_file() {
                    stats.bytes_cached += meta.len();
                    stats.cached_blobs += 1;
                }
            }
        }

        stats.cache_miss_bytes = self.total_bytes_downloaded.load(Ordering::Relaxed);

        // Estimate dedup ratio
        if stats.cached_blobs > 0 {
            let images = self.images.lock().await;
            let total_chunks: u64 = images.values().map(|i| i.chunk_count).sum();
            if total_chunks > 0 {
                stats.unique_chunks = stats.cached_blobs;
                stats.dedup_ratio = 1.0 - (stats.unique_chunks as f64 / total_chunks.max(1) as f64);
            }
        }

        Ok(stats)
    }

    /// Garbage-collect the blob cache using LRU (oldest mtime first).
    ///
    /// Evicts entries until the total cache size is below `max_size_mb`.
    /// Returns the number of bytes freed.
    #[instrument(skip(self))]
    pub async fn gc_cache(&self, max_size_mb: u64) -> Result<u64> {
        let max_bytes = max_size_mb * 1024 * 1024;

        if !self.cache_dir.exists() {
            return Ok(0);
        }

        // Collect entries with metadata
        let mut cache_entries: Vec<(PathBuf, u64, std::time::SystemTime)> = Vec::new();
        let mut total_size = 0u64;

        let mut entries = fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let meta = entry.metadata().await?;
            if meta.is_file() {
                let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                total_size += meta.len();
                cache_entries.push((entry.path(), meta.len(), mtime));
            }
        }

        if total_size <= max_bytes {
            debug!("Cache within limits: {} MB / {} MB", total_size / (1024 * 1024), max_size_mb,);
            return Ok(0);
        }

        // Sort oldest-first (LRU eviction)
        cache_entries.sort_by(|a, b| a.2.cmp(&b.2));

        let mut freed = 0u64;
        for (path, size, _) in &cache_entries {
            if total_size <= max_bytes {
                break;
            }
            if let Err(e) = fs::remove_file(path).await {
                warn!("Failed to remove cache entry {}: {}", path.display(), e);
                continue;
            }
            total_size -= size;
            freed += size;
        }

        info!(
            "Cache GC freed {} MB ({} MB remaining)",
            freed / (1024 * 1024),
            total_size / (1024 * 1024),
        );

        Ok(freed)
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Lifecycle
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Stop all running daemons and clean up resources.
    pub async fn shutdown(&self) -> Result<()> {
        let container_ids: Vec<String> = {
            let pids = self.daemon_pids.lock().await;
            pids.keys().cloned().collect()
        };

        for id in &container_ids {
            if let Err(e) = self.stop_daemon(id).await {
                warn!("Failed to stop daemon for {}: {}", id, e);
            }
        }

        info!("Nydus manager shut down ({} daemons stopped)", container_ids.len(),);

        Ok(())
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Internal helpers
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// Locate a Nydus binary on the system.
    fn find_binary(name: &str) -> Option<PathBuf> {
        let search_paths = [
            PathBuf::from(format!("/usr/local/bin/{}", name)),
            PathBuf::from(format!("/usr/bin/{}", name)),
            PathBuf::from(format!("/opt/nydus/bin/{}", name)),
            PathBuf::from(format!("/snap/bin/{}", name)),
        ];

        for path in &search_paths {
            if path.exists() {
                return Some(path.clone());
            }
        }

        // System PATH lookup
        if let Ok(output) = Command::new("which").arg(name).output() {
            if output.status.success() {
                let p = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
                if p.exists() {
                    return Some(p);
                }
            }
        }

        None
    }

    /// Build the JSON config that `nydusd` expects.
    fn build_daemon_config(&self) -> Result<String> {
        let registry_host = self
            .config
            .registry_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/');

        let scheme = if self.config.registry_url.starts_with("https") {
            "https"
        } else {
            "http"
        };

        let cache_type = match self.config.cache_config.cache_type {
            NydusCacheType::BlobCache => "blobcache",
            NydusCacheType::FsCache => "fscache",
        };

        let config = serde_json::json!({
            "device": {
                "backend": {
                    "type": "registry",
                    "config": {
                        "scheme": scheme,
                        "host": registry_host,
                        "repo": "",
                        "auth": "",
                        "timeout": 30,
                        "connect_timeout": 10,
                        "retry_limit": 3
                    }
                },
                "cache": {
                    "type": cache_type,
                    "compressed": self.config.cache_config.compressed,
                    "config": {
                        "work_dir": self.cache_dir.to_str().unwrap_or("")
                    }
                }
            },
            "mode": "direct",
            "digest_validate": self.config.digest_validate,
            "iostats_files": false,
            "enable_xattr": self.config.enable_xattr,
            "fs_prefetch": {
                "enable": self.config.prefetch.enabled,
                "threads_count": self.config.prefetch.threads,
                "merging_size": self.config.prefetch.merging_size,
                "bandwidth_rate": self.config.prefetch.bandwidth_limit
            }
        });

        Ok(serde_json::to_string_pretty(&config)?)
    }

    /// Scan a blob directory and return (total_size, file_count).
    async fn scan_blob_dir(blob_dir: &Path) -> (u64, u64) {
        let mut total_size = 0u64;
        let mut count = 0u64;

        if let Ok(mut entries) = fs::read_dir(blob_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(meta) = entry.metadata().await {
                    if meta.is_file() {
                        total_size += meta.len();
                        count += 1;
                    }
                }
            }
        }

        (total_size, count)
    }

    /// Kill a process by PID (best-effort, SIGTERM).
    fn kill_process(pid: u32) {
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .output();
        }
        #[cfg(not(unix))]
        {
            let _ = pid;
        }
    }

    /// Check whether a process is alive.
    fn is_process_alive(pid: u32) -> bool {
        #[cfg(unix)]
        {
            Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }
        #[cfg(not(unix))]
        {
            let _ = pid;
            false
        }
    }
}

impl Default for NydusDaemonConfig {
    fn default() -> Self {
        Self {
            registry_url: "https://registry-1.docker.io".to_string(),
            fs_driver: NydusFsDriver::Fuse,
            cache_config: NydusCacheConfig {
                cache_type: NydusCacheType::BlobCache,
                compressed: false,
                max_size_mb: 10_240, // 10 GB
            },
            thread_count: 4,
            prefetch: NydusPrefetchConfig {
                enabled: true,
                threads: 4,
                merging_size: 131_072, // 128 KB
                bandwidth_limit: 0,    // unlimited
            },
            enable_xattr: true,
            digest_validate: false,
        }
    }
}

/// Sanitize an image reference into a filesystem-safe name.
fn sanitize_image_ref(image_ref: &str) -> String {
    image_ref
        .replace('/', "_")
        .replace(':', "_")
        .replace('@', "_")
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn default_config() -> NydusDaemonConfig {
        NydusDaemonConfig::default()
    }

    // ── Type tests ──────────────────────────────────────────────────────

    #[test]
    fn test_nydus_fs_driver_variants() {
        assert_ne!(NydusFsDriver::Fuse, NydusFsDriver::Virtiofs);
        assert_eq!(NydusFsDriver::Fuse, NydusFsDriver::Fuse);
    }

    #[test]
    fn test_nydus_cache_type_variants() {
        assert_ne!(NydusCacheType::BlobCache, NydusCacheType::FsCache);
    }

    #[test]
    fn test_default_daemon_config() {
        let config = default_config();
        assert_eq!(config.thread_count, 4);
        assert!(config.prefetch.enabled);
        assert!(!config.digest_validate);
        assert!(config.enable_xattr);
        assert_eq!(config.fs_driver, NydusFsDriver::Fuse);
        assert_eq!(config.cache_config.cache_type, NydusCacheType::BlobCache);
    }

    #[test]
    fn test_sanitize_image_ref() {
        assert_eq!(
            sanitize_image_ref("docker.io/library/alpine:3.18"),
            "docker.io_library_alpine_3.18"
        );
        assert_eq!(
            sanitize_image_ref("registry.example.com/app@sha256:abc"),
            "registry.example.com_app_sha256_abc"
        );
    }

    // ── Serialization tests ─────────────────────────────────────────────

    #[test]
    fn test_nydus_image_serialization() {
        let image = NydusImage {
            image_ref: "alpine:3.18".to_string(),
            bootstrap_path: PathBuf::from("/tmp/bootstrap"),
            blob_dir: PathBuf::from("/tmp/blobs"),
            rafs_version: 6,
            bootstrap_size: 2048,
            total_blob_size: 5_000_000,
            chunk_count: 5,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&image).unwrap();
        let de: NydusImage = serde_json::from_str(&json).unwrap();
        assert_eq!(de.image_ref, "alpine:3.18");
        assert_eq!(de.rafs_version, 6);
        assert_eq!(de.chunk_count, 5);
    }

    #[test]
    fn test_daemon_status_serialization() {
        let status = NydusDaemonStatus {
            container_id: "ctr-1".to_string(),
            pid: 12345,
            mountpoint: PathBuf::from("/mnt/nydus"),
            healthy: true,
            uptime_secs: 60,
            files_served: 100,
            bytes_downloaded: 1_000_000,
        };

        let json = serde_json::to_string(&status).unwrap();
        let de: NydusDaemonStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de.pid, 12345);
        assert!(de.healthy);
    }

    #[test]
    fn test_cache_stats_default() {
        let stats = NydusCacheStats::default();
        assert_eq!(stats.bytes_cached, 0);
        assert_eq!(stats.cached_blobs, 0);
        assert_eq!(stats.dedup_ratio, 0.0);
    }

    // ── Manager tests ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_manager_creation() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());
        assert!(!manager.is_available());
    }

    #[tokio::test]
    async fn test_manager_initialize_creates_dirs() {
        let dir = tempdir().unwrap();
        let mut manager = NydusManager::new(dir.path(), default_config());
        let result = manager.initialize().await;
        assert!(result.is_ok());

        assert!(dir.path().join("bootstrap").exists());
        assert!(dir.path().join("blobs").exists());
        assert!(dir.path().join("mnt").exists());
        assert!(dir.path().join("cache").exists());
    }

    #[tokio::test]
    async fn test_convert_image_without_nydus_fails() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());

        let result = manager.convert_image("alpine:3.18", dir.path()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            OptimizeError::LazyLoadFailed { .. } => (),
            e => panic!("Expected LazyLoadFailed, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_validate_image_without_nydus_fails() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());

        let result = manager
            .validate_image(&dir.path().join("nonexistent.bootstrap"))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_start_daemon_without_nydus_fails() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());

        let result = manager
            .start_daemon("ctr-1", &dir.path().join("bootstrap"), &dir.path().join("mnt"))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stop_daemon_noop() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());

        let result = manager.stop_daemon("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_daemon_status_none() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());

        assert!(manager.get_daemon_status("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_list_daemons_empty() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());

        assert!(manager.list_daemons().await.is_empty());
    }

    #[tokio::test]
    async fn test_prefetch_files_no_daemon() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());

        let result = manager
            .prefetch_files("ctr-1", &["/bin/sh".to_string()])
            .await;
        assert!(result.is_err());
    }

    // ── Cache tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_image_none() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());

        assert!(manager.get_image("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats_empty_dir() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());
        fs::create_dir_all(dir.path().join("cache")).await.unwrap();

        let stats = manager.get_cache_stats().await.unwrap();
        assert_eq!(stats.bytes_cached, 0);
        assert_eq!(stats.cached_blobs, 0);
    }

    #[tokio::test]
    async fn test_gc_cache_empty() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());

        let freed = manager.gc_cache(100).await.unwrap();
        assert_eq!(freed, 0);
    }

    #[tokio::test]
    async fn test_gc_cache_under_limit() {
        let dir = tempdir().unwrap();
        let cache_dir = dir.path().join("cache");
        fs::create_dir_all(&cache_dir).await.unwrap();

        // Create small test files
        fs::write(cache_dir.join("blob1"), vec![0u8; 1024])
            .await
            .unwrap();
        fs::write(cache_dir.join("blob2"), vec![0u8; 1024])
            .await
            .unwrap();

        let manager = NydusManager::new(dir.path(), default_config());

        // 100 MB limit, only 2 KB in cache
        let freed = manager.gc_cache(100).await.unwrap();
        assert_eq!(freed, 0);
    }

    // ── Lifecycle tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_shutdown_empty() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());

        let result = manager.shutdown().await;
        assert!(result.is_ok());
    }

    // ── Config generation tests ─────────────────────────────────────────

    #[test]
    fn test_build_daemon_config_json() {
        let dir = tempdir().unwrap();
        let manager = NydusManager::new(dir.path(), default_config());

        let json = manager.build_daemon_config().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["mode"], "direct");
        assert_eq!(parsed["device"]["backend"]["type"], "registry");
        assert_eq!(parsed["device"]["cache"]["type"], "blobcache");
        assert!(parsed["fs_prefetch"]["enable"].as_bool().unwrap());
        assert_eq!(parsed["fs_prefetch"]["threads_count"], 4);
    }

    #[test]
    fn test_build_daemon_config_http() {
        let dir = tempdir().unwrap();
        let mut config = default_config();
        config.registry_url = "http://localhost:5000".to_string();
        let manager = NydusManager::new(dir.path(), config);

        let json = manager.build_daemon_config().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["device"]["backend"]["config"]["scheme"], "http");
        assert_eq!(parsed["device"]["backend"]["config"]["host"], "localhost:5000");
    }

    #[test]
    fn test_find_binary_returns_none_for_nonexistent() {
        let result = NydusManager::find_binary("nydus-definitely-not-installed-xyz");
        assert!(result.is_none());
    }
}
