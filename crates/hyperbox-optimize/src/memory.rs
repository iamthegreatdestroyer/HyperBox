//! Dynamic VM memory management.
//!
//! Addresses Docker Desktop's chronic VM memory bloat by actively tracking
//! which RAM pages are actually in use and releasing unused portions back to
//! the host OS. Inspired by OrbStack's approach (Aug 2024).
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  DynamicMemoryManager                                       │
//! │  ┌────────────┐  ┌──────────────┐  ┌────────────────────┐  │
//! │  │  Balloon    │  │ Free-page    │  │ KSM (kernel 6.4+) │  │
//! │  │  Controller │  │ Reporting    │  │ per-process merge  │  │
//! │  └──────┬─────┘  └──────┬───────┘  └────────┬───────────┘  │
//! │         │               │                    │              │
//! │  ┌──────▼───────────────▼────────────────────▼──────────┐  │
//! │  │         cgroup v2 memory controller                   │  │
//! │  │   memory.current · memory.max · memory.stat           │  │
//! │  └──────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Mechanisms
//!
//! 1. **virtio-balloon**: Inflate balloon to reclaim guest pages, deflate
//!    to return memory under load. The guest kernel cooperates by giving
//!    pages to the balloon device.
//!
//! 2. **Free page reporting**: Kernel 5.8+ (`free_page_reporting` driver)
//!    asynchronously reports freed pages to the hypervisor, enabling
//!    host-side reclamation without balloon inflation latency.
//!
//! 3. **KSM (Kernel Same-page Merging)**: kernel 6.4+ allows
//!    `MMF_VM_MERGE_ANY` per-process, enabling 10-50% memory savings for
//!    similar containers (e.g., multiple Node.js or JVM processes).

use crate::error::{OptimizeError, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// ── Constants ────────────────────────────────────────────────────────────────

/// Default polling interval for memory sampling (milliseconds).
const DEFAULT_POLL_INTERVAL_MS: u64 = 1_000;
/// Minimum balloon adjustment step size (bytes) — 1 MiB.
const MIN_BALLOON_STEP: u64 = 1_024 * 1_024;
/// Default high-water-mark ratio above which we start reclaiming.
const DEFAULT_HIGH_WATERMARK: f64 = 0.80;
/// Default low-water-mark ratio below which we stop reclaiming.
const DEFAULT_LOW_WATERMARK: f64 = 0.50;
/// Idle detection window — seconds with < 5% memory change.
const IDLE_WINDOW_SECONDS: u64 = 30;
/// Maximum history samples kept per container.
const MAX_HISTORY_SAMPLES: usize = 3_600;
/// Exponential moving average smoothing factor.
const EMA_ALPHA: f64 = 0.3;

// ── Configuration ────────────────────────────────────────────────────────────

/// Memory management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Enable dynamic balloon management.
    pub balloon_enabled: bool,
    /// Enable free-page reporting (kernel 5.8+).
    pub free_page_reporting: bool,
    /// Enable KSM per-process merging (kernel 6.4+).
    pub ksm_enabled: bool,
    /// High-water-mark ratio (0.0 – 1.0) to begin reclamation.
    pub high_watermark: f64,
    /// Low-water-mark ratio (0.0 – 1.0) to stop reclamation.
    pub low_watermark: f64,
    /// Polling interval in milliseconds.
    pub poll_interval_ms: u64,
    /// Minimum memory guarantee per container (bytes).
    pub min_memory_bytes: u64,
    /// Maximum total memory budget for all containers (bytes, 0 = auto-detect).
    pub max_total_memory_bytes: u64,
    /// Aggressive reclaim during idle (release more pages when idle).
    pub aggressive_idle_reclaim: bool,
    /// Idle detection threshold — fraction of memory change below which
    /// a container is considered idle.
    pub idle_change_threshold: f64,
    /// Path to the cgroup v2 unified hierarchy.
    pub cgroup_root: PathBuf,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            balloon_enabled: true,
            free_page_reporting: true,
            ksm_enabled: false, // opt-in due to side-channel considerations
            high_watermark: DEFAULT_HIGH_WATERMARK,
            low_watermark: DEFAULT_LOW_WATERMARK,
            poll_interval_ms: DEFAULT_POLL_INTERVAL_MS,
            min_memory_bytes: 32 * 1_024 * 1_024, // 32 MiB floor
            max_total_memory_bytes: 0,             // auto-detect from host
            aggressive_idle_reclaim: true,
            idle_change_threshold: 0.05,
            cgroup_root: PathBuf::from("/sys/fs/cgroup"),
        }
    }
}

// ── Per-container memory state ───────────────────────────────────────────────

/// Snapshot of a single container's memory usage at one point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySample {
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// `memory.current` — actual RSS (bytes).
    pub current_bytes: u64,
    /// `memory.max` (or `memory.high`) — configured limit (bytes).
    pub limit_bytes: u64,
    /// Swap usage from `memory.swap.current` (bytes).
    pub swap_bytes: u64,
    /// Inactive file-backed pages from `memory.stat` (bytes).
    pub inactive_file_bytes: u64,
    /// Active file-backed pages from `memory.stat` (bytes).
    pub active_file_bytes: u64,
    /// Anonymous pages from `memory.stat` (bytes).
    pub anon_bytes: u64,
    /// Slab reclaimable from `memory.stat` (bytes).
    pub slab_reclaimable_bytes: u64,
}

impl MemorySample {
    /// Estimate "actually needed" memory: anon + active file + slab.
    #[must_use]
    pub fn working_set_bytes(&self) -> u64 {
        self.anon_bytes
            .saturating_add(self.active_file_bytes)
            .saturating_add(self.slab_reclaimable_bytes)
    }

    /// Usage ratio relative to the limit.
    #[must_use]
    pub fn usage_ratio(&self) -> f64 {
        if self.limit_bytes == 0 {
            return 0.0;
        }
        self.current_bytes as f64 / self.limit_bytes as f64
    }

    /// Reclaimable memory = current - working-set.
    #[must_use]
    pub fn reclaimable_bytes(&self) -> u64 {
        self.current_bytes.saturating_sub(self.working_set_bytes())
    }
}

/// Per-container balloon state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMemoryState {
    /// Container identifier (cgroup path suffix).
    pub container_id: String,
    /// Current balloon inflation (bytes claimed from guest).
    pub balloon_inflated_bytes: u64,
    /// Target balloon size (bytes).
    pub balloon_target_bytes: u64,
    /// Latest sample.
    pub latest_sample: Option<MemorySample>,
    /// Exponential moving average of working-set bytes.
    pub ema_working_set: f64,
    /// Number of consecutive idle polls.
    pub idle_ticks: u64,
    /// Whether the container is considered idle.
    pub is_idle: bool,
    /// KSM savings (bytes reported merged).
    pub ksm_merged_bytes: u64,
    /// History of recent samples.
    #[serde(skip)]
    pub history: Vec<MemorySample>,
}

impl ContainerMemoryState {
    /// Create a new state for a container.
    fn new(container_id: impl Into<String>) -> Self {
        Self {
            container_id: container_id.into(),
            balloon_inflated_bytes: 0,
            balloon_target_bytes: 0,
            latest_sample: None,
            ema_working_set: 0.0,
            idle_ticks: 0,
            is_idle: false,
            ksm_merged_bytes: 0,
            history: Vec::with_capacity(MAX_HISTORY_SAMPLES),
        }
    }

    /// Push a new sample, maintaining the history window.
    fn push_sample(&mut self, sample: MemorySample) {
        let ws = sample.working_set_bytes() as f64;
        if self.ema_working_set < f64::EPSILON {
            self.ema_working_set = ws;
        } else {
            self.ema_working_set = EMA_ALPHA * ws + (1.0 - EMA_ALPHA) * self.ema_working_set;
        }

        self.latest_sample = Some(sample.clone());

        if self.history.len() >= MAX_HISTORY_SAMPLES {
            // Keep the newest half — simple compaction.
            let half = MAX_HISTORY_SAMPLES / 2;
            self.history.drain(..half);
        }
        self.history.push(sample);
    }

    /// Detect whether the container is idle (< threshold change over window).
    fn update_idle(&mut self, threshold: f64) {
        if self.history.len() < 2 {
            self.is_idle = false;
            self.idle_ticks = 0;
            return;
        }

        let latest = &self.history[self.history.len() - 1];
        let prev = &self.history[self.history.len() - 2];

        let delta = if prev.current_bytes > 0 {
            (latest.current_bytes as f64 - prev.current_bytes as f64).abs()
                / prev.current_bytes as f64
        } else {
            0.0
        };

        if delta < threshold {
            self.idle_ticks += 1;
        } else {
            self.idle_ticks = 0;
        }

        self.is_idle = self.idle_ticks >= IDLE_WINDOW_SECONDS;
    }
}

// ── Balloon actions ──────────────────────────────────────────────────────────

/// A balloon adjustment to be applied.
#[derive(Debug, Clone)]
pub struct BalloonAdjustment {
    /// Container identifier.
    pub container_id: String,
    /// Direction: positive = inflate (reclaim), negative = deflate (return).
    pub delta_bytes: i64,
    /// New target balloon size (bytes).
    pub target_bytes: u64,
    /// Reason for the adjustment.
    pub reason: BalloonReason,
}

/// Reason a balloon was adjusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BalloonReason {
    /// Regular reclaim — usage above high watermark.
    HighWatermark,
    /// Aggressive idle reclaim.
    IdleReclaim,
    /// Expansion — usage growing, need more headroom.
    Expansion,
    /// KSM savings applied.
    KsmSavings,
    /// Manual override / reset.
    Manual,
}

impl std::fmt::Display for BalloonReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HighWatermark => write!(f, "high-watermark"),
            Self::IdleReclaim => write!(f, "idle-reclaim"),
            Self::Expansion => write!(f, "expansion"),
            Self::KsmSavings => write!(f, "ksm-savings"),
            Self::Manual => write!(f, "manual"),
        }
    }
}

// ── Global statistics ────────────────────────────────────────────────────────

/// Aggregate statistics across all managed containers.
#[derive(Debug, Default)]
pub struct MemoryManagerStats {
    /// Total polls executed.
    pub polls_completed: AtomicU64,
    /// Total bytes reclaimed via balloon inflation.
    pub bytes_reclaimed: AtomicU64,
    /// Total bytes returned via balloon deflation.
    pub bytes_returned: AtomicU64,
    /// Total KSM savings (bytes).
    pub ksm_savings_bytes: AtomicU64,
    /// Number of balloon adjustments made.
    pub adjustments_made: AtomicU64,
    /// Number of containers currently tracked.
    pub containers_tracked: AtomicU64,
}

impl MemoryManagerStats {
    /// Snapshot the statistics into a serialisable form.
    #[must_use]
    pub fn snapshot(&self) -> MemoryStatsSnapshot {
        MemoryStatsSnapshot {
            polls_completed: self.polls_completed.load(Ordering::Relaxed),
            bytes_reclaimed: self.bytes_reclaimed.load(Ordering::Relaxed),
            bytes_returned: self.bytes_returned.load(Ordering::Relaxed),
            ksm_savings_bytes: self.ksm_savings_bytes.load(Ordering::Relaxed),
            adjustments_made: self.adjustments_made.load(Ordering::Relaxed),
            containers_tracked: self.containers_tracked.load(Ordering::Relaxed),
        }
    }
}

/// Serialisable snapshot of [`MemoryManagerStats`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatsSnapshot {
    /// Total polls executed.
    pub polls_completed: u64,
    /// Total bytes reclaimed via balloon inflation.
    pub bytes_reclaimed: u64,
    /// Total bytes returned via balloon deflation.
    pub bytes_returned: u64,
    /// Total KSM savings (bytes).
    pub ksm_savings_bytes: u64,
    /// Number of balloon adjustments made.
    pub adjustments_made: u64,
    /// Containers currently tracked.
    pub containers_tracked: u64,
}

// ── Free-page reporting ──────────────────────────────────────────────────────

/// Free-page reporting state.
///
/// On kernel 5.8+ the virtio balloon driver exposes free-page hints to the
/// VMM so the host can reclaim zero-cost pages without explicit inflation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreePageReport {
    /// Pages reported free.
    pub pages_reported: u64,
    /// Bytes equivalent.
    pub bytes_reported: u64,
    /// Last report timestamp (ms epoch).
    pub last_report_ms: u64,
}

// ── KSM state ────────────────────────────────────────────────────────────────

/// KSM (Kernel Same-page Merging) controller.
///
/// On kernel 6.4+ the `prctl(PR_SET_MEMORY_MERGE)` call enables per-process
/// page merging without needing `madvise(MADV_MERGEABLE)` annotations.
/// This is especially effective when running multiple similar containers
/// (e.g., many Node.js or JVM instances sharing identical library pages).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KsmStatus {
    /// Whether KSM is globally enabled on the host.
    pub host_enabled: bool,
    /// Pages currently shared (from `/sys/kernel/mm/ksm/pages_sharing`).
    pub pages_sharing: u64,
    /// Pages considered for sharing.
    pub pages_shared: u64,
    /// Bytes saved (pages_sharing × page_size).
    pub bytes_saved: u64,
    /// PIDs with `MMF_VM_MERGE_ANY` enabled.
    pub enabled_pids: Vec<u32>,
}

impl Default for KsmStatus {
    fn default() -> Self {
        Self {
            host_enabled: false,
            pages_sharing: 0,
            pages_shared: 0,
            bytes_saved: 0,
            enabled_pids: Vec::new(),
        }
    }
}

// ── DynamicMemoryManager ─────────────────────────────────────────────────────

/// Dynamic VM memory manager.
///
/// Coordinates balloon, free-page reporting, and KSM to keep host RAM usage
/// proportional to **actual** container working sets, solving Docker Desktop's
/// chronic memory bloat.
///
/// # Usage
///
/// ```rust,no_run
/// use hyperbox_optimize::memory::{DynamicMemoryManager, MemoryConfig};
///
/// # async fn example() -> hyperbox_optimize::Result<()> {
/// let manager = DynamicMemoryManager::new(MemoryConfig::default());
/// manager.register_container("abc123").await;
/// let adjustments = manager.poll_once().await?;
/// for adj in &adjustments {
///     println!("{}: {} bytes ({})", adj.container_id, adj.delta_bytes, adj.reason);
/// }
/// # Ok(())
/// # }
/// ```
pub struct DynamicMemoryManager {
    /// Configuration.
    config: MemoryConfig,
    /// Per-container state keyed by container identifier.
    containers: DashMap<String, ContainerMemoryState>,
    /// Global statistics.
    stats: Arc<MemoryManagerStats>,
    /// KSM status cache.
    ksm_status: Arc<RwLock<KsmStatus>>,
    /// Whether the polling loop is running.
    running: Arc<AtomicBool>,
    /// Host page size (bytes).
    page_size: u64,
}

impl DynamicMemoryManager {
    /// Create a new manager with the given configuration.
    #[must_use]
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            config,
            containers: DashMap::new(),
            stats: Arc::new(MemoryManagerStats::default()),
            ksm_status: Arc::new(RwLock::new(KsmStatus::default())),
            running: Arc::new(AtomicBool::new(false)),
            page_size: Self::detect_page_size(),
        }
    }

    /// Detect the host page size (typically 4096).
    fn detect_page_size() -> u64 {
        // SAFETY: sysconf is memory-safe and returns a libc::c_long.
        // On non-Unix we fall back to 4096.
        #[cfg(unix)]
        {
            // SAFETY: _SC_PAGESIZE is a valid sysconf name and the call
            // does not modify any state.
            let ps = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
            if ps > 0 {
                return ps as u64;
            }
        }
        4_096
    }

    // ── Container lifecycle ──────────────────────────────────────────────

    /// Register a container for memory management.
    pub async fn register_container(&self, container_id: impl Into<String>) {
        let id = container_id.into();
        info!(container = %id, "registering container for dynamic memory management");
        self.containers
            .insert(id.clone(), ContainerMemoryState::new(id));
        self.stats
            .containers_tracked
            .store(self.containers.len() as u64, Ordering::Relaxed);
    }

    /// Unregister a container (e.g., after stop/remove).
    pub async fn unregister_container(&self, container_id: &str) {
        if let Some((_id, state)) = self.containers.remove(container_id) {
            info!(
                container = %container_id,
                balloon_inflated = state.balloon_inflated_bytes,
                "unregistered container from memory management"
            );
        }
        self.stats
            .containers_tracked
            .store(self.containers.len() as u64, Ordering::Relaxed);
    }

    /// Number of tracked containers.
    #[must_use]
    pub fn tracked_count(&self) -> usize {
        self.containers.len()
    }

    // ── Sampling ─────────────────────────────────────────────────────────

    /// Read a `MemorySample` from the cgroup v2 filesystem for one container.
    ///
    /// The cgroup path is `{cgroup_root}/hyperbox/{container_id}` by default.
    pub async fn sample_container(&self, container_id: &str) -> Result<MemorySample> {
        let cg_path = self
            .config
            .cgroup_root
            .join("hyperbox")
            .join(container_id);

        let current_bytes = read_cgroup_u64(&cg_path, "memory.current").await?;
        let limit_bytes = read_cgroup_u64(&cg_path, "memory.max")
            .await
            .unwrap_or(u64::MAX);
        let swap_bytes = read_cgroup_u64(&cg_path, "memory.swap.current")
            .await
            .unwrap_or(0);

        let stat = read_cgroup_stat(&cg_path).await.unwrap_or_default();

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(MemorySample {
            timestamp_ms: now_ms,
            current_bytes,
            limit_bytes,
            swap_bytes,
            inactive_file_bytes: stat.inactive_file,
            active_file_bytes: stat.active_file,
            anon_bytes: stat.anon,
            slab_reclaimable_bytes: stat.slab_reclaimable,
        })
    }

    // ── Poll / decision loop ─────────────────────────────────────────────

    /// Execute one poll cycle: sample all containers, compute balloon
    /// adjustments, and return the list of changes to apply.
    ///
    /// The caller is responsible for actually inflating/deflating the
    /// balloon device (usually via `virtio-balloon` control socket).
    pub async fn poll_once(&self) -> Result<Vec<BalloonAdjustment>> {
        let mut adjustments = Vec::new();

        let container_ids: Vec<String> = self
            .containers
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for cid in &container_ids {
            match self.sample_container(cid).await {
                Ok(sample) => {
                    if let Some(mut state) = self.containers.get_mut(cid) {
                        state.push_sample(sample);
                        state.update_idle(self.config.idle_change_threshold);

                        if let Some(adj) = self.compute_adjustment(&state) {
                            adjustments.push(adj);
                        }
                    }
                }
                Err(e) => {
                    debug!(container = %cid, error = %e, "failed to sample container memory");
                }
            }
        }

        // Apply the computed targets.
        for adj in &adjustments {
            if let Some(mut state) = self.containers.get_mut(&adj.container_id) {
                let prev = state.balloon_inflated_bytes;
                state.balloon_target_bytes = adj.target_bytes;
                state.balloon_inflated_bytes = adj.target_bytes;

                if adj.delta_bytes > 0 {
                    self.stats
                        .bytes_reclaimed
                        .fetch_add(adj.delta_bytes as u64, Ordering::Relaxed);
                } else {
                    self.stats
                        .bytes_returned
                        .fetch_add(adj.delta_bytes.unsigned_abs(), Ordering::Relaxed);
                }
                self.stats.adjustments_made.fetch_add(1, Ordering::Relaxed);

                debug!(
                    container = %adj.container_id,
                    reason = %adj.reason,
                    delta = adj.delta_bytes,
                    prev_balloon = prev,
                    new_balloon = adj.target_bytes,
                    "balloon adjusted"
                );
            }
        }

        self.stats.polls_completed.fetch_add(1, Ordering::Relaxed);
        Ok(adjustments)
    }

    /// Compute a balloon adjustment for a single container.
    fn compute_adjustment(&self, state: &ContainerMemoryState) -> Option<BalloonAdjustment> {
        let sample = state.latest_sample.as_ref()?;
        let limit = sample.limit_bytes;
        if limit == 0 || limit == u64::MAX {
            return None; // no limit set — nothing to manage
        }

        let working_set = state.ema_working_set as u64;
        let headroom = (working_set as f64 * 0.25) as u64; // 25% headroom
        let desired = working_set
            .saturating_add(headroom)
            .max(self.config.min_memory_bytes);

        let current_balloon = state.balloon_inflated_bytes;
        let current_effective = limit.saturating_sub(current_balloon);

        // ── Expansion: container needs more memory ───────────────────────
        if sample.usage_ratio() > self.config.high_watermark {
            let new_effective = limit.min(desired.saturating_add(headroom));
            let new_balloon = limit.saturating_sub(new_effective);
            if new_balloon < current_balloon {
                let delta = current_balloon.saturating_sub(new_balloon);
                if delta >= MIN_BALLOON_STEP {
                    return Some(BalloonAdjustment {
                        container_id: state.container_id.clone(),
                        delta_bytes: -(delta as i64),
                        target_bytes: new_balloon,
                        reason: BalloonReason::Expansion,
                    });
                }
            }
        }

        // ── Idle reclaim: aggressive when container is idle ──────────────
        if state.is_idle && self.config.aggressive_idle_reclaim {
            let target_effective = working_set
                .saturating_add(self.config.min_memory_bytes)
                .max(self.config.min_memory_bytes);
            let new_balloon = limit.saturating_sub(target_effective);
            let delta = new_balloon.saturating_sub(current_balloon);
            if delta >= MIN_BALLOON_STEP {
                return Some(BalloonAdjustment {
                    container_id: state.container_id.clone(),
                    delta_bytes: delta as i64,
                    target_bytes: new_balloon,
                    reason: BalloonReason::IdleReclaim,
                });
            }
        }

        // ── Normal reclaim: working-set well below limit ─────────────────
        if desired < current_effective
            && sample.usage_ratio() < self.config.low_watermark
        {
            let new_balloon = limit.saturating_sub(desired);
            let delta = new_balloon.saturating_sub(current_balloon);
            if delta >= MIN_BALLOON_STEP {
                return Some(BalloonAdjustment {
                    container_id: state.container_id.clone(),
                    delta_bytes: delta as i64,
                    target_bytes: new_balloon,
                    reason: BalloonReason::HighWatermark,
                });
            }
        }

        None
    }

    // ── Background polling loop ──────────────────────────────────────────

    /// Start the background polling loop.
    ///
    /// This spawns a `tokio` task that repeatedly calls [`Self::poll_once`]
    /// at the configured interval. Call [`Self::stop`] to terminate.
    pub fn start_polling(self: &Arc<Self>) {
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            debug!("memory manager polling already running");
            return;
        }

        let manager = Arc::clone(self);
        tokio::spawn(async move {
            info!(
                interval_ms = manager.config.poll_interval_ms,
                "dynamic memory manager polling started"
            );

            let mut interval =
                tokio::time::interval(Duration::from_millis(manager.config.poll_interval_ms));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            while manager.running.load(Ordering::Relaxed) {
                interval.tick().await;
                match manager.poll_once().await {
                    Ok(adjustments) => {
                        if !adjustments.is_empty() {
                            info!(
                                count = adjustments.len(),
                                "applied balloon adjustments"
                            );
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "memory poll cycle failed");
                    }
                }

                // Optionally refresh KSM stats
                if manager.config.ksm_enabled {
                    if let Err(e) = manager.refresh_ksm_status().await {
                        debug!(error = %e, "KSM status refresh failed");
                    }
                }
            }

            info!("dynamic memory manager polling stopped");
        });
    }

    /// Stop the background polling loop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Whether the polling loop is running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    // ── KSM integration ──────────────────────────────────────────────────

    /// Enable KSM (`MMF_VM_MERGE_ANY`) for a container's init process.
    ///
    /// Requires kernel 6.4+ with `CONFIG_KSM=y`.
    pub async fn enable_ksm_for_container(&self, container_id: &str, pid: u32) -> Result<()> {
        if !self.config.ksm_enabled {
            return Err(OptimizeError::PredictionFailed {
                reason: "KSM is disabled in configuration".into(),
            });
        }

        info!(container = %container_id, pid, "enabling KSM for container process");

        // Write to /proc/<pid>/ksm_merging (kernel 6.4+).
        let ksm_path = PathBuf::from(format!("/proc/{pid}/ksm_merging"));
        tokio::fs::write(&ksm_path, "1")
            .await
            .map_err(|e| OptimizeError::Io(e))?;

        // Track the PID.
        let mut ksm = self.ksm_status.write().await;
        if !ksm.enabled_pids.contains(&pid) {
            ksm.enabled_pids.push(pid);
        }

        Ok(())
    }

    /// Refresh global KSM stats from `/sys/kernel/mm/ksm/`.
    async fn refresh_ksm_status(&self) -> Result<()> {
        let ksm_root = Path::new("/sys/kernel/mm/ksm");

        let pages_sharing = read_sysfs_u64(&ksm_root.join("pages_sharing"))
            .await
            .unwrap_or(0);
        let pages_shared = read_sysfs_u64(&ksm_root.join("pages_shared"))
            .await
            .unwrap_or(0);

        let mut status = self.ksm_status.write().await;
        status.host_enabled = ksm_root.join("run").exists();
        status.pages_sharing = pages_sharing;
        status.pages_shared = pages_shared;
        status.bytes_saved = pages_sharing * self.page_size;

        self.stats
            .ksm_savings_bytes
            .store(status.bytes_saved, Ordering::Relaxed);

        Ok(())
    }

    /// Get the current KSM status.
    pub async fn ksm_status(&self) -> KsmStatus {
        self.ksm_status.read().await.clone()
    }

    // ── Free-page reporting ──────────────────────────────────────────────

    /// Check whether the host kernel supports virtio-balloon free-page
    /// reporting.
    pub async fn free_page_reporting_available(&self) -> bool {
        // The feature is available if the virtio-balloon device exposes it.
        // We approximate by checking for the kernel config option.
        let kconfig = Path::new("/boot/config-")
            .parent()
            .unwrap_or(Path::new("/boot"));
        if let Ok(mut entries) = tokio::fs::read_dir(kconfig).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("config-") {
                    if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                        if content.contains("CONFIG_PAGE_REPORTING=y") {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    // ── Query helpers ────────────────────────────────────────────────────

    /// Get the current state of a tracked container.
    #[must_use]
    pub fn container_state(&self, container_id: &str) -> Option<ContainerMemoryState> {
        self.containers.get(container_id).map(|r| r.clone())
    }

    /// Get aggregate statistics.
    #[must_use]
    pub fn stats(&self) -> MemoryStatsSnapshot {
        self.stats.snapshot()
    }

    /// Get the configuration.
    #[must_use]
    pub fn config(&self) -> &MemoryConfig {
        &self.config
    }

    /// Total reclaimable memory across all tracked containers.
    #[must_use]
    pub fn total_reclaimable_bytes(&self) -> u64 {
        self.containers
            .iter()
            .map(|entry| {
                entry
                    .value()
                    .latest_sample
                    .as_ref()
                    .map_or(0, MemorySample::reclaimable_bytes)
            })
            .sum()
    }

    /// Total working-set memory across all tracked containers.
    #[must_use]
    pub fn total_working_set_bytes(&self) -> u64 {
        self.containers
            .iter()
            .map(|entry| {
                entry
                    .value()
                    .latest_sample
                    .as_ref()
                    .map_or(0, MemorySample::working_set_bytes)
            })
            .sum()
    }

    /// Manual balloon reset — deflate all balloons to zero.
    pub async fn reset_all_balloons(&self) {
        for mut entry in self.containers.iter_mut() {
            entry.value_mut().balloon_inflated_bytes = 0;
            entry.value_mut().balloon_target_bytes = 0;
        }
        info!("reset all balloon inflation to zero");
    }

    /// Manual balloon set for a specific container.
    pub async fn set_balloon(&self, container_id: &str, target_bytes: u64) -> Result<()> {
        let mut state = self
            .containers
            .get_mut(container_id)
            .ok_or_else(|| OptimizeError::ResourceExhausted {
                resource: format!("container not tracked: {container_id}"),
            })?;

        let prev = state.balloon_inflated_bytes;
        state.balloon_target_bytes = target_bytes;
        state.balloon_inflated_bytes = target_bytes;

        let delta = target_bytes as i64 - prev as i64;
        info!(
            container = %container_id,
            delta,
            target = target_bytes,
            "manual balloon adjustment"
        );

        Ok(())
    }
}

// ── cgroup v2 helpers ────────────────────────────────────────────────────────

/// Parsed fields from `memory.stat`.
#[derive(Debug, Default)]
struct CgroupMemStat {
    /// `inactive_file` — cache pages not recently used.
    inactive_file: u64,
    /// `active_file` — cache pages recently accessed.
    active_file: u64,
    /// `anon` — anonymous (heap/stack) pages.
    anon: u64,
    /// `slab_reclaimable` — kernel slab pages freeable under pressure.
    slab_reclaimable: u64,
}

/// Read a single u64 value from a cgroup file.
async fn read_cgroup_u64(cg_path: &Path, filename: &str) -> Result<u64> {
    let path = cg_path.join(filename);
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| OptimizeError::Io(e))?;

    let trimmed = content.trim();
    if trimmed == "max" {
        return Ok(u64::MAX);
    }
    trimmed.parse::<u64>().map_err(|_| {
        OptimizeError::PredictionFailed {
            reason: format!("cannot parse {}: {trimmed:?}", path.display()),
        }
    })
}

/// Read and parse `memory.stat` from a cgroup directory.
async fn read_cgroup_stat(cg_path: &Path) -> Result<CgroupMemStat> {
    let path = cg_path.join("memory.stat");
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| OptimizeError::Io(e))?;

    let mut stat = CgroupMemStat::default();
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next().unwrap_or_default();
        let val: u64 = parts
            .next()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        match key {
            "inactive_file" => stat.inactive_file = val,
            "active_file" => stat.active_file = val,
            "anon" => stat.anon = val,
            "slab_reclaimable" => stat.slab_reclaimable = val,
            _ => {}
        }
    }
    Ok(stat)
}

/// Read a single u64 from a sysfs file (e.g., `/sys/kernel/mm/ksm/…`).
async fn read_sysfs_u64(path: &Path) -> Result<u64> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| OptimizeError::Io(e))?;
    content
        .trim()
        .parse::<u64>()
        .map_err(|_| OptimizeError::PredictionFailed {
            reason: format!("cannot parse sysfs {}: {:?}", path.display(), content.trim()),
        })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample(current: u64, limit: u64, anon: u64, active_file: u64) -> MemorySample {
        MemorySample {
            timestamp_ms: 1_700_000_000_000,
            current_bytes: current,
            limit_bytes: limit,
            swap_bytes: 0,
            inactive_file_bytes: 0,
            active_file_bytes: active_file,
            anon_bytes: anon,
            slab_reclaimable_bytes: 0,
        }
    }

    #[test]
    fn sample_working_set() {
        let s = make_sample(500 * 1024 * 1024, 1024 * 1024 * 1024, 200 * 1024 * 1024, 50 * 1024 * 1024);
        assert_eq!(s.working_set_bytes(), 250 * 1024 * 1024);
    }

    #[test]
    fn sample_usage_ratio() {
        let s = make_sample(512 * 1024 * 1024, 1024 * 1024 * 1024, 0, 0);
        let ratio = s.usage_ratio();
        assert!((ratio - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn sample_reclaimable() {
        // current=500M, working_set=250M → reclaimable=250M
        let s = make_sample(500 * 1024 * 1024, 1024 * 1024 * 1024, 200 * 1024 * 1024, 50 * 1024 * 1024);
        assert_eq!(s.reclaimable_bytes(), 250 * 1024 * 1024);
    }

    #[test]
    fn sample_zero_limit() {
        let s = make_sample(100, 0, 50, 0);
        assert!(s.usage_ratio().abs() < f64::EPSILON);
    }

    #[test]
    fn container_state_ema_updates() {
        let mut state = ContainerMemoryState::new("test1");
        let s1 = make_sample(100, 1000, 50, 20);
        state.push_sample(s1);
        // First sample: EMA = working_set = 50 + 20 = 70
        assert!((state.ema_working_set - 70.0).abs() < 1.0);

        let s2 = make_sample(200, 1000, 100, 40);
        state.push_sample(s2);
        // EMA = 0.3 * 140 + 0.7 * 70 = 42 + 49 = 91
        assert!((state.ema_working_set - 91.0).abs() < 1.0);
    }

    #[test]
    fn container_state_idle_detection() {
        let mut state = ContainerMemoryState::new("test2");

        // Push many identical samples → should become idle
        for i in 0..40 {
            let s = make_sample(100, 1000, 50, 20);
            state.push_sample(MemorySample {
                timestamp_ms: 1_700_000_000_000 + i * 1000,
                ..s
            });
            state.update_idle(0.05);
        }
        assert!(state.is_idle, "should be idle after stable usage");
    }

    #[test]
    fn container_state_not_idle_with_changes() {
        let mut state = ContainerMemoryState::new("test3");

        // Push increasing samples → should NOT be idle
        for i in 0..40 {
            let current = 100 + i * 50; // growing memory
            let s = make_sample(current, 10_000, current / 2, current / 4);
            state.push_sample(MemorySample {
                timestamp_ms: 1_700_000_000_000 + i * 1000,
                ..s
            });
            state.update_idle(0.05);
        }
        assert!(!state.is_idle, "should not be idle with growing usage");
    }

    #[test]
    fn balloon_reason_display() {
        assert_eq!(format!("{}", BalloonReason::HighWatermark), "high-watermark");
        assert_eq!(format!("{}", BalloonReason::IdleReclaim), "idle-reclaim");
        assert_eq!(format!("{}", BalloonReason::Expansion), "expansion");
        assert_eq!(format!("{}", BalloonReason::KsmSavings), "ksm-savings");
        assert_eq!(format!("{}", BalloonReason::Manual), "manual");
    }

    #[test]
    fn default_config_sane() {
        let cfg = MemoryConfig::default();
        assert!(cfg.balloon_enabled);
        assert!(!cfg.ksm_enabled); // off by default
        assert!(cfg.high_watermark > cfg.low_watermark);
        assert!(cfg.min_memory_bytes > 0);
        assert_eq!(cfg.max_total_memory_bytes, 0); // auto-detect
    }

    #[test]
    fn stats_snapshot_zero() {
        let stats = MemoryManagerStats::default();
        let snap = stats.snapshot();
        assert_eq!(snap.polls_completed, 0);
        assert_eq!(snap.bytes_reclaimed, 0);
        assert_eq!(snap.adjustments_made, 0);
    }

    #[test]
    fn history_compaction() {
        let mut state = ContainerMemoryState::new("compact_test");
        for i in 0..MAX_HISTORY_SAMPLES + 100 {
            state.push_sample(make_sample(100, 1000, 50, 20));
        }
        // Should have been compacted, never exceed MAX_HISTORY_SAMPLES + 100
        assert!(state.history.len() <= MAX_HISTORY_SAMPLES);
    }

    #[tokio::test]
    async fn register_and_unregister() {
        let mgr = DynamicMemoryManager::new(MemoryConfig::default());
        mgr.register_container("c1").await;
        mgr.register_container("c2").await;
        assert_eq!(mgr.tracked_count(), 2);

        mgr.unregister_container("c1").await;
        assert_eq!(mgr.tracked_count(), 1);

        mgr.unregister_container("c2").await;
        assert_eq!(mgr.tracked_count(), 0);
    }

    #[tokio::test]
    async fn total_working_set_and_reclaimable() {
        let mgr = DynamicMemoryManager::new(MemoryConfig::default());
        mgr.register_container("w1").await;
        mgr.register_container("w2").await;

        // Manually inject samples.
        if let Some(mut s) = mgr.containers.get_mut("w1") {
            s.push_sample(make_sample(500, 1000, 200, 100)); // ws=300
        }
        if let Some(mut s) = mgr.containers.get_mut("w2") {
            s.push_sample(make_sample(400, 1000, 150, 80)); // ws=230
        }

        assert_eq!(mgr.total_working_set_bytes(), 300 + 230);
        assert_eq!(mgr.total_reclaimable_bytes(), (500 - 300) + (400 - 230));
    }

    #[tokio::test]
    async fn manual_balloon_set() {
        let mgr = DynamicMemoryManager::new(MemoryConfig::default());
        mgr.register_container("m1").await;
        mgr.set_balloon("m1", 100).await.unwrap();

        let state = mgr.container_state("m1").unwrap();
        assert_eq!(state.balloon_inflated_bytes, 100);
        assert_eq!(state.balloon_target_bytes, 100);
    }

    #[tokio::test]
    async fn manual_balloon_missing_container() {
        let mgr = DynamicMemoryManager::new(MemoryConfig::default());
        let result = mgr.set_balloon("nonexistent", 100).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn reset_all_balloons() {
        let mgr = DynamicMemoryManager::new(MemoryConfig::default());
        mgr.register_container("r1").await;
        mgr.register_container("r2").await;
        mgr.set_balloon("r1", 500).await.unwrap();
        mgr.set_balloon("r2", 300).await.unwrap();

        mgr.reset_all_balloons().await;

        let s1 = mgr.container_state("r1").unwrap();
        let s2 = mgr.container_state("r2").unwrap();
        assert_eq!(s1.balloon_inflated_bytes, 0);
        assert_eq!(s2.balloon_inflated_bytes, 0);
    }

    #[test]
    fn compute_adjustment_no_limit() {
        let mgr = DynamicMemoryManager::new(MemoryConfig::default());
        let mut state = ContainerMemoryState::new("nolimit");
        state.push_sample(make_sample(500, u64::MAX, 200, 100));

        let adj = mgr.compute_adjustment(&state);
        assert!(adj.is_none(), "should not adjust with no limit");
    }

    #[test]
    fn compute_adjustment_idle_reclaim() {
        let mut config = MemoryConfig::default();
        config.aggressive_idle_reclaim = true;
        let mgr = DynamicMemoryManager::new(config);

        let mut state = ContainerMemoryState::new("idle_reclaim");
        // Push enough identical samples to become idle.
        for _ in 0..40 {
            state.push_sample(make_sample(
                100 * 1024 * 1024,   // 100 MiB current
                1024 * 1024 * 1024,  // 1 GiB limit
                30 * 1024 * 1024,    // 30 MiB anon
                10 * 1024 * 1024,    // 10 MiB active_file
            ));
            state.update_idle(0.05);
        }
        assert!(state.is_idle);

        let adj = mgr.compute_adjustment(&state);
        // Idle container with small working-set → should reclaim.
        assert!(adj.is_some(), "idle container should get reclaim adjustment");
        let adj = adj.unwrap();
        assert!(adj.delta_bytes > 0, "should inflate balloon to reclaim");
        assert_eq!(adj.reason, BalloonReason::IdleReclaim);
    }

    #[test]
    fn ksm_status_defaults() {
        let status = KsmStatus::default();
        assert!(!status.host_enabled);
        assert_eq!(status.bytes_saved, 0);
        assert!(status.enabled_pids.is_empty());
    }

    #[test]
    fn free_page_report_fields() {
        let report = FreePageReport {
            pages_reported: 100,
            bytes_reported: 100 * 4096,
            last_report_ms: 1_700_000_000_000,
        };
        assert_eq!(report.bytes_reported, 409_600);
    }

    #[test]
    fn config_watermark_ordering() {
        let cfg = MemoryConfig::default();
        assert!(
            cfg.high_watermark > cfg.low_watermark,
            "high watermark must exceed low watermark"
        );
    }

    #[test]
    fn page_size_detection() {
        let ps = DynamicMemoryManager::detect_page_size();
        // Must be a power of 2 and at least 4 KiB.
        assert!(ps >= 4096);
        assert!(ps.is_power_of_two());
    }
}
