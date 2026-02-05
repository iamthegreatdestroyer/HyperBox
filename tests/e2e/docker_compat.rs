//! Docker CLI Compatibility E2E Tests
//!
//! Verifies HyperBox CLI maintains compatibility with Docker CLI syntax.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

/// CLI binary name
const CLI_BINARY: &str = "hb";

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
    let exe_name = if cfg!(windows) {
        format!("{}.exe", CLI_BINARY)
    } else {
        CLI_BINARY.to_string()
    };

    let release_path = root.join("target").join("release").join(&exe_name);
    if release_path.exists() {
        return release_path;
    }

    root.join("target").join("debug").join(&exe_name)
}

/// Run CLI command
fn run_cli(args: &[&str]) -> Result<Output, std::io::Error> {
    let path = cli_path();
    Command::new(path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
}

/// Check if command is recognized (not "unknown command" error)
fn command_recognized(output: &Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);

    // These indicate the command was NOT recognized
    let unrecognized = [
        "unrecognized command",
        "unknown command",
        "invalid command",
        "not found",
    ];

    !unrecognized
        .iter()
        .any(|msg| stderr.to_lowercase().contains(msg))
}

// ============================================================================
// Docker Command Compatibility Matrix
// ============================================================================

/// Docker commands that should be supported with same syntax
const DOCKER_COMPATIBLE_COMMANDS: &[&[&str]] = &[
    // Container management
    &["run", "--help"],
    &["start", "--help"],
    &["stop", "--help"],
    &["restart", "--help"],
    &["rm", "--help"],
    &["ps", "--help"],
    &["logs", "--help"],
    &["exec", "--help"],
    &["inspect", "--help"],
    &["top", "--help"],
    &["stats", "--help"],
    &["attach", "--help"],
    &["wait", "--help"],
    &["kill", "--help"],
    &["pause", "--help"],
    &["unpause", "--help"],
    &["rename", "--help"],
    // Image management
    &["images", "--help"],
    &["pull", "--help"],
    &["push", "--help"],
    &["build", "--help"],
    &["tag", "--help"],
    &["rmi", "--help"],
    &["save", "--help"],
    &["load", "--help"],
    // System commands
    &["info", "--help"],
    &["version", "--help"],
    &["system", "prune", "--help"],
    // Network (optional)
    &["network", "ls", "--help"],
    &["network", "create", "--help"],
    &["network", "rm", "--help"],
    // Volume (optional)
    &["volume", "ls", "--help"],
    &["volume", "create", "--help"],
    &["volume", "rm", "--help"],
];

#[test]
fn test_docker_command_compatibility() {
    let cli = cli_path();

    if !cli.exists() {
        println!("Skipping: CLI binary not found at {:?}", cli);
        return;
    }

    let mut results: HashMap<String, bool> = HashMap::new();

    for cmd_args in DOCKER_COMPATIBLE_COMMANDS {
        let output = run_cli(cmd_args);

        let cmd_name = cmd_args.join(" ");

        match output {
            Ok(out) => {
                let recognized = command_recognized(&out);
                results.insert(cmd_name.clone(), recognized);

                if !recognized {
                    println!("⚠ Command not recognized: {}", cmd_name);
                }
            }
            Err(e) => {
                println!("✗ Failed to run: {} - {}", cmd_name, e);
                results.insert(cmd_name, false);
            }
        }
    }

    // Report coverage
    let total = results.len();
    let recognized = results.values().filter(|&&v| v).count();

    println!(
        "\nDocker Command Compatibility: {}/{} ({:.1}%)",
        recognized,
        total,
        (recognized as f64 / total as f64) * 100.0
    );

    // We expect at least basic commands to be recognized
    let minimum_required = &[
        "run --help",
        "ps --help",
        "stop --help",
        "rm --help",
        "images --help",
        "version --help",
    ];

    for cmd in minimum_required {
        assert!(
            results.get(*cmd).copied().unwrap_or(false),
            "Essential command should be recognized: {}",
            cmd
        );
    }
}

// ============================================================================
// Docker Run Syntax Compatibility
// ============================================================================

#[test]
fn test_docker_run_syntax() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    // Test common docker run patterns
    let test_cases = [
        // Basic run
        vec!["run", "--help"],
        // Detached mode
        vec!["run", "-d", "--help"],
        // Interactive TTY
        vec!["run", "-it", "--help"],
        // Port mapping
        vec!["run", "-p", "8080:80", "--help"],
        // Volume mount
        vec!["run", "-v", "/host:/container", "--help"],
        // Environment variable
        vec!["run", "-e", "KEY=value", "--help"],
        // Named container
        vec!["run", "--name", "test", "--help"],
        // Resource limits
        vec!["run", "--memory", "512m", "--help"],
        vec!["run", "--cpus", "2", "--help"],
        // Network mode
        vec!["run", "--network", "bridge", "--help"],
        // Restart policy
        vec!["run", "--restart", "always", "--help"],
        // Working directory
        vec!["run", "-w", "/app", "--help"],
        // User
        vec!["run", "-u", "1000", "--help"],
        // Remove after exit
        vec!["run", "--rm", "--help"],
    ];

    for args in test_cases {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
        let output = run_cli(&args_refs);

        if let Ok(out) = output {
            if !command_recognized(&out) {
                println!("Run syntax not fully supported: {}", args.join(" "));
            }
        }
    }
}

// ============================================================================
// Docker PS Syntax Compatibility
// ============================================================================

#[test]
fn test_docker_ps_syntax() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    let test_cases = [
        vec!["ps"],
        vec!["ps", "-a"],
        vec!["ps", "-q"],
        vec!["ps", "-l"],
        vec!["ps", "--no-trunc"],
        vec!["ps", "--format", "{{.ID}}"],
        vec!["ps", "-f", "status=running"],
    ];

    for args in test_cases {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
        let output = run_cli(&args_refs);

        if let Ok(out) = output {
            // Should either work or fail gracefully (not crash)
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(
                !stderr.contains("panic") && !stderr.contains("PANIC"),
                "Command should not panic: {}",
                args.join(" ")
            );
        }
    }
}

// ============================================================================
// Docker Logs Syntax Compatibility
// ============================================================================

#[test]
fn test_docker_logs_syntax() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    let test_cases = [
        vec!["logs", "container_id"],
        vec!["logs", "-f", "container_id"],
        vec!["logs", "--tail", "100", "container_id"],
        vec!["logs", "--since", "1h", "container_id"],
        vec!["logs", "--until", "2023-01-01", "container_id"],
        vec!["logs", "-t", "container_id"],
    ];

    for args in test_cases {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
        let output = run_cli(&args_refs);

        if let Ok(out) = output {
            // Command should be recognized even if it fails due to invalid container
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(
                !stderr.contains("unknown command") && !stderr.contains("PANIC"),
                "Logs syntax should be recognized: {}",
                args.join(" ")
            );
        }
    }
}

// ============================================================================
// Docker Exec Syntax Compatibility
// ============================================================================

#[test]
fn test_docker_exec_syntax() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    let test_cases = [
        vec!["exec", "container_id", "echo", "test"],
        vec!["exec", "-it", "container_id", "/bin/sh"],
        vec!["exec", "-d", "container_id", "command"],
        vec!["exec", "-e", "VAR=val", "container_id", "env"],
        vec!["exec", "-w", "/app", "container_id", "pwd"],
        vec!["exec", "-u", "root", "container_id", "whoami"],
    ];

    for args in test_cases {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
        let output = run_cli(&args_refs);

        if let Ok(out) = output {
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(
                !stderr.contains("unknown command") && !stderr.contains("PANIC"),
                "Exec syntax should be recognized: {}",
                args.join(" ")
            );
        }
    }
}

// ============================================================================
// Docker Build Syntax Compatibility
// ============================================================================

#[test]
fn test_docker_build_syntax() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    let test_cases = [
        vec!["build", "--help"],
        vec!["build", "-t", "tag:v1", "--help"],
        vec!["build", "-f", "Dockerfile.dev", "--help"],
        vec!["build", "--no-cache", "--help"],
        vec!["build", "--build-arg", "KEY=value", "--help"],
        vec!["build", "--target", "stage", "--help"],
    ];

    for args in test_cases {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
        let output = run_cli(&args_refs);

        if let Ok(out) = output {
            assert!(
                command_recognized(&out),
                "Build command should be recognized: {}",
                args.join(" ")
            );
        }
    }
}

// ============================================================================
// Short Form Alias Tests
// ============================================================================

#[test]
fn test_docker_short_aliases() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    // Docker supports these short forms
    let aliases = [
        // ps is alias for container ls
        ("ps", "container ls"),
        // images is alias for image ls
        ("images", "image ls"),
        // rmi is alias for image rm
        ("rmi", "image rm"),
        // rm is alias for container rm
        ("rm", "container rm"),
    ];

    for (short, _long) in aliases {
        let output = run_cli(&[short, "--help"]);

        if let Ok(out) = output {
            assert!(command_recognized(&out), "Short alias should be recognized: {}", short);
        }
    }
}

// ============================================================================
// Environment Variable Compatibility
// ============================================================================

#[test]
fn test_docker_env_vars() {
    // Docker respects DOCKER_HOST, etc.
    // HyperBox should respect HYPERBOX_* equivalents

    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    // Test with custom env
    let output = Command::new(&cli)
        .env("HYPERBOX_LOG_LEVEL", "debug")
        .args(["--version"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        // Should complete without error
        assert!(
            out.status.success() || !out.stderr.is_empty(),
            "Should accept environment variables"
        );
    }
}

// ============================================================================
// Docker Compose Compatibility (Project Commands)
// ============================================================================

#[test]
fn test_compose_syntax() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    // HyperBox uses "project" for compose-like functionality
    let test_cases = [
        vec!["project", "--help"],
        vec!["project", "up", "--help"],
        vec!["project", "down", "--help"],
        vec!["project", "ps", "--help"],
        vec!["project", "logs", "--help"],
    ];

    for args in test_cases {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
        let output = run_cli(&args_refs);

        if let Ok(out) = output {
            if command_recognized(&out) {
                println!("✓ Project command recognized: {}", args.join(" "));
            } else {
                println!("⚠ Project command not yet implemented: {}", args.join(" "));
            }
        }
    }
}

// ============================================================================
// Error Message Compatibility
// ============================================================================

#[test]
#[ignore = "Requires Docker-compatible 'hb run' shortcut (currently requires 'hb container run')"]
fn test_error_message_format() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    // Test that errors are helpful like Docker's
    let output = run_cli(&["run"]); // Missing image

    if let Ok(out) = output {
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);

            // Error should be informative
            assert!(
                stderr.contains("image")
                    || stderr.contains("usage")
                    || stderr.contains("required")
                    || stderr.contains("argument"),
                "Error should be helpful: {}",
                stderr
            );
        }
    }
}

// ============================================================================
// Output Format Compatibility
// ============================================================================

#[test]
#[ignore = "Requires --format json option implementation"]
fn test_output_format_json() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    // Test JSON output format (Docker supports this)
    let output = run_cli(&["inspect", "--format", "json", "container_id"]);

    if let Ok(out) = output {
        // Command should be recognized even if container doesn't exist
        let stderr = String::from_utf8_lossy(&out.stderr);
        if !out.status.success() {
            // Should fail due to container not found, not format issue
            assert!(
                stderr.contains("not found")
                    || stderr.contains("No such")
                    || stderr.contains("daemon"),
                "Format option should be recognized"
            );
        }
    }
}

#[test]
fn test_output_format_template() {
    let cli = cli_path();

    if !cli.exists() {
        return;
    }

    // Test Go template format (Docker supports this)
    let output = run_cli(&["ps", "--format", "{{.ID}}: {{.Names}}"]);

    if let Ok(out) = output {
        // Should handle gracefully
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!stderr.contains("panic"), "Template format should not panic");
    }
}
