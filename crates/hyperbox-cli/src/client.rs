//! Daemon client for CLI-daemon communication.
//!
//! Provides both HTTP REST API and IPC communication with the hyperboxd daemon.

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Default daemon HTTP address.
pub const DEFAULT_DAEMON_ADDR: &str = "http://127.0.0.1:8080";

/// Windows named pipe path for IPC.
#[cfg(windows)]
pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\hyperbox";

/// Client for communicating with the HyperBox daemon.
#[derive(Clone)]
pub struct DaemonClient {
    http_client: reqwest::Client,
    base_url: String,
}

/// API response wrapper.
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<T>,
}

/// Container information from daemon.
#[derive(Debug, Deserialize, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: Option<String>,
    pub image: String,
    pub status: String,
    pub created_at: String,
    pub ports: Vec<PortMapping>,
}

/// Port mapping.
#[derive(Debug, Deserialize, Clone)]
pub struct PortMapping {
    pub host: u16,
    pub container: u16,
    pub protocol: String,
}

/// Image information from daemon.
#[derive(Debug, Deserialize, Clone)]
pub struct ImageInfo {
    pub id: String,
    pub repo: Option<String>,
    pub tag: Option<String>,
    pub size: u64,
    pub created_at: String,
}

/// Create container request.
#[derive(Debug, Serialize)]
pub struct CreateContainerRequest {
    pub image: String,
    pub name: Option<String>,
    pub env: Option<Vec<String>>,
    pub ports: Option<Vec<PortMappingRequest>>,
    pub volumes: Option<Vec<String>>,
    pub command: Option<Vec<String>>,
}

/// Port mapping in request.
#[derive(Debug, Serialize)]
pub struct PortMappingRequest {
    pub host: u16,
    pub container: u16,
    pub protocol: Option<String>,
}

/// Daemon health status.
#[derive(Debug, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub container_count: usize,
}

impl DaemonClient {
    /// Create a new daemon client with default settings.
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_DAEMON_ADDR)
    }

    /// Create a new daemon client with a custom base URL.
    pub fn with_base_url(base_url: &str) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }

    /// Check if the daemon is running.
    pub async fn is_running(&self) -> bool {
        self.health().await.is_ok()
    }

    /// Get daemon health status.
    pub async fn health(&self) -> Result<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to daemon")?;

        resp.json().await.context("Failed to parse health response")
    }

    /// List all containers.
    pub async fn list_containers(&self, all: bool) -> Result<Vec<ContainerInfo>> {
        let url = if all {
            format!("{}/api/v1/containers?all=true", self.base_url)
        } else {
            format!("{}/api/v1/containers", self.base_url)
        };

        let resp: ApiResponse<Vec<ContainerInfo>> = self.get(&url).await?;
        Ok(resp.data.unwrap_or_default())
    }

    /// Create a new container.
    pub async fn create_container(&self, req: CreateContainerRequest) -> Result<String> {
        let url = format!("{}/api/v1/containers", self.base_url);

        let resp: ApiResponse<serde_json::Value> = self.post(&url, &req).await?;

        if !resp.success {
            anyhow::bail!(resp.message.unwrap_or_else(|| "Unknown error".to_string()));
        }

        resp.data
            .and_then(|d| {
                d.get("id")
                    .and_then(|id| id.as_str().map(|s| s.to_string()))
            })
            .ok_or_else(|| anyhow::anyhow!("No container ID in response"))
    }

    /// Start a container.
    pub async fn start_container(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/v1/containers/{}/start", self.base_url, id);
        let resp: ApiResponse<()> = self.post_empty(&url).await?;

        if !resp.success {
            anyhow::bail!(resp
                .message
                .unwrap_or_else(|| "Failed to start container".to_string()));
        }
        Ok(())
    }

    /// Stop a container.
    pub async fn stop_container(&self, id: &str, timeout: u64) -> Result<()> {
        let url = format!("{}/api/v1/containers/{}/stop?timeout={}", self.base_url, id, timeout);
        let resp: ApiResponse<()> = self.post_empty(&url).await?;

        if !resp.success {
            anyhow::bail!(resp
                .message
                .unwrap_or_else(|| "Failed to stop container".to_string()));
        }
        Ok(())
    }

    /// Restart a container.
    pub async fn restart_container(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/v1/containers/{}/restart", self.base_url, id);
        let resp: ApiResponse<()> = self.post_empty(&url).await?;

        if !resp.success {
            anyhow::bail!(resp
                .message
                .unwrap_or_else(|| "Failed to restart container".to_string()));
        }
        Ok(())
    }

    /// Remove a container.
    pub async fn remove_container(&self, id: &str, force: bool) -> Result<()> {
        let url = format!("{}/api/v1/containers/{}?force={}", self.base_url, id, force);
        let resp: ApiResponse<()> = self.delete(&url).await?;

        if !resp.success {
            anyhow::bail!(resp
                .message
                .unwrap_or_else(|| "Failed to remove container".to_string()));
        }
        Ok(())
    }

    /// Get container logs.
    pub async fn get_logs(
        &self,
        id: &str,
        tail: Option<usize>,
        timestamps: bool,
    ) -> Result<Vec<String>> {
        let mut url = format!("{}/api/v1/containers/{}/logs", self.base_url, id);

        // Build query parameters
        let mut params = Vec::new();
        if let Some(n) = tail {
            params.push(format!("tail={}", n));
        }
        if timestamps {
            params.push("timestamps=true".to_string());
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let resp: ApiResponse<Vec<String>> = self.get(&url).await?;

        if !resp.success {
            anyhow::bail!(resp
                .message
                .unwrap_or_else(|| "Failed to get container logs".to_string()));
        }

        Ok(resp.data.unwrap_or_default())
    }

    /// Stream container logs via Server-Sent Events.
    ///
    /// Returns a stream of log lines that can be consumed as they arrive.
    /// This is used for `--follow` mode in the CLI.
    pub async fn stream_logs(
        &self,
        id: &str,
        tail: Option<usize>,
        timestamps: bool,
    ) -> Result<impl futures::Stream<Item = Result<String, anyhow::Error>>> {
        use futures::StreamExt;

        let mut url = format!("{}/api/v1/containers/{}/logs/stream", self.base_url, id);

        // Build query parameters
        let mut params = Vec::new();
        if let Some(n) = tail {
            params.push(format!("tail={}", n));
        }
        if timestamps {
            params.push("timestamps=true".to_string());
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let resp = self
            .http_client
            .get(&url)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .context("Failed to connect to daemon for log streaming")?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to start log stream: HTTP {}", resp.status());
        }

        // Parse SSE stream
        let stream = resp.bytes_stream().map(|result| {
            result
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
                .and_then(|bytes| {
                    // Parse SSE format: "event: <type>\ndata: <data>\n\n"
                    let text = String::from_utf8_lossy(&bytes);
                    parse_sse_event(&text)
                })
        });

        Ok(stream)
    }

    /// List all images.
    pub async fn list_images(&self) -> Result<Vec<ImageInfo>> {
        let url = format!("{}/api/v1/images", self.base_url);
        let resp: ApiResponse<Vec<ImageInfo>> = self.get(&url).await?;
        Ok(resp.data.unwrap_or_default())
    }

    /// Pull an image.
    pub async fn pull_image(&self, image: &str) -> Result<()> {
        let url = format!("{}/api/v1/images/pull", self.base_url);
        let req = serde_json::json!({ "image": image });
        let resp: ApiResponse<()> = self.post(&url, &req).await?;

        if !resp.success {
            anyhow::bail!(resp
                .message
                .unwrap_or_else(|| "Failed to pull image".to_string()));
        }
        Ok(())
    }

    /// Remove an image.
    pub async fn remove_image(&self, id: &str, force: bool) -> Result<()> {
        let url = format!("{}/api/v1/images/{}?force={}", self.base_url, id, force);
        let resp: ApiResponse<()> = self.delete(&url).await?;

        if !resp.success {
            anyhow::bail!(resp
                .message
                .unwrap_or_else(|| "Failed to remove image".to_string()));
        }
        Ok(())
    }

    // HTTP helper methods

    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let resp = self
            .http_client
            .get(url)
            .send()
            .await
            .context("Failed to connect to daemon")?;

        resp.json().await.context("Failed to parse response")
    }

    async fn post<T: DeserializeOwned, B: Serialize>(&self, url: &str, body: &B) -> Result<T> {
        let resp = self
            .http_client
            .post(url)
            .json(body)
            .send()
            .await
            .context("Failed to connect to daemon")?;

        resp.json().await.context("Failed to parse response")
    }

    async fn post_empty<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let resp = self
            .http_client
            .post(url)
            .send()
            .await
            .context("Failed to connect to daemon")?;

        resp.json().await.context("Failed to parse response")
    }

    async fn delete<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let resp = self
            .http_client
            .delete(url)
            .send()
            .await
            .context("Failed to connect to daemon")?;

        resp.json().await.context("Failed to parse response")
    }
}

impl Default for DaemonClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Send a command via named pipe IPC (Windows).
#[cfg(windows)]
pub async fn ipc_command(command: &str) -> Result<String> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ClientOptions;

    let mut attempts = 0;
    let client = loop {
        match ClientOptions::new().open(DEFAULT_PIPE_NAME) {
            Ok(client) => break client,
            Err(e) if attempts < 3 => {
                attempts += 1;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            Err(e) => return Err(anyhow::anyhow!("Failed to connect to daemon IPC: {}", e)),
        }
    };

    let (reader, mut writer) = tokio::io::split(client);
    let mut reader = BufReader::new(reader);

    // Send command
    writer.write_all(command.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    // Read response
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    Ok(response.trim().to_string())
}

/// Send a command via named pipe IPC (Unix stub).
#[cfg(not(windows))]
pub async fn ipc_command(_command: &str) -> Result<String> {
    Err(anyhow::anyhow!("IPC not implemented for this platform"))
}

/// Parse a Server-Sent Events message.
///
/// SSE format:
/// ```
/// event: <type>
/// data: <content>
///
/// ```
fn parse_sse_event(text: &str) -> Result<String> {
    let mut event_type = "message";
    let mut data = String::new();

    for line in text.lines() {
        if line.starts_with("event:") {
            event_type = line.strip_prefix("event:").unwrap_or("message").trim();
        } else if line.starts_with("data:") {
            data = line.strip_prefix("data:").unwrap_or("").trim().to_string();
        }
    }

    // Handle different event types
    match event_type {
        "log" => Ok(data),
        "error" => Err(anyhow::anyhow!("{}", data)),
        "end" => Err(anyhow::anyhow!("Stream ended")),
        _ => Ok(data),
    }
}
