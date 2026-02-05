//! Project manager - core orchestration of project lifecycle.

use crate::detection::ProjectDetector;
use crate::error::{ProjectError, Result};
use crate::orchestration::ProjectOrchestrator;
use crate::ports::ProjectPortManager;
use crate::resources::ResourcePool;
use crate::{Project, ProjectId, ProjectState};
use dashmap::DashMap;
use hyperbox_core::runtime::ContainerRuntime;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Project manager for orchestrating project lifecycles.
pub struct ProjectManager {
    /// Active projects
    projects: DashMap<ProjectId, Project>,
    /// Projects by path
    path_index: DashMap<PathBuf, ProjectId>,
    /// Port manager
    port_manager: Arc<ProjectPortManager>,
    /// Resource pool
    resource_pool: Arc<ResourcePool>,
    /// Data directory
    data_dir: PathBuf,
    /// Shutdown signal
    shutdown: Arc<RwLock<bool>>,
    /// Container runtime orchestrator (optional - for when runtime is available)
    orchestrator: Option<ProjectOrchestrator>,
}

impl ProjectManager {
    /// Create a new project manager without a runtime.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        let data_dir = data_dir.into();

        Self {
            projects: DashMap::new(),
            path_index: DashMap::new(),
            port_manager: Arc::new(ProjectPortManager::new()),
            resource_pool: Arc::new(ResourcePool::new()),
            data_dir,
            shutdown: Arc::new(RwLock::new(false)),
            orchestrator: None,
        }
    }

    /// Create a new project manager with a container runtime.
    pub fn with_runtime(data_dir: impl Into<PathBuf>, runtime: Arc<dyn ContainerRuntime>) -> Self {
        let data_dir = data_dir.into();

        Self {
            projects: DashMap::new(),
            path_index: DashMap::new(),
            port_manager: Arc::new(ProjectPortManager::new()),
            resource_pool: Arc::new(ResourcePool::new()),
            data_dir,
            shutdown: Arc::new(RwLock::new(false)),
            orchestrator: Some(ProjectOrchestrator::new(runtime)),
        }
    }

    /// Set the container runtime after construction.
    pub fn set_runtime(&mut self, runtime: Arc<dyn ContainerRuntime>) {
        self.orchestrator = Some(ProjectOrchestrator::new(runtime));
    }

    /// Initialize the project manager.
    pub async fn initialize(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.data_dir).await?;
        tokio::fs::create_dir_all(self.data_dir.join("projects")).await?;

        // Load saved projects
        self.load_projects().await?;

        info!("Project manager initialized with {} projects", self.projects.len());
        Ok(())
    }

    /// Load saved projects from disk.
    async fn load_projects(&self) -> Result<()> {
        let projects_dir = self.data_dir.join("projects");

        if !projects_dir.exists() {
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(&projects_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                match self.load_project(&path).await {
                    Ok(project) => {
                        let id = project.id;
                        let project_path = project.root.clone();
                        self.projects.insert(id, project);
                        self.path_index.insert(project_path, id);
                    }
                    Err(e) => {
                        warn!("Failed to load project from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single project from file.
    async fn load_project(&self, path: &Path) -> Result<Project> {
        let content = tokio::fs::read_to_string(path).await?;
        let project: Project = serde_json::from_str(&content)?;
        Ok(project)
    }

    /// Save a project to disk.
    async fn save_project(&self, project: &Project) -> Result<()> {
        let path = self.data_dir
            .join("projects")
            .join(format!("{}.json", project.id));

        let content = serde_json::to_string_pretty(project)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Open a project from a directory.
    pub async fn open(&self, path: impl AsRef<Path>) -> Result<ProjectId> {
        let path = path.as_ref().canonicalize()
            .map_err(|_| ProjectError::DirectoryNotFound { path: path.as_ref().to_path_buf() })?;

        // Check if already opened
        if let Some(id) = self.path_index.get(&path) {
            info!("Project already opened: {}", id.value());
            return Ok(*id.value());
        }

        // Detect project
        let mut project = ProjectDetector::detect(&path).await?;

        // Allocate ports
        for container in &mut project.config.containers {
            for port in &mut container.ports {
                if port.host.is_none() {
                    let allocated = self.port_manager.allocate(project.id, Some(port.container))?;
                    port.host = Some(allocated);
                    project.ports.push(allocated);
                }
            }
        }

        let id = project.id;

        // Save project
        self.save_project(&project).await?;

        // Add to maps
        self.path_index.insert(path, id);
        self.projects.insert(id, project);

        info!("Opened project: {}", id);
        Ok(id)
    }

    /// Get a project by ID.
    pub fn get(&self, id: ProjectId) -> Option<Project> {
        self.projects.get(&id).map(|p| p.value().clone())
    }

    /// Get a project by path.
    pub fn get_by_path(&self, path: &Path) -> Option<Project> {
        self.path_index
            .get(path)
            .and_then(|id| self.projects.get(id.value()).map(|p| p.value().clone()))
    }

    /// List all projects.
    pub fn list(&self) -> Vec<Project> {
        self.projects.iter().map(|p| p.value().clone()).collect()
    }

    /// Start a project.
    pub async fn start(&self, id: ProjectId) -> Result<()> {
        let mut project = self.projects
            .get_mut(&id)
            .ok_or_else(|| ProjectError::NotFound(id.to_string()))?;

        // Validate state transition
        match project.state {
            ProjectState::Running => {
                info!("Project {} is already running", id);
                return Ok(());
            }
            ProjectState::Starting => {
                info!("Project {} is already starting", id);
                return Ok(());
            }
            _ => {}
        }

        info!("Starting project: {} ({:?})", project.name, project.config.project_type);
        project.state = ProjectState::Starting;
        project.touch();

        // Drop the lock before async operations
        drop(project);

        // Start containers
        self.start_containers(id).await?;

        // Update state
        if let Some(mut project) = self.projects.get_mut(&id) {
            project.state = ProjectState::Running;
            project.touch();
        }

        info!("Project {} started successfully", id);
        Ok(())
    }

    /// Start containers for a project.
    async fn start_containers(&self, id: ProjectId) -> Result<()> {
        let project = self.projects
            .get(&id)
            .ok_or_else(|| ProjectError::NotFound(id.to_string()))?
            .clone();

        if let Some(ref orchestrator) = self.orchestrator {
            // Use the real orchestrator to create and start containers
            let container_ids = orchestrator.start_project(&project).await?;

            // Update project with container IDs
            if let Some(mut p) = self.projects.get_mut(&id) {
                p.containers = container_ids;
            }
        } else {
            // No runtime available - log only (for testing/UI-only mode)
            for container_def in &project.config.containers {
                warn!("No runtime available - simulating container start: {}", container_def.name);
                info!("Container {} would start on ports {:?}",
                    container_def.name,
                    container_def.ports.iter()
                        .filter_map(|p| p.host)
                        .collect::<Vec<_>>()
                );
            }
        }

        Ok(())
    }

    /// Stop a project.
    pub async fn stop(&self, id: ProjectId) -> Result<()> {
        let mut project = self.projects
            .get_mut(&id)
            .ok_or_else(|| ProjectError::NotFound(id.to_string()))?;

        if !project.is_active() {
            info!("Project {} is not running", id);
            return Ok(());
        }

        info!("Stopping project: {}", project.name);
        project.state = ProjectState::Stopping;
        project.touch();

        // Drop lock before async operations
        let container_ids = project.containers.clone();
        drop(project);

        // Stop containers using orchestrator if available
        if let Some(ref orchestrator) = self.orchestrator {
            orchestrator.stop_project(&container_ids).await?;
            orchestrator.remove_containers(&container_ids).await?;
        } else {
            // No runtime - just log
            for container_id in &container_ids {
                warn!("No runtime available - simulating container stop: {}", container_id);
            }
        }

        // Update state
        if let Some(mut project) = self.projects.get_mut(&id) {
            project.state = ProjectState::Stopped;
            project.containers.clear();
            project.touch();
        }

        info!("Project {} stopped", id);
        Ok(())
    }

    /// Restart a project.
    pub async fn restart(&self, id: ProjectId) -> Result<()> {
        self.stop(id).await?;
        self.start(id).await?;
        Ok(())
    }

    /// Close a project (stop and remove from manager).
    pub async fn close(&self, id: ProjectId) -> Result<()> {
        // Stop if running
        if let Some(project) = self.get(id) {
            if project.is_active() {
                self.stop(id).await?;
            }

            // Release ports
            for port in &project.ports {
                self.port_manager.release(*port);
            }

            // Remove from maps
            self.path_index.remove(&project.root);
        }

        self.projects.remove(&id);

        // Remove saved project
        let path = self.data_dir
            .join("projects")
            .join(format!("{}.json", id));
        let _ = tokio::fs::remove_file(path).await;

        info!("Closed project: {}", id);
        Ok(())
    }

    /// Get port manager.
    pub fn port_manager(&self) -> Arc<ProjectPortManager> {
        Arc::clone(&self.port_manager)
    }

    /// Get resource pool.
    pub fn resource_pool(&self) -> Arc<ResourcePool> {
        Arc::clone(&self.resource_pool)
    }

    /// Shutdown the project manager.
    pub async fn shutdown(&self) -> Result<()> {
        *self.shutdown.write().await = true;

        // Stop all running projects
        let ids: Vec<ProjectId> = self.projects
            .iter()
            .filter(|p| p.is_active())
            .map(|p| p.id)
            .collect();

        for id in ids {
            if let Err(e) = self.stop(id).await {
                error!("Failed to stop project {}: {}", id, e);
            }
        }

        // Save all projects
        for project in self.projects.iter() {
            if let Err(e) = self.save_project(project.value()).await {
                error!("Failed to save project {}: {}", project.id, e);
            }
        }

        info!("Project manager shutdown complete");
        Ok(())
    }
}
