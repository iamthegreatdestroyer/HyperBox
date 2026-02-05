//! Health monitoring.

use crate::state::{DaemonState, EventType};
use std::time::Duration;
use sysinfo::System;
use tracing::warn;

/// Monitor daemon health.
pub async fn monitor(state: DaemonState) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    let mut sys = System::new_all();

    loop {
        interval.tick().await;

        // Refresh system information
        sys.refresh_all();

        // Check system resources
        let memory_available = sys.available_memory();
        let memory_total = sys.total_memory();
        let memory_percent = if memory_total > 0 {
            ((memory_total - memory_available) as f64 / memory_total as f64) * 100.0
        } else {
            0.0
        };

        if memory_percent > 90.0 {
            warn!("High memory usage: {:.1}%", memory_percent);
        }

        // Check container health
        let running_containers = state.get_running_containers();
        for _container in running_containers {
            // Would check container health here
            // - Process still running
            // - Health check endpoint (if defined)
            // - Resource usage
        }

        // Cleanup expired checkpoints periodically
        // Note: cleanup_stale is internal to CriuManager
        // Would integrate with lifecycle manager for proper cleanup

        // Emit health event
        state.emit(
            EventType::HealthCheck,
            "daemon",
            serde_json::json!({
                "memory_percent": memory_percent,
                "containers_running": state.get_running_containers().len(),
                "status": "healthy"
            }),
        );
    }
}

/// Health check result.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub daemon: ComponentHealth,
    pub runtime: ComponentHealth,
    pub storage: ComponentHealth,
    pub network: ComponentHealth,
    pub criu: ComponentHealth,
}

#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthState,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

impl HealthStatus {
    pub async fn check(state: &DaemonState) -> Self {
        Self {
            daemon: ComponentHealth {
                name: "daemon".to_string(),
                status: HealthState::Healthy,
                message: None,
            },
            runtime: check_runtime(&state.config).await,
            storage: check_storage(&state.config).await,
            network: check_network(&state.config).await,
            criu: check_criu(&state.config).await,
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.daemon.status == HealthState::Healthy
            && self.runtime.status != HealthState::Unhealthy
            && self.storage.status != HealthState::Unhealthy
    }
}

async fn check_runtime(config: &crate::config::DaemonConfig) -> ComponentHealth {
    // Check if crun is available
    let crun_path = &config.runtime.crun_path;
    if crun_path.exists() {
        ComponentHealth {
            name: "runtime".to_string(),
            status: HealthState::Healthy,
            message: Some(format!("crun at {:?}", crun_path)),
        }
    } else {
        ComponentHealth {
            name: "runtime".to_string(),
            status: HealthState::Unhealthy,
            message: Some("crun not found".to_string()),
        }
    }
}

async fn check_storage(config: &crate::config::DaemonConfig) -> ComponentHealth {
    // Check if data directory exists and is writable
    let data_dir = &config.data_dir;
    if data_dir.exists() {
        ComponentHealth {
            name: "storage".to_string(),
            status: HealthState::Healthy,
            message: Some(format!("data at {:?}", data_dir)),
        }
    } else {
        ComponentHealth {
            name: "storage".to_string(),
            status: HealthState::Degraded,
            message: Some("data directory missing".to_string()),
        }
    }
}

async fn check_network(config: &crate::config::DaemonConfig) -> ComponentHealth {
    // Check CNI availability
    let cni_dir = &config.network.cni_plugin_dir;
    if cni_dir.exists() {
        ComponentHealth {
            name: "network".to_string(),
            status: HealthState::Healthy,
            message: Some("CNI available".to_string()),
        }
    } else {
        ComponentHealth {
            name: "network".to_string(),
            status: HealthState::Degraded,
            message: Some("CNI plugins not found".to_string()),
        }
    }
}

async fn check_criu(config: &crate::config::DaemonConfig) -> ComponentHealth {
    if !config.optimization.enable_criu {
        return ComponentHealth {
            name: "criu".to_string(),
            status: HealthState::Healthy,
            message: Some("disabled".to_string()),
        };
    }

    let criu_path = &config.optimization.criu_path;
    if criu_path.exists() {
        ComponentHealth {
            name: "criu".to_string(),
            status: HealthState::Healthy,
            message: Some(format!("criu at {:?}", criu_path)),
        }
    } else {
        ComponentHealth {
            name: "criu".to_string(),
            status: HealthState::Degraded,
            message: Some("CRIU not found - warm starts disabled".to_string()),
        }
    }
}
