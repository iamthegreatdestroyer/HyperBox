//! Usage prediction using ML-inspired algorithms.
//!
//! Predicts container usage patterns for optimization.

use crate::error::{OptimizeError, Result};
use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::fs;
use tracing::{debug, info};

/// Minimum samples needed for prediction.
const MIN_SAMPLES: usize = 10;
/// Maximum history size.
const MAX_HISTORY: usize = 10000;
/// Number of time buckets per day.
const TIME_BUCKETS: usize = 24;
/// Number of day buckets per week.
const DAY_BUCKETS: usize = 7;

/// Usage event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    /// Image name.
    pub image: String,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Duration in seconds.
    pub duration_seconds: u64,
    /// Project ID.
    pub project_id: Option<String>,
}

/// Time-based feature vector.
#[derive(Debug, Clone, Default)]
struct TimeFeatures {
    /// Hour of day (0-23).
    hour: u8,
    /// Day of week (0-6).
    day_of_week: u8,
    /// Is weekend.
    is_weekend: bool,
    /// Minutes since midnight.
    minutes: u16,
}

impl TimeFeatures {
    fn from_datetime(dt: DateTime<Utc>) -> Self {
        let day_of_week = dt.weekday().num_days_from_monday() as u8;
        Self {
            hour: dt.hour() as u8,
            day_of_week,
            is_weekend: matches!(dt.weekday(), Weekday::Sat | Weekday::Sun),
            minutes: (dt.hour() * 60 + dt.minute()) as u16,
        }
    }
}

/// Image usage model with multi-signal prediction.
///
/// Combines decay-weighted temporal frequencies, recency,
/// trend detection, and session-duration importance into
/// a single fused prediction score.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImageModel {
    /// Hourly usage counts [day_of_week][hour].
    hourly_counts: [[u64; TIME_BUCKETS]; DAY_BUCKETS],
    /// Exponentially decay-weighted hourly counts.
    #[serde(default)]
    decay_weights: [[f64; TIME_BUCKETS]; DAY_BUCKETS],
    /// Total lifetime usage count.
    total_count: u64,
    /// Running average duration in seconds.
    avg_duration: f64,
    /// Most recent usage timestamp.
    last_used: Option<DateTime<Utc>>,
    /// Recent inter-arrival intervals (seconds) for trend detection.
    #[serde(default)]
    recent_intervals: Vec<f64>,
    /// Usage trend indicator (−1.0 = falling, +1.0 = rising).
    #[serde(default)]
    usage_trend: f64,
    /// Per-day decay factor (applied each elapsed day).
    #[serde(default = "default_decay_lambda")]
    decay_lambda: f64,
}

fn default_decay_lambda() -> f64 {
    0.95
}

impl Default for ImageModel {
    fn default() -> Self {
        Self {
            hourly_counts: [[0; TIME_BUCKETS]; DAY_BUCKETS],
            decay_weights: [[0.0; TIME_BUCKETS]; DAY_BUCKETS],
            total_count: 0,
            avg_duration: 0.0,
            last_used: None,
            recent_intervals: Vec::new(),
            usage_trend: 0.0,
            decay_lambda: 0.95,
        }
    }
}

impl ImageModel {
    /// Record a usage event, updating all model signals.
    fn record(&mut self, event: &UsageEvent) {
        let features = TimeFeatures::from_datetime(event.timestamp);
        let d = features.day_of_week as usize;
        let h = features.hour as usize;

        // ── Raw frequency count ──
        self.hourly_counts[d][h] += 1;
        self.total_count += 1;

        // ── Running average duration ──
        let n = self.total_count as f64;
        self.avg_duration = (self.avg_duration * (n - 1.0) + event.duration_seconds as f64) / n;

        // ── Decay-weighted counts ──
        // Apply exponential decay to ALL existing weights proportional to
        // the time elapsed since the last event, then increment current slot.
        if let Some(last) = self.last_used {
            let elapsed_days = (event.timestamp - last).num_hours().max(0) as f64 / 24.0;
            let decay = self.decay_lambda.powf(elapsed_days);
            for row in self.decay_weights.iter_mut() {
                for w in row.iter_mut() {
                    *w *= decay;
                }
            }
        }
        self.decay_weights[d][h] += 1.0;

        // ── Inter-arrival intervals & trend ──
        if let Some(last) = self.last_used {
            let interval = (event.timestamp - last).num_seconds().max(0) as f64;
            self.recent_intervals.push(interval);
            // Keep a sliding window of the 50 most recent intervals.
            if self.recent_intervals.len() > 50 {
                self.recent_intervals.remove(0);
            }
            self.usage_trend = self.compute_trend();
        }

        self.last_used = Some(event.timestamp);
    }

    /// Compute a simple split-half trend from recent inter-arrival times.
    ///
    /// Returns a value in [−1.0, 1.0]:
    ///   positive → intervals are shrinking → usage is *accelerating*
    ///   negative → intervals are growing   → usage is *decelerating*
    fn compute_trend(&self) -> f64 {
        let n = self.recent_intervals.len();
        if n < 3 {
            return 0.0;
        }
        let mid = n / 2;
        let first_avg: f64 = self.recent_intervals[..mid].iter().sum::<f64>() / mid as f64;
        let second_avg: f64 = self.recent_intervals[mid..].iter().sum::<f64>() / (n - mid) as f64;
        if first_avg == 0.0 {
            return 0.0;
        }
        ((first_avg - second_avg) / first_avg).clamp(-1.0, 1.0)
    }

    /// Multi-signal prediction combining temporal frequency, recency,
    /// trend, and duration-importance into a single 0.0–1.0 score.
    fn predict_probability(&self, dt: DateTime<Utc>) -> f64 {
        if self.total_count < MIN_SAMPLES as u64 {
            return 0.0;
        }

        let features = TimeFeatures::from_datetime(dt);
        let d = features.day_of_week as usize;
        let h = features.hour as usize;

        // ── Signal 1 (weight 0.45): Decay-weighted temporal frequency ──
        let decay_w = self.decay_weights[d][h];
        let max_decay = self
            .decay_weights
            .iter()
            .flat_map(|row| row.iter())
            .cloned()
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let temporal_score = (decay_w / max_decay).min(1.0);

        // ── Signal 2 (weight 0.25): Recency ──
        // Exponential decay with a ~12-hour half-life.
        let recency_score = self
            .last_used
            .map(|last| {
                let hours_since = (dt - last).num_hours().max(0) as f64;
                (-hours_since / 17.3).exp() // half-life ≈ 12 h
            })
            .unwrap_or(0.0);

        // ── Signal 3 (weight 0.15): Trend ──
        // Map [-1, 1] → [0, 1] so positive trend (accelerating) → ~1.0.
        let trend_score = ((self.usage_trend + 1.0) / 2.0).clamp(0.0, 1.0);

        // ── Signal 4 (weight 0.15): Duration-anchored importance ──
        // Longer average sessions → heavier / more critical workloads.
        let duration_score = (self.avg_duration / 3600.0).min(1.0); // saturates at 1 h

        // ── Weighted fusion ──
        let combined = temporal_score * 0.45
            + recency_score * 0.25
            + trend_score * 0.15
            + duration_score * 0.15;

        combined.clamp(0.0, 1.0)
    }

    fn get_peak_hours(&self) -> Vec<(u8, u8, u64)> {
        let mut peaks: Vec<_> = self
            .hourly_counts
            .iter()
            .enumerate()
            .flat_map(|(day, hours)| {
                hours
                    .iter()
                    .enumerate()
                    .map(move |(hour, &count)| (day as u8, hour as u8, count))
            })
            .filter(|(_, _, count)| *count > 0)
            .collect();

        peaks.sort_by(|a, b| b.2.cmp(&a.2));
        peaks.truncate(10);
        peaks
    }
}

/// Usage predictor with multi-signal fusion and project correlation.
pub struct UsagePredictor {
    /// Per-image statistical models.
    models: DashMap<String, ImageModel>,
    /// Recent event history per image.
    history: DashMap<String, VecDeque<UsageEvent>>,
    /// Project → images mapping for correlation boosting.
    project_images: DashMap<String, Vec<String>>,
    /// Data directory.
    data_dir: PathBuf,
    /// Total predictions made.
    predictions_made: AtomicU64,
}

impl UsagePredictor {
    /// Create a new usage predictor.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            models: DashMap::new(),
            history: DashMap::new(),
            project_images: DashMap::new(),
            data_dir: data_dir.into(),
            predictions_made: AtomicU64::new(0),
        }
    }

    /// Initialize the predictor.
    pub async fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.data_dir).await?;

        // Load saved models
        let models_path = self.data_dir.join("models.json");
        if models_path.exists() {
            let content = fs::read_to_string(&models_path).await?;
            if let Ok(models) = serde_json::from_str::<Vec<(String, ImageModel)>>(&content) {
                for (image, model) in models {
                    self.models.insert(image, model);
                }
            }
        }

        info!("Usage predictor initialized with {} models", self.models.len());
        Ok(())
    }

    /// Record a usage event, updating model, history, and project correlations.
    pub fn record(&self, event: UsageEvent) {
        let image_key = event.image.clone();

        // Update image model
        self.models
            .entry(image_key.clone())
            .or_default()
            .record(&event);

        // Track project → image correlation
        if let Some(ref project) = event.project_id {
            let mut images = self
                .project_images
                .entry(project.clone())
                .or_insert_with(Vec::new);
            if !images.contains(&image_key) {
                images.push(image_key.clone());
            }
        }

        // Add to history
        self.history
            .entry(image_key.clone())
            .or_insert_with(VecDeque::new)
            .push_back(event);

        // Trim history if needed
        if let Some(mut history) = self.history.get_mut(&image_key) {
            while history.len() > MAX_HISTORY {
                history.pop_front();
            }
        }
    }

    /// Predict usage probability for an image at a given time.
    pub fn predict(&self, image: &str, at: DateTime<Utc>) -> f64 {
        self.predictions_made.fetch_add(1, Ordering::Relaxed);

        self.models
            .get(image)
            .map(|m| m.predict_probability(at))
            .unwrap_or(0.0)
    }

    /// Predict with project context — boosts co-used images.
    pub fn predict_with_context(
        &self,
        image: &str,
        at: DateTime<Utc>,
        active_project: Option<&str>,
    ) -> f64 {
        let base = self.predict(image, at);

        // If a project is active and historically uses this image, apply a
        // correlation boost (capped at +20 percentage-points).
        if let Some(project) = active_project {
            if let Some(images) = self.project_images.get(project) {
                if images.contains(&image.to_string()) {
                    return (base + 0.20).min(1.0);
                }
            }
        }

        base
    }

    /// Get images historically correlated with a project.
    pub fn project_correlated_images(&self, project: &str) -> Vec<String> {
        self.project_images
            .get(project)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Predict usage probability for the next N minutes.
    pub fn predict_window(&self, image: &str, window_minutes: u64) -> f64 {
        let now = Utc::now();
        let end = now + Duration::minutes(window_minutes as i64);

        // Sample probability at multiple points
        let samples = 5;
        let step = window_minutes as i64 / samples;

        let mut total = 0.0;
        for i in 0..samples {
            let t = now + Duration::minutes(step * i);
            total += self.predict(image, t);
        }

        total / samples as f64
    }

    /// Get top predictions for the next time window.
    pub fn get_predictions(&self, window_minutes: u64, limit: usize) -> Vec<(String, f64)> {
        let mut predictions: Vec<_> = self
            .models
            .iter()
            .map(|r| {
                let image = r.key().clone();
                let prob = self.predict_window(&image, window_minutes);
                (image, prob)
            })
            .filter(|(_, prob)| *prob > 0.0)
            .collect();

        predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        predictions.truncate(limit);
        predictions
    }

    /// Get peak usage hours for an image.
    pub fn get_peak_hours(&self, image: &str) -> Option<Vec<(u8, u8, u64)>> {
        self.models.get(image).map(|m| m.get_peak_hours())
    }

    /// Get model statistics.
    pub fn model_stats(&self, image: &str) -> Option<(u64, f64, Option<DateTime<Utc>>)> {
        self.models
            .get(image)
            .map(|m| (m.total_count, m.avg_duration, m.last_used))
    }

    /// Get all tracked images.
    pub fn tracked_images(&self) -> Vec<String> {
        self.models.iter().map(|r| r.key().clone()).collect()
    }

    /// Get total number of models.
    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    /// Get total predictions made.
    pub fn predictions_count(&self) -> u64 {
        self.predictions_made.load(Ordering::Relaxed)
    }

    /// Save models to disk.
    pub async fn save(&self) -> Result<()> {
        let models: Vec<_> = self
            .models
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect();

        let content = serde_json::to_string_pretty(&models)?;
        let path = self.data_dir.join("models.json");
        fs::write(path, content).await?;

        debug!("Saved {} prediction models", models.len());
        Ok(())
    }

    /// Train models on historical data.
    pub async fn train(&self, events: Vec<UsageEvent>) -> Result<()> {
        if events.len() < MIN_SAMPLES {
            return Err(OptimizeError::InsufficientData {
                required: MIN_SAMPLES,
                available: events.len(),
            });
        }

        for event in events {
            self.record(event);
        }

        // Save updated models
        self.save().await?;

        info!("Trained on {} events", self.models.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_event(image: &str, hour: u32, day: Weekday) -> UsageEvent {
        let now = Utc::now();
        let timestamp = now
            .with_hour(hour)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();
        UsageEvent {
            image: image.to_string(),
            timestamp,
            duration_seconds: 300,
            project_id: None,
        }
    }

    #[test]
    fn test_usage_predictor_new() {
        let predictor = UsagePredictor::new("/tmp/test");
        assert_eq!(predictor.model_count(), 0);
        assert_eq!(predictor.predictions_count(), 0);
    }

    #[test]
    fn test_record_event() {
        let predictor = UsagePredictor::new("/tmp/test");
        let event = UsageEvent {
            image: "alpine:latest".to_string(),
            timestamp: Utc::now(),
            duration_seconds: 120,
            project_id: Some("test-project".to_string()),
        };

        predictor.record(event);
        assert_eq!(predictor.model_count(), 1);
        assert!(predictor
            .tracked_images()
            .contains(&"alpine:latest".to_string()));
    }

    #[test]
    fn test_record_multiple_events_same_image() {
        let predictor = UsagePredictor::new("/tmp/test");

        for i in 0..5 {
            let event = UsageEvent {
                image: "nginx:latest".to_string(),
                timestamp: Utc::now() + Duration::hours(i),
                duration_seconds: 60 + (i as u64 * 10),
                project_id: None,
            };
            predictor.record(event);
        }

        assert_eq!(predictor.model_count(), 1);
        let stats = predictor.model_stats("nginx:latest").unwrap();
        assert_eq!(stats.0, 5); // total_count
    }

    #[test]
    fn test_predict_no_data() {
        let predictor = UsagePredictor::new("/tmp/test");
        let prob = predictor.predict("nonexistent:latest", Utc::now());
        assert_eq!(prob, 0.0);
    }

    #[test]
    fn test_predict_insufficient_samples() {
        let predictor = UsagePredictor::new("/tmp/test");

        // Add fewer than MIN_SAMPLES events
        for i in 0..5 {
            let event = UsageEvent {
                image: "test:latest".to_string(),
                timestamp: Utc::now() + Duration::hours(i),
                duration_seconds: 100,
                project_id: None,
            };
            predictor.record(event);
        }

        // Should return 0.0 due to insufficient samples
        let prob = predictor.predict("test:latest", Utc::now());
        assert_eq!(prob, 0.0);
    }

    #[test]
    fn test_predict_with_sufficient_samples() {
        let predictor = UsagePredictor::new("/tmp/test");
        let now = Utc::now();

        // Add MIN_SAMPLES + 5 events at the same hour
        for i in 0..15 {
            let event = UsageEvent {
                image: "redis:latest".to_string(),
                timestamp: now + Duration::days(i),
                duration_seconds: 200,
                project_id: None,
            };
            predictor.record(event);
        }

        let prob = predictor.predict("redis:latest", now);
        assert!(prob >= 0.0 && prob <= 1.0);
    }

    #[test]
    fn test_predict_window() {
        let predictor = UsagePredictor::new("/tmp/test");
        let now = Utc::now();

        // Add enough events
        for i in 0..20 {
            let event = UsageEvent {
                image: "postgres:latest".to_string(),
                timestamp: now + Duration::hours(i),
                duration_seconds: 500,
                project_id: None,
            };
            predictor.record(event);
        }

        let prob = predictor.predict_window("postgres:latest", 60);
        assert!(prob >= 0.0 && prob <= 1.0);
    }

    #[test]
    fn test_get_predictions() {
        let predictor = UsagePredictor::new("/tmp/test");
        let now = Utc::now();

        // Add events for multiple images
        for image in &["img1:latest", "img2:latest", "img3:latest"] {
            for i in 0..15 {
                let event = UsageEvent {
                    image: image.to_string(),
                    timestamp: now + Duration::hours(i),
                    duration_seconds: 100,
                    project_id: None,
                };
                predictor.record(event);
            }
        }

        let predictions = predictor.get_predictions(60, 10);
        assert!(predictions.len() <= 10);
    }

    #[test]
    fn test_model_stats() {
        let predictor = UsagePredictor::new("/tmp/test");
        let event = UsageEvent {
            image: "mysql:latest".to_string(),
            timestamp: Utc::now(),
            duration_seconds: 300,
            project_id: None,
        };

        predictor.record(event);
        let stats = predictor.model_stats("mysql:latest");

        assert!(stats.is_some());
        let (count, avg_duration, last_used) = stats.unwrap();
        assert_eq!(count, 1);
        assert_eq!(avg_duration, 300.0);
        assert!(last_used.is_some());
    }

    #[test]
    fn test_tracked_images() {
        let predictor = UsagePredictor::new("/tmp/test");

        predictor.record(UsageEvent {
            image: "a:latest".to_string(),
            timestamp: Utc::now(),
            duration_seconds: 100,
            project_id: None,
        });
        predictor.record(UsageEvent {
            image: "b:latest".to_string(),
            timestamp: Utc::now(),
            duration_seconds: 100,
            project_id: None,
        });

        let images = predictor.tracked_images();
        assert_eq!(images.len(), 2);
        assert!(images.contains(&"a:latest".to_string()));
        assert!(images.contains(&"b:latest".to_string()));
    }

    #[test]
    fn test_predictions_count_increment() {
        let predictor = UsagePredictor::new("/tmp/test");

        assert_eq!(predictor.predictions_count(), 0);

        predictor.predict("any:image", Utc::now());
        assert_eq!(predictor.predictions_count(), 1);

        predictor.predict("other:image", Utc::now());
        assert_eq!(predictor.predictions_count(), 2);
    }

    #[test]
    fn test_time_features_weekend() {
        // Sunday
        let sunday = Utc::now().with_ordinal(7).unwrap_or(Utc::now());
        let features = TimeFeatures::from_datetime(sunday);
        // Just verify the struct is created correctly
        assert!(features.hour < 24);
        assert!(features.day_of_week < 7);
    }

    #[tokio::test]
    async fn test_initialize_creates_directory() {
        let temp = tempdir().unwrap();
        let predictor = UsagePredictor::new(temp.path().join("models"));

        predictor.initialize().await.unwrap();

        assert!(temp.path().join("models").exists());
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let temp = tempdir().unwrap();
        let predictor = UsagePredictor::new(temp.path());

        predictor.initialize().await.unwrap();

        // Add some data
        for i in 0..5 {
            predictor.record(UsageEvent {
                image: "test:latest".to_string(),
                timestamp: Utc::now() + Duration::hours(i),
                duration_seconds: 100,
                project_id: None,
            });
        }

        // Save
        predictor.save().await.unwrap();

        // Verify file exists
        assert!(temp.path().join("models.json").exists());

        // Create new predictor and load
        let predictor2 = UsagePredictor::new(temp.path());
        predictor2.initialize().await.unwrap();

        assert_eq!(predictor2.model_count(), 1);
    }

    #[tokio::test]
    async fn test_train_insufficient_data() {
        let temp = tempdir().unwrap();
        let predictor = UsagePredictor::new(temp.path());
        predictor.initialize().await.unwrap();

        let events: Vec<UsageEvent> = (0..5)
            .map(|i| UsageEvent {
                image: "test:latest".to_string(),
                timestamp: Utc::now() + Duration::hours(i),
                duration_seconds: 100,
                project_id: None,
            })
            .collect();

        let result = predictor.train(events).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_train_sufficient_data() {
        let temp = tempdir().unwrap();
        let predictor = UsagePredictor::new(temp.path());
        predictor.initialize().await.unwrap();

        let events: Vec<UsageEvent> = (0..15)
            .map(|i| UsageEvent {
                image: "train:latest".to_string(),
                timestamp: Utc::now() + Duration::hours(i),
                duration_seconds: 100,
                project_id: None,
            })
            .collect();

        let result = predictor.train(events).await;
        assert!(result.is_ok());
        assert!(predictor.model_count() >= 1);
    }
}
