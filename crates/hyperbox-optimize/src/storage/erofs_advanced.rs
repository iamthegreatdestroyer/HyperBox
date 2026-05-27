//! Advanced EROFS Features - Performance Measurement & Statistics
//!
//! Provides performance benchmarking and detailed statistics for EROFS storage backend.

use std::time::{SystemTime, UNIX_EPOCH};

/// Performance metrics for image operations
#[derive(Debug, Clone)]
pub struct ErofsPerformanceMetrics {
    /// Image size in bytes
    pub image_size: u64,
    /// Mount/unmount time in milliseconds
    pub operation_time_ms: u64,
    /// Throughput in bytes per second
    pub throughput_bps: u64,
    /// Number of cache hits
    pub cache_hits: u64,
    /// Number of cache misses
    pub cache_misses: u64,
    /// Timestamp
    pub timestamp: u64,
}

impl ErofsPerformanceMetrics {
    /// Create new metrics
    pub fn new(image_size: u64, operation_time_ms: u64) -> Self {
        let throughput_bps = if operation_time_ms > 0 {
            (image_size * 1000) / operation_time_ms
        } else {
            0
        };

        Self {
            image_size,
            operation_time_ms,
            throughput_bps,
            cache_hits: 0,
            cache_misses: 0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    /// Calculate cache hit ratio (0.0-1.0)
    pub fn cache_hit_ratio(&self) -> f64 {
        let total = (self.cache_hits + self.cache_misses) as f64;
        if total == 0.0 {
            0.0
        } else {
            self.cache_hits as f64 / total
        }
    }

    /// Check if performance meets target (30-50% improvement over composefs)
    /// Assumes composefs baseline of ~7 MB/s
    pub fn meets_target(&self) -> bool {
        let composefs_baseline_bps = 7 * 1024 * 1024; // 7 MB/s
        let improvement_factor = self.throughput_bps as f64 / composefs_baseline_bps as f64;
        improvement_factor >= 1.3 // At least 30% improvement
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStatistics {
    /// Total cache size in bytes
    pub total_size: u64,
    /// Used cache in bytes
    pub used_size: u64,
    /// Number of cached items
    pub item_count: usize,
    /// Average item size in bytes
    pub avg_item_size: u64,
}

impl CacheStatistics {
    /// Create new statistics
    pub fn new(total_size: u64, used_size: u64, item_count: usize) -> Self {
        let avg_item_size = if item_count > 0 {
            used_size / item_count as u64
        } else {
            0
        };

        Self {
            total_size,
            used_size,
            item_count,
            avg_item_size,
        }
    }

    /// Get cache utilization percentage (0-100)
    pub fn utilization_percent(&self) -> f64 {
        if self.total_size == 0 {
            0.0
        } else {
            (self.used_size as f64 / self.total_size as f64) * 100.0
        }
    }

    /// Check if cache is full
    pub fn is_full(&self) -> bool {
        self.utilization_percent() >= 95.0
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.item_count == 0
    }
}

/// Benchmark result comparing EROFS vs composefs
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// EROFS throughput in bytes/sec
    pub erofs_throughput_bps: u64,
    /// Composefs baseline (assumed 7 MB/s)
    pub composefs_throughput_bps: u64,
    /// Improvement percentage (0-100+)
    pub improvement_percent: f64,
    /// EROFS meets performance target (30-50%+)
    pub meets_target: bool,
}

impl BenchmarkResult {
    /// Create from EROFS metrics
    pub fn new(erofs_throughput_bps: u64) -> Self {
        let composefs_throughput_bps = 7 * 1024 * 1024; // 7 MB/s baseline

        let improvement_percent = if composefs_throughput_bps > 0 {
            ((erofs_throughput_bps as f64 - composefs_throughput_bps as f64)
                / composefs_throughput_bps as f64)
                * 100.0
        } else {
            0.0
        };

        let meets_target = improvement_percent >= 30.0;

        Self {
            erofs_throughput_bps,
            composefs_throughput_bps,
            improvement_percent,
            meets_target,
        }
    }

    /// Get performance rating (Poor, Fair, Good, Excellent)
    pub fn rating(&self) -> &'static str {
        match self.improvement_percent as i32 {
            ..=10 => "Poor",
            11..=20 => "Fair",
            21..=40 => "Good",
            _ => "Excellent",
        }
    }
}

/// Fallback strategy for unsupported EROFS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackStrategy {
    /// Use composefs as fallback
    Composefs,
    /// Use overlay filesystem
    Overlay,
    /// Use native ext4
    Ext4,
}

impl FallbackStrategy {
    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            FallbackStrategy::Composefs => "Composefs (compressed)",
            FallbackStrategy::Overlay => "Overlay (copy-on-write)",
            FallbackStrategy::Ext4 => "Ext4 (native)",
        }
    }

    /// Get performance multiplier vs EROFS
    pub fn performance_multiplier(&self) -> f64 {
        match self {
            FallbackStrategy::Composefs => 0.7, // 70% of EROFS speed
            FallbackStrategy::Overlay => 0.5,   // 50% of EROFS speed
            FallbackStrategy::Ext4 => 1.0,      // Same as EROFS (no compression)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_erofs_performance_metrics() {
        let metrics = ErofsPerformanceMetrics::new(104857600, 100); // 100MB in 100ms
        assert_eq!(metrics.image_size, 104857600);
        assert_eq!(metrics.operation_time_ms, 100);
        assert_eq!(metrics.throughput_bps, 1048576000); // 1GB/s
    }

    #[test]
    fn test_cache_hit_ratio() {
        let mut metrics = ErofsPerformanceMetrics::new(52428800, 50);
        metrics.cache_hits = 80;
        metrics.cache_misses = 20;

        let ratio = metrics.cache_hit_ratio();
        assert!((ratio - 0.8).abs() < 0.01); // 80%
    }

    #[test]
    fn test_cache_statistics() {
        let stats = CacheStatistics::new(10 * 1024 * 1024 * 1024, 6 * 1024 * 1024 * 1024, 1000);
        assert_eq!(stats.utilization_percent() as i32, 60);
        assert!(!stats.is_full());
        assert!(!stats.is_empty());
    }

    #[test]
    fn test_benchmark_result_improvement() {
        let result = BenchmarkResult::new(10 * 1024 * 1024); // 10 MB/s EROFS
        assert!((result.improvement_percent - 42.85).abs() < 1.0); // ~43% improvement
        assert!(result.meets_target);
        assert_eq!(result.rating(), "Excellent");
    }

    #[test]
    fn test_fallback_strategy_descriptions() {
        assert_eq!(
            FallbackStrategy::Composefs.description(),
            "Composefs (compressed)"
        );
        assert_eq!(FallbackStrategy::Overlay.description(), "Overlay (copy-on-write)");
        assert_eq!(FallbackStrategy::Ext4.description(), "Ext4 (native)");
    }

    #[test]
    fn test_fallback_performance_multipliers() {
        assert_eq!(FallbackStrategy::Composefs.performance_multiplier(), 0.7);
        assert_eq!(FallbackStrategy::Overlay.performance_multiplier(), 0.5);
        assert_eq!(FallbackStrategy::Ext4.performance_multiplier(), 1.0);
    }

    #[test]
    fn test_cache_full_detection() {
        // 9728 MB / 10240 MB = 95.0% — exactly at threshold
        let stats_full = CacheStatistics::new(10 * 1024 * 1024 * 1024, 9728 * 1024 * 1024, 1000);
        assert!(stats_full.is_full()); // 95% used

        let stats_empty = CacheStatistics::new(10 * 1024 * 1024 * 1024, 0, 0);
        assert!(stats_empty.is_empty());
    }

    #[test]
    fn test_meets_target() {
        let good_metrics = ErofsPerformanceMetrics::new(104857600, 70); // Fast: ~1.4 GB/s
        assert!(good_metrics.meets_target()); // >1.3x composefs

        // 100 MB in 20000 ms = 5 MB/s, below the 9.1 MB/s (1.3x) threshold
        let poor_metrics = ErofsPerformanceMetrics::new(104857600, 20000);
        assert!(!poor_metrics.meets_target());
    }
}
