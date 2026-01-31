//! Application state management.

use crate::commands::*;
use crate::daemon::DaemonClient;
use std::path::PathBuf;
use tokio::sync::RwLock;

/// Application state.
pub struct AppState {
    /// Daemon client
    daemon: RwLock<Option<DaemonClient>>,

    /// User settings
    settings: RwLock<Settings>,

    /// Settings file path
    settings_path: PathBuf,
}

impl AppState {
    /// Create new application state.
    pub fn new() -> Self {
        let settings_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("hyperbox")
            .join("settings.json");

        let settings = if settings_path.exists() {
            std::fs::read_to_string(&settings_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Settings::default()
        };

        Self {
            daemon: RwLock::new(None),
            settings: RwLock::new(settings),
            settings_path,
        }
    }

    /// Connect to daemon.
    pub async fn connect_daemon(&self) -> anyhow::Result<()> {
        let client = DaemonClient::connect("http://127.0.0.1:8080").await?;
        *self.daemon.write().await = Some(client);
        Ok(())
    }

    /// Check if daemon is connected.
    pub async fn is_daemon_connected(&self) -> bool {
        let guard = self.daemon.read().await;
        if let Some(ref client) = *guard {
            client.ping().await.is_ok()
        } else {
            false
        }
    }

    /// Start daemon.
    pub async fn start_daemon(&self) -> anyhow::Result<()> {
        // Would spawn daemon process here
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("hyperboxd").spawn()?;
        }

        // Wait for daemon to start
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        self.connect_daemon().await
    }

    /// Stop daemon.
    pub async fn stop_daemon(&self) -> anyhow::Result<()> {
        // Would send shutdown signal here
        *self.daemon.write().await = None;
        Ok(())
    }

    /// Get system info.
    pub async fn get_system_info(&self) -> anyhow::Result<SystemInfo> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.get_system_info().await
    }

    /// List containers.
    pub async fn list_containers(
        &self,
        all: bool,
        project_id: Option<String>,
    ) -> anyhow::Result<Vec<Container>> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.list_containers(all, project_id).await
    }

    /// Get container.
    pub async fn get_container(&self, id: &str) -> anyhow::Result<Container> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.get_container(id).await
    }

    /// Create container.
    pub async fn create_container(
        &self,
        request: CreateContainerRequest,
    ) -> anyhow::Result<Container> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.create_container(request).await
    }

    /// Start container.
    pub async fn start_container(&self, id: &str) -> anyhow::Result<()> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.start_container(id).await
    }

    /// Stop container.
    pub async fn stop_container(&self, id: &str, create_checkpoint: bool) -> anyhow::Result<()> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.stop_container(id, create_checkpoint).await
    }

    /// Restart container.
    pub async fn restart_container(&self, id: &str) -> anyhow::Result<()> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.restart_container(id).await
    }

    /// Remove container.
    pub async fn remove_container(&self, id: &str, force: bool) -> anyhow::Result<()> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.remove_container(id, force).await
    }

    /// Get container logs.
    pub async fn get_container_logs(
        &self,
        id: &str,
        tail: Option<u32>,
        _follow: bool,
    ) -> anyhow::Result<Vec<String>> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.get_container_logs(id, tail).await
    }

    /// Get container stats.
    pub async fn get_container_stats(&self, id: &str) -> anyhow::Result<ContainerStats> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.get_container_stats(id).await
    }

    /// List images.
    pub async fn list_images(&self) -> anyhow::Result<Vec<Image>> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.list_images().await
    }

    /// Pull image.
    pub async fn pull_image(&self, image: &str, platform: Option<&str>) -> anyhow::Result<Image> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.pull_image(image, platform).await
    }

    /// Remove image.
    pub async fn remove_image(&self, id: &str, _force: bool) -> anyhow::Result<()> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.remove_image(id).await
    }

    /// List projects.
    pub async fn list_projects(&self) -> anyhow::Result<Vec<Project>> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.list_projects().await
    }

    /// Open project.
    pub async fn open_project(&self, path: &str, name: Option<&str>) -> anyhow::Result<Project> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.open_project(path, name).await
    }

    /// Close project.
    pub async fn close_project(&self, id: &str, _stop_containers: bool) -> anyhow::Result<()> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.close_project(id).await
    }

    /// Start project.
    pub async fn start_project(&self, id: &str) -> anyhow::Result<()> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.start_project(id).await
    }

    /// Stop project.
    pub async fn stop_project(&self, id: &str) -> anyhow::Result<()> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.stop_project(id).await
    }

    /// Get project status.
    pub async fn get_project_status(&self, id: &str) -> anyhow::Result<ProjectStatus> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.get_project_status(id).await
    }

    /// Get performance metrics.
    pub async fn get_performance_metrics(&self) -> anyhow::Result<PerformanceMetrics> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.get_performance_metrics().await
    }

    /// Run benchmark.
    pub async fn run_benchmark(
        &self,
        image: &str,
        compare_docker: bool,
    ) -> anyhow::Result<BenchmarkResult> {
        let guard = self.daemon.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        client.run_benchmark(image, compare_docker).await
    }

    /// Get settings.
    pub async fn get_settings(&self) -> anyhow::Result<Settings> {
        Ok(self.settings.read().await.clone())
    }

    /// Update settings.
    pub async fn update_settings(&self, settings: Settings) -> anyhow::Result<()> {
        // Save to file
        if let Some(parent) = self.settings_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&settings)?;
        std::fs::write(&self.settings_path, content)?;

        *self.settings.write().await = settings;
        Ok(())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
