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

/// Image usage model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ImageModel {
    /// Hourly usage counts [day][hour].
    hourly_counts: [[u64; TIME_BUCKETS]; DAY_BUCKETS],
    /// Total usage count.
    total_count: u64,
    /// Average duration.
    avg_duration: f64,
    /// Last used.
    last_used: Option<DateTime<Utc>>,
}

impl ImageModel {
    fn record(&mut self, event: &UsageEvent) {
        let features = TimeFeatures::from_datetime(event.timestamp);
        self.hourly_counts[features.day_of_week as usize][features.hour as usize] += 1;
        self.total_count += 1;

        // Update running average of duration
        let n = self.total_count as f64;
        self.avg_duration = (self.avg_duration * (n - 1.0) + event.duration_seconds as f64) / n;
        self.last_used = Some(event.timestamp);
    }

    fn predict_probability(&self, dt: DateTime<Utc>) -> f64 {
        if self.total_count < MIN_SAMPLES as u64 {
            return 0.0;
        }

        let features = TimeFeatures::from_datetime(dt);
        let hour_count = self.hourly_counts[features.day_of_week as usize][features.hour as usize];

        // Simple probability based on historical frequency
        let max_count = self
            .hourly_counts
            .iter()
            .flat_map(|row| row.iter())
            .max()
            .copied()
            .unwrap_or(1);

        hour_count as f64 / max_count as f64
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

/// Usage predictor.
pub struct UsagePredictor {
    /// Image models.
    models: DashMap<String, ImageModel>,
    /// Recent events.
    history: DashMap<String, VecDeque<UsageEvent>>,
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

    /// Record a usage event.
    pub fn record(&self, event: UsageEvent) {
        let image_key = event.image.clone();

        // Update model
        self.models
            .entry(image_key.clone())
            .or_default()
            .record(&event);

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
