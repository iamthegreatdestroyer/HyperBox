//! # HyperBox Core
//!
//! Core container runtime abstraction layer for HyperBox.
//! Provides a unified interface to multiple container runtimes (crun, youki, runc, Firecracker).
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    ContainerRuntime Trait                   │
//! ├─────────────────────────────────────────────────────────────┤
//! │  CrunRuntime  │  YoukiRuntime  │  FirecrackerRuntime  │ ... │
//! ├─────────────────────────────────────────────────────────────┤
//! │            Isolation Layer (cgroups v2, namespaces)         │
//! ├─────────────────────────────────────────────────────────────┤
//! │            Storage Layer (composefs, image layers)          │
//! ├─────────────────────────────────────────────────────────────┤
//! │            Network Layer (eBPF, CNI integration)            │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Performance Targets
//!
//! | Operation | Target | Docker Baseline |
//! |-----------|--------|-----------------|
//! | Container create | <30ms | 150ms |
//! | Container start | <20ms | 75ms |
//! | Container lifecycle | <50ms | 225ms |

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub mod error;
pub mod isolation;
pub mod network;
pub mod runtime;
pub mod storage;
pub mod types;

pub use error::{CoreError, Result};
pub use runtime::{ContainerRuntime, RuntimeConfig, RuntimeType};
pub use types::*;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default runtime to use when not specified
pub const DEFAULT_RUNTIME: RuntimeType = RuntimeType::Crun;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::error::{CoreError, Result};
    pub use crate::runtime::{ContainerRuntime, RuntimeConfig, RuntimeType};
    pub use crate::types::{
        ContainerId, ContainerSpec, ContainerState, ContainerStats, ExecResult, ExecSpec, ImageRef,
        LogOptions,
    };
}
