//! Container orchestration - bridges parsed compose specs to runtime operations.
//!
//! This module provides the conversion and orchestration logic to translate
//! project container definitions (from Compose files) into actual runtime operations.

use crate::config::{ContainerDef, PortDef, ResourceDef};
use crate::error::{ProjectError, Result};
use crate::Project;
use hyperbox_core::runtime::ContainerRuntime;
use hyperbox_core::types::{
    ContainerId, ContainerSpec, ImageRef, Mount, MountType, PortMapping, Protocol, ResourceLimits,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Orchestrator for managing project containers via a runtime.
pub struct ProjectOrchestrator {
    runtime: Arc<dyn ContainerRuntime>,
}

impl ProjectOrchestrator {
    /// Create a new project orchestrator with the given runtime.
    pub fn new(runtime: Arc<dyn ContainerRuntime>) -> Self {
        Self { runtime }
    }

    /// Start all containers for a project in dependency order.
    ///
    /// Returns the list of started container IDs.
    pub async fn start_project(&self, project: &Project) -> Result<Vec<ContainerId>> {
        let containers = &project.config.containers;
        if containers.is_empty() {
            info!("Project {} has no containers to start", project.name);
            return Ok(Vec::new());
        }

        // Build dependency graph and get topological order
        let order = self.topological_sort(containers)?;
        info!(
            "Starting {} containers for project {} in order: {:?}",
            containers.len(),
            project.name,
            order
        );

        let mut started_ids = Vec::new();

        for container_name in order {
            let container_def = containers
                .iter()
                .find(|c| c.name == container_name)
                .ok_or_else(|| ProjectError::ContainerNotFound(container_name.clone()))?;

            // Convert to runtime spec
            let spec = self.container_def_to_spec(container_def, project)?;

            debug!(
                "Creating container: {} (image: {})",
                spec.name.as_deref().unwrap_or("unnamed"),
                spec.image
            );

            // Create and start container
            match self.runtime.create(spec).await {
                Ok(container_id) => {
                    debug!("Container {} created with ID: {}", container_name, container_id);

                    if let Err(e) = self.runtime.start(&container_id).await {
                        error!("Failed to start container {}: {}", container_name, e);
                        // Rollback: stop previously started containers
                        self.stop_containers(&started_ids).await;
                        return Err(ProjectError::ContainerStart {
                            container: container_name.clone(),
                            reason: e.to_string(),
                        });
                    }

                    info!(
                        "Started container {} ({}) on ports {:?}",
                        container_name,
                        container_id.short(),
                        container_def
                            .ports
                            .iter()
                            .filter_map(|p| p.host)
                            .collect::<Vec<_>>()
                    );

                    started_ids.push(container_id);
                }
                Err(e) => {
                    error!("Failed to create container {}: {}", container_name, e);
                    // Rollback: stop previously started containers
                    self.stop_containers(&started_ids).await;
                    return Err(ProjectError::ContainerCreate {
                        container: container_name.clone(),
                        reason: e.to_string(),
                    });
                }
            }
        }

        Ok(started_ids)
    }

    /// Stop all containers for a project in reverse dependency order.
    pub async fn stop_project(&self, container_ids: &[ContainerId]) -> Result<()> {
        // Stop in reverse order (last started = first stopped)
        self.stop_containers(container_ids).await;
        Ok(())
    }

    /// Stop a list of containers gracefully.
    async fn stop_containers(&self, ids: &[ContainerId]) {
        for id in ids.iter().rev() {
            debug!("Stopping container: {}", id);
            if let Err(e) = self.runtime.stop(id, Duration::from_secs(10)).await {
                warn!("Failed to stop container {}: {}", id, e);
            }
        }
    }

    /// Remove a list of containers.
    pub async fn remove_containers(&self, ids: &[ContainerId]) -> Result<()> {
        for id in ids {
            debug!("Removing container: {}", id);
            if let Err(e) = self.runtime.remove(id).await {
                warn!("Failed to remove container {}: {}", id, e);
            }
        }
        Ok(())
    }

    /// Convert a project ContainerDef to a runtime ContainerSpec.
    pub fn container_def_to_spec(
        &self,
        def: &ContainerDef,
        project: &Project,
    ) -> Result<ContainerSpec> {
        // Build container name with project prefix
        let name = format!("{}-{}", project.name, def.name);

        // Parse image
        let image = ImageRef::parse(&def.image);

        // Convert ports
        let ports: Vec<PortMapping> = def
            .ports
            .iter()
            .map(|p| self.port_def_to_mapping(p))
            .collect();

        // Convert environment variables
        let env = def.environment.clone();

        // Convert volumes/mounts
        let mounts = self.parse_volume_mounts(&def.volumes, &project.root)?;

        // Convert resource limits
        let resources = self.resource_def_to_limits(def.resources.as_ref());

        // Build command
        let command = def.command.clone().unwrap_or_default();

        // Build labels
        let mut labels = HashMap::new();
        labels.insert("hyperbox.project".to_string(), project.name.clone());
        labels.insert("hyperbox.project.id".to_string(), project.id.to_string());
        labels.insert("hyperbox.service".to_string(), def.name.clone());

        Ok(ContainerSpec {
            name: Some(name),
            image,
            command,
            args: Vec::new(),
            env,
            working_dir: None,
            user: None,
            mounts,
            ports,
            resources,
            labels,
            restart_policy: hyperbox_core::types::RestartPolicy::No,
            hostname: None,
            privileged: false,
            read_only_rootfs: false,
            tty: false,
            stdin_open: false,
        })
    }

    /// Convert a PortDef to a PortMapping.
    fn port_def_to_mapping(&self, port: &PortDef) -> PortMapping {
        let protocol = match port.protocol.to_lowercase().as_str() {
            "udp" => Protocol::Udp,
            _ => Protocol::Tcp,
        };

        PortMapping {
            host_port: port.host.unwrap_or(0),
            container_port: port.container,
            protocol,
            host_ip: None,
        }
    }

    /// Parse volume mount strings (e.g., "./data:/app/data" or "named-volume:/data").
    fn parse_volume_mounts(
        &self,
        volumes: &[String],
        project_root: &PathBuf,
    ) -> Result<Vec<Mount>> {
        let mut mounts = Vec::new();

        for vol in volumes {
            let parts: Vec<&str> = vol.split(':').collect();
            if parts.len() >= 2 {
                let source_str = parts[0];
                let target = PathBuf::from(parts[1]);
                let read_only = parts.get(2).map(|s| *s == "ro").unwrap_or(false);

                // Determine if bind mount or named volume
                let (source, mount_type) = if source_str.starts_with('.')
                    || source_str.starts_with('/')
                    || source_str.starts_with('~')
                {
                    // Relative or absolute path - bind mount
                    let path = if source_str.starts_with('.') {
                        project_root.join(source_str)
                    } else {
                        PathBuf::from(source_str)
                    };
                    (path, MountType::Bind)
                } else {
                    // Named volume - use a volume directory
                    let volume_path = project_root
                        .join(".hyperbox")
                        .join("volumes")
                        .join(source_str);
                    (volume_path, MountType::Volume)
                };

                mounts.push(Mount {
                    source,
                    target,
                    read_only,
                    mount_type,
                });
            }
        }

        Ok(mounts)
    }

    /// Convert ResourceDef to ResourceLimits.
    fn resource_def_to_limits(&self, res: Option<&ResourceDef>) -> ResourceLimits {
        match res {
            Some(r) => {
                // Parse cpu_limit string (e.g., "0.5") to millicores
                let cpu_millicores = r
                    .cpu_limit
                    .as_ref()
                    .and_then(|cpu| cpu.parse::<f64>().ok().map(|v| (v * 1000.0) as u64));

                // Parse memory_limit string (e.g., "512m") to bytes
                let memory_bytes = r
                    .memory_limit
                    .as_ref()
                    .map(|mem| self.parse_memory_string(mem));

                ResourceLimits {
                    cpu_millicores,
                    memory_bytes,
                    memory_swap_bytes: None,
                    pids_limit: Some(4096),
                    io_read_bps: None,
                    io_write_bps: None,
                }
            }
            None => ResourceLimits::default(),
        }
    }

    /// Parse a memory string like "512m", "1g", "256M" to bytes.
    fn parse_memory_string(&self, mem: &str) -> u64 {
        let mem = mem.trim().to_lowercase();
        let (num_part, unit) = if mem.ends_with("gb") || mem.ends_with("g") {
            let num = mem.trim_end_matches(|c| c == 'g' || c == 'b');
            (num, 1024 * 1024 * 1024)
        } else if mem.ends_with("mb") || mem.ends_with("m") {
            let num = mem.trim_end_matches(|c| c == 'm' || c == 'b');
            (num, 1024 * 1024)
        } else if mem.ends_with("kb") || mem.ends_with("k") {
            let num = mem.trim_end_matches(|c| c == 'k' || c == 'b');
            (num, 1024)
        } else {
            (mem.as_str(), 1)
        };

        num_part.parse::<u64>().unwrap_or(0) * unit
    }

    /// Topological sort of containers based on depends_on.
    fn topological_sort(&self, containers: &[ContainerDef]) -> Result<Vec<String>> {
        // Build adjacency list
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Initialize
        for c in containers {
            graph.entry(c.name.clone()).or_default();
            in_degree.entry(c.name.clone()).or_insert(0);
        }

        // Build edges (dependency -> dependent)
        for c in containers {
            for dep in &c.depends_on {
                graph.entry(dep.clone()).or_default().push(c.name.clone());
                *in_degree.entry(c.name.clone()).or_default() += 1;
            }
        }

        // Kahn's algorithm
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop() {
            result.push(node.clone());

            if let Some(neighbors) = graph.get(&node) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }

        if result.len() != containers.len() {
            return Err(ProjectError::CyclicDependency);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ContainerDef;
    use async_trait::async_trait;
    use tokio::io::{AsyncRead, AsyncWrite};

    fn make_container(name: &str, depends_on: Vec<&str>) -> ContainerDef {
        ContainerDef {
            name: name.to_string(),
            image: "alpine:latest".to_string(),
            dockerfile: None,
            context: None,
            command: None,
            ports: vec![],
            volumes: vec![],
            environment: HashMap::new(),
            depends_on: depends_on.into_iter().map(String::from).collect(),
            healthcheck: None,
            resources: None,
        }
    }

    #[test]
    fn test_topological_sort_linear() {
        // a -> b -> c
        let containers = vec![
            make_container("a", vec![]),
            make_container("b", vec!["a"]),
            make_container("c", vec!["b"]),
        ];

        let orchestrator = ProjectOrchestrator {
            runtime: Arc::new(DummyRuntime),
        };

        let order = orchestrator.topological_sort(&containers).unwrap();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_topological_sort_parallel() {
        // a -> b, a -> c, b -> d, c -> d
        let containers = vec![
            make_container("a", vec![]),
            make_container("b", vec!["a"]),
            make_container("c", vec!["a"]),
            make_container("d", vec!["b", "c"]),
        ];

        let orchestrator = ProjectOrchestrator {
            runtime: Arc::new(DummyRuntime),
        };

        let order = orchestrator.topological_sort(&containers).unwrap();
        assert!(order[0] == "a");
        assert!(order.contains(&"b".to_string()));
        assert!(order.contains(&"c".to_string()));
        assert!(order[3] == "d");
    }

    // Dummy runtime for testing
    struct DummyRuntime;

    #[async_trait]
    impl ContainerRuntime for DummyRuntime {
        fn name(&self) -> &'static str {
            "dummy"
        }
        async fn version(&self) -> hyperbox_core::error::Result<String> {
            Ok("1.0".to_string())
        }
        async fn is_available(&self) -> bool {
            true
        }
        async fn create(&self, _: ContainerSpec) -> hyperbox_core::error::Result<ContainerId> {
            Ok(ContainerId::new())
        }
        async fn start(&self, _: &ContainerId) -> hyperbox_core::error::Result<()> {
            Ok(())
        }
        async fn stop(&self, _: &ContainerId, _: Duration) -> hyperbox_core::error::Result<()> {
            Ok(())
        }
        async fn kill(&self, _: &ContainerId, _: &str) -> hyperbox_core::error::Result<()> {
            Ok(())
        }
        async fn remove(&self, _: &ContainerId) -> hyperbox_core::error::Result<()> {
            Ok(())
        }
        async fn pause(&self, _: &ContainerId) -> hyperbox_core::error::Result<()> {
            Ok(())
        }
        async fn resume(&self, _: &ContainerId) -> hyperbox_core::error::Result<()> {
            Ok(())
        }
        async fn state(
            &self,
            _: &ContainerId,
        ) -> hyperbox_core::error::Result<hyperbox_core::types::ContainerState> {
            Ok(hyperbox_core::types::ContainerState::Running)
        }
        async fn list(
            &self,
        ) -> hyperbox_core::error::Result<Vec<(ContainerId, hyperbox_core::types::ContainerState)>>
        {
            Ok(vec![])
        }
        async fn stats(
            &self,
            _: &ContainerId,
        ) -> hyperbox_core::error::Result<hyperbox_core::types::ContainerStats> {
            unimplemented!()
        }
        async fn exec(
            &self,
            _: &ContainerId,
            _: hyperbox_core::types::ExecSpec,
        ) -> hyperbox_core::error::Result<hyperbox_core::types::ExecResult> {
            unimplemented!()
        }
        async fn attach(
            &self,
            _: &ContainerId,
        ) -> hyperbox_core::error::Result<(
            Box<dyn AsyncWrite + Send + Unpin>,
            Box<dyn AsyncRead + Send + Unpin>,
            Box<dyn AsyncRead + Send + Unpin>,
        )> {
            unimplemented!()
        }
        async fn logs(
            &self,
            _: &ContainerId,
            _: hyperbox_core::types::LogOptions,
        ) -> hyperbox_core::error::Result<Box<dyn AsyncRead + Send + Unpin>> {
            unimplemented!()
        }
        async fn wait(&self, _: &ContainerId) -> hyperbox_core::error::Result<i32> {
            Ok(0)
        }
        async fn checkpoint(
            &self,
            _: &ContainerId,
            _: &std::path::Path,
        ) -> hyperbox_core::error::Result<hyperbox_core::types::CheckpointId> {
            Ok(hyperbox_core::types::CheckpointId::new("test"))
        }
        async fn restore(
            &self,
            _: &std::path::Path,
            _: ContainerSpec,
        ) -> hyperbox_core::error::Result<ContainerId> {
            Ok(ContainerId::new())
        }
        async fn update(
            &self,
            _: &ContainerId,
            _: hyperbox_core::types::ResourceLimits,
        ) -> hyperbox_core::error::Result<()> {
            Ok(())
        }
        async fn top(
            &self,
            _: &ContainerId,
        ) -> hyperbox_core::error::Result<Vec<hyperbox_core::runtime::ProcessInfo>> {
            Ok(vec![])
        }
        async fn pull_image(
            &self,
            _: &hyperbox_core::types::ImageRef,
        ) -> hyperbox_core::error::Result<()> {
            Ok(())
        }
        async fn image_exists(&self, _: &str) -> hyperbox_core::error::Result<bool> {
            Ok(true)
        }
        async fn list_images(
            &self,
        ) -> hyperbox_core::error::Result<Vec<hyperbox_core::runtime::ImageInfo>> {
            Ok(vec![])
        }
    }
}
