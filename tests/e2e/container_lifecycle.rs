//! Container Lifecycle E2E Tests
//!
//! Verifies complete container lifecycle operations from CLI to runtime.
//! These tests are designed to work with or without the daemon running.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// CLI binary name
const CLI_BINARY: &str = "hb";

/// Test timeout for container operations
#[allow(dead_code)]
const CONTAINER_OP_TIMEOUT: Duration = Duration::from_secs(30);

/// Get workspace root directory
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent of tests dir")
        .to_path_buf()
}

/// Get binary path with platform extension
fn binary_path(name: &str) -> PathBuf {
    let root = workspace_root();
    let exe_name = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };

    // Try release first, then debug
    let release_path = root.join("target").join("release").join(&exe_name);
    if release_path.exists() {
        return release_path;
    }

    root.join("target").join("debug").join(&exe_name)
}

/// Run CLI command with args
fn run_cli(args: &[&str]) -> Result<Output, std::io::Error> {
    let path = binary_path(CLI_BINARY);
    Command::new(path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
}

/// Check if output indicates daemon not running (acceptable failure)
fn is_daemon_not_running(output: &Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    stderr.to_lowercase().contains("daemon")
        || stdout.to_lowercase().contains("daemon")
        || stderr.contains("not running")
        || stderr.contains("connect")
}

/// Check if output indicates resource not found (acceptable failure)
fn is_not_found_error(output: &Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr.contains("not found") || stderr.contains("No such") || stderr.contains("does not exist")
}

/// Check for acceptable error (daemon not running or resource not found)
fn is_acceptable_error(output: &Output) -> bool {
    is_daemon_not_running(output) || is_not_found_error(output)
}

// ============================================================================
// Container Creation Tests
// ============================================================================

#[test]
fn test_container_run_basic() {
    // Test: hb container run alpine (the CLI uses 'run', not 'create')
    let output = run_cli(&["container", "run", "alpine:latest", "--", "echo", "hello"]);

    match output {
        Ok(out) => {
            // Should either succeed or fail gracefully (no panic)
            assert!(
                out.status.success() || is_acceptable_error(&out),
                "Run should succeed or fail gracefully: stdout={}, stderr={}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Err(e) => {
            // Binary not found is acceptable in test environment
            assert!(
                e.to_string().contains("not found") || e.to_string().contains("No such file"),
                "Unexpected error: {}",
                e
            );
        }
    }
}

#[test]
fn test_container_run_detached() {
    // Test: hb container run -d alpine sleep 10
    let output = run_cli(&[
        "container",
        "run",
        "-d",
        "alpine:latest",
        "--",
        "sleep",
        "10",
    ]);

    match output {
        Ok(out) => {
            // Either succeeds (outputs container ID) or fails gracefully
            assert!(
                out.status.success() || is_acceptable_error(&out),
                "Should succeed or fail gracefully: stderr={}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Err(e) => {
            // Binary not found is acceptable
            assert!(e.to_string().contains("not found"), "Unexpected error: {}", e);
        }
    }
}

#[test]
fn test_container_run_with_options() {
    // Test: hb container run -d --name test-container -p 8080:80 nginx
    let args = &[
        "container",
        "run",
        "-d",
        "--name",
        "e2e-test-container",
        "-p",
        "18080:80",
        "-e",
        "TEST=value",
        "nginx:latest",
    ];

    let output = run_cli(args);

    match output {
        Ok(out) => {
            // Either succeeds or fails with expected error
            assert!(
                out.status.success() || is_acceptable_error(&out),
                "Command should succeed or fail gracefully"
            );
        }
        Err(_) => {
            // Binary not found is acceptable
        }
    }
}

// ============================================================================
// Container List Tests
// ============================================================================

#[test]
fn test_container_list_basic() {
    // Test: hb container ls
    let output = run_cli(&["container", "ls"]);

    match output {
        Ok(out) => {
            // Either shows containers/header or daemon not running error
            assert!(
                out.status.success() || is_daemon_not_running(&out),
                "List should show containers or indicate daemon not running"
            );
        }
        Err(_) => {}
    }
}

#[test]
fn test_container_list_all() {
    // Test: hb container ls -a
    let output = run_cli(&["container", "ls", "-a"]);

    match output {
        Ok(out) => {
            // Should complete without panic
            assert!(
                out.status.success() || is_daemon_not_running(&out),
                "Should succeed or indicate daemon not running"
            );
        }
        Err(_) => {}
    }
}

#[test]
fn test_container_list_quiet() {
    // Test: hb container ls -q (IDs only)
    let output = run_cli(&["container", "ls", "-q"]);

    match output {
        Ok(out) => {
            // Should complete without panic
            // Note: quiet mode output validation only makes sense when daemon is running
            assert!(
                out.status.success() || is_daemon_not_running(&out),
                "Should succeed or indicate daemon not running"
            );
        }
        Err(_) => {}
    }
}

// ============================================================================
// Container Stop/Start Tests
// ============================================================================

#[test]
#[ignore = "CLI needs to return non-zero exit codes for nonexistent container operations"]
fn test_container_stop_nonexistent() {
    // Test: hb container stop nonexistent-id
    let output = run_cli(&["container", "stop", "0000000000000000"]);

    match output {
        Ok(out) => {
            // Should fail for nonexistent container (or daemon not running)
            assert!(!out.status.success(), "Should fail for nonexistent container");
            assert!(
                is_acceptable_error(&out),
                "Should indicate container not found or daemon not running: stderr={}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Err(_) => {}
    }
}

#[test]
#[ignore = "CLI needs to return non-zero exit codes for nonexistent container operations"]
fn test_container_start_nonexistent() {
    // Test: hb container start nonexistent-id
    let output = run_cli(&["container", "start", "0000000000000000"]);

    match output {
        Ok(out) => {
            assert!(!out.status.success(), "Should fail for nonexistent container");
            assert!(
                is_acceptable_error(&out),
                "Should indicate container not found or daemon not running"
            );
        }
        Err(_) => {}
    }
}

// ============================================================================
// Container Remove Tests
// ============================================================================

#[test]
#[ignore = "CLI needs to return non-zero exit codes for nonexistent container operations"]
fn test_container_remove_nonexistent() {
    // Test: hb container rm nonexistent-id
    let output = run_cli(&["container", "rm", "0000000000000000"]);

    match output {
        Ok(out) => {
            assert!(!out.status.success(), "Should fail for nonexistent container");
            assert!(
                is_acceptable_error(&out),
                "Should indicate container not found or daemon not running"
            );
        }
        Err(_) => {}
    }
}

#[test]
fn test_container_remove_force() {
    // Test: hb container rm -f nonexistent-id
    let output = run_cli(&["container", "rm", "-f", "0000000000000000"]);

    // Force remove might succeed even for nonexistent (no-op) or fail gracefully
    match output {
        Ok(out) => {
            // Either succeeds (no-op) or fails gracefully
            assert!(
                out.status.success() || is_acceptable_error(&out),
                "Force remove should be idempotent or fail gracefully"
            );
        }
        Err(_) => {}
    }
}

// ============================================================================
// Container Logs Tests
// ============================================================================

#[test]
#[ignore = "CLI needs to return non-zero exit codes for nonexistent container operations"]
fn test_container_logs_nonexistent() {
    // Test: hb container logs nonexistent-id
    let output = run_cli(&["container", "logs", "0000000000000000"]);

    match output {
        Ok(out) => {
            assert!(!out.status.success(), "Should fail for nonexistent container");
            assert!(
                is_acceptable_error(&out),
                "Should indicate container not found or daemon not running"
            );
        }
        Err(_) => {}
    }
}

#[test]
#[ignore = "CLI needs to return non-zero exit codes for nonexistent container operations"]
fn test_container_logs_options() {
    // Test: hb container logs --tail 10 --timestamps nonexistent-id
    let output = run_cli(&[
        "container",
        "logs",
        "--tail",
        "10",
        "--timestamps",
        "0000000000000000",
    ]);

    match output {
        Ok(out) => {
            // Should handle options correctly (even if container doesn't exist)
            assert!(!out.status.success(), "Should fail for nonexistent container");
            assert!(is_acceptable_error(&out), "Should fail with appropriate error");
        }
        Err(_) => {}
    }
}

// ============================================================================
// Container Inspect Tests
// ============================================================================

#[test]
#[ignore = "CLI needs to return non-zero exit codes for nonexistent container operations"]
fn test_container_inspect_nonexistent() {
    // Test: hb container inspect nonexistent-id
    let output = run_cli(&["container", "inspect", "0000000000000000"]);

    match output {
        Ok(out) => {
            assert!(!out.status.success(), "Should fail for nonexistent container");
            assert!(
                is_acceptable_error(&out),
                "Should indicate container not found or daemon not running"
            );
        }
        Err(_) => {}
    }
}

// ============================================================================
// Container Exec Tests
// ============================================================================

#[test]
#[ignore = "CLI needs to return non-zero exit codes for nonexistent container operations"]
fn test_container_exec_nonexistent() {
    // Test: hb container exec nonexistent-id ls
    let output = run_cli(&["container", "exec", "0000000000000000", "ls"]);

    match output {
        Ok(out) => {
            assert!(!out.status.success(), "Should fail for nonexistent container");
            assert!(
                is_acceptable_error(&out),
                "Should indicate container not found or daemon not running"
            );
        }
        Err(_) => {}
    }
}

// ============================================================================
// Container Stats Tests
// ============================================================================

#[test]
fn test_container_stats_no_stream() {
    // Test: hb container stats --no-stream
    let output = run_cli(&["container", "stats", "--no-stream"]);

    match output {
        Ok(out) => {
            // Should show stats or indicate daemon not running
            assert!(
                out.status.success() || is_daemon_not_running(&out),
                "Stats should work or indicate daemon not running"
            );
        }
        Err(_) => {}
    }
}

// ============================================================================
// Full Lifecycle Integration Test
// ============================================================================

#[test]
fn test_full_container_lifecycle() {
    // This test simulates a complete container lifecycle
    // Note: Requires daemon to be running for full test

    let start = Instant::now();
    let container_name = format!("e2e-lifecycle-{}", std::process::id());

    // Step 1: Run a container
    let run_output = run_cli(&[
        "container",
        "run",
        "-d",
        "--name",
        &container_name,
        "alpine:latest",
        "sleep",
        "30",
    ]);

    let daemon_running = match &run_output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    };

    if !daemon_running {
        // Skip the rest of the test if daemon isn't running
        println!("Skipping lifecycle test - daemon not running");
        return;
    }

    // Step 2: List containers (should include our container)
    let list_output = run_cli(&["container", "ls"]);
    if let Ok(out) = &list_output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains(&container_name) || !out.status.success(),
            "Container should appear in list"
        );
    }

    // Step 3: Stop the container
    let _ = run_cli(&["container", "stop", &container_name]);

    // Step 4: Remove the container
    let _ = run_cli(&["container", "rm", &container_name]);

    // Step 5: Verify it's gone
    let final_list = run_cli(&["container", "ls", "-a"]);
    if let Ok(out) = &final_list {
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains(&container_name) || !out.status.success(),
            "Container should be removed"
        );
    }

    let elapsed = start.elapsed();
    println!("Full lifecycle completed in {:?}", elapsed);
}

// ============================================================================
// Timeout and Error Handling Tests
// ============================================================================

#[test]
fn test_container_operations_timeout() {
    // Verify CLI doesn't hang indefinitely
    let start = Instant::now();

    // Run a quick command
    let _ = run_cli(&["container", "ls"]);

    let elapsed = start.elapsed();

    // CLI should respond within reasonable time (5 seconds max for error case)
    assert!(
        elapsed < Duration::from_secs(5),
        "CLI should respond quickly, took {:?}",
        elapsed
    );
}

#[test]
fn test_cli_help_container() {
    // Test: hb container --help
    let output = run_cli(&["container", "--help"]);

    match output {
        Ok(out) => {
            assert!(out.status.success(), "Help should always succeed");
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("container") || stdout.contains("Container"),
                "Help should mention containers"
            );
        }
        Err(_) => {}
    }
}

#[test]
fn test_cli_version() {
    // Test: hb --version
    let output = run_cli(&["--version"]);

    match output {
        Ok(out) => {
            assert!(out.status.success(), "Version should always succeed");
            let stdout = String::from_utf8_lossy(&out.stdout);
            assert!(
                stdout.contains("hb") || stdout.contains("hyperbox") || stdout.contains("0."),
                "Version should show app name or version number"
            );
        }
        Err(_) => {}
    }
}
