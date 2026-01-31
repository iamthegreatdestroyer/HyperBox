//! Tauri commands - exposed to the frontend.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

// === System Commands ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub version: String,
    pub api_version: String,
    pub runtime: String,
    pub os: String,
    pub arch: String,
    pub containers_running: u32,
    pub containers_paused: u32,
    pub containers_stopped: u32,
    pub images: u32,
    pub daemon_connected: bool,
}

#[tauri::command]
pub async fn get_system_info(state: State<'_, Arc<AppState>>) -> Result<SystemInfo, String> {
    let connected = state.is_daemon_connected().await;

    if !connected {
        return Ok(SystemInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            api_version: "1.0".to_string(),
            runtime: "unknown".to_string(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            containers_running: 0,
            containers_paused: 0,
            containers_stopped: 0,
            images: 0,
            daemon_connected: false,
        });
    }

    state.get_system_info().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn check_daemon_status(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    Ok(state.is_daemon_connected().await)
}

/// Alias for check_daemon_status - returns daemon connection status
#[tauri::command]
pub async fn get_daemon_status(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    check_daemon_status(state).await
}

#[tauri::command]
pub async fn start_daemon(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.start_daemon().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_daemon(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.stop_daemon().await.map_err(|e| e.to_string())
}

// === Container Commands ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub created: String,
    pub ports: Vec<String>,
    pub project_id: Option<String>,
    pub has_checkpoint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub memory_percent: f64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub block_read: u64,
    pub block_write: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContainerRequest {
    pub image: String,
    pub name: Option<String>,
    pub command: Option<Vec<String>>,
    pub env: Option<Vec<String>>,
    pub ports: Option<Vec<PortMapping>>,
    pub volumes: Option<Vec<String>>,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host: u16,
    pub container: u16,
    pub protocol: Option<String>,
}

#[tauri::command]
pub async fn list_containers(
    state: State<'_, Arc<AppState>>,
    all: bool,
    project_id: Option<String>,
) -> Result<Vec<Container>, String> {
    state
        .list_containers(all, project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_container(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Container, String> {
    state.get_container(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_container(
    state: State<'_, Arc<AppState>>,
    request: CreateContainerRequest,
) -> Result<Container, String> {
    state
        .create_container(request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_container(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    state
        .start_container(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_container(
    state: State<'_, Arc<AppState>>,
    id: String,
    create_checkpoint: bool,
) -> Result<(), String> {
    state
        .stop_container(&id, create_checkpoint)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restart_container(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    state
        .restart_container(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_container(
    state: State<'_, Arc<AppState>>,
    id: String,
    force: bool,
) -> Result<(), String> {
    state
        .remove_container(&id, force)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_container_logs(
    state: State<'_, Arc<AppState>>,
    id: String,
    tail: Option<u32>,
    follow: bool,
) -> Result<Vec<String>, String> {
    state
        .get_container_logs(&id, tail, follow)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_container_stats(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<ContainerStats, String> {
    state
        .get_container_stats(&id)
        .await
        .map_err(|e| e.to_string())
}

// === Image Commands ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: String,
    pub tags: Vec<String>,
    pub size: u64,
    pub created: String,
    pub is_estargz: bool,
}

#[tauri::command]
pub async fn list_images(state: State<'_, Arc<AppState>>) -> Result<Vec<Image>, String> {
    state.list_images().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pull_image(
    state: State<'_, Arc<AppState>>,
    image: String,
    platform: Option<String>,
) -> Result<Image, String> {
    state
        .pull_image(&image, platform.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_image(
    state: State<'_, Arc<AppState>>,
    id: String,
    force: bool,
) -> Result<(), String> {
    state
        .remove_image(&id, force)
        .await
        .map_err(|e| e.to_string())
}

// === Project Commands ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub project_type: String,
    pub status: String,
    pub containers: Vec<String>,
    pub ports: Vec<u16>,
    pub created: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStatus {
    pub id: String,
    pub status: String,
    pub containers_running: u32,
    pub containers_stopped: u32,
    pub ports_in_use: Vec<u16>,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub disk_mb: u64,
}

#[tauri::command]
pub async fn list_projects(state: State<'_, Arc<AppState>>) -> Result<Vec<Project>, String> {
    state.list_projects().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_project(
    state: State<'_, Arc<AppState>>,
    path: String,
    name: Option<String>,
) -> Result<Project, String> {
    state
        .open_project(&path, name.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn close_project(
    state: State<'_, Arc<AppState>>,
    id: String,
    stop_containers: bool,
) -> Result<(), String> {
    state
        .close_project(&id, stop_containers)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_project(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    state.start_project(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_project(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    state.stop_project(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_project_status(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<ProjectStatus, String> {
    state
        .get_project_status(&id)
        .await
        .map_err(|e| e.to_string())
}

// === Performance Commands ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub cold_start_avg_ms: f64,
    pub warm_start_avg_ms: f64,
    pub speedup_factor: f64,
    pub lazy_load_hit_rate: f64,
    pub prewarm_hit_rate: f64,
    pub checkpoints_active: u32,
    pub containers_prewarmed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub hyperbox_cold_ms: f64,
    pub hyperbox_warm_ms: f64,
    pub docker_cold_ms: Option<f64>,
    pub docker_warm_ms: Option<f64>,
    pub speedup_cold: Option<f64>,
    pub speedup_warm: Option<f64>,
}

#[tauri::command]
pub async fn get_performance_metrics(
    state: State<'_, Arc<AppState>>,
) -> Result<PerformanceMetrics, String> {
    state
        .get_performance_metrics()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_benchmark(
    state: State<'_, Arc<AppState>>,
    image: String,
    compare_docker: bool,
) -> Result<BenchmarkResult, String> {
    state
        .run_benchmark(&image, compare_docker)
        .await
        .map_err(|e| e.to_string())
}

// === Settings Commands ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    pub auto_start_daemon: bool,
    pub enable_notifications: bool,
    pub enable_criu: bool,
    pub enable_lazy_loading: bool,
    pub enable_prewarm: bool,
    pub max_prewarmed: u32,
    pub prewarm_threshold: f64,
    pub default_runtime: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            auto_start_daemon: true,
            enable_notifications: true,
            enable_criu: true,
            enable_lazy_loading: true,
            enable_prewarm: true,
            max_prewarmed: 10,
            prewarm_threshold: 0.7,
            default_runtime: "crun".to_string(),
        }
    }
}

#[tauri::command]
pub async fn get_settings(state: State<'_, Arc<AppState>>) -> Result<Settings, String> {
    state.get_settings().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, Arc<AppState>>,
    settings: Settings,
) -> Result<(), String> {
    state
        .update_settings(settings)
        .await
        .map_err(|e| e.to_string())
}

/// Reset settings to defaults
#[tauri::command]
pub async fn reset_settings(state: State<'_, Arc<AppState>>) -> Result<Settings, String> {
    let defaults = Settings::default();
    state
        .update_settings(defaults.clone())
        .await
        .map_err(|e| e.to_string())?;
    Ok(defaults)
}
