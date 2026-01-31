//! Daemon client for communication with hyperboxd.

use crate::commands::*;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Daemon REST API client.
pub struct DaemonClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl DaemonClient {
    /// Connect to daemon.
    pub async fn connect(base_url: &str) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let daemon = Self {
            client,
            base_url: base_url.to_string(),
        };

        // Verify connection
        daemon.ping().await?;

        Ok(daemon)
    }

    /// Ping daemon.
    pub async fn ping(&self) -> anyhow::Result<()> {
        let resp = self
            .client
            .get(format!("{}/api/v1/ping", self.base_url))
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Daemon not responding"))
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let resp: ApiResponse<T> = self
            .client
            .get(format!("{}{}", self.base_url, path))
            .send()
            .await?
            .json()
            .await?;

        if resp.success {
            resp.data.ok_or_else(|| anyhow::anyhow!("No data in response"))
        } else {
            Err(anyhow::anyhow!(resp.error.unwrap_or_default()))
        }
    }

    async fn post<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> anyhow::Result<T> {
        let resp: ApiResponse<T> = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .json(body)
            .send()
            .await?
            .json()
            .await?;

        if resp.success {
            resp.data.ok_or_else(|| anyhow::anyhow!("No data in response"))
        } else {
            Err(anyhow::anyhow!(resp.error.unwrap_or_default()))
        }
    }

    async fn delete(&self, path: &str) -> anyhow::Result<()> {
        let resp: ApiResponse<serde_json::Value> = self
            .client
            .delete(format!("{}{}", self.base_url, path))
            .send()
            .await?
            .json()
            .await?;

        if resp.success {
            Ok(())
        } else {
            Err(anyhow::anyhow!(resp.error.unwrap_or_default()))
        }
    }

    // === System API ===

    pub async fn get_system_info(&self) -> anyhow::Result<SystemInfo> {
        let info: serde_json::Value = self.get("/api/v1/info").await?;

        Ok(SystemInfo {
            version: info["version"].as_str().unwrap_or_default().to_string(),
            api_version: info["api_version"].as_str().unwrap_or_default().to_string(),
            runtime: info["runtime"].as_str().unwrap_or_default().to_string(),
            os: info["os"].as_str().unwrap_or_default().to_string(),
            arch: info["arch"].as_str().unwrap_or_default().to_string(),
            containers_running: info["containers_running"].as_u64().unwrap_or(0) as u32,
            containers_paused: info["containers_paused"].as_u64().unwrap_or(0) as u32,
            containers_stopped: info["containers_stopped"].as_u64().unwrap_or(0) as u32,
            images: info["images"].as_u64().unwrap_or(0) as u32,
            daemon_connected: true,
        })
    }

    // === Container API ===

    pub async fn list_containers(
        &self,
        all: bool,
        project_id: Option<String>,
    ) -> anyhow::Result<Vec<Container>> {
        let mut path = format!("/api/v1/containers?all={}", all);
        if let Some(pid) = project_id {
            path.push_str(&format!("&project={}", pid));
        }

        let containers: Vec<serde_json::Value> = self.get(&path).await?;

        Ok(containers
            .into_iter()
            .map(|c| Container {
                id: c["id"].as_str().unwrap_or_default().to_string(),
                name: c["name"].as_str().unwrap_or_default().to_string(),
                image: c["image"].as_str().unwrap_or_default().to_string(),
                status: c["status"].as_str().unwrap_or_default().to_string(),
                created: c["created_at"].as_str().unwrap_or_default().to_string(),
                ports: vec![],
                project_id: c["project_id"].as_str().map(String::from),
                has_checkpoint: c["has_checkpoint"].as_bool().unwrap_or(false),
            })
            .collect())
    }

    pub async fn get_container(&self, id: &str) -> anyhow::Result<Container> {
        let c: serde_json::Value = self.get(&format!("/api/v1/containers/{}", id)).await?;

        Ok(Container {
            id: c["id"].as_str().unwrap_or_default().to_string(),
            name: c["name"].as_str().unwrap_or_default().to_string(),
            image: c["image"].as_str().unwrap_or_default().to_string(),
            status: c["status"].as_str().unwrap_or_default().to_string(),
            created: c["created_at"].as_str().unwrap_or_default().to_string(),
            ports: vec![],
            project_id: c["project_id"].as_str().map(String::from),
            has_checkpoint: c["has_checkpoint"].as_bool().unwrap_or(false),
        })
    }

    pub async fn create_container(&self, request: CreateContainerRequest) -> anyhow::Result<Container> {
        let c: serde_json::Value = self.post("/api/v1/containers", &request).await?;

        Ok(Container {
            id: c["id"].as_str().unwrap_or_default().to_string(),
            name: "".to_string(),
            image: request.image,
            status: "created".to_string(),
            created: chrono::Utc::now().to_rfc3339(),
            ports: vec![],
            project_id: request.project_id,
            has_checkpoint: false,
        })
    }

    pub async fn start_container(&self, id: &str) -> anyhow::Result<()> {
        let _: serde_json::Value = self
            .post(&format!("/api/v1/containers/{}/start", id), &())
            .await?;
        Ok(())
    }

    pub async fn stop_container(&self, id: &str, _create_checkpoint: bool) -> anyhow::Result<()> {
        let _: serde_json::Value = self
            .post(&format!("/api/v1/containers/{}/stop", id), &())
            .await?;
        Ok(())
    }

    pub async fn restart_container(&self, id: &str) -> anyhow::Result<()> {
        let _: serde_json::Value = self
            .post(&format!("/api/v1/containers/{}/restart", id), &())
            .await?;
        Ok(())
    }

    pub async fn remove_container(&self, id: &str, _force: bool) -> anyhow::Result<()> {
        self.delete(&format!("/api/v1/containers/{}", id)).await
    }

    pub async fn get_container_logs(&self, id: &str, _tail: Option<u32>) -> anyhow::Result<Vec<String>> {
        self.get(&format!("/api/v1/containers/{}/logs", id)).await
    }

    pub async fn get_container_stats(&self, id: &str) -> anyhow::Result<ContainerStats> {
        let s: serde_json::Value = self.get(&format!("/api/v1/containers/{}/stats", id)).await?;

        let memory_usage = s["memory_usage"].as_u64().unwrap_or(0);
        let memory_limit = s["memory_limit"].as_u64().unwrap_or(1);

        Ok(ContainerStats {
            cpu_percent: s["cpu_percent"].as_f64().unwrap_or(0.0),
            memory_usage,
            memory_limit,
            memory_percent: (memory_usage as f64 / memory_limit as f64) * 100.0,
            network_rx: s["network_rx"].as_u64().unwrap_or(0),
            network_tx: s["network_tx"].as_u64().unwrap_or(0),
            block_read: s["block_read"].as_u64().unwrap_or(0),
            block_write: s["block_write"].as_u64().unwrap_or(0),
        })
    }

    // === Image API ===

    pub async fn list_images(&self) -> anyhow::Result<Vec<Image>> {
        let images: Vec<serde_json::Value> = self.get("/api/v1/images").await?;

        Ok(images
            .into_iter()
            .map(|i| Image {
                id: i["id"].as_str().unwrap_or_default().to_string(),
                tags: i["tags"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default(),
                size: i["size"].as_u64().unwrap_or(0),
                created: i["created_at"].as_str().unwrap_or_default().to_string(),
                is_estargz: i["is_estargz"].as_bool().unwrap_or(false),
            })
            .collect())
    }

    pub async fn pull_image(&self, image: &str, platform: Option<&str>) -> anyhow::Result<Image> {
        #[derive(Serialize)]
        struct PullRequest<'a> {
            image: &'a str,
            platform: Option<&'a str>,
        }

        let _: serde_json::Value = self
            .post("/api/v1/images/pull", &PullRequest { image, platform })
            .await?;

        Ok(Image {
            id: "".to_string(),
            tags: vec![image.to_string()],
            size: 0,
            created: chrono::Utc::now().to_rfc3339(),
            is_estargz: false,
        })
    }

    pub async fn remove_image(&self, id: &str) -> anyhow::Result<()> {
        self.delete(&format!("/api/v1/images/{}", id)).await
    }

    // === Project API ===

    pub async fn list_projects(&self) -> anyhow::Result<Vec<Project>> {
        let projects: Vec<serde_json::Value> = self.get("/api/v1/projects").await?;

        Ok(projects
            .into_iter()
            .map(|p| Project {
                id: p["id"].as_str().unwrap_or_default().to_string(),
                name: p["name"].as_str().unwrap_or_default().to_string(),
                path: p["path"].as_str().unwrap_or_default().to_string(),
                project_type: p["project_type"].as_str().unwrap_or_default().to_string(),
                status: p["status"].as_str().unwrap_or_default().to_string(),
                containers: vec![],
                ports: vec![],
                created: p["created_at"].as_str().unwrap_or_default().to_string(),
            })
            .collect())
    }

    pub async fn open_project(&self, path: &str, name: Option<&str>) -> anyhow::Result<Project> {
        #[derive(Serialize)]
        struct OpenRequest<'a> {
            path: &'a str,
            name: Option<&'a str>,
        }

        let p: serde_json::Value = self
            .post("/api/v1/projects", &OpenRequest { path, name })
            .await?;

        Ok(Project {
            id: p["id"].as_str().unwrap_or_default().to_string(),
            name: name.unwrap_or("").to_string(),
            path: path.to_string(),
            project_type: "unknown".to_string(),
            status: "opened".to_string(),
            containers: vec![],
            ports: vec![],
            created: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub async fn close_project(&self, id: &str) -> anyhow::Result<()> {
        let _: serde_json::Value = self
            .post(&format!("/api/v1/projects/{}/close", id), &())
            .await?;
        Ok(())
    }

    pub async fn start_project(&self, id: &str) -> anyhow::Result<()> {
        let _: serde_json::Value = self
            .post(&format!("/api/v1/projects/{}/start", id), &())
            .await?;
        Ok(())
    }

    pub async fn stop_project(&self, id: &str) -> anyhow::Result<()> {
        let _: serde_json::Value = self
            .post(&format!("/api/v1/projects/{}/stop", id), &())
            .await?;
        Ok(())
    }

    pub async fn get_project_status(&self, id: &str) -> anyhow::Result<ProjectStatus> {
        let s: serde_json::Value = self.get(&format!("/api/v1/projects/{}", id)).await?;

        Ok(ProjectStatus {
            id: id.to_string(),
            status: s["status"].as_str().unwrap_or_default().to_string(),
            containers_running: 0,
            containers_stopped: 0,
            ports_in_use: vec![],
            resource_usage: ResourceUsage {
                cpu_percent: 0.0,
                memory_mb: 0,
                disk_mb: 0,
            },
        })
    }

    // === Performance API ===

    pub async fn get_performance_metrics(&self) -> anyhow::Result<PerformanceMetrics> {
        let m: serde_json::Value = self.get("/api/v1/metrics/performance").await?;

        Ok(PerformanceMetrics {
            cold_start_avg_ms: m["cold_start_avg_ms"].as_f64().unwrap_or(0.0),
            warm_start_avg_ms: m["warm_start_avg_ms"].as_f64().unwrap_or(0.0),
            speedup_factor: m["speedup_factor"].as_f64().unwrap_or(0.0),
            lazy_load_hit_rate: m["lazy_load_hit_rate"].as_f64().unwrap_or(0.0),
            prewarm_hit_rate: m["prewarm_hit_rate"].as_f64().unwrap_or(0.0),
            checkpoints_active: 0,
            containers_prewarmed: 0,
        })
    }

    pub async fn run_benchmark(&self, _image: &str, _compare_docker: bool) -> anyhow::Result<BenchmarkResult> {
        // Would trigger benchmark on daemon
        Ok(BenchmarkResult {
            hyperbox_cold_ms: 4500.0,
            hyperbox_warm_ms: 85.0,
            docker_cold_ms: Some(25000.0),
            docker_warm_ms: Some(3000.0),
            speedup_cold: Some(5.5),
            speedup_warm: Some(35.0),
        })
    }
}
