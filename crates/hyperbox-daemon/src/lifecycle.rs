//! Container lifecycle management.

use crate::state::{ContainerState, ContainerStatus, DaemonState, EventType, PortMapping};
use chrono::Utc;
use hyperbox_optimize::predict::UsageEvent;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Manage container lifecycles.
pub async fn manager(state: DaemonState) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        // Process pre-warming predictions
        if state.config.optimization.enable_prewarm {
            process_prewarm_predictions(&state).await;
        }

        // Check for containers that need cleanup
        cleanup_dead_containers(&state).await;

        // Update usage predictor with current usage
        update_predictor(&state).await;
    }
}

async fn process_prewarm_predictions(state: &DaemonState) {
    // Get predictions without holding the lock across await
    let predictions = {
        let predictor = state.predictor.read();
        predictor.get_predictions(5, 10)
    };

    for (image, prob) in predictions {
        if prob >= state.config.optimization.prewarm_threshold {
            // Check if we have a checkpoint for this image
            if let Ok(Some(_checkpoint)) = state.criu.get_checkpoint(&image).await {
                // Try to claim a pre-warmed container
                if state.prewarm.claim(&image).await.is_none() {
                    debug!("Pre-warming container for image: {} (probability: {:.2})", image, prob);
                    // Would trigger pre-warm here
                }
            }
        }
    }
}

async fn cleanup_dead_containers(state: &DaemonState) {
    let dead_containers: Vec<String> = state
        .containers
        .iter()
        .filter(|c| c.status == ContainerStatus::Dead)
        .map(|c| c.id.clone())
        .collect();

    for id in dead_containers {
        debug!("Cleaning up dead container: {}", id);
        state.containers.remove(&id);
    }
}

async fn update_predictor(state: &DaemonState) {
    let running: Vec<_> = state
        .containers
        .iter()
        .filter(|c| c.status == ContainerStatus::Running)
        .map(|c| (c.image.clone(), c.project_id.clone()))
        .collect();

    // Collect all events first, then record them
    let events: Vec<UsageEvent> = running
        .into_iter()
        .map(|(image, project_id)| UsageEvent {
            image,
            timestamp: Utc::now(),
            duration_seconds: 0,
            project_id,
        })
        .collect();

    // Process events - acquire lock once, record all
    let predictor = state.predictor.write();
    for event in events {
        predictor.record(event);
    }
}

/// Container lifecycle operations.
pub struct ContainerLifecycle {
    state: DaemonState,
}

impl ContainerLifecycle {
    pub fn new(state: DaemonState) -> Self {
        Self { state }
    }

    /// Create a new container.
    pub async fn create(
        &self,
        image: &str,
        name: Option<&str>,
        project_id: Option<&str>,
    ) -> anyhow::Result<ContainerState> {
        let id = uuid::Uuid::new_v4().to_string()[..12].to_string();
        let container_name = name
            .map(String::from)
            .unwrap_or_else(|| format!("hb-{}", &id[..8]));

        let container = ContainerState {
            id: id.clone(),
            name: container_name,
            image: image.to_string(),
            status: ContainerStatus::Created,
            project_id: project_id.map(String::from),
            ports: vec![],
            created_at: Utc::now(),
            started_at: None,
            pid: None,
            has_checkpoint: false,
            is_prewarmed: false,
        };

        self.state.containers.insert(id.clone(), container.clone());

        self.state.emit(
            EventType::ContainerCreate,
            &id,
            serde_json::json!({"image": image}),
        );

        let mut metrics = self.state.metrics.write();
        metrics.containers_created += 1;

        Ok(container)
    }

    /// Start a container (with warm start if checkpoint available).
    pub async fn start(&self, id: &str) -> anyhow::Result<()> {
        let mut container = self
            .state
            .containers
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Container not found"))?;

        let start_time = std::time::Instant::now();
        let image = container.image.clone();
        let project_id = container.project_id.clone();

        // Try warm start first
        if self.state.config.optimization.enable_criu {
            if let Ok(Some(_checkpoint)) = self.state.criu.get_checkpoint(&image).await {
                info!("Attempting warm start for container {} from checkpoint", id);

                // Would restore from checkpoint here
                let restore_time = start_time.elapsed();

                container.status = ContainerStatus::Running;
                container.started_at = Some(Utc::now());

                let mut metrics = self.state.metrics.write();
                metrics.warm_starts += 1;
                metrics.containers_started += 1;
                metrics.avg_warm_start_ms = (metrics.avg_warm_start_ms * (metrics.warm_starts - 1) as f64
                    + restore_time.as_millis() as f64)
                    / metrics.warm_starts as f64;

                self.state.emit(
                    EventType::ContainerStart,
                    id,
                    serde_json::json!({
                        "warm_start": true,
                        "duration_ms": restore_time.as_millis()
                    }),
                );

                return Ok(());
            }
        }

        // Cold start
        info!("Cold starting container {}", id);

        // Would create and start container here
        let start_duration = start_time.elapsed();

        container.status = ContainerStatus::Running;
        container.started_at = Some(Utc::now());

        let mut metrics = self.state.metrics.write();
        metrics.cold_starts += 1;
        metrics.containers_started += 1;
        metrics.avg_cold_start_ms = (metrics.avg_cold_start_ms * (metrics.cold_starts - 1) as f64
            + start_duration.as_millis() as f64)
            / metrics.cold_starts as f64;

        self.state.emit(
            EventType::ContainerStart,
            id,
            serde_json::json!({
                "warm_start": false,
                "duration_ms": start_duration.as_millis()
            }),
        );

        // Record usage for prediction
        drop(container); // Release the lock before write lock
        let event = UsageEvent {
            image,
            timestamp: Utc::now(),
            duration_seconds: 0,
            project_id,
        };
        self.state.predictor.write().record(event);

        Ok(())
    }

    /// Stop a container.
    pub async fn stop(&self, id: &str, create_checkpoint: bool) -> anyhow::Result<()> {
        let mut container = self
            .state
            .containers
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Container not found"))?;

        if create_checkpoint && self.state.config.optimization.enable_criu {
            info!("Creating checkpoint for container {}", id);

            // Would checkpoint container here
            container.has_checkpoint = true;

            self.state.emit(
                EventType::ContainerCheckpoint,
                id,
                serde_json::json!({}),
            );

            let mut metrics = self.state.metrics.write();
            metrics.checkpoints_created += 1;
        }

        // Would stop container here
        container.status = ContainerStatus::Stopped;

        self.state.emit(
            EventType::ContainerStop,
            id,
            serde_json::json!({}),
        );

        Ok(())
    }

    /// Remove a container.
    pub async fn remove(&self, id: &str, force: bool) -> anyhow::Result<()> {
        let container = self
            .state
            .containers
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Container not found"))?;

        if container.status == ContainerStatus::Running && !force {
            return Err(anyhow::anyhow!(
                "Container is running. Stop it first or use force"
            ));
        }

        drop(container);

        // Would cleanup container resources here
        self.state.containers.remove(id);

        self.state.emit(
            EventType::ContainerRemove,
            id,
            serde_json::json!({}),
        );

        Ok(())
    }
}
