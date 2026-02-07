//! Container runtime abstraction layer.
//!
//! This module provides a unified interface to multiple OCI-compatible container runtimes.
//!
//! # Runtime Selection
//!
//! HyperBox supports multiple runtimes with automatic platform detection:
//! - **Docker (Bollard)**: Default on Windows, works wherever Docker is installed
//! - **crun**: Recommended on Linux for best performance
//! - **youki**: Rust-based alternative on Linux
//! - **runc**: Reference OCI runtime
//! - **wasmtime**: WebAssembly workloads via Wasmtime CLI
//!
//! # Cross-Platform Architecture
//!
//! The runtime layer uses a unified trait (`ContainerRuntime`) that abstracts
//! away differences between backends. This enables:
//! - Windows: Docker Desktop via Bollard API
//! - Linux: Native OCI runtimes (crun, youki, runc)
//! - macOS: Docker Desktop via Bollard API

mod crun;
mod docker;
mod registry;
mod traits;
#[cfg(feature = "wasm")]
mod wasm;
#[cfg(feature = "youki")]
mod youki;

pub use crun::CrunRuntime;
pub use docker::DockerRuntime;
pub use registry::RuntimeRegistry;
pub use traits::{ContainerRuntime, ImageInfo, ProcessInfo};
#[cfg(feature = "wasm")]
pub use wasm::WasmRuntime;
#[cfg(feature = "youki")]
pub use youki::YoukiRuntime;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported runtime types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeType {
    /// Docker via Bollard API - works on all platforms
    Docker,
    /// crun - fast C-based OCI runtime (Linux)
    Crun,
    /// youki - Rust-based OCI runtime (Linux)
    Youki,
    /// runc - reference OCI runtime (Linux)
    Runc,
    /// Firecracker - microVM-based isolation (Linux)
    Firecracker,
    /// Wasmtime - WebAssembly workloads via Wasmtime CLI
    Wasm,
}

impl RuntimeType {
    /// Get the default binary name for this runtime.
    #[must_use]
    pub fn binary_name(&self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Crun => "crun",
            Self::Youki => "youki",
            Self::Runc => "runc",
            Self::Firecracker => "firecracker",
            Self::Wasm => "wasmtime",
        }
    }

    /// Get typical binary paths to search.
    #[must_use]
    pub fn search_paths(&self) -> Vec<PathBuf> {
        let binary = self.binary_name();
        match self {
            Self::Docker => vec![
                // Docker is accessed via API, not binary path
                PathBuf::from("/var/run/docker.sock"),
                PathBuf::from("//./pipe/docker_engine"),
            ],
            Self::Wasm => vec![
                PathBuf::from("/usr/bin/wasmtime"),
                PathBuf::from("/usr/local/bin/wasmtime"),
                PathBuf::from(format!(
                    "{}/.wasmtime/bin/wasmtime",
                    std::env::var("HOME").unwrap_or_default()
                )),
                PathBuf::from("C:\\Program Files\\Wasmtime\\bin\\wasmtime.exe"),
            ],
            _ => vec![
                PathBuf::from(format!("/usr/bin/{binary}")),
                PathBuf::from(format!("/usr/local/bin/{binary}")),
                PathBuf::from(format!("/opt/hyperbox/bin/{binary}")),
                PathBuf::from(format!("C:\\Program Files\\HyperBox\\bin\\{binary}.exe")),
            ],
        }
    }

    /// Check if this runtime is available on the current platform.
    #[must_use]
    pub fn is_platform_compatible(&self) -> bool {
        match self {
            Self::Docker => true, // Docker works everywhere
            Self::Crun | Self::Youki | Self::Runc => cfg!(unix),
            Self::Firecracker => cfg!(target_os = "linux"),
            Self::Wasm => true, // WASM runs on all platforms
        }
    }

    /// Get the recommended runtime for the current platform.
    #[must_use]
    pub fn recommended() -> Self {
        if cfg!(windows) || cfg!(target_os = "macos") {
            Self::Docker
        } else {
            Self::Crun
        }
    }
}

impl std::fmt::Display for RuntimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Docker => write!(f, "docker"),
            Self::Crun => write!(f, "crun"),
            Self::Youki => write!(f, "youki"),
            Self::Runc => write!(f, "runc"),
            Self::Firecracker => write!(f, "firecracker"),
            Self::Wasm => write!(f, "wasm"),
        }
    }
}

/// Runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Runtime type
    pub runtime_type: RuntimeType,
    /// Path to runtime binary (or socket for Docker)
    pub binary_path: Option<PathBuf>,
    /// Root directory for runtime state
    pub root_dir: PathBuf,
    /// Enable debug logging
    pub debug: bool,
    /// Timeout for operations
    pub timeout_seconds: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            runtime_type: RuntimeType::recommended(),
            binary_path: None,
            root_dir: if cfg!(windows) {
                PathBuf::from("C:\\ProgramData\\HyperBox\\runtime")
            } else {
                PathBuf::from("/var/lib/hyperbox/runtime")
            },
            debug: false,
            timeout_seconds: 30,
        }
    }
}
