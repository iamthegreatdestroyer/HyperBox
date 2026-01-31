//! Daemon state management.

use crate::config::DaemonConfig;
use crate::error::{DaemonError, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use hyperbox_optimize::criu::CriuManager;
use hyperbox_optimize::lazy_load::LazyLayerLoader;
use hyperbox_optimize::predict::UsagePredictor;
use hyperbox_optimize::prewarm::PrewarmManager;
use hyperbox_project::manager::ProjectManager;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Shared daemon state.
#[derive(Clone)]
pub struct DaemonState {
    /// Configuration
    pub config: DaemonConfig,

    /// Active containers
    pub containers: Arc<DashMap<String, ContainerState>>,

    /// Active projects
    pub projects: Arc<ProjectManager>,

    /// Container images
    pub images: Arc<DashMap<String, ImageState>>,

    /// CRIU manager for checkpointing
    pub criu: Arc<CriuManager>,

    /// Lazy layer loader
    pub lazy_loader: Arc<LazyLayerLoader>,

    /// Pre-warm manager
    pub prewarm: Arc<PrewarmManager>,

    /// Usage predictor
    pub predictor: Arc<RwLock<UsagePredictor>>,

    /// Event broadcaster
    pub events: broadcast::Sender<DaemonEvent>,

    /// Daemon metrics
    pub metrics: Arc<RwLock<DaemonMetrics>>,

    /// Start time
    pub started_at: DateTime<Utc>,
}

/// Container state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerState {
    /// Container ID
    pub id: String,

    /// Container name
    pub name: String,

    /// Image name
    pub image: String,

    /// Container status
    pub status: ContainerStatus,

    /// Associated project ID
    pub project_id: Option<String>,

    /// Port mappings
    pub ports: Vec<PortMapping>,

    /// Created at
    pub created_at: DateTime<Utc>,

    /// Started at
    pub started_at: Option<DateTime<Utc>>,

    /// Process ID
    pub pid: Option<u32>,

    /// Has checkpoint
    pub has_checkpoint: bool,

    /// Is pre-warmed
    pub is_prewarmed: bool,
}

/// Container status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerStatus {
    Created,
    Running,
    Paused,
    Stopped,
    Removing,
    Dead,
}

/// Port mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Host port
    pub host_port: u16,

    /// Container port
    pub container_port: u16,

    /// Protocol (tcp/udp)
    pub protocol: String,
}

/// Image state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageState {
    /// Image ID
    pub id: String,

    /// Repository tags
    pub tags: Vec<String>,

    /// Size in bytes
    pub size: u64,

    /// Created at
    pub created_at: DateTime<Utc>,

    /// Is eStargz format
    pub is_estargz: bool,

    /// Layer digests
    pub layers: Vec<String>,
}

/// Daemon event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonEvent {
    /// Event ID
    pub id: String,

    /// Event type
    pub event_type: EventType,

    /// Event target (container/image/project ID)
    pub target: String,

    /// Event data
    pub data: serde_json::Value,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    ContainerCreate,
    ContainerStart,
    ContainerStop,
    ContainerRemove,
    ContainerCheckpoint,
    ContainerRestore,
    ImagePull,
    ImageRemove,
    ProjectOpen,
    ProjectStart,
    ProjectStop,
    ProjectClose,
    DaemonStart,
    DaemonStop,
    HealthCheck,
}

/// Daemon metrics.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DaemonMetrics {
    /// Total containers created
    pub containers_created: u64,

    /// Total containers started
    pub containers_started: u64,

    /// Total cold starts
    pub cold_starts: u64,

    /// Total warm starts (CRIU)
    pub warm_starts: u64,

    /// Average cold start time in ms
    pub avg_cold_start_ms: f64,

    /// Average warm start time in ms
    pub avg_warm_start_ms: f64,

    /// Total images pulled
    pub images_pulled: u64,

    /// Total lazy load hits
    pub lazy_load_hits: u64,

    /// Total lazy load misses
    pub lazy_load_misses: u64,

    /// Total pre-warm hits
    pub prewarm_hits: u64,

    /// Total pre-warm misses
    pub prewarm_misses: u64,

    /// Total checkpoints created
    pub checkpoints_created: u64,

    /// Total restores
    pub restores: u64,
}

impl DaemonState {
    /// Create new daemon state.
    pub async fn new(config: DaemonConfig) -> Result<Self> {
        // Ensure directories exist
        config
            .ensure_directories()
            .map_err(|e| DaemonError::Internal(e.to_string()))?;

        // Create event channel
        let (events, _) = broadcast::channel(1024);

        // Initialize CRIU manager (not async)
        let criu = CriuManager::new(config.optimization.checkpoints_dir.clone());

        // Initialize lazy loader
        let lazy_loader = LazyLayerLoader::new(
            config.storage.images_dir.join("layers"),
            "https://registry-1.docker.io",
        );

        // Initialize predictor - create Arc first so we can share it
        let predictor = Arc::new(RwLock::new(UsagePredictor::new(config.data_dir.join("models"))));

        // Initialize pre-warm manager with a dummy predictor (shares via state)
        let prewarm_predictor = Arc::new(UsagePredictor::new(config.data_dir.join("models")));
        let prewarm = PrewarmManager::with_config(
            prewarm_predictor,
            config.data_dir.join("prewarm"),
            hyperbox_optimize::prewarm::PrewarmConfig {
                max_prewarmed: config.optimization.max_prewarmed,
                threshold: config.optimization.prewarm_threshold,
                lookahead_seconds: config.optimization.prewarm_lookahead_seconds,
                cleanup_interval_seconds: 60,
                prewarm_ttl_seconds: 600,
            },
        );

        Ok(Self {
            config: config.clone(),
            containers: Arc::new(DashMap::new()),
            projects: Arc::new(ProjectManager::new(config.data_dir.join("projects"))),
            images: Arc::new(DashMap::new()),
            criu: Arc::new(criu),
            lazy_loader: Arc::new(lazy_loader),
            prewarm: Arc::new(prewarm),
            predictor,
            events,
            metrics: Arc::new(RwLock::new(DaemonMetrics::default())),
            started_at: Utc::now(),
        })
    }

    /// Emit an event.
    pub fn emit(&self, event_type: EventType, target: &str, data: serde_json::Value) {
        let event = DaemonEvent {
            id: Uuid::new_v4().to_string(),
            event_type,
            target: target.to_string(),
            data,
            timestamp: Utc::now(),
        };

        let _ = self.events.send(event);
    }

    /// Save state to disk.
    pub async fn save(&self) -> Result<()> {
        // Save predictor models
        self.predictor
            .read()
            .save()
            .await
            .map_err(|e| DaemonError::State(e.to_string()))?;

        // Would save other state here

        Ok(())
    }

    /// Get uptime.
    pub fn uptime(&self) -> chrono::Duration {
        Utc::now() - self.started_at
    }

    /// Get container by ID.
    pub fn get_container(&self, id: &str) -> Option<ContainerState> {
        self.containers.get(id).map(|c| c.clone())
    }

    /// Get all containers.
    pub fn get_containers(&self) -> Vec<ContainerState> {
        self.containers.iter().map(|c| c.clone()).collect()
    }

    /// Get running containers.
    pub fn get_running_containers(&self) -> Vec<ContainerState> {
        self.containers
            .iter()
            .filter(|c| c.status == ContainerStatus::Running)
            .map(|c| c.clone())
            .collect()
    }
}

impl std::fmt::Display for ContainerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerStatus::Created => write!(f, "created"),
            ContainerStatus::Running => write!(f, "running"),
            ContainerStatus::Paused => write!(f, "paused"),
            ContainerStatus::Stopped => write!(f, "stopped"),
            ContainerStatus::Removing => write!(f, "removing"),
            ContainerStatus::Dead => write!(f, "dead"),
        }
    }
}
