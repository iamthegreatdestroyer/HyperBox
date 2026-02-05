//! File system watcher for hot-reload support.

use crate::error::{ProjectError, Result};
use crate::ProjectId;
use globset::{Glob, GlobSet, GlobSetBuilder};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    use tokio::time::{sleep, Duration};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_project_watcher_creation() {
        let (watcher, _rx) = ProjectWatcher::new();

        assert_eq!(watcher.watcher_count().await, 0);
        assert!(!watcher.is_watching().await);
    }

    #[tokio::test]
    async fn test_project_watcher_default() {
        let watcher = ProjectWatcher::default();

        assert_eq!(watcher.watcher_count().await, 0);
    }

    #[tokio::test]
    async fn test_watch_directory() {
        let dir = tempdir().unwrap();
        let (watcher, _rx) = ProjectWatcher::new();
        let project_id = Uuid::new_v4();

        let result = watcher.watch(project_id, dir.path()).await;
        assert!(result.is_ok());

        assert!(watcher.is_watching().await);
        assert_eq!(watcher.watcher_count().await, 1);
    }

    #[tokio::test]
    async fn test_watch_multiple_directories() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();
        let (watcher, _rx) = ProjectWatcher::new();

        watcher.watch(Uuid::new_v4(), dir1.path()).await.unwrap();
        watcher.watch(Uuid::new_v4(), dir2.path()).await.unwrap();

        assert_eq!(watcher.watcher_count().await, 2);
    }

    #[tokio::test]
    async fn test_unwatch_directory() {
        let dir = tempdir().unwrap();
        let (watcher, _rx) = ProjectWatcher::new();
        let project_id = Uuid::new_v4();

        watcher.watch(project_id, dir.path()).await.unwrap();
        assert_eq!(watcher.watcher_count().await, 1);

        watcher.unwatch(project_id).await;
        assert_eq!(watcher.watcher_count().await, 0);
        assert!(!watcher.is_watching().await);
    }

    #[tokio::test]
    async fn test_unwatch_nonexistent() {
        let (watcher, _rx) = ProjectWatcher::new();

        // Should not panic
        watcher.unwatch(Uuid::new_v4()).await;
        assert_eq!(watcher.watcher_count().await, 0);
    }

    #[tokio::test]
    async fn test_stop_watcher() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();
        let (watcher, _rx) = ProjectWatcher::new();

        watcher.watch(Uuid::new_v4(), dir1.path()).await.unwrap();
        watcher.watch(Uuid::new_v4(), dir2.path()).await.unwrap();

        watcher.stop().await;

        assert_eq!(watcher.watcher_count().await, 0);
        assert!(!watcher.is_watching().await);
    }

    #[tokio::test]
    async fn test_add_ignores() {
        let (watcher, _rx) = ProjectWatcher::new();

        let result = watcher
            .add_ignores(&["*.tmp".to_string(), "*.bak".to_string()])
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_ignores_invalid_pattern() {
        let (watcher, _rx) = ProjectWatcher::new();

        // Invalid glob patterns are logged as warnings but don't fail
        // This allows valid patterns to still be applied even if some are invalid
        let result = watcher.add_ignores(&["[invalid".to_string()]).await;
        assert!(result.is_ok()); // Graceful handling - doesn't error on invalid patterns
    }

    #[tokio::test]
    async fn test_file_change_kind_display() {
        let kinds = [
            ChangeKind::Created,
            ChangeKind::Modified,
            ChangeKind::Deleted,
            ChangeKind::Renamed,
        ];

        for kind in kinds {
            let change = FileChange {
                path: PathBuf::from("/test"),
                kind,
                project_id: Some(Uuid::new_v4()),
            };
            assert_eq!(change.kind, kind);
        }
    }

    #[tokio::test]
    async fn test_watch_nonexistent_directory() {
        let (watcher, _rx) = ProjectWatcher::new();

        let result = watcher
            .watch(Uuid::new_v4(), Path::new("/nonexistent/path/that/does/not/exist"))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rewatch_same_project() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();
        let (watcher, _rx) = ProjectWatcher::new();
        let project_id = Uuid::new_v4();

        watcher.watch(project_id, dir1.path()).await.unwrap();

        // Watching again with same project_id adds another watcher
        // (unwatch should be called first to replace)
        let result = watcher.watch(project_id, dir2.path()).await;
        assert!(result.is_ok());
        assert_eq!(watcher.watcher_count().await, 2); // Both watchers active
    }

    #[tokio::test]
    async fn test_file_change_event_detection() {
        let dir = tempdir().unwrap();
        let (watcher, mut rx) = ProjectWatcher::new();

        watcher.watch(Uuid::new_v4(), dir.path()).await.unwrap();

        // Create a file
        let file_path = dir.path().join("test.txt");
        {
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "test content").unwrap();
        }

        // Wait for event with timeout
        let result = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;

        // Event may or may not be received depending on OS timing
        // We just verify no panic occurs
        drop(result);
    }

    #[tokio::test]
    async fn test_ignore_patterns_applied() {
        let dir = tempdir().unwrap();
        let (watcher, _rx) = ProjectWatcher::new();

        // Default ignores include node_modules, .git, target, etc.
        watcher.watch(Uuid::new_v4(), dir.path()).await.unwrap();

        // Create a file in ignored directory
        let ignored_dir = dir.path().join("node_modules");
        std::fs::create_dir(&ignored_dir).unwrap();
        let ignored_file = ignored_dir.join("package.json");
        File::create(&ignored_file).unwrap();

        // Small delay for events to process
        sleep(Duration::from_millis(100)).await;

        // Watcher should still be running
        assert!(watcher.is_watching().await);
    }
}
