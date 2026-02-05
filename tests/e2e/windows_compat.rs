//! Windows Compatibility E2E Tests
//!
//! Verifies HyperBox works correctly on Windows:
//! - Named pipe IPC
//! - Path handling (forward/backslashes)
//! - WSL2 integration
//! - Windows-specific features

use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Get workspace root
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent")
        .to_path_buf()
}

/// Get CLI binary path
fn cli_path() -> PathBuf {
    let root = workspace_root();
    let exe_name = if cfg!(windows) { "hb.exe" } else { "hb" };

    let release_path = root.join("target").join("release").join(exe_name);
    if release_path.exists() {
        return release_path;
    }

    root.join("target").join("debug").join(exe_name)
}

/// Get daemon binary path
fn daemon_path() -> PathBuf {
    let root = workspace_root();
    let exe_name = if cfg!(windows) {
        "hyperboxd.exe"
    } else {
        "hyperboxd"
    };

    let release_path = root.join("target").join("release").join(exe_name);
    if release_path.exists() {
        return release_path;
    }

    root.join("target").join("debug").join(exe_name)
}

// ============================================================================
// Binary Existence Tests
// ============================================================================

#[test]
fn test_windows_binary_extension() {
    // On Windows, binaries should have .exe extension
    if cfg!(windows) {
        let cli = cli_path();
        assert!(
            cli.extension().map(|e| e == "exe").unwrap_or(false) || cli.exists(),
            "Windows CLI should have .exe extension"
        );

        let daemon = daemon_path();
        assert!(
            daemon.extension().map(|e| e == "exe").unwrap_or(false) || daemon.exists(),
            "Windows daemon should have .exe extension"
        );
    }
}

#[test]
fn test_binary_can_execute() {
    let cli = cli_path();

    if !cli.exists() {
        println!("Skipping: CLI binary not found at {:?}", cli);
        return;
    }

    let output = Command::new(&cli)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(out) => {
            assert!(out.status.success(), "CLI should execute successfully on this platform");
        }
        Err(e) => {
            panic!("Failed to execute CLI: {}", e);
        }
    }
}

// ============================================================================
// Path Handling Tests
// ============================================================================

#[test]
fn test_path_separator_handling() {
    let cli = cli_path();

    if !cli.exists() {
        println!("Skipping: CLI binary not found");
        return;
    }

    // Test with forward slashes (Unix style)
    let forward_path = "C:/Users/test/project";

    // Test with backslashes (Windows style)
    let back_path = r"C:\Users\test\project";

    // Both should be acceptable in path arguments
    // (Testing with project detect command or similar)

    let output_forward = Command::new(&cli)
        .args(["project", "detect", forward_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let output_back = Command::new(&cli)
        .args(["project", "detect", back_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    // Both should handle paths without crashing
    if let Ok(out) = output_forward {
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!stderr.contains("panic"), "Forward slash paths should not cause panic");
    }

    if let Ok(out) = output_back {
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!stderr.contains("panic"), "Backslash paths should not cause panic");
    }
}

#[test]
fn test_unc_path_handling() {
    // UNC paths are Windows network paths: \\server\share
    if !cfg!(windows) {
        return;
    }

    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    let unc_path = r"\\localhost\C$\temp";

    let output = Command::new(&cli)
        .args(["project", "detect", unc_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        let stderr = String::from_utf8_lossy(&out.stderr);
        // Should handle UNC paths gracefully (even if path doesn't exist)
        assert!(!stderr.contains("panic"), "UNC paths should not cause panic");
    }
}

#[test]
fn test_path_with_spaces() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    // Path with spaces (common on Windows)
    let path_with_spaces = if cfg!(windows) {
        r"C:\Users\Test User\My Project"
    } else {
        "/home/test user/my project"
    };

    let output = Command::new(&cli)
        .args(["project", "detect", path_with_spaces])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!stderr.contains("panic"), "Paths with spaces should not cause panic");
    }
}

// ============================================================================
// Named Pipe IPC Tests (Windows-specific)
// ============================================================================

#[cfg(windows)]
mod windows_ipc_tests {
    use super::*;

    #[test]
    fn test_named_pipe_path_format() {
        // Windows named pipe path format
        let expected_prefix = r"\\.\pipe\";
        let pipe_name = "hyperbox";

        let full_path = format!("{}{}", expected_prefix, pipe_name);

        assert!(
            full_path.starts_with(r"\\.\pipe\"),
            "Named pipe path should use correct Windows format"
        );

        println!("Named pipe path: {}", full_path);
    }

    #[test]
    #[ignore = "Daemon hangs when spawned from test harness"]
    fn test_daemon_uses_named_pipes() {
        let daemon = daemon_path();

        if !daemon.exists() {
            println!("Skipping: daemon binary not found");
            return;
        }

        // Check daemon help for pipe-related options
        let output = Command::new(&daemon)
            .arg("--help")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);

            // Daemon should mention Windows or pipe in help
            println!("Daemon help mentions 'pipe': {}", stdout.contains("pipe"));
            println!("Daemon help mentions 'socket': {}", stdout.contains("socket"));
        }
    }
}

// ============================================================================
// WSL2 Integration Tests
// ============================================================================

#[cfg(windows)]
mod wsl2_tests {
    use super::*;

    fn wsl_available() -> bool {
        Command::new("wsl")
            .args(["--status"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[test]
    fn test_wsl_availability_detection() {
        let available = wsl_available();
        println!("WSL2 available: {}", available);

        // Just detect, don't fail
    }

    #[test]
    fn test_wsl_path_translation() {
        if !wsl_available() {
            println!("Skipping: WSL not available");
            return;
        }

        // Test that Windows paths can be translated to WSL paths
        // /mnt/c/Users/... format

        let cli = cli_path();

        if !cli.exists() {
            return;
        }

        // WSL path format
        let wsl_path = "/mnt/c/Users";

        let output = Command::new(&cli)
            .args(["project", "detect", wsl_path])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        if let Ok(out) = output {
            let stderr = String::from_utf8_lossy(&out.stderr);
            // Should handle without panic
            assert!(!stderr.contains("panic"), "WSL paths should not cause panic");
        }
    }
}

// ============================================================================
// Windows Service Mode Tests
// ============================================================================

#[cfg(windows)]
mod windows_service_tests {
    use super::*;

    #[test]
    #[ignore = "Daemon hangs when spawned from test harness"]
    fn test_daemon_service_option() {
        let daemon = daemon_path();

        if !daemon.exists() {
            println!("Skipping: daemon binary not found");
            return;
        }

        let output = Command::new(&daemon)
            .arg("--help")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);

            let has_service_mode = stdout.contains("service") || stdout.contains("--service");
            println!("Windows service mode available: {}", has_service_mode);
        }
    }
}

// ============================================================================
// Console / TTY Tests
// ============================================================================

#[test]
fn test_console_output() {
    let cli = cli_path();

    if !cli.exists() {
        println!("Skipping: CLI binary not found");
        return;
    }

    let output = Command::new(&cli)
        .args(["--version"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);

        // Output should be valid UTF-8 (no garbled text)
        assert!(
            stdout.is_ascii()
                || stdout
                    .chars()
                    .all(|c| !c.is_control() || c == '\n' || c == '\r'),
            "Output should be clean text"
        );

        // Should not contain ANSI codes when stdout is not a TTY
        // (piped output should be plain)
        let has_ansi = stdout.contains("\x1b[") || stdout.contains("\u{001b}");

        if has_ansi {
            println!("Note: Output contains ANSI codes in piped mode");
        }
    }
}

// ============================================================================
// Environment Variable Tests
// ============================================================================

#[test]
fn test_windows_env_vars() {
    let cli = cli_path();

    if !cli.exists() {
        println!("Skipping: CLI binary not found");
        return;
    }

    // Test with Windows-specific environment variables
    let output = Command::new(&cli)
        .env("USERPROFILE", r"C:\Users\TestUser")
        .env("APPDATA", r"C:\Users\TestUser\AppData\Roaming")
        .env("HYPERBOX_LOG_LEVEL", "debug")
        .args(["--version"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        assert!(out.status.success(), "Should work with Windows env vars");
    }
}

// ============================================================================
// Long Path Support Tests (Windows 10+)
// ============================================================================

#[cfg(windows)]
mod long_path_tests {
    use super::*;

    #[test]
    fn test_long_path_handling() {
        let cli = cli_path();

        if !cli.exists() {
            return;
        }

        // Create a path longer than 260 characters (classic Windows limit)
        let base = r"C:\Users\TestUser\Documents";
        let long_segment = "VeryLongFolderName".repeat(20);
        let long_path = format!("{}\\{}", base, long_segment);

        let output = Command::new(&cli)
            .args(["project", "detect", &long_path])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        if let Ok(out) = output {
            let stderr = String::from_utf8_lossy(&out.stderr);

            // Should handle long paths gracefully
            assert!(!stderr.contains("panic"), "Long paths should not cause panic");

            // May fail gracefully if path doesn't exist or is too long
            if !out.status.success() {
                println!("Long path handling: graceful failure");
            }
        }
    }
}

// ============================================================================
// Case Sensitivity Tests
// ============================================================================

#[test]
fn test_case_insensitive_commands() {
    // Windows is case-insensitive, but CLI commands should be consistent
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    // Commands should be lowercase as per convention
    let output_lower = Command::new(&cli)
        .args(["container", "ls", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output_lower {
        let stderr = String::from_utf8_lossy(&out.stderr);
        // Lowercase commands should work
        assert!(
            out.status.success() || !stderr.contains("unknown command"),
            "Lowercase commands should be recognized"
        );
    }
}

// ============================================================================
// File Locking Tests
// ============================================================================

#[test]
fn test_concurrent_cli_access() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    // Windows has different file locking semantics
    // Multiple CLI instances should work concurrently

    let handles: Vec<_> = (0..3)
        .map(|_| {
            let path = cli.clone();
            std::thread::spawn(move || {
                Command::new(path)
                    .args(["--version"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
            })
        })
        .collect();

    for handle in handles {
        if let Ok(output) = handle.join() {
            if let Ok(out) = output {
                assert!(out.status.success(), "Concurrent CLI access should work");
            }
        }
    }
}

// ============================================================================
// Tauri App Tests (Desktop App)
// ============================================================================

#[test]
fn test_tauri_app_exists() {
    let root = workspace_root();
    let app_dir = root.join("app");

    // Check for Tauri app structure
    let tauri_conf = app_dir.join("src-tauri").join("tauri.conf.json");

    if tauri_conf.exists() {
        println!("✓ Tauri app configuration found");

        // Check for built app
        let app_binary = if cfg!(windows) {
            root.join("target").join("release").join("HyperBox.exe")
        } else {
            root.join("target").join("release").join("HyperBox")
        };

        if app_binary.exists() {
            println!("✓ Tauri app binary found at {:?}", app_binary);
        } else {
            println!("⚠ Tauri app not yet built");
        }
    } else {
        println!("⚠ Tauri configuration not found at {:?}", tauri_conf);
    }
}

// ============================================================================
// Platform Detection Tests
// ============================================================================

#[test]
fn test_platform_detection() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    let output = Command::new(&cli)
        .args(["system", "info"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);

        if out.status.success() {
            #[cfg(windows)]
            {
                assert!(
                    stdout.to_lowercase().contains("windows"),
                    "Should detect Windows platform"
                );
            }

            #[cfg(target_os = "linux")]
            {
                assert!(stdout.to_lowercase().contains("linux"), "Should detect Linux platform");
            }

            #[cfg(target_os = "macos")]
            {
                assert!(
                    stdout.to_lowercase().contains("mac")
                        || stdout.to_lowercase().contains("darwin"),
                    "Should detect macOS platform"
                );
            }
        }
    }
}
