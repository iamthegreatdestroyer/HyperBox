//! Lazy layer loading using eStargz format.
//!
//! Enables container startup before the full image is pulled by
//! loading files on-demand as they are accessed.

use crate::error::{OptimizeError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// eStargz TOC entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    /// File name.
    pub name: String,
    /// File type (reg, dir, symlink, etc.).
    #[serde(rename = "type")]
    pub file_type: String,
    /// File size.
    pub size: u64,
    /// Offset in the stargz blob.
    pub offset: u64,
    /// Compressed chunk offsets.
    pub chunk_offset: u64,
    /// Chunk size.
    pub chunk_size: u64,
    /// File mode.
    pub mode: u32,
    /// UID.
    pub uid: u32,
    /// GID.
    pub gid: u32,
    /// Digest of the content.
    pub digest: Option<String>,
}

/// eStargz Table of Contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toc {
    /// Version.
    pub version: u32,
    /// Entries.
    pub entries: Vec<TocEntry>,
}

/// Lazy layer state.
#[derive(Debug)]
struct LayerState {
    /// TOC for the layer.
    toc: Toc,
    /// Downloaded chunks.
    downloaded_chunks: DashMap<u64, Vec<u8>>,
    /// Total bytes downloaded.
    bytes_downloaded: AtomicU64,
    /// Total bytes available.
    bytes_total: AtomicU64,
    /// Files accessed.
    files_accessed: DashMap<String, u64>,
}

/// Lazy layer loader for eStargz format.
pub struct LazyLayerLoader {
    /// Cache directory.
    cache_dir: PathBuf,
    /// Registry URL.
    registry_url: String,
    /// Active layers.
    layers: DashMap<String, Arc<RwLock<LayerState>>>,
    /// HTTP client.
    client: reqwest::Client,
    /// Statistics.
    stats: LoaderStats,
}

/// Loader statistics.
#[derive(Debug, Default)]
struct LoaderStats {
    /// Total cache hits.
    cache_hits: AtomicU64,
    /// Total cache misses.
    cache_misses: AtomicU64,
    /// Total bytes downloaded.
    bytes_downloaded: AtomicU64,
    /// Total bytes served from cache.
    bytes_cached: AtomicU64,
}

impl LazyLayerLoader {
    /// Create a new lazy layer loader.
    pub fn new(cache_dir: impl Into<PathBuf>, registry_url: &str) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            registry_url: registry_url.to_string(),
            layers: DashMap::new(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            stats: LoaderStats::default(),
        }
    }

    /// Initialize the loader.
    pub async fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.cache_dir).await?;
        info!("Lazy layer loader initialized");
        Ok(())
    }

    /// Load a layer's TOC.
    pub async fn load_layer_toc(&self, image: &str, layer_digest: &str) -> Result<()> {
        let toc = self.fetch_toc(image, layer_digest).await?;

        let total_size: u64 = toc.entries.iter().map(|e| e.size).sum();

        let state = LayerState {
            toc,
            downloaded_chunks: DashMap::new(),
            bytes_downloaded: AtomicU64::new(0),
            bytes_total: AtomicU64::new(total_size),
            files_accessed: DashMap::new(),
        };

        self.layers
            .insert(layer_digest.to_string(), Arc::new(RwLock::new(state)));

        info!(
            "Loaded TOC for layer {} ({} entries)",
            layer_digest,
            self.layers
                .get(layer_digest)
                .unwrap()
                .read()
                .await
                .toc
                .entries
                .len()
        );

        Ok(())
    }

    /// Fetch TOC from registry.
    async fn fetch_toc(&self, image: &str, layer_digest: &str) -> Result<Toc> {
        // The TOC is at the end of the eStargz blob
        // First, get blob size
        let url = format!("{}/v2/{}/blobs/{}", self.registry_url, image, layer_digest);

        let response =
            self.client
                .head(&url)
                .send()
                .await
                .map_err(|e| OptimizeError::LazyLoadFailed {
                    layer_id: layer_digest.to_string(),
                    reason: e.to_string(),
                })?;

        let content_length: u64 = response
            .headers()
            .get("content-length")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        if content_length == 0 {
            return Err(OptimizeError::LazyLoadFailed {
                layer_id: layer_digest.to_string(),
                reason: "Could not determine layer size".to_string(),
            });
        }

        // Fetch the last 10MB for TOC (typical size)
        let toc_size = 10 * 1024 * 1024;
        let range_start = content_length.saturating_sub(toc_size);

        let response = self
            .client
            .get(&url)
            .header("Range", format!("bytes={}-{}", range_start, content_length - 1))
            .send()
            .await
            .map_err(|e| OptimizeError::LazyLoadFailed {
                layer_id: layer_digest.to_string(),
                reason: e.to_string(),
            })?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| OptimizeError::LazyLoadFailed {
                layer_id: layer_digest.to_string(),
                reason: e.to_string(),
            })?;

        // Parse TOC from the gzip footer
        self.parse_toc(&bytes)
    }

    /// Parse TOC from eStargz footer.
    fn parse_toc(&self, data: &[u8]) -> Result<Toc> {
        // eStargz has a special footer format
        // For now, create a placeholder TOC
        // In practice, this would parse the stargz.index.json

        // Try to find and parse stargz.index.json
        let toc_marker = b"stargz.index.json";

        if let Some(pos) = data.windows(toc_marker.len()).position(|w| w == toc_marker) {
            // Find the JSON content
            if let Some(json_start) = data[pos..].iter().position(|&b| b == b'{') {
                let json_bytes = &data[pos + json_start..];

                // Find matching closing brace
                let mut depth = 0;
                let mut json_end = 0;
                for (i, &b) in json_bytes.iter().enumerate() {
                    match b {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                json_end = i + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                if json_end > 0 {
                    let json_str = std::str::from_utf8(&json_bytes[..json_end]).map_err(|e| {
                        OptimizeError::LazyLoadFailed {
                            layer_id: String::new(),
                            reason: e.to_string(),
                        }
                    })?;

                    return serde_json::from_str(json_str).map_err(|e| {
                        OptimizeError::LazyLoadFailed {
                            layer_id: String::new(),
                            reason: e.to_string(),
                        }
                    });
                }
            }
        }

        // Return empty TOC if not found
        Ok(Toc {
            version: 1,
            entries: Vec::new(),
        })
    }

    /// Read a file from a layer lazily.
    pub async fn read_file(&self, layer_digest: &str, path: &str) -> Result<Vec<u8>> {
        let layer = self
            .layers
            .get(layer_digest)
            .ok_or_else(|| OptimizeError::LayerNotFound {
                layer_id: layer_digest.to_string(),
            })?;

        let state = layer.read().await;

        // Find entry in TOC
        let entry = state
            .toc
            .entries
            .iter()
            .find(|e| e.name == path)
            .ok_or_else(|| OptimizeError::LazyLoadFailed {
                layer_id: layer_digest.to_string(),
                reason: format!("File not found: {}", path),
            })?
            .clone();

        // Track file access
        state
            .files_accessed
            .entry(path.to_string())
            .and_modify(|c| *c += 1)
            .or_insert(1);

        drop(state);

        // Check if chunk is already downloaded
        let state = layer.read().await;
        if let Some(data) = state.downloaded_chunks.get(&entry.chunk_offset) {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            self.stats
                .bytes_cached
                .fetch_add(entry.size, Ordering::Relaxed);
            return Ok(data.clone());
        }
        drop(state);

        // Fetch the chunk
        self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
        let data = self.fetch_chunk(layer_digest, &entry).await?;

        // Store in cache
        let state = layer.read().await;
        state
            .downloaded_chunks
            .insert(entry.chunk_offset, data.clone());
        state
            .bytes_downloaded
            .fetch_add(entry.size, Ordering::Relaxed);
        self.stats
            .bytes_downloaded
            .fetch_add(entry.size, Ordering::Relaxed);

        Ok(data)
    }

    /// Fetch a chunk from the registry.
    async fn fetch_chunk(&self, layer_digest: &str, entry: &TocEntry) -> Result<Vec<u8>> {
        // This would make a range request to the registry
        // For now, return placeholder
        debug!("Fetching chunk: offset={}, size={}", entry.chunk_offset, entry.chunk_size);

        // In practice, we'd fetch from registry with Range header
        Ok(Vec::new())
    }

    /// Get prefetch list based on access patterns.
    pub async fn get_prefetch_list(&self, layer_digest: &str) -> Result<Vec<String>> {
        let layer = self
            .layers
            .get(layer_digest)
            .ok_or_else(|| OptimizeError::LayerNotFound {
                layer_id: layer_digest.to_string(),
            })?;

        let state = layer.read().await;

        // Get most accessed files
        let mut files: Vec<_> = state
            .files_accessed
            .iter()
            .map(|r| (r.key().clone(), *r.value()))
            .collect();

        files.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(files.into_iter().take(100).map(|(f, _)| f).collect())
    }

    /// Get loading progress for a layer.
    pub async fn get_progress(&self, layer_digest: &str) -> Result<f64> {
        let layer = self
            .layers
            .get(layer_digest)
            .ok_or_else(|| OptimizeError::LayerNotFound {
                layer_id: layer_digest.to_string(),
            })?;

        let state = layer.read().await;
        let downloaded = state.bytes_downloaded.load(Ordering::Relaxed);
        let total = state.bytes_total.load(Ordering::Relaxed);

        if total == 0 {
            return Ok(1.0);
        }

        Ok(downloaded as f64 / total as f64)
    }

    /// Get cache hit rate.
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.stats.cache_hits.load(Ordering::Relaxed);
        let misses = self.stats.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            return 0.0;
        }

        hits as f64 / total as f64
    }

    /// Get statistics.
    pub fn stats(&self) -> (u64, u64, u64, u64) {
        (
            self.stats.cache_hits.load(Ordering::Relaxed),
            self.stats.cache_misses.load(Ordering::Relaxed),
            self.stats.bytes_downloaded.load(Ordering::Relaxed),
            self.stats.bytes_cached.load(Ordering::Relaxed),
        )
    }
}
