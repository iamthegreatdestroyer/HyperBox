//! Memory management and monitoring module.
//!
//! Provides kernel-level memory pressure detection and dynamic swap tuning.
//! Monitors Pressure Stall Information (PSI) metrics and automatically adjusts
//! system swappiness based on real-time memory demand.

pub mod psi;

pub use psi::{PSIMemory, PSIMonitor, SwapTuner};
