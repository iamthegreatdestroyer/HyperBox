//! # HyperBox Daemon
//!
//! Background daemon service for HyperBox container runtime.
//! Provides eBPF tracing, observability, and gRPC API.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub mod observability;

pub use observability::{SpanGenerator, SpanContext, SpanStatus};

/// Daemon version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
