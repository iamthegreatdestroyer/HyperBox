//! IPC (Unix socket) handler.

use crate::state::DaemonState;
use std::path::PathBuf;
use tracing::info;

/// Handle IPC connections.
pub async fn serve(state: DaemonState, socket_path: PathBuf) -> anyhow::Result<()> {
    info!("IPC socket at {:?}", socket_path);

    // Ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove existing socket
    let _ = std::fs::remove_file(&socket_path);

    // On Unix, we'd use tokio's UnixListener
    // On Windows, we'd use named pipes
    // For now, this is a placeholder

    #[cfg(unix)]
    {
        use tokio::net::UnixListener;

        let listener = UnixListener::bind(&socket_path)?;
        info!("Unix socket listening at {:?}", socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let state = state.clone();
                    tokio::spawn(async move {
                        handle_connection(state, stream).await;
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    #[cfg(not(unix))]
    {
        // Windows named pipe support would go here
        tokio::time::sleep(std::time::Duration::from_secs(u64::MAX)).await;
    }

    Ok(())
}

#[cfg(unix)]
async fn handle_connection(state: DaemonState, stream: tokio::net::UnixStream) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let response = handle_command(&state, line.trim()).await;
                if let Err(e) = writer.write_all(response.as_bytes()).await {
                    tracing::error!("Failed to write response: {}", e);
                    break;
                }
                if let Err(e) = writer.write_all(b"\n").await {
                    tracing::error!("Failed to write newline: {}", e);
                    break;
                }
            }
            Err(e) => {
                tracing::error!("Failed to read command: {}", e);
                break;
            }
        }
    }
}

#[cfg(unix)]
async fn handle_command(state: &DaemonState, command: &str) -> String {
    // Simple command protocol for IPC
    // Format: COMMAND [args...]
    // Response: JSON

    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return r#"{"error": "empty command"}"#.to_string();
    }

    match parts[0].to_uppercase().as_str() {
        "PING" => r#"{"result": "PONG"}"#.to_string(),
        "INFO" => {
            serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "containers": state.containers.len(),
                "images": state.images.len(),
                "uptime_seconds": state.uptime().num_seconds()
            })
            .to_string()
        }
        "CONTAINERS" => {
            let containers: Vec<_> = state.containers.iter().map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "name": c.name,
                    "status": c.status.to_string()
                })
            }).collect();
            serde_json::json!({"containers": containers}).to_string()
        }
        "METRICS" => {
            let metrics = state.metrics.read();
            serde_json::to_string(&*metrics).unwrap_or_default()
        }
        _ => r#"{"error": "unknown command"}"#.to_string(),
    }
}
