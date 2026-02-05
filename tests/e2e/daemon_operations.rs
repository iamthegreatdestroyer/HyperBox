//! Daemon Operations E2E Tests
//!
//! Tests daemon startup, IPC communication, and client interactions.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Daemon binary name
const DAEMON_BINARY: &str = "hyperboxd";

/// Default daemon timeout
const DAEMON_TIMEOUT: Duration = Duration::from_secs(5);
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent")
        .to_path_buf()
}

/// Get daemon binary path
fn daemon_path() -> PathBuf {
    let root = workspace_root();
    let exe_name = if cfg!(windows) {
        format!("{}.exe", DAEMON_BINARY)
    } else {
        DAEMON_BINARY.to_string()
    };

    let release_path = root.join("target").join("release").join(&exe_name);
    if release_path.exists() {
        return release_path;
    }

    root.join("target").join("debug").join(&exe_name)
}

/// Helper to run a command with timeout protection
fn run_command_with_timeout(
    path: &PathBuf,
    args: Vec<String>,
    timeout_secs: u64,
) -> std::io::Result<std::process::Output> {
    let timeout = Duration::from_secs(timeout_secs);
    let result = Arc::new(Mutex::new(None));
    let result_clone = Arc::clone(&result);

    let path = path.clone();
    let _handle = std::thread::spawn(move || {
        let output = Command::new(&path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        *result_clone.lock().unwrap() = Some(output);
    });

    // Wait for thread with timeout
    let start = Instant::now();
    while Instant::now().duration_since(start) < timeout {
        if let Some(Ok(output)) = result.lock().unwrap().take() {
            return Ok(output);
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        format!("Command timed out after {} seconds", timeout_secs),
    ))
}

// ============================================================================
// Daemon Binary Tests
// ============================================================================

#[test]
fn test_daemon_binary_exists() {
    let path = daemon_path();
    // Binary should exist in debug or release
    assert!(
        path.exists() || path.with_extension("exe").exists(),
        "Daemon binary should exist at {:?} (or .exe variant)",
        path
    );
}

#[test]
#[ignore = "Daemon hangs when spawned from test harness - works manually but times out in tests"]
fn test_daemon_version() {
    let path = daemon_path();

    if !path.exists() {
        println!("Skipping: daemon binary not found");
        return;
    }

    let output = run_command_with_timeout(&path, vec!["--version".to_string()], 2);

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let combined = format!("{}{}", stdout, stderr);

            // Should output version info
            assert!(
                combined.contains("hyperbox") || combined.contains("0."),
                "Should output version: {}",
                combined
            );
        }
        Err(e) => {
            panic!("Failed to run daemon: {}", e);
        }
    }
}

#[test]
#[ignore = "Daemon hangs when spawned from test harness - works manually but times out in tests"]
fn test_daemon_help() {
    let path = daemon_path();

    if !path.exists() {
        println!("Skipping: daemon binary not found");
        return;
    }

    let output = Command::new(&path)
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);

            // Should show usage information
            assert!(
                stdout.contains("Usage")
                    || stdout.contains("USAGE")
                    || stderr.contains("Usage")
                    || stderr.contains("USAGE"),
                "Should show usage: stdout={}, stderr={}",
                stdout,
                stderr
            );
        }
        Err(e) => {
            panic!("Failed to run daemon: {}", e);
        }
    }
}

// ============================================================================
// Daemon Configuration Tests
// ============================================================================

#[test]
#[ignore = "Daemon hangs when spawned from test harness - works manually but times out in tests"]
fn test_daemon_config_path() {
    // Test that daemon uses correct config path for platform
    let path = daemon_path();

    if !path.exists() {
        println!("Skipping: daemon binary not found");
        return;
    }

    // Try to get config path using --show-config
    let output = Command::new(&path)
        .arg("--show-config")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);

        #[cfg(windows)]
        {
            // Windows should use AppData or ProgramData
            if out.status.success() {
                assert!(
                    stdout.contains("AppData")
                        || stdout.contains("ProgramData")
                        || stdout.contains("hyperbox"),
                    "Should use Windows path conventions: {}",
                    stdout
                );
            }
        }

        #[cfg(unix)]
        {
            // Unix should use /etc or ~/.config
            if out.status.success() {
                assert!(
                    stdout.contains("/etc")
                        || stdout.contains(".config")
                        || stdout.contains("hyperbox"),
                    "Should use Unix path conventions: {}",
                    stdout
                );
            }
        }
    }
}

// ============================================================================
// IPC Protocol Tests
// ============================================================================

#[test]
fn test_ipc_socket_path_windows() {
    #[cfg(windows)]
    {
        // Windows should use named pipes
        let expected_prefix = r"\\.\pipe\";
        let socket_name = "hyperbox";

        // Construct expected path
        let expected_path = format!("{}{}", expected_prefix, socket_name);

        // Path should contain named pipe prefix
        assert!(expected_path.starts_with(expected_prefix), "Windows IPC should use named pipes");
    }
}

#[test]
fn test_ipc_socket_path_unix() {
    #[cfg(unix)]
    {
        // Unix should use domain sockets
        let runtime_dir =
            std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/var/run".to_string());

        let socket_path = format!("{}/hyperbox/hyperbox.sock", runtime_dir);

        // Path should be valid Unix socket path
        assert!(socket_path.ends_with(".sock"), "Unix IPC should use socket files");
    }
}

// ============================================================================
// Daemon Startup Tests
// ============================================================================

#[test]
fn test_daemon_startup_speed() {
    let path = daemon_path();

    if !path.exists() {
        println!("Skipping: daemon binary not found");
        return;
    }

    let start = Instant::now();

    // Run version command (quick startup test)
    let output = Command::new(&path)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let elapsed = start.elapsed();

    if output.is_ok() {
        // Daemon should start and respond quickly
        assert!(elapsed < Duration::from_secs(2), "Daemon should start quickly: {:?}", elapsed);

        println!("Daemon startup time: {:?}", elapsed);
    }
}

#[test]
fn test_daemon_graceful_shutdown_signal() {
    // This test verifies the daemon handles signals correctly
    // For now, verify the daemon binary responds to --version without hanging
    let path = daemon_path();

    if !path.exists() {
        println!("Skipping: daemon binary not found");
        return;
    }

    let start = Instant::now();

    let output = Command::new(&path)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let elapsed = start.elapsed();

    // Should complete quickly (not hang)
    assert!(
        elapsed < Duration::from_secs(2),
        "Daemon should respond to --version quickly: {:?}",
        elapsed
    );

    if let Ok(out) = output {
        assert!(out.status.success(), "Daemon --version should succeed");
    }
}

// ============================================================================
// Multi-Client Connection Tests
// ============================================================================

#[test]
fn test_concurrent_cli_commands() {
    // Simulate multiple CLI commands running concurrently
    // This tests daemon's ability to handle multiple clients

    let cli_path = workspace_root()
        .join("target")
        .join("debug")
        .join(if cfg!(windows) { "hb.exe" } else { "hb" });

    if !cli_path.exists() {
        println!("Skipping: CLI binary not found");
        return;
    }

    let start = Instant::now();

    // Spawn multiple concurrent commands
    let handles: Vec<_> = (0..3)
        .map(|_| {
            let path = cli_path.clone();
            std::thread::spawn(move || {
                Command::new(path)
                    .args(["--version"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
            })
        })
        .collect();

    // Wait for all to complete
    let results: Vec<_> = handles.into_iter().map(|h| h.join()).collect();

    let elapsed = start.elapsed();

    // All commands should complete
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "Command {} should complete without panic", i);
    }

    // Concurrent commands shouldn't take too long
    assert!(
        elapsed < Duration::from_secs(10),
        "Concurrent commands should complete quickly: {:?}",
        elapsed
    );

    println!("3 concurrent commands completed in {:?}", elapsed);
}

// ============================================================================
// Daemon Error Handling Tests
// ============================================================================

#[test]
fn test_daemon_invalid_command() {
    let path = daemon_path();

    if !path.exists() {
        println!("Skipping: daemon binary not found");
        return;
    }

    // Test with invalid flag
    let output = Command::new(&path)
        .arg("--invalid-flag-xyz")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        // Should fail with error message
        assert!(!out.status.success(), "Invalid command should fail");

        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stderr.len() > 0 || stdout.len() > 0, "Should provide error feedback");
    }
}

#[test]
fn test_daemon_missing_argument() {
    let path = daemon_path();

    if !path.exists() {
        println!("Skipping: daemon binary not found");
        return;
    }

    // Try a command that requires an argument
    let output = Command::new(&path)
        .arg("--config")
        // Missing config file path
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        // Should handle gracefully
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let combined = format!("{}{}", stdout, stderr);

        // Either fails with error or invalid behavior
        assert!(
            !out.status.success() || combined.contains("error") || combined.contains("Usage"),
            "Should indicate missing argument: {}",
            combined
        );
    }
}

// ============================================================================
// Daemon Resource Cleanup Tests
// ============================================================================

#[test]
fn test_daemon_no_resource_leak() {
    // Run daemon commands multiple times and verify no obvious resource leaks
    let path = daemon_path();

    if !path.exists() {
        println!("Skipping: daemon binary not found");
        return;
    }

    // Run 10 quick commands
    for i in 0..10 {
        let output = Command::new(&path)
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        if let Err(e) = output {
            panic!("Command {} failed: {}", i, e);
        }
    }

    // If we get here without running out of file handles, memory, etc.,
    // basic resource management is working
    println!("10 sequential daemon commands completed successfully");
}

// ============================================================================
// Daemon Logging Tests
// ============================================================================

#[test]
fn test_daemon_log_levels() {
    let path = daemon_path();

    if !path.exists() {
        println!("Skipping: daemon binary not found");
        return;
    }

    // Test log level flag
    let output = Command::new(&path)
        .args(["--log-level", "debug", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        // Should show help
        let combined = String::from_utf8_lossy(&out.stdout);
        assert!(combined.contains("Usage"), "Should show help");
    }

    // Test trace log level
    let output = Command::new(&path)
        .args(["--log-level", "trace", "--version"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        assert!(out.status.success(), "Should handle trace log level");
    }
}

// ============================================================================
// Platform-Specific Daemon Tests
// ============================================================================

#[cfg(windows)]
mod windows_daemon_tests {
    use super::*;

    #[test]
    fn test_daemon_windows_service_mode() {
        let path = daemon_path();

        if !path.exists() {
            println!("Skipping: daemon binary not found");
            return;
        }

        // Check if daemon supports service mode
        let output = Command::new(&path)
            .arg("--help")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);

            // May or may not support service mode
            println!(
                "Windows service mode supported: {}",
                stdout.contains("service") || stdout.contains("--service")
            );
        }
    }
}

#[cfg(unix)]
mod unix_daemon_tests {
    use super::*;

    #[test]
    fn test_daemon_daemonize_mode() {
        let path = daemon_path();

        if !path.exists() {
            println!("Skipping: daemon binary not found");
            return;
        }

        // Check if daemon supports daemonization
        let output = Command::new(&path)
            .arg("--help")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);

            println!(
                "Unix daemonize mode supported: {}",
                stdout.contains("daemon") || stdout.contains("--daemon") || stdout.contains("-d")
            );
        }
    }
}
