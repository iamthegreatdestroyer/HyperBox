//! HTTP/REST API server.

use crate::state::{ContainerState, DaemonState, EventType};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

/// Serve the REST API.
pub async fn serve(state: DaemonState, socket_path: PathBuf) -> anyhow::Result<()> {
    let app = create_router(state);

    // For now, use TCP. Unix socket support would go here.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;

    info!("REST API listening on http://127.0.0.1:8080");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Create the API router.
pub fn create_router(state: DaemonState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // System
        .route("/api/v1/info", get(system_info))
        .route("/api/v1/version", get(version))
        .route("/api/v1/ping", get(ping))
        .route("/api/v1/events", get(events))
        // Containers
        .route("/api/v1/containers", get(list_containers))
        .route("/api/v1/containers", post(create_container))
        .route("/api/v1/containers/:id", get(get_container))
        .route("/api/v1/containers/:id", delete(remove_container))
        .route("/api/v1/containers/:id/start", post(start_container))
        .route("/api/v1/containers/:id/stop", post(stop_container))
        .route("/api/v1/containers/:id/restart", post(restart_container))
        .route("/api/v1/containers/:id/checkpoint", post(checkpoint_container))
        .route("/api/v1/containers/:id/restore", post(restore_container))
        .route("/api/v1/containers/:id/logs", get(container_logs))
        .route("/api/v1/containers/:id/stats", get(container_stats))
        // Images
        .route("/api/v1/images", get(list_images))
        .route("/api/v1/images/pull", post(pull_image))
        .route("/api/v1/images/:id", get(get_image))
        .route("/api/v1/images/:id", delete(remove_image))
        // Projects
        .route("/api/v1/projects", get(list_projects))
        .route("/api/v1/projects", post(open_project))
        .route("/api/v1/projects/:id", get(get_project))
        .route("/api/v1/projects/:id/start", post(start_project))
        .route("/api/v1/projects/:id/stop", post(stop_project))
        .route("/api/v1/projects/:id/close", post(close_project))
        // Metrics
        .route("/api/v1/metrics", get(metrics))
        .route("/api/v1/metrics/performance", get(performance_metrics))
        // Health
        .route("/health", get(health))
        .route("/ready", get(ready))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

// === Request/Response Types ===

#[derive(Serialize)]
struct SystemInfo {
    version: String,
    api_version: String,
    runtime: String,
    os: String,
    arch: String,
    containers_running: usize,
    containers_paused: usize,
    containers_stopped: usize,
    images: usize,
}

#[derive(Serialize)]
struct VersionInfo {
    version: String,
    api_version: String,
    git_commit: String,
    built: String,
}

#[derive(Deserialize)]
struct CreateContainerRequest {
    image: String,
    name: Option<String>,
    command: Option<Vec<String>>,
    env: Option<Vec<String>>,
    ports: Option<Vec<PortMappingRequest>>,
    volumes: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct PortMappingRequest {
    host: u16,
    container: u16,
    protocol: Option<String>,
}

#[derive(Deserialize)]
struct PullImageRequest {
    image: String,
    platform: Option<String>,
}

#[derive(Deserialize)]
struct OpenProjectRequest {
    path: String,
    name: Option<String>,
}

#[derive(Deserialize)]
struct ListQuery {
    all: Option<bool>,
    project: Option<String>,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: &str) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message.to_string()),
        }
    }
}

// === System Handlers ===

async fn system_info(State(state): State<DaemonState>) -> Json<SystemInfo> {
    let running = state
        .containers
        .iter()
        .filter(|c| c.status == crate::state::ContainerStatus::Running)
        .count();
    let paused = state
        .containers
        .iter()
        .filter(|c| c.status == crate::state::ContainerStatus::Paused)
        .count();
    let stopped = state
        .containers
        .iter()
        .filter(|c| c.status == crate::state::ContainerStatus::Stopped)
        .count();

    Json(SystemInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: "1.0".to_string(),
        runtime: state.config.runtime.default_runtime.clone(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        containers_running: running,
        containers_paused: paused,
        containers_stopped: stopped,
        images: state.images.len(),
    })
}

async fn version() -> Json<VersionInfo> {
    Json(VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: "1.0".to_string(),
        git_commit: "dev".to_string(),
        built: "2024-01-15".to_string(),
    })
}

async fn ping() -> &'static str {
    "OK"
}

async fn events(State(state): State<DaemonState>) -> impl IntoResponse {
    // Would implement SSE here for real-time events
    Json(ApiResponse::success("Events endpoint - would stream SSE"))
}

// === Container Handlers ===

async fn list_containers(
    State(state): State<DaemonState>,
    Query(query): Query<ListQuery>,
) -> Json<ApiResponse<Vec<ContainerState>>> {
    let containers: Vec<_> = state
        .containers
        .iter()
        .filter(|c| {
            if let Some(ref project_id) = query.project {
                c.project_id.as_ref() == Some(project_id)
            } else {
                true
            }
        })
        .filter(|c| {
            query.all.unwrap_or(false) || c.status == crate::state::ContainerStatus::Running
        })
        .map(|c| c.clone())
        .collect();

    Json(ApiResponse::success(containers))
}

async fn create_container(
    State(state): State<DaemonState>,
    Json(req): Json<CreateContainerRequest>,
) -> impl IntoResponse {
    // Would create container here
    let id = uuid::Uuid::new_v4().to_string()[..12].to_string();

    state.emit(EventType::ContainerCreate, &id, serde_json::json!({"image": req.image}));

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(serde_json::json!({
            "id": id,
            "status": "created"
        }))),
    )
}

async fn get_container(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.get_container(&id) {
        Some(container) => (StatusCode::OK, Json(ApiResponse::success(container))),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::success(ContainerState {
                id: String::new(),
                name: String::new(),
                image: String::new(),
                status: crate::state::ContainerStatus::Dead,
                project_id: None,
                ports: vec![],
                created_at: chrono::Utc::now(),
                started_at: None,
                pid: None,
                has_checkpoint: false,
                is_prewarmed: false,
            })),
        ),
    }
}

async fn start_container(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.emit(EventType::ContainerStart, &id, serde_json::json!({}));
    Json(ApiResponse::success(serde_json::json!({"started": true})))
}

async fn stop_container(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.emit(EventType::ContainerStop, &id, serde_json::json!({}));
    Json(ApiResponse::success(serde_json::json!({"stopped": true})))
}

async fn restart_container(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    Json(ApiResponse::success(serde_json::json!({"restarted": true})))
}

async fn remove_container(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.emit(EventType::ContainerRemove, &id, serde_json::json!({}));
    Json(ApiResponse::success(serde_json::json!({"removed": true})))
}

async fn checkpoint_container(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.emit(EventType::ContainerCheckpoint, &id, serde_json::json!({}));
    Json(ApiResponse::success(serde_json::json!({"checkpointed": true})))
}

async fn restore_container(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.emit(EventType::ContainerRestore, &id, serde_json::json!({}));
    Json(ApiResponse::success(serde_json::json!({"restored": true})))
}

async fn container_logs(Path(id): Path<String>) -> impl IntoResponse {
    Json(ApiResponse::success(vec!["Container started", "Listening on port 3000"]))
}

async fn container_stats(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    Json(ApiResponse::success(serde_json::json!({
        "cpu_percent": 0.5,
        "memory_usage": 128 * 1024 * 1024,
        "memory_limit": 2 * 1024 * 1024 * 1024,
        "network_rx": 1024 * 1024,
        "network_tx": 512 * 1024,
        "block_read": 10 * 1024 * 1024,
        "block_write": 5 * 1024 * 1024
    })))
}

// === Image Handlers ===

async fn list_images(State(state): State<DaemonState>) -> impl IntoResponse {
    let images: Vec<_> = state.images.iter().map(|i| i.clone()).collect();
    Json(ApiResponse::success(images))
}

async fn pull_image(
    State(state): State<DaemonState>,
    Json(req): Json<PullImageRequest>,
) -> impl IntoResponse {
    state.emit(EventType::ImagePull, &req.image, serde_json::json!({"platform": req.platform}));
    Json(ApiResponse::success(serde_json::json!({
        "image": req.image,
        "status": "pulled"
    })))
}

async fn get_image(State(state): State<DaemonState>, Path(id): Path<String>) -> impl IntoResponse {
    match state.images.get(&id) {
        Some(image) => Json(ApiResponse::success(image.clone())),
        None => Json(ApiResponse::success(crate::state::ImageState {
            id: String::new(),
            tags: vec![],
            size: 0,
            created_at: chrono::Utc::now(),
            is_estargz: false,
            layers: vec![],
        })),
    }
}

async fn remove_image(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.emit(EventType::ImageRemove, &id, serde_json::json!({}));
    Json(ApiResponse::success(serde_json::json!({"removed": true})))
}

// === Project Handlers ===

async fn list_projects(State(state): State<DaemonState>) -> impl IntoResponse {
    let projects: Vec<_> = state.projects.list().into_iter().collect();
    Json(ApiResponse::success(projects))
}

async fn open_project(
    State(state): State<DaemonState>,
    Json(req): Json<OpenProjectRequest>,
) -> impl IntoResponse {
    state.emit(EventType::ProjectOpen, &req.path, serde_json::json!({"name": req.name}));
    Json(ApiResponse::success(serde_json::json!({
        "path": req.path,
        "status": "opened"
    })))
}

async fn get_project(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let project_id = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Json(ApiResponse::<serde_json::Value> {
                success: false,
                data: None,
                error: Some("Invalid project ID".to_string()),
            });
        }
    };
    match state.projects.get(project_id) {
        Some(project) => Json(ApiResponse {
            success: true,
            data: Some(serde_json::to_value(project).unwrap()),
            error: None,
        }),
        None => Json(ApiResponse {
            success: true,
            data: Some(serde_json::json!(null)),
            error: None,
        }),
    }
}

async fn start_project(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.emit(EventType::ProjectStart, &id, serde_json::json!({}));
    Json(ApiResponse::success(serde_json::json!({"started": true})))
}

async fn stop_project(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.emit(EventType::ProjectStop, &id, serde_json::json!({}));
    Json(ApiResponse::success(serde_json::json!({"stopped": true})))
}

async fn close_project(
    State(state): State<DaemonState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.emit(EventType::ProjectClose, &id, serde_json::json!({}));
    Json(ApiResponse::success(serde_json::json!({"closed": true})))
}

// === Metrics Handlers ===

async fn metrics(State(state): State<DaemonState>) -> impl IntoResponse {
    let metrics = state.metrics.read().clone();
    Json(ApiResponse::success(metrics))
}

async fn performance_metrics(State(state): State<DaemonState>) -> impl IntoResponse {
    let metrics = state.metrics.read();
    Json(ApiResponse::success(serde_json::json!({
        "cold_start_avg_ms": metrics.avg_cold_start_ms,
        "warm_start_avg_ms": metrics.avg_warm_start_ms,
        "lazy_load_hit_rate": if metrics.lazy_load_hits + metrics.lazy_load_misses > 0 {
            metrics.lazy_load_hits as f64 / (metrics.lazy_load_hits + metrics.lazy_load_misses) as f64
        } else { 0.0 },
        "prewarm_hit_rate": if metrics.prewarm_hits + metrics.prewarm_misses > 0 {
            metrics.prewarm_hits as f64 / (metrics.prewarm_hits + metrics.prewarm_misses) as f64
        } else { 0.0 },
        "speedup_factor": if metrics.avg_warm_start_ms > 0.0 {
            metrics.avg_cold_start_ms / metrics.avg_warm_start_ms
        } else { 0.0 }
    })))
}

// === Health Handlers ===

async fn health(State(state): State<DaemonState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "uptime_seconds": state.uptime().num_seconds()
    }))
}

async fn ready() -> &'static str {
    "OK"
}
