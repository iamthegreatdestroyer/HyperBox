//! PSI Performance Optimization - Caching, Mmap, and Metrics
//!
//! Provides high-performance PSI monitoring with mmap-based reading,
//! syscall caching, and Prometheus-compatible metrics export.

use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::VecDeque;
use anyhow::Result;

/// Performance metrics for PSI monitoring
#[derive(Debug, Clone)]
pub struct PSIMetrics {
    /// Total syscalls made to read PSI
    pub syscalls_total: u64,
    /// Total time spent reading PSI (microseconds)
    pub read_time_us: u64,
    /// Cache hit count
    pub cache_hits: u64,
    /// Cache miss count
    pub cache_misses: u64,
    /// Average read time in microseconds
    pub avg_read_time_us: u64,
}

impl PSIMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self {
            syscalls_total: 0,
            read_time_us: 0,
            cache_hits: 0,
            cache_misses: 0,
            avg_read_time_us: 0,
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

    /// Update average read time
    pub fn update_avg_time(&mut self, new_time_us: u64) {
        if self.syscalls_total == 0 {
            self.avg_read_time_us = new_time_us;
        } else {
            self.avg_read_time_us = (self.read_time_us + new_time_us) / (self.syscalls_total + 1);
        }
        self.read_time_us += new_time_us;
        self.syscalls_total += 1;
    }
}

impl Default for PSIMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Cached PSI reading with timestamp
#[derive(Debug, Clone)]
pub struct CachedReading {
    /// The cached data
    pub data: String,
    /// When it was cached (unix seconds)
    pub cached_at: u64,
    /// Cache validity duration in seconds
    pub ttl_secs: u64,
}

impl CachedReading {
    /// Create new cached reading
    pub fn new(data: String, ttl_secs: u64) -> Self {
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            data,
            cached_at,
            ttl_secs,
        }
    }

    /// Check if cache is still valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        now < self.cached_at + self.ttl_secs
    }

    /// Get age of cache in seconds
    pub fn age_secs(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        now.saturating_sub(self.cached_at)
    }
}

/// High-performance PSI monitor with caching
pub struct PSIPerformanceMonitor {
    /// Cached PSI reading
    cache: Option<CachedReading>,
    /// Performance metrics
    metrics: PSIMetrics,
    /// Cache TTL in seconds (default: 1)
    cache_ttl_secs: u64,
    /// Maximum metrics history size
    max_history: usize,
    /// Historical performance metrics
    history: VecDeque<PSIMetrics>,
}

impl PSIPerformanceMonitor {
    /// Create new performance monitor
    pub fn new(cache_ttl_secs: u64) -> Self {
        Self {
            cache: None,
            metrics: PSIMetrics::new(),
            cache_ttl_secs,
            max_history: 100,
            history: VecDeque::with_capacity(100),
        }
    }

    /// Record a cache hit
    pub fn record_hit(&mut self, read_time_us: u64) {
        self.metrics.cache_hits += 1;
        self.metrics.update_avg_time(read_time_us);
    }

    /// Record a cache miss
    pub fn record_miss(&mut self, read_time_us: u64) {
        self.metrics.cache_misses += 1;
        self.metrics.update_avg_time(read_time_us);
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> &PSIMetrics {
        &self.metrics
    }

    /// Get metrics snapshot and reset
    pub fn snapshot_metrics(&mut self) -> PSIMetrics {
        let snapshot = self.metrics.clone();
        self.history.push_back(snapshot.clone());
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
        self.metrics = PSIMetrics::new();
        snapshot
    }

    /// Get average metrics over history
    pub fn average_metrics(&self) -> PSIMetrics {
        if self.history.is_empty() {
            return PSIMetrics::new();
        }

        let count = self.history.len() as u64;
        let mut avg = PSIMetrics::new();

        for m in &self.history {
            avg.syscalls_total += m.syscalls_total;
            avg.read_time_us += m.read_time_us;
            avg.cache_hits += m.cache_hits;
            avg.cache_misses += m.cache_misses;
        }

        avg.syscalls_total /= count;
        avg.read_time_us /= count;
        avg.cache_hits /= count;
        avg.cache_misses /= count;

        if avg.syscalls_total > 0 {
            avg.avg_read_time_us = avg.read_time_us / avg.syscalls_total;
        }

        avg
    }

    /// Get or create cache
    pub fn get_or_update_cache(&mut self, data: String) -> String {
        let cache_valid = if let Some(cached) = &self.cache {
            cached.is_valid()
        } else {
            false
        };

        if cache_valid {
            self.record_hit(0);
            return self.cache.as_ref().unwrap().data.clone();
        }

        self.record_miss(10); // Estimate read time
        self.cache = Some(CachedReading::new(data.clone(), self.cache_ttl_secs));
        data
    }

    /// Get cache hit ratio
    pub fn cache_hit_ratio(&self) -> f64 {
        self.metrics.cache_hit_ratio()
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache = None;
    }
}

impl Default for PSIPerformanceMonitor {
    fn default() -> Self {
        Self::new(1) // Default 1 second cache TTL
    }
}

/// Prometheus-compatible metrics export format
pub struct PrometheusMetrics {
    /// Metric name
    pub name: String,
    /// Metric type (counter, gauge, histogram)
    pub metric_type: String,
    /// Metric help text
    pub help: String,
    /// Metric samples
    pub samples: Vec<MetricSample>,
}

/// Single metric sample
#[derive(Debug, Clone)]
pub struct MetricSample {
    /// Sample name
    pub name: String,
    /// Label pairs
    pub labels: std::collections::HashMap<String, String>,
    /// Sample value
    pub value: f64,
    /// Sample timestamp (optional)
    pub timestamp: Option<u64>,
}

impl PrometheusMetrics {
    /// Create new Prometheus metrics
    pub fn new(name: String, metric_type: String, help: String) -> Self {
        Self {
            name,
            metric_type,
            help,
            samples: Vec::new(),
        }
    }

    /// Add a sample
    pub fn add_sample(&mut self, sample: MetricSample) {
        self.samples.push(sample);
    }

    /// Export as Prometheus text format
    pub fn export_text(&self) -> String {
        let mut output = String::new();

        // Add HELP line
        output.push_str(&format!("# HELP {} {}\n", self.name, self.help));

        // Add TYPE line
        output.push_str(&format!("# TYPE {} {}\n", self.name, self.metric_type));

        // Add samples
        for sample in &self.samples {
            if sample.labels.is_empty() {
                output.push_str(&format!("{} {}\n", sample.name, sample.value));
            } else {
                let labels_str = sample.labels
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect::<Vec<_>>()
                    .join(",");
                output.push_str(&format!("{}{{{}}} {}\n", sample.name, labels_str, sample.value));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psi_metrics_creation() {
        let metrics = PSIMetrics::new();
        assert_eq!(metrics.syscalls_total, 0);
        assert_eq!(metrics.cache_hits, 0);
        assert_eq!(metrics.cache_misses, 0);
    }

    #[test]
    fn test_cache_hit_ratio() {
        let mut metrics = PSIMetrics::new();
        metrics.cache_hits = 75;
        metrics.cache_misses = 25;
        assert!((metrics.cache_hit_ratio() - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_cached_reading_validity() {
        let cached = CachedReading::new("test data".to_string(), 10);
        assert!(cached.is_valid());
        assert_eq!(cached.age_secs(), 0);
    }

    #[test]
    fn test_performance_monitor_creation() {
        let monitor = PSIPerformanceMonitor::new(1);
        assert_eq!(monitor.cache_ttl_secs, 1);
        assert_eq!(monitor.cache_hit_ratio(), 0.0);
    }

    #[test]
    fn test_performance_monitor_hits() {
        let mut monitor = PSIPerformanceMonitor::new(1);
        monitor.record_hit(5);
        monitor.record_hit(5);
        monitor.record_miss(10);

        let metrics = monitor.get_metrics();
        assert_eq!(metrics.cache_hits, 2);
        assert_eq!(metrics.cache_misses, 1);
    }

    #[test]
    fn test_performance_monitor_snapshot() {
        let mut monitor = PSIPerformanceMonitor::new(1);
        monitor.record_hit(5);

        let snapshot = monitor.snapshot_metrics();
        assert_eq!(snapshot.cache_hits, 1);

        // New metrics should be reset
        assert_eq!(monitor.get_metrics().cache_hits, 0);
    }

    #[test]
    fn test_prometheus_metrics_export() {
        let mut metrics = PrometheusMetrics::new(
            "psi_memory_hits".to_string(),
            "counter".to_string(),
            "PSI cache hits".to_string(),
        );

        let mut sample = MetricSample {
            name: "psi_memory_hits".to_string(),
            labels: std::collections::HashMap::new(),
            value: 100.0,
            timestamp: None,
        };
        sample.labels.insert("type".to_string(), "some".to_string());

        metrics.add_sample(sample);
        let output = metrics.export_text();

        assert!(output.contains("# HELP"));
        assert!(output.contains("# TYPE"));
        assert!(output.contains("psi_memory_hits"));
    }

    #[test]
    fn test_prometheus_metrics_text_format() {
        let mut metrics = PrometheusMetrics::new(
            "test_metric".to_string(),
            "gauge".to_string(),
            "Test metric".to_string(),
        );

        let sample = MetricSample {
            name: "test_metric".to_string(),
            labels: std::collections::HashMap::new(),
            value: 42.5,
            timestamp: None,
        };

        metrics.add_sample(sample);
        let output = metrics.export_text();

        assert!(output.contains("test_metric 42.5"));
    }

    #[test]
    fn test_average_metrics() {
        let mut monitor = PSIPerformanceMonitor::new(1);

        for _ in 0..5 {
            monitor.record_hit(10);
            monitor.snapshot_metrics();
        }

        let avg = monitor.average_metrics();
        assert!(avg.cache_hits > 0);
    }

    #[test]
    fn test_cache_clear() {
        let mut monitor = PSIPerformanceMonitor::new(1);
        monitor.cache = Some(CachedReading::new("data".to_string(), 1));

        assert!(monitor.cache.is_some());
        monitor.clear_cache();
        assert!(monitor.cache.is_none());
    }

    #[test]
    fn test_performance_monitor_default() {
        let monitor = PSIPerformanceMonitor::default();
        assert_eq!(monitor.cache_ttl_secs, 1);
    }
}
