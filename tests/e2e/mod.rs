//! HyperBox End-to-End (E2E) Test Suite
//!
//! This module contains comprehensive E2E tests that verify the complete
//! HyperBox system works correctly from CLI through daemon to runtime.
//!
//! ## Test Categories
//!
//! - **Container Lifecycle**: Create, start, stop, remove containers
//! - **Image Operations**: Pull, build, push, remove images
//! - **Project Management**: Auto-detection, compose, hot-reload
//! - **System Operations**: Info, prune, events
//! - **Performance**: Cold/warm start, memory overhead
//!
//! ## Running E2E Tests
//!
//! ```bash
//! # Run all E2E tests
//! cargo test --test e2e -- --nocapture
//!
//! # Run specific test
//! cargo test --test e2e test_container_lifecycle -- --nocapture
//! ```

pub mod container_lifecycle;
pub mod daemon_operations;
pub mod docker_compat;
pub mod performance;
pub mod windows_compat;
