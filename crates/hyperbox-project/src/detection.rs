//! Project type detection.
//!
//! Automatically detects project type from directory contents.

use crate::config::{ContainerDef, PortDef, ProjectType};
use crate::error::{ProjectError, Result};
use crate::Project;
use serde::Deserialize;
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};

/// Project detector.
pub struct ProjectDetector;

impl ProjectDetector {
    /// Detect project type from directory.
    pub async fn detect(path: &Path) -> Result<Project> {
        if !path.exists() {
            return Err(ProjectError::DirectoryNotFound {
                path: path.to_path_buf(),
            });
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".to_string());

        let mut project = Project::new(&name, path.to_path_buf());

        // Detect project type
        project.config.project_type = Self::detect_type(path).await?;

        // Auto-configure based on type
        Self::configure_project(&mut project).await?;

        info!("Detected project: {} ({:?})", name, project.config.project_type);
        Ok(project)
    }

    /// Detect project type from markers.
    async fn detect_type(path: &Path) -> Result<ProjectType> {
        // Check for compose files first
        if path.join("docker-compose.yml").exists()
            || path.join("docker-compose.yaml").exists()
            || path.join("compose.yml").exists()
            || path.join("compose.yaml").exists()
        {
            debug!("Found Docker Compose file");
            return Ok(ProjectType::Compose);
        }

        // Check for Kubernetes manifests
        if path.join("k8s").exists()
            || path.join("kubernetes").exists()
            || path.join("helm").exists()
        {
            debug!("Found Kubernetes manifests");
            return Ok(ProjectType::Kubernetes);
        }

        // Check for language-specific markers
        if path.join("package.json").exists() {
            debug!("Found package.json - Node.js project");
            return Ok(ProjectType::Node);
        }

        if path.join("Cargo.toml").exists() {
            debug!("Found Cargo.toml - Rust project");
            return Ok(ProjectType::Rust);
        }

        if path.join("go.mod").exists() {
            debug!("Found go.mod - Go project");
            return Ok(ProjectType::Go);
        }

        if path.join("requirements.txt").exists()
            || path.join("pyproject.toml").exists()
            || path.join("setup.py").exists()
            || path.join("Pipfile").exists()
        {
            debug!("Found Python markers");
            return Ok(ProjectType::Python);
        }

        if path.join("pom.xml").exists()
            || path.join("build.gradle").exists()
            || path.join("build.gradle.kts").exists()
        {
            debug!("Found Java/JVM markers");
            return Ok(ProjectType::Java);
        }

        if Self::has_extension(path, "csproj").await
            || Self::has_extension(path, "fsproj").await
            || path.join("*.sln").exists()
        {
            debug!("Found .NET markers");
            return Ok(ProjectType::DotNet);
        }

        if path.join("Gemfile").exists() {
            debug!("Found Gemfile - Ruby project");
            return Ok(ProjectType::Ruby);
        }

        if path.join("composer.json").exists() {
            debug!("Found composer.json - PHP project");
            return Ok(ProjectType::Php);
        }

        // Default to generic if Dockerfile exists
        if path.join("Dockerfile").exists() {
            return Ok(ProjectType::Generic);
        }

        Err(ProjectError::DetectionFailed {
            path: path.to_path_buf(),
        })
    }

    /// Check if directory contains files with given extension.
    async fn has_extension(path: &Path, ext: &str) -> bool {
        if let Ok(mut entries) = fs::read_dir(path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Some(e) = entry.path().extension() {
                    if e == ext {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Configure project based on detected type.
    async fn configure_project(project: &mut Project) -> Result<()> {
        match project.config.project_type {
            ProjectType::Node => Self::configure_node(project).await,
            ProjectType::Python => Self::configure_python(project).await,
            ProjectType::Rust => Self::configure_rust(project).await,
            ProjectType::Go => Self::configure_go(project).await,
            ProjectType::Java => Self::configure_java(project).await,
            ProjectType::DotNet => Self::configure_dotnet(project).await,
            ProjectType::Compose => Self::configure_compose(project).await,
            _ => Ok(()),
        }
    }

    async fn configure_node(project: &mut Project) -> Result<()> {
        // Read package.json for more info
        let pkg_path = project.root.join("package.json");
        if pkg_path.exists() {
            if let Ok(content) = fs::read_to_string(&pkg_path).await {
                if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Check for frameworks
                    if let Some(deps) = pkg.get("dependencies") {
                        let is_next = deps.get("next").is_some();
                        let is_react = deps.get("react").is_some();
                        let is_express = deps.get("express").is_some();

                        let port = if is_next { 3000 } else if is_express { 3000 } else { 8080 };

                        let image = if is_next {
                            "node:20-alpine"
                        } else if is_react {
                            "node:20-alpine"
                        } else {
                            "node:20-alpine"
                        };

                        project.config.containers.push(ContainerDef {
                            name: project.name.clone(),
                            image: image.to_string(),
                            dockerfile: project.root.join("Dockerfile").exists().then(|| "Dockerfile".into()),
                            context: Some(".".into()),
                            command: Some(vec!["npm".into(), "start".into()]),
                            ports: vec![PortDef {
                                container: port,
                                host: None,
                                protocol: "tcp".into(),
                            }],
                            volumes: vec![".:​/app".into()],
                            environment: Default::default(),
                            depends_on: vec![],
                            healthcheck: None,
                            resources: None,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    async fn configure_python(project: &mut Project) -> Result<()> {
        project.config.containers.push(ContainerDef {
            name: project.name.clone(),
            image: "python:3.12-slim".to_string(),
            dockerfile: project.root.join("Dockerfile").exists().then(|| "Dockerfile".into()),
            context: Some(".".into()),
            command: Some(vec!["python".into(), "-m".into(), "flask".into(), "run".into()]),
            ports: vec![PortDef {
                container: 5000,
                host: None,
                protocol: "tcp".into(),
            }],
            volumes: vec![".:​/app".into()],
            environment: [("PYTHONUNBUFFERED".into(), "1".into())].into_iter().collect(),
            depends_on: vec![],
            healthcheck: None,
            resources: None,
        });
        Ok(())
    }

    async fn configure_rust(project: &mut Project) -> Result<()> {
        project.config.containers.push(ContainerDef {
            name: project.name.clone(),
            image: "rust:1.75-slim".to_string(),
            dockerfile: project.root.join("Dockerfile").exists().then(|| "Dockerfile".into()),
            context: Some(".".into()),
            command: Some(vec!["cargo".into(), "run".into(), "--release".into()]),
            ports: vec![PortDef {
                container: 8080,
                host: None,
                protocol: "tcp".into(),
            }],
            volumes: vec![".:​/app".into(), "cargo-cache:/usr/local/cargo/registry".into()],
            environment: Default::default(),
            depends_on: vec![],
            healthcheck: None,
            resources: None,
        });
        Ok(())
    }

    async fn configure_go(project: &mut Project) -> Result<()> {
        project.config.containers.push(ContainerDef {
            name: project.name.clone(),
            image: "golang:1.22-alpine".to_string(),
            dockerfile: project.root.join("Dockerfile").exists().then(|| "Dockerfile".into()),
            context: Some(".".into()),
            command: Some(vec!["go".into(), "run".into(), ".".into()]),
            ports: vec![PortDef {
                container: 8080,
                host: None,
                protocol: "tcp".into(),
            }],
            volumes: vec![".:​/app".into(), "go-cache:/go/pkg".into()],
            environment: Default::default(),
            depends_on: vec![],
            healthcheck: None,
            resources: None,
        });
        Ok(())
    }

    async fn configure_java(project: &mut Project) -> Result<()> {
        let is_gradle = project.root.join("build.gradle").exists()
            || project.root.join("build.gradle.kts").exists();

        let cmd = if is_gradle {
            vec!["./gradlew".into(), "bootRun".into()]
        } else {
            vec!["mvn".into(), "spring-boot:run".into()]
        };

        project.config.containers.push(ContainerDef {
            name: project.name.clone(),
            image: "eclipse-temurin:21-jdk".to_string(),
            dockerfile: project.root.join("Dockerfile").exists().then(|| "Dockerfile".into()),
            context: Some(".".into()),
            command: Some(cmd),
            ports: vec![PortDef {
                container: 8080,
                host: None,
                protocol: "tcp".into(),
            }],
            volumes: vec![".:​/app".into(), "m2-cache:/root/.m2".into()],
            environment: Default::default(),
            depends_on: vec![],
            healthcheck: None,
            resources: None,
        });
        Ok(())
    }

    async fn configure_dotnet(project: &mut Project) -> Result<()> {
        project.config.containers.push(ContainerDef {
            name: project.name.clone(),
            image: "mcr.microsoft.com/dotnet/sdk:8.0".to_string(),
            dockerfile: project.root.join("Dockerfile").exists().then(|| "Dockerfile".into()),
            context: Some(".".into()),
            command: Some(vec!["dotnet".into(), "run".into()]),
            ports: vec![PortDef {
                container: 5000,
                host: None,
                protocol: "tcp".into(),
            }],
            volumes: vec![".:​/app".into()],
            environment: [("DOTNET_ENVIRONMENT".into(), "Development".into())].into_iter().collect(),
            depends_on: vec![],
            healthcheck: None,
            resources: None,
        });
        Ok(())
    }

    async fn configure_compose(project: &mut Project) -> Result<()> {
        // Find the compose file
        let compose_path = Self::find_compose_file(&project.root)?;

        // Read and parse the compose file
        let content = fs::read_to_string(&compose_path).await.map_err(|e| {
            ProjectError::ConfigError {
                path: compose_path.clone(),
                message: format!("Failed to read compose file: {}", e),
            }
        })?;

        let compose: ComposeFile = serde_yaml::from_str(&content).map_err(|e| {
            ProjectError::ConfigError {
                path: compose_path.clone(),
                message: format!("Failed to parse compose file: {}", e),
            }
        })?;

        debug!("Parsed compose file with {} services", compose.services.len());

        // Convert each service to our ContainerDef format
        for (name, service) in compose.services.iter() {
            let container = Self::compose_service_to_container(name, service, project);
            project.config.containers.push(container);
        }

        // Convert volumes if present
        if let Some(volumes) = &compose.volumes {
            for (name, _vol_config) in volumes.iter() {
                project.config.volumes.push(crate::config::VolumeDef {
                    name: name.clone(),
                    source: None,
                    driver_opts: std::collections::HashMap::new(),
                });
            }
        }

        // Configure network if present
        if let Some(ref networks) = compose.networks {
            if let Some((name, _)) = networks.iter().next() {
                project.config.network.name = Some(name.clone());
            }
        }

        info!(
            "Configured {} containers from compose file",
            project.config.containers.len()
        );

        Ok(())
    }

    /// Find the compose file in the project directory.
    fn find_compose_file(root: &Path) -> Result<std::path::PathBuf> {
        let candidates = [
            "docker-compose.yml",
            "docker-compose.yaml",
            "compose.yml",
            "compose.yaml",
        ];

        for name in candidates {
            let path = root.join(name);
            if path.exists() {
                return Ok(path);
            }
        }

        Err(ProjectError::ConfigError {
            path: root.to_path_buf(),
            message: "No compose file found".to_string(),
        })
    }

    /// Convert a compose service definition to our ContainerDef.
    fn compose_service_to_container(
        name: &str,
        service: &ComposeService,
        project: &Project,
    ) -> ContainerDef {
        // Parse ports
        let ports = service
            .ports
            .as_ref()
            .map(|p| Self::parse_compose_ports(p))
            .unwrap_or_default();

        // Parse environment variables
        let environment = service
            .environment
            .as_ref()
            .map(|e| Self::parse_compose_env(e))
            .unwrap_or_default();

        // Parse volumes
        let volumes = service
            .volumes
            .as_ref()
            .map(|v| v.iter().map(|s| s.clone()).collect())
            .unwrap_or_default();

        // Parse command
        let command = service.command.as_ref().map(|c| match c {
            StringOrList::String(s) => {
                // Split command string on whitespace
                s.split_whitespace().map(String::from).collect()
            }
            StringOrList::List(l) => l.clone(),
        });

        // Determine image
        let image = service
            .image
            .clone()
            .unwrap_or_else(|| format!("{}-{}", project.name, name));

        // Build context/dockerfile
        let (dockerfile, context) = match &service.build {
            Some(BuildConfig::Simple(path)) => (
                Some(std::path::PathBuf::from(path).join("Dockerfile")),
                Some(std::path::PathBuf::from(path)),
            ),
            Some(BuildConfig::Extended(config)) => (
                config.dockerfile.clone().map(std::path::PathBuf::from),
                config.context.clone().map(std::path::PathBuf::from),
            ),
            None => (None, None),
        };

        // Parse healthcheck
        let healthcheck = service.healthcheck.as_ref().map(|hc| {
            crate::config::HealthCheck {
                test: hc.test.clone().unwrap_or_default(),
                interval: hc.interval.clone().unwrap_or_else(|| "30s".into()),
                timeout: hc.timeout.clone().unwrap_or_else(|| "10s".into()),
                retries: hc.retries.unwrap_or(3),
                start_period: hc.start_period.clone(),
            }
        });

        // Parse resource limits
        let resources = service.deploy.as_ref().and_then(|d| {
            d.resources.as_ref().map(|r| {
                crate::config::ResourceDef {
                    memory_limit: r.limits.as_ref().and_then(|l| l.memory.clone()),
                    cpu_limit: r.limits.as_ref().and_then(|l| l.cpus.clone()),
                    memory_reservation: r.reservations.as_ref().and_then(|r| r.memory.clone()),
                    cpu_reservation: r.reservations.as_ref().and_then(|r| r.cpus.clone()),
                }
            })
        });

        ContainerDef {
            name: name.to_string(),
            image,
            dockerfile,
            context,
            command,
            ports,
            volumes,
            environment,
            depends_on: service.depends_on.clone().unwrap_or_default(),
            healthcheck,
            resources,
        }
    }

    /// Parse compose port definitions.
    fn parse_compose_ports(ports: &[ComposePort]) -> Vec<PortDef> {
        ports
            .iter()
            .filter_map(|p| match p {
                ComposePort::Short(s) => Self::parse_port_string(s),
                ComposePort::Long(lp) => Some(PortDef {
                    container: lp.target,
                    host: lp.published,
                    protocol: lp.protocol.clone().unwrap_or_else(|| "tcp".into()),
                }),
            })
            .collect()
    }

    /// Parse a port string like "8080" or "3000:8080" or "3000:8080/udp".
    fn parse_port_string(s: &str) -> Option<PortDef> {
        // Handle protocol suffix
        let (port_part, protocol) = if s.contains('/') {
            let parts: Vec<&str> = s.split('/').collect();
            (parts[0], parts.get(1).copied().unwrap_or("tcp"))
        } else {
            (s, "tcp")
        };

        // Handle host:container mapping
        if port_part.contains(':') {
            let parts: Vec<&str> = port_part.split(':').collect();
            let host_port = parts[0].parse().ok();
            let container_port = parts.get(1).and_then(|p| p.parse().ok())?;

            Some(PortDef {
                container: container_port,
                host: host_port,
                protocol: protocol.to_string(),
            })
        } else {
            // Just container port
            let container_port = port_part.parse().ok()?;
            Some(PortDef {
                container: container_port,
                host: None,
                protocol: protocol.to_string(),
            })
        }
    }

    /// Parse compose environment variables.
    fn parse_compose_env(env: &ComposeEnvironment) -> std::collections::HashMap<String, String> {
        match env {
            ComposeEnvironment::List(list) => {
                list.iter()
                    .filter_map(|s| {
                        let parts: Vec<&str> = s.splitn(2, '=').collect();
                        if parts.len() == 2 {
                            Some((parts[0].to_string(), parts[1].to_string()))
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            ComposeEnvironment::Map(map) => {
                map.iter()
                    .map(|(k, v)| (k.clone(), v.clone().unwrap_or_default()))
                    .collect()
            }
        }
    }
}

// ============================================================================
// Docker Compose File Types
// ============================================================================

/// Docker Compose file structure.
#[derive(Debug, Deserialize)]
struct ComposeFile {
    /// Compose file version (optional in v3+)
    #[serde(default)]
    version: Option<String>,
    /// Service definitions
    services: std::collections::HashMap<String, ComposeService>,
    /// Volume definitions
    #[serde(default)]
    volumes: Option<std::collections::HashMap<String, Option<ComposeVolume>>>,
    /// Network definitions
    #[serde(default)]
    networks: Option<std::collections::HashMap<String, Option<ComposeNetwork>>>,
}

/// Docker Compose service definition.
#[derive(Debug, Deserialize)]
struct ComposeService {
    /// Container image
    image: Option<String>,
    /// Build configuration
    build: Option<BuildConfig>,
    /// Command to run
    command: Option<StringOrList>,
    /// Entrypoint
    entrypoint: Option<StringOrList>,
    /// Port mappings
    ports: Option<Vec<ComposePort>>,
    /// Volume mounts
    volumes: Option<Vec<String>>,
    /// Environment variables
    environment: Option<ComposeEnvironment>,
    /// Service dependencies
    depends_on: Option<Vec<String>>,
    /// Health check configuration
    healthcheck: Option<ComposeHealthcheck>,
    /// Deployment configuration
    deploy: Option<ComposeDeploy>,
    /// Restart policy
    restart: Option<String>,
    /// Container name
    container_name: Option<String>,
    /// Working directory
    working_dir: Option<String>,
    /// User
    user: Option<String>,
    /// Labels
    labels: Option<std::collections::HashMap<String, String>>,
}

/// Build configuration (simple path or extended).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BuildConfig {
    /// Simple path
    Simple(String),
    /// Extended configuration
    Extended(BuildExtended),
}

/// Extended build configuration.
#[derive(Debug, Deserialize)]
struct BuildExtended {
    /// Build context
    context: Option<String>,
    /// Dockerfile path
    dockerfile: Option<String>,
    /// Build arguments
    args: Option<std::collections::HashMap<String, String>>,
    /// Target stage
    target: Option<String>,
}

/// String or list value.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrList {
    String(String),
    List(Vec<String>),
}

/// Compose port definition.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ComposePort {
    /// Short syntax: "8080" or "3000:8080"
    Short(String),
    /// Long syntax
    Long(ComposePortLong),
}

/// Long port syntax.
#[derive(Debug, Deserialize)]
struct ComposePortLong {
    /// Target (container) port
    target: u16,
    /// Published (host) port
    published: Option<u16>,
    /// Protocol
    protocol: Option<String>,
}

/// Environment variable formats.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ComposeEnvironment {
    /// List format: ["VAR=value"]
    List(Vec<String>),
    /// Map format: {VAR: value}
    Map(std::collections::HashMap<String, Option<String>>),
}

/// Compose healthcheck.
#[derive(Debug, Deserialize)]
struct ComposeHealthcheck {
    /// Test command
    test: Option<Vec<String>>,
    /// Interval
    interval: Option<String>,
    /// Timeout
    timeout: Option<String>,
    /// Retries
    retries: Option<u32>,
    /// Start period
    start_period: Option<String>,
}

/// Compose deployment configuration.
#[derive(Debug, Deserialize)]
struct ComposeDeploy {
    /// Resource limits
    resources: Option<ComposeResources>,
    /// Replicas
    replicas: Option<u32>,
}

/// Compose resource configuration.
#[derive(Debug, Deserialize)]
struct ComposeResources {
    /// Resource limits
    limits: Option<ResourceSpec>,
    /// Resource reservations
    reservations: Option<ResourceSpec>,
}

/// Resource specification.
#[derive(Debug, Deserialize)]
struct ResourceSpec {
    /// CPU limit (e.g., "0.5")
    cpus: Option<String>,
    /// Memory limit (e.g., "512M")
    memory: Option<String>,
}

/// Compose volume configuration.
#[derive(Debug, Deserialize)]
struct ComposeVolume {
    /// Volume driver
    driver: Option<String>,
    /// Driver options
    driver_opts: Option<std::collections::HashMap<String, String>>,
}

/// Compose network configuration.
#[derive(Debug, Deserialize)]
struct ComposeNetwork {
    /// Network driver
    driver: Option<String>,
    /// External network
    external: Option<bool>,
}
