//! HyperBox Project - Project-Centric Container Isolation
//!
//! This crate provides project-aware container management, enabling:
//! - Automatic project detection from directory structure
//! - Per-project container isolation and networking
//! - Shared resource management across project containers
//! - Hot-reload and development workflows
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    ProjectManager                           │
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │ Project Detection & Configuration                       ││
//! │  │ - Dockerfile, docker-compose.yml detection              ││
//! │  │ - Language/framework inference                          ││
//! │  │ - Dependency analysis                                   ││
//! │  └─────────────────────────────────────────────────────────┘│
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │ Container Orchestration                                  ││
//! │  │ - Automatic port allocation                              ││
//! │  │ - Network isolation per project                          ││
//! │  │ - Volume management                                      ││
//! │  └─────────────────────────────────────────────────────────┘│
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │ Development Workflows                                    ││
//! │  │ - Hot-reload support                                     ││
//! │  │ - Build caching                                          ││
//! │  │ - Test environment management                            ││
//! │  └─────────────────────────────────────────────────────────┘│
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod config;
pub mod detection;
pub mod error;
pub mod manager;
pub mod ports;
pub mod resources;
#[cfg(feature = "watch")]
pub mod watcher;

pub use config::ProjectConfig;
pub use detection::ProjectDetector;
pub use error::{ProjectError, Result};
pub use manager::ProjectManager;
pub use ports::ProjectPortManager;
pub use resources::ResourcePool;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Project identifier.
pub type ProjectId = Uuid;

/// Project state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectState {
    /// Project is detected but not started
    Inactive,
    /// Project containers are starting
    Starting,
    /// Project is running
    Running,
    /// Project is stopping
    Stopping,
    /// Project has stopped
    Stopped,
    /// Project is in error state
    Error,
}

impl Default for ProjectState {
    fn default() -> Self {
        Self::Inactive
    }
}

/// Project information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project ID
    pub id: ProjectId,
    /// Project name (usually directory name)
    pub name: String,
    /// Project root directory
    pub root: PathBuf,
    /// Current state
    pub state: ProjectState,
    /// Project configuration
    pub config: ProjectConfig,
    /// Container IDs associated with this project
    pub containers: Vec<hyperbox_core::types::ContainerId>,
    /// Allocated ports
    pub ports: Vec<u16>,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

impl Project {
    /// Create a new project.
    pub fn new(name: impl Into<String>, root: PathBuf) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            root,
            state: ProjectState::Inactive,
            config: ProjectConfig::default(),
            containers: Vec::new(),
            ports: Vec::new(),
            created_at: now,
            last_activity: now,
        }
    }

    /// Check if project is active (running or starting).
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self.state, ProjectState::Running | ProjectState::Starting)
    }

    /// Update last activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = chrono::Utc::now();
    }
}
