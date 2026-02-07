//! FastCDC Content-Defined Chunking & Deduplication Engine
//!
//! Implements sub-linear deduplication using:
//! - **FastCDC**: Content-defined chunking at >1 GiB/s throughput
//! - **Bloom Filter**: O(1) dedup checks with 1.2MB for 1M chunks at 1% FPR
//! - **Content Merkle Trees**: Logarithmic image diff in O(log n)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐    ┌──────────────┐    ┌─────────────┐
//! │  Raw Layer   │───▶│  FastCDC      │───▶│ Chunk Hashes│
//! │  Data        │    │  Chunker      │    │  + Sizes    │
//! └─────────────┘    └──────────────┘    └──────┬──────┘
//!                                                │
//!                     ┌──────────────┐           │
//!                     │ Bloom Filter │◀──────────┘
//!                     │  O(1) Check  │
//!                     └──────┬───────┘
//!                            │
//!                    ┌───────┴──────┐
//!                    │              │
//!                  New           Duplicate
//!                    │              │
//!              ┌─────▼─────┐  ┌────▼─────┐
//!              │  Compress  │  │ Ref Count│
//!              │  + Store   │  │ Increment│
//!              └───────────┘  └──────────┘
//! ```
//!
//! # Performance Targets
//!
//! - Chunking throughput: >1 GiB/s
//! - Dedup check: O(1) via bloom filter
//! - Image diff: O(log n) via content Merkle tree
//! - 10-15% better deduplication than fixed-size chunking

use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, info};

use crate::error::{OptimizeError, Result};

// ─── Constants ───────────────────────────────────────────────────────────────

/// Default minimum chunk size (2 KB).
const DEFAULT_MIN_CHUNK: usize = 2048;
/// Default average chunk size (8 KB).
const DEFAULT_AVG_CHUNK: usize = 8192;
/// Default maximum chunk size (64 KB).
const DEFAULT_MAX_CHUNK: usize = 65536;
/// Default normalization level for FastCDC.
const DEFAULT_NORMALIZATION: u32 = 1;
/// Default expected items for bloom filter (1 million chunks).
const DEFAULT_BLOOM_EXPECTED: usize = 1_000_000;
/// Default bloom filter false positive rate (1%).
const DEFAULT_BLOOM_FPR: f64 = 0.01;

// ─── Gear Table (Compile-Time Generated) ─────────────────────────────────────

/// Generate the gear table at compile time using splitmix64 mixing function.
/// Each byte value maps to a well-distributed u64 for the rolling hash.
const fn generate_gear_table() -> [u64; 256] {
    let mut table = [0u64; 256];
    let mut i: usize = 0;
    while i < 256 {
        // splitmix64 mixing function — excellent distribution
        let mut h = i as u64;
        h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 30;
        h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 27;
        h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
        h ^= h >> 31;
        table[i] = h;
        i += 1;
    }
    table
}

/// Pre-computed gear table for the rolling hash (256 entries, one per byte value).
static GEAR_TABLE: [u64; 256] = generate_gear_table();

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Compute SHA-256 hash of data, returning a fixed 32-byte array.
fn sha256_hash(data: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(data);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&digest);
    hash
}

/// Format a hash as a hex string (for display/logging).
fn hash_hex(hash: &[u8; 32]) -> String {
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

// ─── Configuration Types ─────────────────────────────────────────────────────

/// Configuration for FastCDC content-defined chunking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkConfig {
    /// Minimum chunk size in bytes.
    pub min_size: usize,
    /// Target average chunk size in bytes.
    pub avg_size: usize,
    /// Maximum chunk size in bytes.
    pub max_size: usize,
    /// Normalization level (1 or 2). Higher = more uniform chunk sizes.
    pub normalization: u32,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            min_size: DEFAULT_MIN_CHUNK,
            avg_size: DEFAULT_AVG_CHUNK,
            max_size: DEFAULT_MAX_CHUNK,
            normalization: DEFAULT_NORMALIZATION,
        }
    }
}

impl ChunkConfig {
    /// Create a configuration optimized for container image layers.
    /// Uses 4KB avg for high dedup ratio on typical layer content.
    #[must_use]
    pub fn for_container_layers() -> Self {
        Self {
            min_size: 1024,
            avg_size: 4096,
            max_size: 32768,
            normalization: 2,
        }
    }

    /// Create a configuration optimized for large binary blobs.
    /// Uses 16KB avg for higher throughput with acceptable dedup.
    #[must_use]
    pub fn for_large_blobs() -> Self {
        Self {
            min_size: 4096,
            avg_size: 16384,
            max_size: 131_072,
            normalization: 1,
        }
    }

    /// Validate configuration parameters.
    fn validate(&self) -> Result<()> {
        if self.min_size == 0 {
            return Err(OptimizeError::DedupFailed {
                reason: "min_size must be > 0".into(),
            });
        }
        if self.avg_size <= self.min_size {
            return Err(OptimizeError::DedupFailed {
                reason: "avg_size must be > min_size".into(),
            });
        }
        if self.max_size <= self.avg_size {
            return Err(OptimizeError::DedupFailed {
                reason: "max_size must be > avg_size".into(),
            });
        }
        if self.normalization == 0 || self.normalization > 3 {
            return Err(OptimizeError::DedupFailed {
                reason: "normalization must be 1-3".into(),
            });
        }
        Ok(())
    }
}

/// Compression mode for stored chunks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressionMode {
    /// No compression.
    None,
    /// Zstd compression with configurable level (1-22).
    Zstd {
        /// Compression level (1=fast, 22=max).
        level: i32,
    },
    /// LZ4 fast compression.
    Lz4,
}

impl Default for CompressionMode {
    fn default() -> Self {
        Self::Zstd { level: 3 }
    }
}

/// A chunk boundary identified by the FastCDC algorithm.
#[derive(Debug, Clone)]
pub struct ChunkBoundary {
    /// Byte offset of the chunk start in the original data.
    pub offset: usize,
    /// Length of the chunk in bytes.
    pub length: usize,
}

// ─── Bloom Filter ────────────────────────────────────────────────────────────

/// Space-efficient probabilistic set membership structure.
///
/// Provides O(1) "definitely not seen" / "probably seen" checks.
/// For 1M items at 1% FPR: ~1.2MB RAM, 7 hash functions.
pub struct BloomFilter {
    /// Bit array stored as u64 words.
    bits: Vec<u64>,
    /// Total number of bits in the filter.
    num_bits: usize,
    /// Number of hash functions.
    num_hashes: u32,
    /// Number of items inserted.
    num_items: usize,
}

impl std::fmt::Debug for BloomFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BloomFilter")
            .field("num_bits", &self.num_bits)
            .field("num_hashes", &self.num_hashes)
            .field("num_items", &self.num_items)
            .field("memory_bytes", &self.memory_bytes())
            .finish()
    }
}

impl BloomFilter {
    /// Create a new bloom filter sized for the expected number of items
    /// and desired false positive rate.
    ///
    /// # Arguments
    /// - `expected_items`: Expected number of unique items
    /// - `fpr`: Desired false positive rate (e.g., 0.01 for 1%)
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    #[must_use]
    pub fn new(expected_items: usize, fpr: f64) -> Self {
        let fpr = fpr.clamp(1e-10, 0.5);
        let n = expected_items.max(1) as f64;

        // m = -(n * ln(p)) / (ln(2))^2
        let ln2 = std::f64::consts::LN_2;
        let num_bits = (-(n * fpr.ln()) / (ln2 * ln2)).ceil() as usize;
        let num_bits = num_bits.max(64); // minimum 64 bits

        // k = (m/n) * ln(2)
        let num_hashes = ((num_bits as f64 / n) * ln2).ceil() as u32;
        let num_hashes = num_hashes.clamp(1, 30);

        let num_words = (num_bits + 63) / 64;
        Self {
            bits: vec![0u64; num_words],
            num_bits,
            num_hashes,
            num_items: 0,
        }
    }

    /// Create a bloom filter with explicit bit count and hash count.
    #[must_use]
    pub fn with_capacity(num_bits: usize, num_hashes: u32) -> Self {
        let num_bits = num_bits.max(64);
        let num_words = (num_bits + 63) / 64;
        Self {
            bits: vec![0u64; num_words],
            num_bits,
            num_hashes: num_hashes.max(1),
            num_items: 0,
        }
    }

    /// Compute hash indices using Enhanced Double Hashing.
    /// Derives k positions from a single SHA-256 hash by splitting
    /// the 32-byte hash into two 8-byte halves for h1 and h2.
    fn hash_indices(&self, key: &[u8; 32]) -> impl Iterator<Item = usize> + '_ {
        let h1 = u64::from_le_bytes([
            key[0], key[1], key[2], key[3], key[4], key[5], key[6], key[7],
        ]);
        let h2 = u64::from_le_bytes([
            key[8], key[9], key[10], key[11], key[12], key[13], key[14], key[15],
        ]);
        let num_bits = self.num_bits;
        (0..self.num_hashes).map(move |i| {
            let combined = h1.wrapping_add(u64::from(i).wrapping_mul(h2));
            #[allow(clippy::cast_possible_truncation)]
            let idx = (combined % num_bits as u64) as usize;
            idx
        })
    }

    /// Insert a chunk hash into the bloom filter.
    pub fn insert(&mut self, key: &[u8; 32]) {
        let indices: Vec<usize> = self.hash_indices(key).collect();
        for idx in indices {
            let word = idx / 64;
            let bit = idx % 64;
            self.bits[word] |= 1u64 << bit;
        }
        self.num_items += 1;
    }

    /// Check if a chunk hash might be in the set.
    /// Returns `false` → definitely not present.
    /// Returns `true` → probably present (with configured FPR).
    #[must_use]
    pub fn possibly_contains(&self, key: &[u8; 32]) -> bool {
        self.hash_indices(key).all(|idx| {
            let word = idx / 64;
            let bit = idx % 64;
            (self.bits[word] >> bit) & 1 == 1
        })
    }

    /// Estimate the current false positive rate based on fill ratio.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn estimated_fpr(&self) -> f64 {
        if self.num_items == 0 {
            return 0.0;
        }
        let m = self.num_bits as f64;
        let k = f64::from(self.num_hashes);
        let n = self.num_items as f64;
        // FPR ≈ (1 - e^(-kn/m))^k
        (1.0 - (-k * n / m).exp()).powf(k)
    }

    /// Number of items currently in the filter.
    #[must_use]
    pub fn items_count(&self) -> usize {
        self.num_items
    }

    /// Memory usage of the filter in bytes.
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.bits.len() * 8
    }

    /// Fill ratio (proportion of bits set to 1).
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn fill_ratio(&self) -> f64 {
        let set_bits: usize = self.bits.iter().map(|w| w.count_ones() as usize).sum();
        set_bits as f64 / self.num_bits as f64
    }

    /// Clear all entries from the filter.
    pub fn clear(&mut self) {
        self.bits.fill(0);
        self.num_items = 0;
    }
}

// ─── FastCDC Chunker ─────────────────────────────────────────────────────────

/// FastCDC content-defined chunking engine.
///
/// Uses gear-based rolling hash with normalized chunking to split data
/// into variable-size chunks at content-determined boundaries. This produces
/// chunks that are stable across insertions/deletions in the data stream,
/// enabling 10-15% better deduplication than fixed-size chunking.
pub struct FastCdcChunker {
    /// Chunking configuration.
    config: ChunkConfig,
    /// Mask for min_size..avg_size range (easier to match → smaller chunks).
    mask_s: u64,
    /// Mask for avg_size..max_size range (harder to match → larger chunks).
    mask_l: u64,
}

impl std::fmt::Debug for FastCdcChunker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FastCdcChunker")
            .field("config", &self.config)
            .field("mask_s", &format_args!("{:#018x}", self.mask_s))
            .field("mask_l", &format_args!("{:#018x}", self.mask_l))
            .finish()
    }
}

impl FastCdcChunker {
    /// Create a new FastCDC chunker with the given configuration.
    ///
    /// # Errors
    /// Returns error if configuration is invalid.
    pub fn new(config: ChunkConfig) -> Result<Self> {
        config.validate()?;

        // Compute masks for normalized chunking.
        // bits = log2(avg_size)
        // mask_s has MORE bits set → easier to match (for min..avg range)
        // mask_l has FEWER bits set → harder to match (for avg..max range)
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let bits = (config.avg_size as f64).log2() as u32;
        let mask_s = (1u64 << (bits + config.normalization)) - 1;
        let mask_l = (1u64 << (bits.saturating_sub(config.normalization))) - 1;

        Ok(Self {
            config,
            mask_s,
            mask_l,
        })
    }

    /// Split data into variable-size chunks using FastCDC algorithm.
    ///
    /// Returns a list of chunk boundaries (offset + length).
    /// Each chunk's content hash is NOT computed here — the caller
    /// should hash each chunk separately for dedup checks.
    #[must_use]
    pub fn chunk(&self, data: &[u8]) -> Vec<ChunkBoundary> {
        if data.is_empty() {
            return Vec::new();
        }

        let mut boundaries = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let remaining = data.len() - offset;

            // If remaining data fits in one chunk, emit it all
            if remaining <= self.config.min_size {
                boundaries.push(ChunkBoundary {
                    offset,
                    length: remaining,
                });
                break;
            }

            let chunk_len = self.find_boundary(&data[offset..]);
            boundaries.push(ChunkBoundary {
                offset,
                length: chunk_len,
            });
            offset += chunk_len;
        }

        boundaries
    }

    /// Find the next chunk boundary using gear-based rolling hash.
    ///
    /// Uses two-phase normalized chunking:
    /// 1. `min_size..avg_size`: Use `mask_s` (easier to match → cut sooner)
    /// 2. `avg_size..max_size`: Use `mask_l` (harder to match → allow larger)
    fn find_boundary(&self, data: &[u8]) -> usize {
        let n = data.len().min(self.config.max_size);

        if n <= self.config.min_size {
            return n;
        }

        let mut hash: u64 = 0;
        let mut i = self.config.min_size;

        // Phase 1: min_size to avg_size with mask_s (easier boundary)
        let mid = n.min(self.config.avg_size);
        while i < mid {
            hash = (hash << 1).wrapping_add(GEAR_TABLE[data[i] as usize]);
            if (hash & self.mask_s) == 0 {
                return i + 1;
            }
            i += 1;
        }

        // Phase 2: avg_size to max_size with mask_l (harder boundary)
        while i < n {
            hash = (hash << 1).wrapping_add(GEAR_TABLE[data[i] as usize]);
            if (hash & self.mask_l) == 0 {
                return i + 1;
            }
            i += 1;
        }

        // No boundary found — cut at max_size
        n
    }

    /// Get the chunking configuration.
    #[must_use]
    pub fn config(&self) -> &ChunkConfig {
        &self.config
    }
}

// ─── Stored Chunk ────────────────────────────────────────────────────────────

/// A chunk stored in the chunk store (internal representation).
#[derive(Debug)]
#[allow(dead_code)]
struct StoredChunk {
    /// SHA-256 hash of the original (uncompressed) chunk data.
    hash: [u8; 32],
    /// Original (uncompressed) size in bytes.
    original_size: usize,
    /// Compressed chunk data.
    compressed_data: Vec<u8>,
    /// Compression mode used.
    compression: CompressionMode,
    /// Number of layers referencing this chunk.
    ref_count: u32,
    /// Timestamp when chunk was first stored.
    stored_at: chrono::DateTime<chrono::Utc>,
}

// ─── Chunk Store ─────────────────────────────────────────────────────────────

/// Concurrent chunk store with compression support.
///
/// Stores unique chunks in memory with optional compression (zstd or lz4).
/// Thread-safe via `DashMap` for concurrent access.
pub struct ChunkStore {
    /// Hash → stored chunk mapping.
    chunks: DashMap<[u8; 32], StoredChunk>,
    /// Base path for future disk persistence.
    base_path: PathBuf,
    /// Compression mode for new chunks.
    compression: CompressionMode,
    /// Total compressed bytes stored.
    total_stored_bytes: AtomicU64,
    /// Total original (uncompressed) bytes of stored chunks.
    total_original_bytes: AtomicU64,
    /// Count of unique chunks stored.
    unique_count: AtomicU64,
}

impl std::fmt::Debug for ChunkStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChunkStore")
            .field("base_path", &self.base_path)
            .field("compression", &self.compression)
            .field("unique_chunks", &self.unique_count.load(Ordering::Relaxed))
            .field("stored_bytes", &self.total_stored_bytes.load(Ordering::Relaxed))
            .finish()
    }
}

impl ChunkStore {
    /// Create a new chunk store.
    #[must_use]
    pub fn new(base_path: PathBuf, compression: CompressionMode) -> Self {
        Self {
            chunks: DashMap::new(),
            base_path,
            compression,
            total_stored_bytes: AtomicU64::new(0),
            total_original_bytes: AtomicU64::new(0),
            unique_count: AtomicU64::new(0),
        }
    }

    /// Store a chunk. Returns `Some(compressed_size)` if new, `None` if duplicate.
    pub fn store_chunk(&self, hash: [u8; 32], data: &[u8]) -> Result<Option<usize>> {
        // Check if already stored (fast path)
        if let Some(mut entry) = self.chunks.get_mut(&hash) {
            entry.ref_count += 1;
            return Ok(None);
        }

        // Compress and store
        let compressed = self.compress_data(data)?;
        let compressed_size = compressed.len();

        let chunk = StoredChunk {
            hash,
            original_size: data.len(),
            compressed_data: compressed,
            compression: self.compression,
            ref_count: 1,
            stored_at: chrono::Utc::now(),
        };

        // Use entry API to handle race condition
        self.chunks.entry(hash).or_insert_with(|| {
            #[allow(clippy::cast_possible_truncation)]
            self.total_stored_bytes
                .fetch_add(compressed_size as u64, Ordering::Relaxed);
            #[allow(clippy::cast_possible_truncation)]
            self.total_original_bytes
                .fetch_add(data.len() as u64, Ordering::Relaxed);
            self.unique_count.fetch_add(1, Ordering::Relaxed);
            chunk
        });

        Ok(Some(compressed_size))
    }

    /// Check if a chunk exists in the store.
    #[must_use]
    pub fn contains(&self, hash: &[u8; 32]) -> bool {
        self.chunks.contains_key(hash)
    }

    /// Increment the reference count for a chunk.
    pub fn increment_ref(&self, hash: &[u8; 32]) {
        if let Some(mut entry) = self.chunks.get_mut(hash) {
            entry.ref_count += 1;
        }
    }

    /// Retrieve and decompress a chunk.
    pub fn get_chunk(&self, hash: &[u8; 32]) -> Result<Vec<u8>> {
        let entry = self
            .chunks
            .get(hash)
            .ok_or_else(|| OptimizeError::DedupFailed {
                reason: format!("Chunk not found: {}", hash_hex(hash)),
            })?;
        self.decompress_data(&entry.compressed_data, &entry.compression)
    }

    /// Total compressed bytes stored.
    #[must_use]
    pub fn total_stored_bytes(&self) -> u64 {
        self.total_stored_bytes.load(Ordering::Relaxed)
    }

    /// Total original (uncompressed) bytes of stored chunks.
    #[must_use]
    pub fn total_original_bytes(&self) -> u64 {
        self.total_original_bytes.load(Ordering::Relaxed)
    }

    /// Number of unique chunks in the store.
    #[must_use]
    pub fn unique_chunks(&self) -> u64 {
        self.unique_count.load(Ordering::Relaxed)
    }

    /// Compression ratio (compressed / original). Lower is better.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn compression_ratio(&self) -> f64 {
        let original = self.total_original_bytes.load(Ordering::Relaxed);
        let stored = self.total_stored_bytes.load(Ordering::Relaxed);
        if original == 0 {
            return 1.0;
        }
        stored as f64 / original as f64
    }

    /// Remove a chunk from the store (decrement ref count, remove if zero).
    pub fn release_chunk(&self, hash: &[u8; 32]) -> bool {
        let mut removed = false;
        self.chunks.remove_if(hash, |_, chunk| {
            if chunk.ref_count <= 1 {
                removed = true;
                true // remove the entry
            } else {
                false
            }
        });
        // If not removed, just decrement
        if !removed {
            if let Some(mut entry) = self.chunks.get_mut(hash) {
                entry.ref_count = entry.ref_count.saturating_sub(1);
            }
        }
        removed
    }

    /// Compress data according to the configured mode.
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.compression {
            CompressionMode::None => Ok(data.to_vec()),
            CompressionMode::Zstd { level } => {
                zstd::encode_all(io::Cursor::new(data), level).map_err(OptimizeError::Io)
            }
            CompressionMode::Lz4 => Ok(lz4_flex::compress_prepend_size(data)),
        }
    }

    /// Decompress data according to the specified mode.
    fn decompress_data(&self, data: &[u8], compression: &CompressionMode) -> Result<Vec<u8>> {
        match compression {
            CompressionMode::None => Ok(data.to_vec()),
            CompressionMode::Zstd { .. } => {
                zstd::decode_all(io::Cursor::new(data)).map_err(OptimizeError::Io)
            }
            CompressionMode::Lz4 => {
                lz4_flex::decompress_size_prepended(data).map_err(|e| OptimizeError::DedupFailed {
                    reason: format!("LZ4 decompression failed: {e}"),
                })
            }
        }
    }
}

// ─── Dedup Result ────────────────────────────────────────────────────────────

/// Result of processing a single layer through the deduplicator.
#[derive(Debug, Clone)]
pub struct DedupResult {
    /// Total number of chunks produced.
    pub total_chunks: usize,
    /// Number of new (unique) chunks stored.
    pub new_chunks: usize,
    /// Number of duplicate chunks found.
    pub duplicate_chunks: usize,
    /// Original layer size in bytes.
    pub original_size: u64,
    /// Compressed size of newly stored chunks.
    pub stored_size: u64,
    /// Dedup ratio: duplicate_chunks / total_chunks (0.0 - 1.0).
    pub dedup_ratio: f64,
    /// SHA-256 hashes of all chunks (in order).
    pub chunk_hashes: Vec<[u8; 32]>,
    /// Time taken to process the layer.
    pub processing_time: Duration,
}

// ─── Dedup Stats ─────────────────────────────────────────────────────────────

/// Cumulative deduplication statistics across all processed layers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DedupStats {
    /// Total bytes of raw data processed.
    pub total_bytes_processed: u64,
    /// Total compressed bytes stored (new chunks only).
    pub total_bytes_stored: u64,
    /// Total number of chunks seen across all layers.
    pub total_chunks_seen: u64,
    /// Number of unique chunks stored.
    pub unique_chunks: u64,
    /// Number of duplicate chunks detected.
    pub duplicate_chunks: u64,
    /// Number of bloom filter checks performed.
    pub bloom_checks: u64,
    /// Number of bloom filter false positives.
    pub bloom_false_positives: u64,
    /// Number of layers processed.
    pub layers_processed: u64,
    /// Average chunk size across all chunks.
    pub average_chunk_size: f64,
    /// Overall dedup ratio (0.0 = no dedup, 1.0 = all dupes).
    pub dedup_ratio: f64,
    /// Compression ratio (compressed / original, lower = better).
    pub compression_ratio: f64,
}

// ─── Chunk Deduplicator ──────────────────────────────────────────────────────

/// FastCDC content-defined chunking deduplicator.
///
/// Combines FastCDC chunking, bloom filter indexing, and compressed chunk
/// storage for wire-speed deduplication of container image layers.
pub struct ChunkDeduplicator {
    /// FastCDC chunking engine.
    chunker: FastCdcChunker,
    /// Bloom filter for fast "not seen" checks.
    bloom: RwLock<BloomFilter>,
    /// Compressed chunk store.
    store: ChunkStore,
    /// Cumulative statistics.
    stats: RwLock<DedupStats>,
}

impl std::fmt::Debug for ChunkDeduplicator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChunkDeduplicator")
            .field("chunker", &self.chunker)
            .field("bloom", &*self.bloom.read())
            .field("store", &self.store)
            .finish()
    }
}

impl ChunkDeduplicator {
    /// Create a new deduplicator with default bloom filter settings.
    ///
    /// Uses 1M expected chunks at 1% FPR (~1.2MB bloom filter).
    pub fn new(config: ChunkConfig, store_path: PathBuf) -> Result<Self> {
        Self::with_options(
            config,
            store_path,
            CompressionMode::default(),
            DEFAULT_BLOOM_EXPECTED,
            DEFAULT_BLOOM_FPR,
        )
    }

    /// Create a deduplicator with custom compression mode.
    pub fn with_compression(
        config: ChunkConfig,
        store_path: PathBuf,
        compression: CompressionMode,
    ) -> Result<Self> {
        Self::with_options(
            config,
            store_path,
            compression,
            DEFAULT_BLOOM_EXPECTED,
            DEFAULT_BLOOM_FPR,
        )
    }

    /// Create a deduplicator with full customization.
    pub fn with_options(
        config: ChunkConfig,
        store_path: PathBuf,
        compression: CompressionMode,
        expected_chunks: usize,
        fpr: f64,
    ) -> Result<Self> {
        let chunker = FastCdcChunker::new(config)?;
        let bloom = BloomFilter::new(expected_chunks, fpr);
        let store = ChunkStore::new(store_path, compression);

        info!(
            bloom_memory_kb = bloom.memory_bytes() / 1024,
            bloom_hashes = bloom.num_hashes,
            expected_chunks,
            fpr,
            "Initialized ChunkDeduplicator"
        );

        Ok(Self {
            chunker,
            bloom: RwLock::new(bloom),
            store,
            stats: RwLock::new(DedupStats::default()),
        })
    }

    /// Process a layer for deduplication.
    ///
    /// Splits the layer into chunks, checks each against the bloom filter
    /// and chunk store, and stores only new (unique) chunks with compression.
    #[allow(clippy::cast_precision_loss)]
    pub fn process_layer(&self, layer_id: &str, data: &[u8]) -> Result<DedupResult> {
        let start = Instant::now();
        info!(layer_id, data_len = data.len(), "Processing layer for deduplication");

        let boundaries = self.chunker.chunk(data);
        let mut new_chunks = 0usize;
        let mut dup_chunks = 0usize;
        let mut chunk_hashes = Vec::with_capacity(boundaries.len());
        let mut new_stored_bytes = 0u64;

        for boundary in &boundaries {
            let chunk_data = &data[boundary.offset..boundary.offset + boundary.length];
            let hash = sha256_hash(chunk_data);
            chunk_hashes.push(hash);

            // Fast path: bloom filter check
            let might_exist = self.bloom.read().possibly_contains(&hash);
            self.stats.write().bloom_checks += 1;

            if might_exist && self.store.contains(&hash) {
                // Confirmed duplicate
                self.store.increment_ref(&hash);
                dup_chunks += 1;
                debug!(
                    chunk_offset = boundary.offset,
                    chunk_len = boundary.length,
                    "Duplicate chunk"
                );
            } else {
                // New chunk (or bloom false positive)
                if might_exist {
                    self.stats.write().bloom_false_positives += 1;
                }
                match self.store.store_chunk(hash, chunk_data)? {
                    Some(compressed_size) => {
                        self.bloom.write().insert(&hash);
                        new_chunks += 1;
                        new_stored_bytes += compressed_size as u64;
                    }
                    None => {
                        // Race: another thread stored it between our check and store
                        dup_chunks += 1;
                    }
                }
            }
        }

        // Update cumulative stats
        {
            let mut stats = self.stats.write();
            stats.total_bytes_processed += data.len() as u64;
            stats.total_bytes_stored += new_stored_bytes;
            stats.total_chunks_seen += boundaries.len() as u64;
            stats.unique_chunks += new_chunks as u64;
            stats.duplicate_chunks += dup_chunks as u64;
            stats.layers_processed += 1;

            if stats.total_chunks_seen > 0 {
                stats.average_chunk_size =
                    stats.total_bytes_processed as f64 / stats.total_chunks_seen as f64;
                stats.dedup_ratio =
                    1.0 - (stats.unique_chunks as f64 / stats.total_chunks_seen as f64);
            }
            stats.compression_ratio = self.store.compression_ratio();
        }

        let total_chunks = boundaries.len();
        let dedup_ratio = if total_chunks > 0 {
            dup_chunks as f64 / total_chunks as f64
        } else {
            0.0
        };

        info!(
            layer_id,
            total_chunks,
            new_chunks,
            dup_chunks,
            dedup_ratio,
            elapsed_us = start.elapsed().as_micros(),
            "Layer deduplication complete"
        );

        Ok(DedupResult {
            total_chunks,
            new_chunks,
            duplicate_chunks: dup_chunks,
            original_size: data.len() as u64,
            stored_size: new_stored_bytes,
            dedup_ratio,
            chunk_hashes,
            processing_time: start.elapsed(),
        })
    }

    /// Process multiple layers in sequence, returning results for each.
    pub fn process_layers(&self, layers: &[(&str, &[u8])]) -> Result<Vec<DedupResult>> {
        layers
            .iter()
            .map(|(id, data)| self.process_layer(id, data))
            .collect()
    }

    /// Get cumulative deduplication statistics.
    #[must_use]
    pub fn stats(&self) -> DedupStats {
        self.stats.read().clone()
    }

    /// Get bloom filter memory usage in bytes.
    #[must_use]
    pub fn bloom_memory_bytes(&self) -> usize {
        self.bloom.read().memory_bytes()
    }

    /// Get bloom filter estimated false positive rate.
    #[must_use]
    pub fn bloom_estimated_fpr(&self) -> f64 {
        self.bloom.read().estimated_fpr()
    }

    /// Get number of unique chunks in the store.
    #[must_use]
    pub fn unique_chunks(&self) -> u64 {
        self.store.unique_chunks()
    }

    /// Retrieve a chunk by its hash.
    pub fn get_chunk(&self, hash: &[u8; 32]) -> Result<Vec<u8>> {
        self.store.get_chunk(hash)
    }

    /// Reset cumulative statistics.
    pub fn reset_stats(&self) {
        *self.stats.write() = DedupStats::default();
    }
}

// ─── Content Merkle Tree ─────────────────────────────────────────────────────

/// A node in the Content-Defined Merkle Tree.
#[derive(Debug, Clone)]
pub struct MerkleNode {
    /// Combined hash of this subtree.
    pub hash: [u8; 32],
    /// Left child (if internal node).
    left: Option<Box<MerkleNode>>,
    /// Right child (if internal node).
    right: Option<Box<MerkleNode>>,
    /// Whether this is a leaf node.
    is_leaf: bool,
}

/// Content-Defined Merkle Tree for O(log n) image layer diffing.
///
/// Builds a binary hash tree over chunk hashes. Subtrees with identical
/// root hashes represent identical content, enabling logarithmic-time
/// diff between two versions of a layer.
#[derive(Debug, Clone)]
pub struct ContentMerkleTree {
    /// Root node of the tree.
    root: Option<MerkleNode>,
    /// Ordered chunk hashes (leaves of the tree).
    leaf_hashes: Vec<[u8; 32]>,
}

/// Result of diffing two Merkle trees.
#[derive(Debug, Clone)]
pub struct MerkleDiff {
    /// Indices of leaves that differ between old and new trees.
    pub changed_leaf_indices: Vec<usize>,
    /// Total leaves in the old tree.
    pub total_leaves_old: usize,
    /// Total leaves in the new tree.
    pub total_leaves_new: usize,
    /// Whether the trees have different structures (different leaf counts).
    pub structural_changes: bool,
}

impl MerkleDiff {
    /// Fraction of leaves that changed (0.0 = identical, 1.0 = completely different).
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn change_ratio(&self) -> f64 {
        let max_leaves = self.total_leaves_old.max(self.total_leaves_new);
        if max_leaves == 0 {
            return 0.0;
        }
        self.changed_leaf_indices.len() as f64 / max_leaves as f64
    }
}

impl ContentMerkleTree {
    /// Build a Merkle tree from an ordered list of chunk hashes.
    #[must_use]
    pub fn build(chunk_hashes: Vec<[u8; 32]>) -> Self {
        let root = if chunk_hashes.is_empty() {
            None
        } else {
            Some(Self::build_subtree(&chunk_hashes))
        };
        Self {
            root,
            leaf_hashes: chunk_hashes,
        }
    }

    /// Recursively build a balanced Merkle subtree.
    fn build_subtree(hashes: &[[u8; 32]]) -> MerkleNode {
        if hashes.len() == 1 {
            return MerkleNode {
                hash: hashes[0],
                left: None,
                right: None,
                is_leaf: true,
            };
        }

        let mid = hashes.len() / 2;
        let left = Self::build_subtree(&hashes[..mid]);
        let right = Self::build_subtree(&hashes[mid..]);

        let combined = Self::combine_hashes(&left.hash, &right.hash);
        MerkleNode {
            hash: combined,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
            is_leaf: false,
        }
    }

    /// Combine two child hashes into a parent hash.
    fn combine_hashes(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut data = [0u8; 64];
        data[..32].copy_from_slice(left);
        data[32..].copy_from_slice(right);
        sha256_hash(&data)
    }

    /// Get the root hash of the tree.
    #[must_use]
    pub fn root_hash(&self) -> Option<[u8; 32]> {
        self.root.as_ref().map(|n| n.hash)
    }

    /// Number of leaf nodes (chunks).
    #[must_use]
    pub fn leaf_count(&self) -> usize {
        self.leaf_hashes.len()
    }

    /// Get the ordered leaf hashes.
    #[must_use]
    pub fn leaf_hashes(&self) -> &[[u8; 32]] {
        &self.leaf_hashes
    }

    /// Compute the diff between this tree and another.
    ///
    /// For equal-sized trees, achieves O(log n + k) where k is the number
    /// of changed chunks. For different-sized trees, falls back to
    /// element-wise comparison.
    #[must_use]
    pub fn diff(&self, other: &ContentMerkleTree) -> MerkleDiff {
        let old_count = self.leaf_hashes.len();
        let new_count = other.leaf_hashes.len();

        // Different number of chunks → structural change
        if old_count != new_count {
            let common = old_count.min(new_count);
            let mut changed: Vec<usize> = (0..common)
                .filter(|&i| self.leaf_hashes[i] != other.leaf_hashes[i])
                .collect();
            // All chunks beyond the common range are "changed"
            let extra_start = common;
            let extra_end = old_count.max(new_count);
            changed.extend(extra_start..extra_end);

            return MerkleDiff {
                changed_leaf_indices: changed,
                total_leaves_old: old_count,
                total_leaves_new: new_count,
                structural_changes: true,
            };
        }

        // Same size → use O(log n) tree diff
        let mut changed = Vec::new();
        if old_count > 0 {
            Self::diff_nodes(&self.root, &other.root, 0, old_count, &mut changed);
        }

        MerkleDiff {
            changed_leaf_indices: changed,
            total_leaves_old: old_count,
            total_leaves_new: new_count,
            structural_changes: false,
        }
    }

    /// Recursively diff two tree nodes, collecting changed leaf indices.
    fn diff_nodes(
        old: &Option<MerkleNode>,
        new: &Option<MerkleNode>,
        leaf_offset: usize,
        leaf_count: usize,
        changed: &mut Vec<usize>,
    ) {
        match (old, new) {
            (None, None) => {}
            (Some(_), None) | (None, Some(_)) => {
                // One side is missing — all leaves changed
                for i in leaf_offset..leaf_offset + leaf_count {
                    changed.push(i);
                }
            }
            (Some(old_node), Some(new_node)) => {
                // If hashes match, subtrees are identical — skip
                if old_node.hash == new_node.hash {
                    return;
                }

                // If either is a leaf, it changed
                if old_node.is_leaf || new_node.is_leaf {
                    changed.push(leaf_offset);
                    return;
                }

                // Recurse into children
                let left_count = leaf_count / 2;
                let right_count = leaf_count - left_count;

                Self::diff_nodes(
                    &old_node.left.as_deref().cloned(),
                    &new_node.left.as_deref().cloned(),
                    leaf_offset,
                    left_count,
                    changed,
                );
                Self::diff_nodes(
                    &old_node.right.as_deref().cloned(),
                    &new_node.right.as_deref().cloned(),
                    leaf_offset + left_count,
                    right_count,
                    changed,
                );
            }
        }
    }
}

// ─── Dedup Manager (High-Level API) ──────────────────────────────────────────

/// High-level deduplication manager for container image layers.
///
/// Wraps `ChunkDeduplicator` with Merkle tree caching for efficient
/// layer-to-layer diffing.
pub struct DedupManager {
    /// Core deduplicator engine.
    deduplicator: ChunkDeduplicator,
    /// Cached Merkle trees keyed by layer ID.
    trees: DashMap<String, ContentMerkleTree>,
}

impl std::fmt::Debug for DedupManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DedupManager")
            .field("deduplicator", &self.deduplicator)
            .field("cached_trees", &self.trees.len())
            .finish()
    }
}

impl DedupManager {
    /// Create a dedup manager with default settings.
    pub fn new(store_path: PathBuf) -> Result<Self> {
        let deduplicator = ChunkDeduplicator::new(ChunkConfig::default(), store_path)?;
        Ok(Self {
            deduplicator,
            trees: DashMap::new(),
        })
    }

    /// Create a dedup manager with custom configuration.
    pub fn with_config(
        config: ChunkConfig,
        store_path: PathBuf,
        compression: CompressionMode,
    ) -> Result<Self> {
        let deduplicator = ChunkDeduplicator::with_compression(config, store_path, compression)?;
        Ok(Self {
            deduplicator,
            trees: DashMap::new(),
        })
    }

    /// Process and deduplicate a container image layer.
    ///
    /// Also builds and caches a Merkle tree for the layer, enabling
    /// O(log n) diff with future versions.
    pub fn process_image_layer(&self, layer_id: &str, data: &[u8]) -> Result<DedupResult> {
        let result = self.deduplicator.process_layer(layer_id, data)?;

        // Build and cache Merkle tree
        let tree = ContentMerkleTree::build(result.chunk_hashes.clone());
        self.trees.insert(layer_id.to_string(), tree);

        Ok(result)
    }

    /// Diff two previously processed layers using their cached Merkle trees.
    ///
    /// Returns `None` if either layer hasn't been processed yet.
    #[must_use]
    pub fn diff_layers(&self, layer_a: &str, layer_b: &str) -> Option<MerkleDiff> {
        let tree_a = self.trees.get(layer_a)?;
        let tree_b = self.trees.get(layer_b)?;
        Some(tree_a.diff(&tree_b))
    }

    /// Get the Merkle tree for a processed layer.
    #[must_use]
    pub fn get_tree(&self, layer_id: &str) -> Option<ContentMerkleTree> {
        self.trees.get(layer_id).map(|t| t.clone())
    }

    /// Get cumulative deduplication statistics.
    #[must_use]
    pub fn stats(&self) -> DedupStats {
        self.deduplicator.stats()
    }

    /// Get bloom filter memory usage.
    #[must_use]
    pub fn bloom_memory_bytes(&self) -> usize {
        self.deduplicator.bloom_memory_bytes()
    }

    /// Number of cached Merkle trees.
    #[must_use]
    pub fn cached_trees(&self) -> usize {
        self.trees.len()
    }

    /// Remove a cached Merkle tree for a layer.
    pub fn evict_tree(&self, layer_id: &str) -> Option<ContentMerkleTree> {
        self.trees.remove(layer_id).map(|(_, t)| t)
    }

    /// Clear all cached Merkle trees.
    pub fn clear_trees(&self) {
        self.trees.clear();
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // ── Bloom Filter Tests ───────────────────────────────────────────────

    #[test]
    fn test_bloom_filter_basic() {
        let mut bloom = BloomFilter::new(1000, 0.01);
        let hash_a = sha256_hash(b"hello world");
        let hash_b = sha256_hash(b"goodbye world");

        assert!(!bloom.possibly_contains(&hash_a));
        assert!(!bloom.possibly_contains(&hash_b));

        bloom.insert(&hash_a);
        assert!(bloom.possibly_contains(&hash_a));
        assert_eq!(bloom.items_count(), 1);

        bloom.insert(&hash_b);
        assert!(bloom.possibly_contains(&hash_a));
        assert!(bloom.possibly_contains(&hash_b));
        assert_eq!(bloom.items_count(), 2);
    }

    #[test]
    fn test_bloom_filter_no_false_negatives() {
        let mut bloom = BloomFilter::new(10000, 0.01);
        let mut hashes = Vec::new();

        for i in 0..1000_u32 {
            let hash = sha256_hash(&i.to_le_bytes());
            bloom.insert(&hash);
            hashes.push(hash);
        }

        // No false negatives — every inserted item must be found
        for hash in &hashes {
            assert!(bloom.possibly_contains(hash), "False negative detected!");
        }
    }

    #[test]
    fn test_bloom_filter_fpr_reasonable() {
        let mut bloom = BloomFilter::new(10000, 0.01);

        // Insert 10000 items
        for i in 0..10000_u32 {
            let hash = sha256_hash(&i.to_le_bytes());
            bloom.insert(&hash);
        }

        // Check 10000 items NOT in the set
        let mut false_positives = 0;
        for i in 10000..20000_u32 {
            let hash = sha256_hash(&i.to_le_bytes());
            if bloom.possibly_contains(&hash) {
                false_positives += 1;
            }
        }

        // FPR should be roughly ≤ 2% (we target 1%, allow some margin)
        let fpr = false_positives as f64 / 10000.0;
        assert!(
            fpr < 0.03,
            "FPR too high: {fpr:.4} (expected < 0.03, got {false_positives}/10000)"
        );
    }

    #[test]
    fn test_bloom_filter_memory_size() {
        let bloom = BloomFilter::new(1_000_000, 0.01);
        let memory_kb = bloom.memory_bytes() / 1024;
        // Should be ~1.2MB (1200 KB) for 1M items at 1% FPR
        assert!(
            memory_kb > 1000 && memory_kb < 1500,
            "Unexpected bloom filter size: {memory_kb} KB"
        );
    }

    #[test]
    fn test_bloom_filter_clear() {
        let mut bloom = BloomFilter::new(100, 0.01);
        let hash = sha256_hash(b"test");
        bloom.insert(&hash);
        assert!(bloom.possibly_contains(&hash));

        bloom.clear();
        assert!(!bloom.possibly_contains(&hash));
        assert_eq!(bloom.items_count(), 0);
    }

    // ── FastCDC Chunker Tests ────────────────────────────────────────────

    #[test]
    fn test_fastcdc_basic_chunking() {
        let chunker = FastCdcChunker::new(ChunkConfig::default()).unwrap();
        let data = vec![0u8; 100_000];
        let chunks = chunker.chunk(&data);

        assert!(!chunks.is_empty(), "Should produce at least one chunk");

        // Verify chunks cover entire input
        let total_len: usize = chunks.iter().map(|c| c.length).sum();
        assert_eq!(total_len, data.len());

        // Verify no overlaps
        let mut offset = 0;
        for chunk in &chunks {
            assert_eq!(chunk.offset, offset);
            offset += chunk.length;
        }
    }

    #[test]
    fn test_fastcdc_chunk_size_bounds() {
        let config = ChunkConfig::default();
        let chunker = FastCdcChunker::new(config.clone()).unwrap();

        // Random-ish data for realistic chunking
        let data: Vec<u8> = (0..500_000_u32)
            .map(|i| {
                let h = i.wrapping_mul(2654435761);
                (h >> 16) as u8
            })
            .collect();

        let chunks = chunker.chunk(&data);
        for (i, chunk) in chunks.iter().enumerate() {
            // All chunks except possibly the last must be >= min_size
            if i < chunks.len() - 1 {
                assert!(
                    chunk.length >= config.min_size,
                    "Chunk {i} too small: {} < {}",
                    chunk.length,
                    config.min_size
                );
            }
            // All chunks must be <= max_size
            assert!(
                chunk.length <= config.max_size,
                "Chunk {i} too large: {} > {}",
                chunk.length,
                config.max_size
            );
        }
    }

    #[test]
    fn test_fastcdc_deterministic() {
        let chunker = FastCdcChunker::new(ChunkConfig::default()).unwrap();
        let data = vec![42u8; 100_000];

        let chunks1 = chunker.chunk(&data);
        let chunks2 = chunker.chunk(&data);

        assert_eq!(chunks1.len(), chunks2.len());
        for (a, b) in chunks1.iter().zip(chunks2.iter()) {
            assert_eq!(a.offset, b.offset);
            assert_eq!(a.length, b.length);
        }
    }

    #[test]
    fn test_fastcdc_empty_data() {
        let chunker = FastCdcChunker::new(ChunkConfig::default()).unwrap();
        let chunks = chunker.chunk(&[]);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_fastcdc_small_data() {
        let chunker = FastCdcChunker::new(ChunkConfig::default()).unwrap();
        let data = vec![0u8; 100]; // Smaller than min_size
        let chunks = chunker.chunk(&data);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].length, 100);
    }

    #[test]
    fn test_fastcdc_content_sensitivity() {
        let chunker = FastCdcChunker::new(ChunkConfig::default()).unwrap();

        // Two slightly different inputs should share some chunk boundaries
        let mut data_a = vec![0u8; 100_000];
        let mut data_b = data_a.clone();

        // Modify a small section in the middle
        for byte in &mut data_b[50_000..50_100] {
            *byte = 0xFF;
        }

        let chunks_a = chunker.chunk(&data_a);
        let chunks_b = chunker.chunk(&data_b);

        // With content-defined chunking, most boundaries should be the same
        let set_a: HashSet<usize> = chunks_a.iter().map(|c| c.offset).collect();
        let set_b: HashSet<usize> = chunks_b.iter().map(|c| c.offset).collect();

        let common = set_a.intersection(&set_b).count();
        let total = set_a.len().max(set_b.len());

        // At least 50% of boundaries should be shared (CDC property)
        assert!(common * 2 >= total, "Too few shared boundaries: {common}/{total}");
    }

    #[test]
    fn test_chunk_config_validation() {
        assert!(ChunkConfig {
            min_size: 0,
            avg_size: 100,
            max_size: 200,
            normalization: 1,
        }
        .validate()
        .is_err());

        assert!(ChunkConfig {
            min_size: 100,
            avg_size: 50, // avg < min
            max_size: 200,
            normalization: 1,
        }
        .validate()
        .is_err());

        assert!(ChunkConfig {
            min_size: 100,
            avg_size: 200,
            max_size: 150, // max < avg
            normalization: 1,
        }
        .validate()
        .is_err());
    }

    // ── Chunk Store Tests ────────────────────────────────────────────────

    #[test]
    fn test_chunk_store_basic() {
        let store = ChunkStore::new(PathBuf::from("/tmp/test-store"), CompressionMode::None);
        let data = b"hello world";
        let hash = sha256_hash(data);

        // Store new chunk
        let result = store.store_chunk(hash, data).unwrap();
        assert!(result.is_some());
        assert!(store.contains(&hash));
        assert_eq!(store.unique_chunks(), 1);

        // Retrieve chunk
        let retrieved = store.get_chunk(&hash).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_chunk_store_duplicate() {
        let store = ChunkStore::new(PathBuf::from("/tmp/test-store"), CompressionMode::None);
        let data = b"duplicate data";
        let hash = sha256_hash(data);

        let result1 = store.store_chunk(hash, data).unwrap();
        assert!(result1.is_some());

        let result2 = store.store_chunk(hash, data).unwrap();
        assert!(result2.is_none()); // duplicate
        assert_eq!(store.unique_chunks(), 1); // still 1
    }

    #[test]
    fn test_chunk_store_zstd_compression() {
        let store =
            ChunkStore::new(PathBuf::from("/tmp/test-store"), CompressionMode::Zstd { level: 3 });
        let data = b"compressible data that repeats repeats repeats repeats repeats";
        let hash = sha256_hash(data);

        let result = store.store_chunk(hash, data).unwrap();
        assert!(result.is_some());

        // Compressed size should be less than original for compressible data
        let compressed_size = result.unwrap();
        assert!(compressed_size <= data.len());

        // Retrieve and verify
        let retrieved = store.get_chunk(&hash).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_chunk_store_lz4_compression() {
        let store = ChunkStore::new(PathBuf::from("/tmp/test-store"), CompressionMode::Lz4);
        let data = b"more compressible data repeating repeating repeating repeating repeating";
        let hash = sha256_hash(data);

        store.store_chunk(hash, data).unwrap();
        let retrieved = store.get_chunk(&hash).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_chunk_store_get_missing() {
        let store = ChunkStore::new(PathBuf::from("/tmp/test-store"), CompressionMode::None);
        let hash = sha256_hash(b"nonexistent");
        assert!(store.get_chunk(&hash).is_err());
    }

    // ── Chunk Deduplicator Tests ─────────────────────────────────────────

    #[test]
    fn test_deduplicator_process_layer() {
        let dedup =
            ChunkDeduplicator::new(ChunkConfig::default(), PathBuf::from("/tmp/test-dedup"))
                .unwrap();

        let data = vec![0u8; 100_000];
        let result = dedup.process_layer("layer-1", &data).unwrap();

        assert!(result.total_chunks > 0);
        assert_eq!(result.new_chunks + result.duplicate_chunks, result.total_chunks);
        assert_eq!(result.chunk_hashes.len(), result.total_chunks);
    }

    #[test]
    fn test_deduplicator_detects_duplicates() {
        let dedup =
            ChunkDeduplicator::new(ChunkConfig::default(), PathBuf::from("/tmp/test-dedup"))
                .unwrap();

        let data = vec![0u8; 100_000];

        // First pass: all chunks are new
        let result1 = dedup.process_layer("layer-1", &data).unwrap();
        assert_eq!(result1.duplicate_chunks, 0);
        assert!(result1.new_chunks > 0);

        // Second pass with same data: all chunks should be duplicates
        let result2 = dedup.process_layer("layer-2", &data).unwrap();
        assert_eq!(result2.new_chunks, 0);
        assert_eq!(result2.duplicate_chunks, result2.total_chunks);
        assert!((result2.dedup_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_deduplicator_partial_overlap() {
        let dedup =
            ChunkDeduplicator::new(ChunkConfig::default(), PathBuf::from("/tmp/test-dedup"))
                .unwrap();

        let data_a = vec![0u8; 100_000];
        let _ = dedup.process_layer("layer-a", &data_a).unwrap();

        // Create data with partial overlap (first half same, second half different)
        let mut data_b = vec![0u8; 100_000];
        for byte in &mut data_b[50_000..] {
            *byte = 0xFF;
        }

        let result = dedup.process_layer("layer-b", &data_b).unwrap();
        // Should have both new and duplicate chunks
        assert!(result.new_chunks > 0, "Should have some new chunks");
        assert!(result.duplicate_chunks > 0, "Should have some duplicates");
    }

    #[test]
    fn test_deduplicator_stats() {
        let dedup =
            ChunkDeduplicator::new(ChunkConfig::default(), PathBuf::from("/tmp/test-dedup"))
                .unwrap();

        let data = vec![42u8; 50_000];
        dedup.process_layer("layer-1", &data).unwrap();
        dedup.process_layer("layer-2", &data).unwrap();

        let stats = dedup.stats();
        assert_eq!(stats.layers_processed, 2);
        assert!(stats.total_bytes_processed > 0);
        assert!(stats.unique_chunks > 0);
        assert!(stats.duplicate_chunks > 0);
        assert!(stats.average_chunk_size > 0.0);
    }

    #[test]
    fn test_deduplicator_bloom_memory() {
        let dedup =
            ChunkDeduplicator::new(ChunkConfig::default(), PathBuf::from("/tmp/test-dedup"))
                .unwrap();

        let memory = dedup.bloom_memory_bytes();
        // Default bloom: 1M items at 1% FPR ≈ 1.2 MB
        assert!(memory > 1_000_000, "Bloom filter too small: {memory}");
        assert!(memory < 2_000_000, "Bloom filter too large: {memory}");
    }

    // ── Content Merkle Tree Tests ────────────────────────────────────────

    #[test]
    fn test_merkle_tree_build() {
        let hashes: Vec<[u8; 32]> = (0..8_u32).map(|i| sha256_hash(&i.to_le_bytes())).collect();

        let tree = ContentMerkleTree::build(hashes.clone());

        assert!(tree.root_hash().is_some());
        assert_eq!(tree.leaf_count(), 8);
        assert_eq!(tree.leaf_hashes().len(), 8);
    }

    #[test]
    fn test_merkle_tree_root_deterministic() {
        let hashes: Vec<[u8; 32]> = (0..5_u32).map(|i| sha256_hash(&i.to_le_bytes())).collect();

        let tree1 = ContentMerkleTree::build(hashes.clone());
        let tree2 = ContentMerkleTree::build(hashes);

        assert_eq!(tree1.root_hash(), tree2.root_hash());
    }

    #[test]
    fn test_merkle_tree_empty() {
        let tree = ContentMerkleTree::build(Vec::new());
        assert!(tree.root_hash().is_none());
        assert_eq!(tree.leaf_count(), 0);
    }

    #[test]
    fn test_merkle_tree_single_leaf() {
        let hash = sha256_hash(b"single");
        let tree = ContentMerkleTree::build(vec![hash]);
        assert_eq!(tree.root_hash(), Some(hash));
        assert_eq!(tree.leaf_count(), 1);
    }

    #[test]
    fn test_merkle_diff_identical() {
        let hashes: Vec<[u8; 32]> = (0..8_u32).map(|i| sha256_hash(&i.to_le_bytes())).collect();

        let tree1 = ContentMerkleTree::build(hashes.clone());
        let tree2 = ContentMerkleTree::build(hashes);

        let diff = tree1.diff(&tree2);
        assert!(diff.changed_leaf_indices.is_empty());
        assert!(!diff.structural_changes);
        assert!((diff.change_ratio() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_merkle_diff_single_change() {
        let hashes_a: Vec<[u8; 32]> = (0..8_u32).map(|i| sha256_hash(&i.to_le_bytes())).collect();

        let mut hashes_b = hashes_a.clone();
        hashes_b[3] = sha256_hash(b"changed");

        let tree_a = ContentMerkleTree::build(hashes_a);
        let tree_b = ContentMerkleTree::build(hashes_b);

        let diff = tree_a.diff(&tree_b);
        assert_eq!(diff.changed_leaf_indices, vec![3]);
        assert!(!diff.structural_changes);
    }

    #[test]
    fn test_merkle_diff_all_different() {
        let hashes_a: Vec<[u8; 32]> = (0..4_u32).map(|i| sha256_hash(&i.to_le_bytes())).collect();

        let hashes_b: Vec<[u8; 32]> = (100..104_u32)
            .map(|i| sha256_hash(&i.to_le_bytes()))
            .collect();

        let tree_a = ContentMerkleTree::build(hashes_a);
        let tree_b = ContentMerkleTree::build(hashes_b);

        let diff = tree_a.diff(&tree_b);
        assert_eq!(diff.changed_leaf_indices.len(), 4);
        assert!((diff.change_ratio() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_merkle_diff_structural_change() {
        let hashes_a: Vec<[u8; 32]> = (0..4_u32).map(|i| sha256_hash(&i.to_le_bytes())).collect();

        let hashes_b: Vec<[u8; 32]> = (0..6_u32) // different count
            .map(|i| sha256_hash(&i.to_le_bytes()))
            .collect();

        let tree_a = ContentMerkleTree::build(hashes_a);
        let tree_b = ContentMerkleTree::build(hashes_b);

        let diff = tree_a.diff(&tree_b);
        assert!(diff.structural_changes);
        // Extra leaves (indices 4, 5) should be in the diff
        assert!(diff.changed_leaf_indices.contains(&4));
        assert!(diff.changed_leaf_indices.contains(&5));
    }

    // ── Dedup Manager Tests ──────────────────────────────────────────────

    #[test]
    fn test_dedup_manager_basic() {
        let mgr = DedupManager::new(PathBuf::from("/tmp/test-mgr")).unwrap();

        let data = vec![0u8; 50_000];
        let result = mgr.process_image_layer("layer-1", &data).unwrap();
        assert!(result.total_chunks > 0);

        // Tree should be cached
        assert_eq!(mgr.cached_trees(), 1);
        assert!(mgr.get_tree("layer-1").is_some());
    }

    #[test]
    fn test_dedup_manager_diff() {
        let mgr = DedupManager::new(PathBuf::from("/tmp/test-mgr")).unwrap();

        let data_a = vec![0u8; 50_000];
        mgr.process_image_layer("layer-a", &data_a).unwrap();

        // Same data → same chunks → zero diff
        mgr.process_image_layer("layer-b", &data_a).unwrap();

        let diff = mgr.diff_layers("layer-a", "layer-b");
        assert!(diff.is_some());
        let diff = diff.unwrap();
        assert!(diff.changed_leaf_indices.is_empty(), "Identical data should produce no diff");
    }

    #[test]
    fn test_dedup_manager_evict() {
        let mgr = DedupManager::new(PathBuf::from("/tmp/test-mgr")).unwrap();

        let data = vec![0u8; 50_000];
        mgr.process_image_layer("layer-1", &data).unwrap();
        assert_eq!(mgr.cached_trees(), 1);

        mgr.evict_tree("layer-1");
        assert_eq!(mgr.cached_trees(), 0);
    }

    // ── Gear Table Tests ─────────────────────────────────────────────────

    #[test]
    fn test_gear_table_unique() {
        let mut seen = HashSet::new();
        for value in &GEAR_TABLE {
            assert!(seen.insert(*value), "Gear table has duplicate value: {value}");
        }
    }

    #[test]
    fn test_gear_table_nonzero() {
        // Index 0 maps to 0 via splitmix, but all others should be nonzero.
        // Verify the table has reasonable distribution.
        let nonzero = GEAR_TABLE.iter().filter(|&&v| v != 0).count();
        assert!(nonzero >= 254, "Too many zero entries in gear table: {}/256 nonzero", nonzero);
    }

    // ── Helper Tests ─────────────────────────────────────────────────────

    #[test]
    fn test_sha256_hash_deterministic() {
        let hash1 = sha256_hash(b"test data");
        let hash2 = sha256_hash(b"test data");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_sha256_hash_different_inputs() {
        let hash1 = sha256_hash(b"data a");
        let hash2 = sha256_hash(b"data b");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_hex_format() {
        let hash = sha256_hash(b"");
        let hex = hash_hex(&hash);
        assert_eq!(hex.len(), 64); // 32 bytes × 2 hex chars
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ── Compression Round-Trip Tests ─────────────────────────────────────

    #[test]
    fn test_compression_roundtrip_none() {
        let store = ChunkStore::new(PathBuf::from("/tmp/test"), CompressionMode::None);
        let data = b"test data for compression";
        let compressed = store.compress_data(data).unwrap();
        let decompressed = store
            .decompress_data(&compressed, &CompressionMode::None)
            .unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compression_roundtrip_zstd() {
        let store = ChunkStore::new(PathBuf::from("/tmp/test"), CompressionMode::Zstd { level: 3 });
        let data = b"test data for zstd compression that repeats repeats repeats";
        let compressed = store.compress_data(data).unwrap();
        let decompressed = store
            .decompress_data(&compressed, &CompressionMode::Zstd { level: 3 })
            .unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compression_roundtrip_lz4() {
        let store = ChunkStore::new(PathBuf::from("/tmp/test"), CompressionMode::Lz4);
        let data = b"test data for lz4 compression that repeats repeats repeats";
        let compressed = store.compress_data(data).unwrap();
        let decompressed = store
            .decompress_data(&compressed, &CompressionMode::Lz4)
            .unwrap();
        assert_eq!(decompressed, data);
    }
}
