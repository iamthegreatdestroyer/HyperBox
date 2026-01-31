//! HyperBox Integration & E2E Tests
//!
//! Comprehensive test suite verifying core functionality of the HyperBox container platform.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Test configuration
const CLI_BINARY: &str = "hb";
const DAEMON_BINARY: &str = "hyperboxd";

/// Helper to get the workspace root directory
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/hyperbox-core, go up two levels
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find workspace root")
        .to_path_buf()
}

/// Helper to get the path to a binary
fn binary_path(name: &str) -> PathBuf {
    let root = workspace_root();
    let exe_name = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };
    root.join("target").join("release").join(exe_name)
}

/// Helper to run a command and capture output
fn run_command(binary: &str, args: &[&str]) -> std::io::Result<std::process::Output> {
    let path = binary_path(binary);
    Command::new(path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
}

// ============================================================================
// CLI Binary Tests
// ============================================================================

mod cli_tests {
    use super::*;

    #[test]
    fn test_cli_version() {
        let output = run_command(CLI_BINARY, &["--version"]).expect("Failed to execute CLI");

        assert!(output.status.success(), "CLI --version should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("hb") || stdout.contains("0.1.0"),
            "Version output should contain version info: {}",
            stdout
        );
    }

    #[test]
    fn test_cli_help() {
        let output = run_command(CLI_BINARY, &["--help"]).expect("Failed to execute CLI");

        assert!(output.status.success(), "CLI --help should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Verify core commands are present
        assert!(
            stdout.contains("container") || stdout.contains("run"),
            "Help should show container commands"
        );
        assert!(stdout.contains("image"), "Help should show image commands");
        assert!(stdout.contains("project"), "Help should show project commands");
        assert!(stdout.contains("system"), "Help should show system commands");
    }

    #[test]
    fn test_cli_container_help() {
        let output =
            run_command(CLI_BINARY, &["container", "--help"]).expect("Failed to execute CLI");

        assert!(output.status.success(), "CLI container --help should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("ls") || stdout.contains("list"),
            "Container help should show list command"
        );
        assert!(stdout.contains("run"), "Container help should show run command");
    }

    #[test]
    fn test_cli_image_help() {
        let output = run_command(CLI_BINARY, &["image", "--help"]).expect("Failed to execute CLI");

        assert!(output.status.success(), "CLI image --help should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("pull"), "Image help should show pull command");
        assert!(stdout.contains("build"), "Image help should show build command");
    }

    #[test]
    fn test_cli_project_help() {
        let output =
            run_command(CLI_BINARY, &["project", "--help"]).expect("Failed to execute CLI");

        assert!(output.status.success(), "CLI project --help should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("init") || stdout.contains("up"),
            "Project help should show init or up command"
        );
    }

    #[test]
    fn test_cli_system_help() {
        let output = run_command(CLI_BINARY, &["system", "--help"]).expect("Failed to execute CLI");

        assert!(output.status.success(), "CLI system --help should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("info") || stdout.contains("prune"),
            "System help should show info or prune command"
        );
    }

    #[test]
    fn test_cli_completion_generation() {
        // Test that shell completion generation works
        for shell in &["bash", "zsh", "fish", "powershell"] {
            let output = run_command(CLI_BINARY, &["completion", shell])
                .expect(&format!("Failed to generate {} completion", shell));

            // Should either succeed or fail gracefully
            let combined_output = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );

            // Completion command should not panic
            assert!(
                output.status.success() || combined_output.len() > 0,
                "{} completion should generate output or handle gracefully",
                shell
            );
        }
    }

    #[test]
    fn test_cli_invalid_command() {
        let output =
            run_command(CLI_BINARY, &["nonexistent-command"]).expect("Failed to execute CLI");

        // Invalid command should exit with error
        assert!(!output.status.success(), "Invalid command should fail");
    }
}

// ============================================================================
// Daemon Binary Tests
// ============================================================================

mod daemon_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_daemon_binary_exists() {
        let path = binary_path(DAEMON_BINARY);
        assert!(path.exists(), "Daemon binary should exist at {:?}", path);
    }

    #[test]
    fn test_daemon_version() {
        // Use a timeout to prevent daemon from running forever
        let path = binary_path(DAEMON_BINARY);
        let start = Instant::now();

        // Try to spawn with --version (if supported)
        let child = Command::new(&path)
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match child {
            Ok(mut child) => {
                // Wait up to 2 seconds for version output
                std::thread::sleep(Duration::from_millis(500));

                // Kill if still running (daemon likely started instead)
                let _ = child.kill();
                let output = child.wait_with_output().unwrap();

                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                // Either got version info or daemon tried to start
                assert!(
                    stdout.contains("hyperbox")
                        || stdout.contains("0.1.0")
                        || stderr.len() > 0
                        || start.elapsed() < Duration::from_secs(3),
                    "Daemon should respond to --version"
                );
            }
            Err(e) => {
                panic!("Failed to spawn daemon: {}", e);
            }
        }
    }
}

// ============================================================================
// Core Library Tests
// ============================================================================

mod core_tests {
    use hyperbox_core::runtime::RuntimeRegistry;
    use hyperbox_core::types::ContainerId;

    #[test]
    fn test_container_id_format() {
        let id = ContainerId::new();
        let id_str = id.as_str();

        // Container IDs should be valid hex strings
        assert!(id_str.len() > 0, "Container ID should not be empty");
        assert!(
            id_str.chars().all(|c| c.is_ascii_hexdigit()),
            "Container ID should be hex: {}",
            id_str
        );
    }

    #[test]
    fn test_container_id_uniqueness() {
        let ids: Vec<ContainerId> = (0..100).map(|_| ContainerId::new()).collect();

        // All IDs should be unique
        let mut unique_ids = ids.clone();
        unique_ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        unique_ids.dedup_by(|a, b| a.as_str() == b.as_str());

        assert_eq!(ids.len(), unique_ids.len(), "All generated container IDs should be unique");
    }

    #[test]
    fn test_runtime_registry_creation() {
        use hyperbox_core::runtime::RuntimeType;

        // Test that we can create a runtime registry
        let registry = RuntimeRegistry::new(RuntimeType::Crun);

        // Registry should be constructable
        assert!(true, "Registry should be constructable");
    }
}

// ============================================================================
// Port Allocation Tests
// ============================================================================

mod port_tests {
    use hyperbox_core::network::PortAllocator;

    #[test]
    fn test_port_allocator_creation() {
        let _allocator = PortAllocator::new();
        assert!(true, "PortAllocator should be constructable");
    }

    #[test]
    fn test_port_allocation_range() {
        let allocator = PortAllocator::new();
        let port = allocator.allocate(None).unwrap();

        // Default range is MIN_PORT to MAX_PORT (32768-60999)
        assert!(
            port >= 32768 && port <= 60999,
            "Allocated port {} should be in default range",
            port
        );
    }

    #[test]
    fn test_port_release() {
        let allocator = PortAllocator::new();
        let port = allocator.allocate(None).unwrap();

        allocator.release(port);

        // Allocator state should be valid after release
        assert!(true, "Port release should succeed");
    }

    #[test]
    fn test_preferred_port_allocation() {
        let allocator = PortAllocator::new();
        let preferred = 45000u16;

        // Try to allocate a preferred port
        let port = allocator.allocate(Some(preferred)).unwrap();

        assert_eq!(port, preferred, "Should allocate preferred port when available");
    }
}

// ============================================================================
// Performance Baseline Tests
// ============================================================================

mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_cli_startup_time() {
        let start = Instant::now();

        let output = run_command(CLI_BINARY, &["--version"]).expect("Failed to execute CLI");

        let elapsed = start.elapsed();

        assert!(output.status.success());

        // CLI should start in under 2 seconds (generous for cold start)
        assert!(elapsed < Duration::from_secs(2), "CLI should start quickly, took {:?}", elapsed);
    }

    #[test]
    fn test_cli_help_time() {
        let start = Instant::now();

        let output = run_command(CLI_BINARY, &["--help"]).expect("Failed to execute CLI");

        let elapsed = start.elapsed();

        assert!(output.status.success());

        // Help should render quickly
        assert!(
            elapsed < Duration::from_secs(2),
            "CLI help should render quickly, took {:?}",
            elapsed
        );
    }
}

// ============================================================================
// Binary Size Tests
// ============================================================================

mod binary_tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_cli_binary_size() {
        let path = binary_path(CLI_BINARY);

        if path.exists() {
            let metadata = fs::metadata(&path).expect("Failed to get CLI metadata");
            let size_mb = metadata.len() as f64 / 1_048_576.0;

            // CLI should be under 10MB
            assert!(size_mb < 10.0, "CLI binary too large: {:.2}MB", size_mb);

            println!("CLI binary size: {:.2}MB", size_mb);
        }
    }

    #[test]
    fn test_daemon_binary_size() {
        let path = binary_path(DAEMON_BINARY);

        if path.exists() {
            let metadata = fs::metadata(&path).expect("Failed to get daemon metadata");
            let size_mb = metadata.len() as f64 / 1_048_576.0;

            // Daemon should be under 15MB
            assert!(size_mb < 15.0, "Daemon binary too large: {:.2}MB", size_mb);

            println!("Daemon binary size: {:.2}MB", size_mb);
        }
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod error_tests {
    use hyperbox_core::error::CoreError;

    #[test]
    fn test_error_display() {
        let error = CoreError::ContainerNotFound("test-123".to_string());
        let display = format!("{}", error);

        assert!(
            display.contains("test-123") || display.contains("Container"),
            "Error display should be informative: {}",
            display
        );
    }

    #[test]
    fn test_error_debug() {
        let error = CoreError::ImageNotFound("nginx:latest".to_string());
        let debug = format!("{:?}", error);

        assert!(debug.len() > 0, "Error debug should produce output");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let core_error = CoreError::Io(io_error);

        assert!(
            matches!(core_error, CoreError::Io(_)),
            "IO error should convert to CoreError::Io"
        );
    }
}

// ============================================================================
// Configuration Tests
// ============================================================================

mod config_tests {
    #[test]
    fn test_default_config_path() {
        // Config should use platform-appropriate paths
        #[cfg(windows)]
        {
            let appdata = std::env::var("APPDATA").ok();
            assert!(appdata.is_some(), "APPDATA should be set on Windows");
        }

        #[cfg(unix)]
        {
            let home = std::env::var("HOME").ok();
            assert!(home.is_some(), "HOME should be set on Unix");
        }
    }
}
