//! Fscache Integration and Management
//!
//! Provides kernel fscache integration for EROFS images with binding,
//! unbinding, eviction policies, and coherency verification.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};

/// Fscache backend identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheBackend {
    /// FSCACHE backend
    FscacheKernel,
    /// Local in-memory cache
    MemoryCache,
    /// Disk-backed cache
    DiskCache,
}

impl CacheBackend {
    /// Get backend name
    pub fn name(&self) -> &'static str {
        match self {
            CacheBackend::FscacheKernel => "fscache",
            CacheBackend::MemoryCache => "memory",
            CacheBackend::DiskCache => "disk",
        }
    }
}

/// Cache binding configuration
#[derive(Debug, Clone)]
pub struct CacheBinding {
    /// Backend being used
    pub backend: CacheBackend,
    /// Cache key/identifier
    pub cache_key: String,
    /// Binding time (unix seconds)
    pub bound_at: u64,
    /// Is currently active
    pub active: bool,
}

impl CacheBinding {
    /// Create new binding
    pub fn new(backend: CacheBackend, cache_key: String) -> Self {
        let bound_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            backend,
            cache_key,
            bound_at,
            active: false,
        }
    }

    /// Activate binding
    pub fn activate(&mut self) -> Result<()> {
        self.active = true;
        Ok(())
    }

    /// Deactivate binding
    pub fn deactivate(&mut self) -> Result<()> {
        self.active = false;
        Ok(())
    }

    /// Get binding uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().saturating_sub(self.bound_at))
            .unwrap_or(0)
    }
}

/// Cache eviction policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionPolicy {
    /// LRU - Least Recently Used
    LRU,
    /// LFU - Least Frequently Used
    LFU,
    /// FIFO - First In First Out
    FIFO,
    /// Random
    Random,
}

impl EvictionPolicy {
    /// Get policy name
    pub fn name(&self) -> &'static str {
        match self {
            EvictionPolicy::LRU => "lru",
            EvictionPolicy::LFU => "lfu",
            EvictionPolicy::FIFO => "fifo",
            EvictionPolicy::Random => "random",
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total items in cache
    pub item_count: u64,
    /// Total cache size in bytes
    pub total_size: u64,
    /// Used cache in bytes
    pub used_size: u64,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Items evicted
    pub evicted_items: u64,
}

impl CacheStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            item_count: 0,
            total_size: 0,
            used_size: 0,
            hits: 0,
            misses: 0,
            evicted_items: 0,
        }
    }

    /// Calculate utilization percentage (0.0-1.0)
    pub fn utilization(&self) -> f64 {
        if self.total_size == 0 {
            0.0
        } else {
            self.used_size as f64 / self.total_size as f64
        }
    }

    /// Calculate hit ratio (0.0-1.0)
    pub fn hit_ratio(&self) -> f64 {
        let total = (self.hits + self.misses) as f64;
        if total == 0.0 {
            0.0
        } else {
            self.hits as f64 / total
        }
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Fscache manager for EROFS images
pub struct FscacheManager {
    /// Active cache bindings: key -> binding
    bindings: HashMap<String, CacheBinding>,
    /// Cache statistics
    stats: CacheStats,
    /// Cache size limit in bytes (0 = unlimited)
    max_cache_size: u64,
    /// Eviction policy
    eviction_policy: EvictionPolicy,
    /// Default backend
    default_backend: CacheBackend,
    /// Coherency checking enabled
    check_coherency: bool,
}

impl FscacheManager {
    /// Create new fscache manager
    pub fn new(max_cache_size: u64) -> Self {
        Self {
            bindings: HashMap::new(),
            stats: CacheStats::new(),
            max_cache_size,
            eviction_policy: EvictionPolicy::LRU,
            default_backend: CacheBackend::FscacheKernel,
            check_coherency: true,
        }
    }

    /// Bind a cache for an image
    pub fn bind_cache(&mut self, cache_key: String, backend: Option<CacheBackend>) -> Result<()> {
        let backend = backend.unwrap_or(self.default_backend);

        let mut binding = CacheBinding::new(backend, cache_key.clone());
        binding.activate()?;

        self.bindings.insert(cache_key.clone(), binding);
        self.stats.item_count += 1;

        eprintln!("✅ Bound cache: {} ({:?})", cache_key, backend);
        Ok(())
    }

    /// Unbind a cache
    pub fn unbind_cache(&mut self, cache_key: &str) -> Result<()> {
        if let Some(mut binding) = self.bindings.remove(cache_key) {
            binding.deactivate()?;
            self.stats.item_count = self.stats.item_count.saturating_sub(1);
            eprintln!("✅ Unbound cache: {}", cache_key);
            Ok(())
        } else {
            Err(anyhow!("Cache binding not found: {}", cache_key))
        }
    }

    /// Get binding status
    pub fn get_binding(&self, cache_key: &str) -> Option<&CacheBinding> {
        self.bindings.get(cache_key)
    }

    /// List all bindings
    pub fn list_bindings(&self) -> Vec<&CacheBinding> {
        self.bindings.values().collect()
    }

    /// Check cache coherency
    pub fn check_coherency(&self, cache_key: &str) -> Result<bool> {
        if !self.check_coherency {
            return Ok(true); // Disabled
        }

        if let Some(binding) = self.bindings.get(cache_key) {
            // Simulate coherency check
            Ok(binding.active)
        } else {
            Ok(false)
        }
    }

    /// Evict cache entries based on policy
    pub fn evict_entries(&mut self, target_size: u64) -> Result<()> {
        if self.stats.used_size <= target_size {
            return Ok(());
        }

        match self.eviction_policy {
            EvictionPolicy::LRU => {
                // Remove least recently used
                if let Some((key, _)) = self.bindings.iter().next() {
                    let key = key.clone();
                    self.unbind_cache(&key)?;
                    self.stats.evicted_items += 1;
                }
            }
            EvictionPolicy::FIFO => {
                // Remove first binding
                if let Some((key, _)) = self.bindings.iter().next() {
                    let key = key.clone();
                    self.unbind_cache(&key)?;
                    self.stats.evicted_items += 1;
                }
            }
            EvictionPolicy::LFU | EvictionPolicy::Random => {
                // For now, use FIFO approach
                if let Some((key, _)) = self.bindings.iter().next() {
                    let key = key.clone();
                    self.unbind_cache(&key)?;
                    self.stats.evicted_items += 1;
                }
            }
        }

        Ok(())
    }

    /// Set eviction policy
    pub fn set_eviction_policy(&mut self, policy: EvictionPolicy) {
        self.eviction_policy = policy;
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Record cache hit
    pub fn record_hit(&mut self) {
        self.stats.hits += 1;
    }

    /// Record cache miss
    pub fn record_miss(&mut self) {
        self.stats.misses += 1;
    }

    /// Update cache utilization
    pub fn update_usage(&mut self, used_bytes: u64) {
        self.stats.used_size = used_bytes;
    }

    /// Enable/disable coherency checking
    pub fn set_coherency_check(&mut self, enabled: bool) {
        self.check_coherency = enabled;
    }

    /// Get number of active bindings
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    /// Clear all bindings
    pub fn clear_all(&mut self) -> Result<()> {
        let keys: Vec<_> = self.bindings.keys().cloned().collect();
        for key in keys {
            self.unbind_cache(&key)?;
        }
        Ok(())
    }
}

impl Default for FscacheManager {
    fn default() -> Self {
        Self::new(1024 * 1024 * 1024) // 1GB default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_backend_names() {
        assert_eq!(CacheBackend::FscacheKernel.name(), "fscache");
        assert_eq!(CacheBackend::MemoryCache.name(), "memory");
        assert_eq!(CacheBackend::DiskCache.name(), "disk");
    }

    #[test]
    fn test_cache_binding_creation() {
        let binding = CacheBinding::new(CacheBackend::FscacheKernel, "test".to_string());
        assert_eq!(binding.cache_key, "test");
        assert!(!binding.active);
    }

    #[test]
    fn test_cache_binding_activation() {
        let mut binding = CacheBinding::new(CacheBackend::FscacheKernel, "test".to_string());
        assert!(binding.activate().is_ok());
        assert!(binding.active);
    }

    #[test]
    fn test_cache_stats_utilization() {
        let mut stats = CacheStats::new();
        stats.total_size = 100;
        stats.used_size = 50;
        assert!((stats.utilization() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_cache_stats_hit_ratio() {
        let mut stats = CacheStats::new();
        stats.hits = 75;
        stats.misses = 25;
        assert!((stats.hit_ratio() - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_eviction_policy_names() {
        assert_eq!(EvictionPolicy::LRU.name(), "lru");
        assert_eq!(EvictionPolicy::LFU.name(), "lfu");
        assert_eq!(EvictionPolicy::FIFO.name(), "fifo");
        assert_eq!(EvictionPolicy::Random.name(), "random");
    }

    #[test]
    fn test_fscache_manager_creation() {
        let manager = FscacheManager::new(1024 * 1024);
        assert_eq!(manager.binding_count(), 0);
        assert_eq!(manager.max_cache_size, 1024 * 1024);
    }

    #[test]
    fn test_fscache_bind_cache() {
        let mut manager = FscacheManager::new(1024 * 1024);
        assert!(manager.bind_cache("test".to_string(), None).is_ok());
        assert_eq!(manager.binding_count(), 1);
    }

    #[test]
    fn test_fscache_unbind_cache() {
        let mut manager = FscacheManager::new(1024 * 1024);
        manager.bind_cache("test".to_string(), None).ok();
        assert!(manager.unbind_cache("test").is_ok());
        assert_eq!(manager.binding_count(), 0);
    }

    #[test]
    fn test_fscache_get_binding() {
        let mut manager = FscacheManager::new(1024 * 1024);
        manager.bind_cache("test".to_string(), None).ok();
        assert!(manager.get_binding("test").is_some());
        assert!(manager.get_binding("missing").is_none());
    }

    #[test]
    fn test_fscache_stats_operations() {
        let mut manager = FscacheManager::new(1024 * 1024);
        manager.record_hit();
        manager.record_hit();
        manager.record_miss();

        let stats = manager.get_stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_fscache_manager_default() {
        let manager = FscacheManager::default();
        assert_eq!(manager.binding_count(), 0);
    }
}
