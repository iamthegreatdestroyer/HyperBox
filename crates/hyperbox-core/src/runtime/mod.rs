//! Container runtime abstraction layer.
//!
//! This module provides a unified interface to multiple OCI-compatible container runtimes.

mod crun;
mod registry;
mod traits;

pub use crun::CrunRuntime;
pub use registry::RuntimeRegistry;
pub use traits::ContainerRuntime;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported runtime types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeType {
    /// crun - fast C-based OCI runtime
    Crun,
    /// youki - Rust-based OCI runtime
    Youki,
    /// runc - reference OCI runtime
    Runc,
    /// Firecracker - microVM-based isolation
    Firecracker,
}

impl RuntimeType {
    /// Get the default binary name for this runtime.
    #[must_use]
    pub fn binary_name(&self) -> &'static str {
        match self {
            Self::Crun => "crun",
            Self::Youki => "youki",
            Self::Runc => "runc",
            Self::Firecracker => "firecracker",
        }
    }

    /// Get typical binary paths to search.
    #[must_use]
    pub fn search_paths(&self) -> Vec<PathBuf> {
        let binary = self.binary_name();
        vec![
            PathBuf::from(format!("/usr/bin/{binary}")),
            PathBuf::from(format!("/usr/local/bin/{binary}")),
            PathBuf::from(format!("/opt/hyperbox/bin/{binary}")),
            PathBuf::from(format!("C:\\Program Files\\HyperBox\\bin\\{binary}.exe")),
        ]
    }
}

impl std::fmt::Display for RuntimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Crun => write!(f, "crun"),
            Self::Youki => write!(f, "youki"),
            Self::Runc => write!(f, "runc"),
            Self::Firecracker => write!(f, "firecracker"),
        }
    }
}

/// Runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Runtime type
    pub runtime_type: RuntimeType,
    /// Path to runtime binary
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
            runtime_type: RuntimeType::Crun,
            binary_path: None,
            root_dir: PathBuf::from("/var/lib/hyperbox/runtime"),
            debug: false,
            timeout_seconds: 30,
        }
    }
}
