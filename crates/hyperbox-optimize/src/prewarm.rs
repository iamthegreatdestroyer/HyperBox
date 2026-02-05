//! Predictive container pre-warming.
//!
//! Pre-warms containers based on usage patterns to achieve
//! near-instant startup times.

use crate::error::{OptimizeError, Result};
use crate::predict::UsagePredictor;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time;
use tracing::{debug, info};

/// Pre-warm configuration.
#[derive(Debug, Clone)]
pub struct PrewarmConfig {
    /// Maximum number of pre-warmed containers.
    pub max_prewarmed: usize,
    /// Pre-warm threshold (probability to trigger).
    pub threshold: f64,
    /// Prediction lookahead in seconds.
    pub lookahead_seconds: u64,
    /// Cleanup interval in seconds.
    pub cleanup_interval_seconds: u64,
    /// Pre-warmed container TTL in seconds.
    pub prewarm_ttl_seconds: u64,
}

impl Default for PrewarmConfig {
    fn default() -> Self {
        Self {
            max_prewarmed: 10,
            threshold: 0.7,
            lookahead_seconds: 300, // 5 minutes
            cleanup_interval_seconds: 60,
            prewarm_ttl_seconds: 600, // 10 minutes
        }
    }
}

/// Pre-warmed container state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrewarmedContainer {
    /// Container ID.
    pub container_id: String,
    /// Image name.
    pub image: String,
    /// When pre-warmed.
    pub prewarmed_at: DateTime<Utc>,
    /// Expires at.
    pub expires_at: DateTime<Utc>,
    /// Whether it was used.
    pub was_used: bool,
    /// Prediction score that triggered pre-warming.
    pub prediction_score: f64,
}

/// Pre-warm manager.
pub struct PrewarmManager {
    /// Configuration.
    config: PrewarmConfig,
    /// Usage predictor.
    predictor: Arc<UsagePredictor>,
    /// Pre-warmed containers.
    prewarmed: DashMap<String, PrewarmedContainer>,
    /// Statistics.
    stats: Arc<PrewarmStats>,
    /// Data directory.
    data_dir: PathBuf,
    /// Shutdown signal.
    shutdown: Arc<RwLock<bool>>,
}

/// Pre-warm statistics.
#[derive(Debug, Default)]
struct PrewarmStats {
    /// Total containers pre-warmed.
    total_prewarmed: AtomicU64,
    /// Containers that were used.
    hits: AtomicU64,
    /// Containers that expired unused.
    misses: AtomicU64,
    /// Average prediction score for hits.
    total_hit_score: AtomicU64, // Stored as u64 * 1000
}

impl PrewarmManager {
    /// Create a new pre-warm manager.
    pub fn new(predictor: Arc<UsagePredictor>, data_dir: impl Into<PathBuf>) -> Self {
        Self::with_config(predictor, data_dir, PrewarmConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(
        predictor: Arc<UsagePredictor>,
        data_dir: impl Into<PathBuf>,
        config: PrewarmConfig,
    ) -> Self {
        Self {
            config,
            predictor,
            prewarmed: DashMap::new(),
            stats: Arc::new(PrewarmStats::default()),
            data_dir: data_dir.into(),
            shutdown: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize the pre-warm manager.
    pub async fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.data_dir).await?;
        info!("Pre-warm manager initialized");
        Ok(())
    }

    /// Start the pre-warm background task.
    pub async fn start(&self) {
        let shutdown = Arc::clone(&self.shutdown);
        let config = self.config.clone();

        // Start prediction loop
        tokio::spawn(async move {
            let mut interval =
                time::interval(std::time::Duration::from_secs(config.lookahead_seconds / 2));

            loop {
                interval.tick().await;

                if *shutdown.read().await {
                    break;
                }

                // Run predictions and pre-warm
                // In practice, this would call the predictor and trigger pre-warming
            }
        });

        // Start cleanup loop
        let shutdown = Arc::clone(&self.shutdown);
        let prewarmed = self.prewarmed.clone();
        let stats = Arc::clone(&self.stats);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval =
                time::interval(std::time::Duration::from_secs(config.cleanup_interval_seconds));

            loop {
                interval.tick().await;

                if *shutdown.read().await {
                    break;
                }

                // Clean up expired pre-warmed containers
                let now = Utc::now();
                let to_remove: Vec<_> = prewarmed
                    .iter()
                    .filter(|r| r.expires_at < now && !r.was_used)
                    .map(|r| r.key().clone())
                    .collect();

                for id in to_remove {
                    if let Some((_, container)) = prewarmed.remove(&id) {
                        debug!("Cleaning up expired pre-warmed container: {}", id);
                        // Would stop the container here
                        stats.misses.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });

        info!("Pre-warm manager started");
    }

    /// Pre-warm a container.
    pub async fn prewarm(&self, image: &str, prediction_score: f64) -> Result<String> {
        // Check limits
        if self.prewarmed.len() >= self.config.max_prewarmed {
            return Err(OptimizeError::ResourceExhausted {
                resource: "pre-warmed container slots".to_string(),
            });
        }

        // Check if already pre-warmed
        if self.prewarmed.iter().any(|r| r.image == image) {
            debug!("Image {} already pre-warmed", image);
            return Err(OptimizeError::PrewarmFailed {
                image: image.to_string(),
                reason: "Already pre-warmed".to_string(),
            });
        }

        // Would create container here
        let container_id = format!("prewarm-{}", uuid::Uuid::new_v4());
        let now = Utc::now();

        let container = PrewarmedContainer {
            container_id: container_id.clone(),
            image: image.to_string(),
            prewarmed_at: now,
            expires_at: now + Duration::seconds(self.config.prewarm_ttl_seconds as i64),
            was_used: false,
            prediction_score,
        };

        self.prewarmed.insert(container_id.clone(), container);
        self.stats.total_prewarmed.fetch_add(1, Ordering::Relaxed);

        info!(
            "Pre-warmed container {} for image {} (score: {:.2})",
            container_id, image, prediction_score
        );

        Ok(container_id)
    }

    /// Try to claim a pre-warmed container.
    pub async fn claim(&self, image: &str) -> Option<String> {
        // Find a pre-warmed container for this image
        let container_id = self
            .prewarmed
            .iter()
            .find(|r| r.image == image && !r.was_used && r.expires_at > Utc::now())
            .map(|r| r.container_id.clone());

        if let Some(id) = container_id.clone() {
            if let Some(mut container) = self.prewarmed.get_mut(&id) {
                container.was_used = true;
                self.stats.hits.fetch_add(1, Ordering::Relaxed);

                // Track prediction score for accuracy calculation
                let score_int = (container.prediction_score * 1000.0) as u64;
                self.stats
                    .total_hit_score
                    .fetch_add(score_int, Ordering::Relaxed);

                info!("Claimed pre-warmed container {} for image {}", id, image);
            }
        }

        container_id
    }

    /// Get suggestions for images to pre-warm.
    pub async fn get_suggestions(&self) -> Vec<(String, f64)> {
        // Get predictions from the usage predictor
        let now = Utc::now();
        let lookahead = std::time::Duration::from_secs(self.config.lookahead_seconds);

        // In practice, this would query the predictor
        // For now, return empty list
        Vec::new()
    }

    /// Get pre-warm hit rate.
    pub fn hit_rate(&self) -> f64 {
        let hits = self.stats.hits.load(Ordering::Relaxed);
        let misses = self.stats.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            return 0.0;
        }

        hits as f64 / total as f64
    }

    /// Get average prediction score for hits.
    pub fn average_hit_score(&self) -> f64 {
        let hits = self.stats.hits.load(Ordering::Relaxed);
        let total_score = self.stats.total_hit_score.load(Ordering::Relaxed);

        if hits == 0 {
            return 0.0;
        }

        (total_score as f64 / 1000.0) / hits as f64
    }

    /// Get statistics.
    pub fn stats(&self) -> (u64, u64, u64, f64) {
        (
            self.stats.total_prewarmed.load(Ordering::Relaxed),
            self.stats.hits.load(Ordering::Relaxed),
            self.stats.misses.load(Ordering::Relaxed),
            self.hit_rate(),
        )
    }

    /// Stop the pre-warm manager.
    pub async fn stop(&self) {
        *self.shutdown.write().await = true;

        // Clean up all pre-warmed containers
        for container in self.prewarmed.iter() {
            debug!("Stopping pre-warmed container: {}", container.container_id);
            // Would stop container here
        }

        self.prewarmed.clear();
        info!("Pre-warm manager stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_predictor(path: &std::path::Path) -> Arc<UsagePredictor> {
        Arc::new(UsagePredictor::new(path))
    }

    #[test]
    fn test_prewarm_config_default() {
        let config = PrewarmConfig::default();
        assert_eq!(config.max_prewarmed, 10);
        assert_eq!(config.threshold, 0.7);
        assert_eq!(config.lookahead_seconds, 300);
        assert_eq!(config.cleanup_interval_seconds, 60);
        assert_eq!(config.prewarm_ttl_seconds, 600);
    }

    #[test]
    fn test_prewarm_manager_new() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let manager = PrewarmManager::new(predictor, temp.path().join("prewarm"));

        let (total, hits, misses, rate) = manager.stats();
        assert_eq!(total, 0);
        assert_eq!(hits, 0);
        assert_eq!(misses, 0);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_prewarm_manager_with_config() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let config = PrewarmConfig {
            max_prewarmed: 5,
            threshold: 0.5,
            lookahead_seconds: 120,
            cleanup_interval_seconds: 30,
            prewarm_ttl_seconds: 300,
        };

        let manager = PrewarmManager::with_config(predictor, temp.path(), config);
        assert_eq!(manager.config.max_prewarmed, 5);
        assert_eq!(manager.config.threshold, 0.5);
    }

    #[tokio::test]
    async fn test_initialize_creates_directory() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let prewarm_dir = temp.path().join("prewarm_data");
        let manager = PrewarmManager::new(predictor, &prewarm_dir);

        manager.initialize().await.unwrap();
        assert!(prewarm_dir.exists());
    }

    #[tokio::test]
    async fn test_prewarm_container() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let manager = PrewarmManager::new(predictor, temp.path());
        manager.initialize().await.unwrap();

        let result = manager.prewarm("alpine:latest", 0.85).await;
        assert!(result.is_ok());

        let container_id = result.unwrap();
        assert!(container_id.starts_with("prewarm-"));

        let (total, _, _, _) = manager.stats();
        assert_eq!(total, 1);
    }

    #[tokio::test]
    async fn test_prewarm_duplicate_image() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let manager = PrewarmManager::new(predictor, temp.path());
        manager.initialize().await.unwrap();

        // First prewarm succeeds
        manager.prewarm("nginx:latest", 0.9).await.unwrap();

        // Second prewarm of same image fails
        let result = manager.prewarm("nginx:latest", 0.95).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_prewarm_max_limit() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let config = PrewarmConfig {
            max_prewarmed: 2,
            ..Default::default()
        };
        let manager = PrewarmManager::with_config(predictor, temp.path(), config);
        manager.initialize().await.unwrap();

        // Fill up slots
        manager.prewarm("img1:latest", 0.8).await.unwrap();
        manager.prewarm("img2:latest", 0.8).await.unwrap();

        // Third should fail
        let result = manager.prewarm("img3:latest", 0.8).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_claim_prewarmed_container() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let manager = PrewarmManager::new(predictor, temp.path());
        manager.initialize().await.unwrap();

        // Prewarm a container
        manager.prewarm("redis:latest", 0.9).await.unwrap();

        // Claim it
        let claimed = manager.claim("redis:latest").await;
        assert!(claimed.is_some());

        let (_, hits, _, _) = manager.stats();
        assert_eq!(hits, 1);
    }

    #[tokio::test]
    async fn test_claim_nonexistent_image() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let manager = PrewarmManager::new(predictor, temp.path());
        manager.initialize().await.unwrap();

        let claimed = manager.claim("nonexistent:latest").await;
        assert!(claimed.is_none());
    }

    #[tokio::test]
    async fn test_claim_already_used() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let manager = PrewarmManager::new(predictor, temp.path());
        manager.initialize().await.unwrap();

        manager.prewarm("mysql:latest", 0.85).await.unwrap();

        // First claim succeeds
        let first = manager.claim("mysql:latest").await;
        assert!(first.is_some());

        // Second claim fails (already used)
        let second = manager.claim("mysql:latest").await;
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn test_hit_rate_calculation() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let manager = PrewarmManager::new(predictor, temp.path());
        manager.initialize().await.unwrap();

        // No data yet
        assert_eq!(manager.hit_rate(), 0.0);

        // Prewarm and claim (hit)
        manager.prewarm("hit:latest", 0.9).await.unwrap();
        manager.claim("hit:latest").await;

        let (_, hits, _, rate) = manager.stats();
        assert_eq!(hits, 1);
        // Rate only counts after cleanup categorizes misses
    }

    #[tokio::test]
    async fn test_average_hit_score() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let manager = PrewarmManager::new(predictor, temp.path());
        manager.initialize().await.unwrap();

        // No hits yet
        assert_eq!(manager.average_hit_score(), 0.0);

        // Prewarm with known score and claim
        manager.prewarm("score:latest", 0.75).await.unwrap();
        manager.claim("score:latest").await;

        let avg = manager.average_hit_score();
        assert!((avg - 0.75).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_get_suggestions() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let manager = PrewarmManager::new(predictor, temp.path());
        manager.initialize().await.unwrap();

        // Currently returns empty vec (implementation placeholder)
        let suggestions = manager.get_suggestions().await;
        assert!(suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_stop_clears_prewarmed() {
        let temp = tempdir().unwrap();
        let predictor = create_test_predictor(temp.path());
        let manager = PrewarmManager::new(predictor, temp.path());
        manager.initialize().await.unwrap();

        manager.prewarm("stop:latest", 0.8).await.unwrap();
        assert_eq!(manager.prewarmed.len(), 1);

        manager.stop().await;
        assert_eq!(manager.prewarmed.len(), 0);
    }

    #[test]
    fn test_prewarmed_container_struct() {
        let now = Utc::now();
        let container = PrewarmedContainer {
            container_id: "test-123".to_string(),
            image: "test:latest".to_string(),
            prewarmed_at: now,
            expires_at: now + Duration::seconds(600),
            was_used: false,
            prediction_score: 0.92,
        };

        assert!(!container.was_used);
        assert!(container.expires_at > now);
        assert_eq!(container.prediction_score, 0.92);
    }
}
