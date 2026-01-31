//! Project type detection.
//!
//! Automatically detects project type from directory contents.

use crate::config::{ContainerDef, PortDef, ProjectType};
use crate::error::{ProjectError, Result};
use crate::Project;
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
        // Parse docker-compose.yml and convert to our format
        // This would involve full compose file parsing
        // For now, we'll mark it for special handling
        debug!("Compose project - will use native compose handling");
        Ok(())
    }
}
