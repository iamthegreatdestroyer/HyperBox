//! Storage optimization module for container images.
//!
//! Provides EROFS (Enhanced Read-Only File System) support with advanced
//! mount operations, fscache integration, and performance metrics.

pub mod erofs;
pub mod erofs_advanced;
pub mod erofs_mount;

pub use erofs::ErofsBackend;
pub use erofs_advanced::{ErofsPerformanceMetrics, CacheStatistics, BenchmarkResult, FallbackStrategy};
pub use erofs_mount::{MountManager, MountHandle, MountStats, MountInfo, MountOptions};
