//! Health check command for HyperBox daemon and dependencies.

use anyhow::Result;
use clap::Args;
use colored::*;
use std::fs;
use std::path::PathBuf;

/// Check HyperBox health status
#[derive(Args)]
pub struct HealthCommand;

pub async fn run(_cmd: HealthCommand) -> Result<()> {
    println!("{}", "HyperBox Health Check:".bold());

    let daemon_ok = check_daemon_socket().await;
    let crun_ok = check_crun_binary().await;
    let docker_ok = check_docker_socket().await;

    let daemon_status = if daemon_ok {
        "✅".green()
    } else {
        "❌".red()
    };

    let crun_status = if crun_ok { "✅".green() } else { "❌".red() };

    let docker_status = if docker_ok {
        "✅".green()
    } else {
        "❌".red()
    };

    println!("  Daemon:     {}", daemon_status);
    println!("  crun:       {}", crun_status);
    println!("  Docker:     {}", docker_status);

    if daemon_ok && crun_ok && docker_ok {
        Ok(())
    } else {
        eprintln!("\n{}", "Some components are not healthy.".red());
        std::process::exit(1);
    }
}

/// Check if HyperBox daemon socket exists and is responsive
async fn check_daemon_socket() -> bool {
    let socket_paths = vec![
        PathBuf::from("/run/hyperbox/hyperbox.sock"),
        PathBuf::from("/var/run/hyperbox/hyperbox.sock"),
        PathBuf::from("/tmp/hyperbox/hyperbox.sock"),
    ];

    // Add XDG Runtime Dir path if available
    let mut paths = socket_paths;
    if let Some(runtime_dir) = dirs::runtime_dir() {
        paths.push(runtime_dir.join("hyperbox/hyperbox.sock"));
    }

    // Add home directory path if available
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".hyperbox/daemon.sock"));
    }

    for socket_path in paths {
        if let Ok(metadata) = fs::metadata(&socket_path) {
            // On Unix systems, verify it's actually a socket
            #[cfg(unix)]
            {
                use std::os::unix::fs::FileTypeExt;
                if metadata.file_type().is_socket() {
                    return true;
                }
            }
            // On non-Unix systems, just check if the path exists and is a file
            #[cfg(not(unix))]
            {
                if metadata.is_file() {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if crun binary is available and executable
async fn check_crun_binary() -> bool {
    match which::which("crun") {
        Ok(crun_path) => {
            // Verify it's executable
            if let Ok(metadata) = fs::metadata(&crun_path) {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = metadata.permissions().mode();
                    // Check if executable bit is set for owner, group, or others
                    return (mode & 0o111) != 0;
                }
                #[cfg(not(unix))]
                {
                    return !metadata.permissions().readonly();
                }
            }
            false
        }
        Err(_) => false,
    }
}

/// Check if Docker daemon socket is accessible
async fn check_docker_socket() -> bool {
    let mut socket_paths = vec![
        PathBuf::from("/var/run/docker.sock"),
        PathBuf::from("/run/docker.sock"),
    ];

    // Add home directory docker socket paths
    if let Some(home) = dirs::home_dir() {
        socket_paths.push(home.join(".docker/run/docker.sock"));
        socket_paths.push(home.join(".docker/desktop/docker.sock"));
        socket_paths.push(home.join(".docker/distd.sock"));
    }

    for socket_path in socket_paths {
        if let Ok(metadata) = fs::metadata(&socket_path) {
            // On Unix systems, verify it's actually a socket
            #[cfg(unix)]
            {
                use std::os::unix::fs::FileTypeExt;
                if metadata.file_type().is_socket() {
                    return true;
                }
            }
            // On non-Unix systems, just check if the path exists and is a file
            #[cfg(not(unix))]
            {
                if metadata.is_file() {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_functions_dont_panic() {
        // These should not panic
        let _daemon = check_daemon_socket().await;
        let _crun = check_crun_binary().await;
        let _docker = check_docker_socket().await;
    }
}
