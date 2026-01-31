//! File system watcher for hot-reload support.

use crate::ProjectId;
use crate::error::{ProjectError, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

/// File change event.
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Changed file path.
    pub path: PathBuf,
    /// Kind of change.
    pub kind: ChangeKind,
    /// Project ID (if applicable).
    pub project_id: Option<ProjectId>,
}

/// Kind of file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// File created.
    Created,
    /// File modified.
    Modified,
    /// File deleted.
    Deleted,
    /// File renamed.
    Renamed,
}

/// Project file watcher for hot-reload.
pub struct ProjectWatcher {
    /// Active watchers by project.
    watchers: RwLock<Vec<(ProjectId, RecommendedWatcher)>>,
    /// Ignore patterns.
    ignore_patterns: RwLock<GlobSet>,
    /// Event sender.
    event_tx: mpsc::Sender<FileChange>,
    /// Running flag.
    running: Arc<RwLock<bool>>,
}

impl ProjectWatcher {
    /// Create a new project watcher.
    pub fn new() -> (Self, mpsc::Receiver<FileChange>) {
        let (tx, rx) = mpsc::channel(1000);

        let watcher = Self {
            watchers: RwLock::new(Vec::new()),
            ignore_patterns: RwLock::new(Self::default_ignores()),
            event_tx: tx,
            running: Arc::new(RwLock::new(true)),
        };

        (watcher, rx)
    }

    /// Create default ignore patterns.
    fn default_ignores() -> GlobSet {
        let mut builder = GlobSetBuilder::new();

        let patterns = [
            "**/node_modules/**",
            "**/.git/**",
            "**/target/**",
            "**/__pycache__/**",
            "**/*.pyc",
            "**/venv/**",
            "**/.venv/**",
            "**/dist/**",
            "**/build/**",
            "**/.next/**",
            "**/coverage/**",
            "**/.cache/**",
            "**/tmp/**",
            "**/*.log",
            "**/*.lock",
            "**/package-lock.json",
            "**/yarn.lock",
            "**/pnpm-lock.yaml",
            "**/Cargo.lock",
        ];

        for pattern in patterns {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }

        builder.build().unwrap_or_else(|_| GlobSet::empty())
    }

    /// Watch a project directory.
    pub async fn watch(&self, project_id: ProjectId, path: &Path) -> Result<()> {
        let tx = self.event_tx.clone();
        let ignore = self.ignore_patterns.read().await.clone();
        let project_path = path.to_path_buf();

        let (notify_tx, notify_rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            notify_tx,
            Config::default()
                .with_poll_interval(Duration::from_millis(100))
                .with_compare_contents(false),
        )
        .map_err(|e| ProjectError::Watcher(e.to_string()))?;

        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| ProjectError::Watcher(e.to_string()))?;

        self.watchers.write().await.push((project_id, watcher));

        // Spawn event processing task
        let running = Arc::clone(&self.running);
        tokio::spawn(async move {
            loop {
                if !*running.read().await {
                    break;
                }

                match notify_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Ok(event)) => {
                        Self::process_event(&tx, project_id, &event, &ignore, &project_path).await;
                    }
                    Ok(Err(e)) => {
                        warn!("Watch error: {}", e);
                    }
                    Err(_) => {
                        // Timeout, continue
                    }
                }
            }
        });

        info!("Started watching project {} at {:?}", project_id, path);
        Ok(())
    }

    /// Process a notify event.
    async fn process_event(
        tx: &mpsc::Sender<FileChange>,
        project_id: ProjectId,
        event: &Event,
        ignore: &GlobSet,
        base_path: &Path,
    ) {
        for path in &event.paths {
            // Check ignore patterns
            let relative = path.strip_prefix(base_path).unwrap_or(path);
            if ignore.is_match(relative) {
                continue;
            }

            let kind = match event.kind {
                EventKind::Create(_) => ChangeKind::Created,
                EventKind::Modify(_) => ChangeKind::Modified,
                EventKind::Remove(_) => ChangeKind::Deleted,
                _ => continue,
            };

            let change = FileChange {
                path: path.clone(),
                kind,
                project_id: Some(project_id),
            };

            debug!("File change: {:?}", change);

            if tx.send(change).await.is_err() {
                error!("Failed to send file change event");
            }
        }
    }

    /// Stop watching a project.
    pub async fn unwatch(&self, project_id: ProjectId) {
        let mut watchers = self.watchers.write().await;
        watchers.retain(|(id, _)| *id != project_id);
        info!("Stopped watching project {}", project_id);
    }

    /// Add ignore patterns.
    pub async fn add_ignores(&self, patterns: &[String]) -> Result<()> {
        let mut builder = GlobSetBuilder::new();

        // Add new patterns
        for pattern in patterns {
            match Glob::new(pattern) {
                Ok(glob) => {
                    builder.add(glob);
                }
                Err(e) => {
                    warn!("Invalid glob pattern '{}': {}", pattern, e);
                }
            }
        }

        // Build and update the glob set
        match builder.build() {
            Ok(glob_set) => {
                *self.ignore_patterns.write().await = glob_set;
            }
            Err(e) => {
                warn!("Failed to build glob set: {}", e);
            }
        }

        Ok(())
    }

    /// Stop all watchers.
    pub async fn stop(&self) {
        *self.running.write().await = false;
        self.watchers.write().await.clear();
        info!("Stopped all file watchers");
    }

    /// Check if any watchers are active.
    pub async fn is_watching(&self) -> bool {
        !self.watchers.read().await.is_empty()
    }

    /// Get number of active watchers.
    pub async fn watcher_count(&self) -> usize {
        self.watchers.read().await.len()
    }
}

impl Default for ProjectWatcher {
    fn default() -> Self {
        Self::new().0
    }
}
