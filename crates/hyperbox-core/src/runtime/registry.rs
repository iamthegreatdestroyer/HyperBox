//! Runtime registry for managing multiple container runtimes.

use crate::error::{CoreError, Result};
use crate::runtime::{ContainerRuntime, CrunRuntime, RuntimeConfig, RuntimeType};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::info;

/// Registry for managing container runtime implementations.
///
/// The registry provides a centralized place to manage multiple runtime
/// implementations and select the appropriate one based on configuration
/// or container requirements.
pub struct RuntimeRegistry {
    runtimes: DashMap<RuntimeType, Arc<dyn ContainerRuntime>>,
    default_runtime: RuntimeType,
}

impl RuntimeRegistry {
    /// Create a new runtime registry.
    #[must_use]
    pub fn new(default_runtime: RuntimeType) -> Self {
        Self {
            runtimes: DashMap::new(),
            default_runtime,
        }
    }

    /// Initialize the registry with available runtimes.
    pub async fn initialize(&self, config: &RuntimeConfig) -> Result<()> {
        info!("Initializing runtime registry");

        // Try to initialize crun
        match CrunRuntime::new(config.clone()).await {
            Ok(runtime) => {
                self.register(RuntimeType::Crun, Arc::new(runtime));
                info!("Registered crun runtime");
            }
            Err(e) => {
                tracing::warn!("crun not available: {}", e);
            }
        }

        // Would also try youki, runc, firecracker here

        if self.runtimes.is_empty() {
            return Err(CoreError::RuntimeNotAvailable {
                runtime: "any".to_string(),
                path: std::path::PathBuf::from("/usr/bin/crun"),
            });
        }

        Ok(())
    }

    /// Register a runtime implementation.
    pub fn register(&self, runtime_type: RuntimeType, runtime: Arc<dyn ContainerRuntime>) {
        self.runtimes.insert(runtime_type, runtime);
    }

    /// Get a runtime by type.
    pub fn get(&self, runtime_type: RuntimeType) -> Option<Arc<dyn ContainerRuntime>> {
        self.runtimes.get(&runtime_type).map(|r| Arc::clone(&r))
    }

    /// Get the default runtime.
    pub fn default(&self) -> Option<Arc<dyn ContainerRuntime>> {
        self.get(self.default_runtime)
            .or_else(|| self.runtimes.iter().next().map(|r| Arc::clone(r.value())))
    }

    /// List available runtimes.
    pub fn available(&self) -> Vec<RuntimeType> {
        self.runtimes.iter().map(|r| *r.key()).collect()
    }

    /// Check if a runtime is available.
    pub fn has(&self, runtime_type: RuntimeType) -> bool {
        self.runtimes.contains_key(&runtime_type)
    }
}

impl Default for RuntimeRegistry {
    fn default() -> Self {
        Self::new(RuntimeType::Crun)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = RuntimeRegistry::new(RuntimeType::Crun);
        // New registry with no runtimes registered should have empty available list
        assert!(registry.available().is_empty());
    }

    #[test]
    fn test_default_registry() {
        let registry = <RuntimeRegistry as Default>::default();
        assert!(registry.available().is_empty());
        assert!(registry.default().is_none()); // No runtimes registered yet
    }
}
