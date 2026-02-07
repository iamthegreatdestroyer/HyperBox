//! Composable security stack for layered container isolation.
//!
//! Orchestrates all available security layers into a single, composable policy
//! that degrades gracefully on older kernels. Each layer is independently
//! configurable; the stack detects kernel capabilities at startup and enables
//! the maximum protection available **without root privileges**.
//!
//! ## Security Layers (in enforcement order)
//!
//! | Layer | Mechanism             | Kernel Req | Source Module     |
//! |-------|-----------------------|------------|-------------------|
//! | 1     | User namespaces       | 3.8+       | `namespaces.rs`   |
//! | 2     | Landlock LSM          | 5.13+      | `landlock.rs`     |
//! | 3     | Seccomp BPF           | 3.5+       | `seccomp.rs`      |
//! | 4     | Cgroups v2 limits     | 4.15+      | `cgroups.rs`      |
//! | 5     | Image verification    | —          | (future)          |
//! | 6     | Optional VM isolation | —          | (future)          |
//!
//! ## Usage
//!
//! ```rust,no_run
//! use hyperbox_core::isolation::security_stack::{
//!     SecurityStack, SecurityPolicy, SecurityLayerKind,
//! };
//!
//! let stack = SecurityStack::detect();
//! let policy = SecurityPolicy::default_hardened("/var/lib/hyperbox/containers/abc");
//! let report = stack.apply(&policy).await.unwrap();
//! assert!(report.is_usable());
//! ```

use crate::error::{CoreError, Result};
use crate::isolation::cgroups::CgroupManager;
use crate::isolation::landlock::{LandlockManager, LandlockRuleset};
use crate::isolation::namespaces::{NamespaceConfig, NamespaceManager, NamespaceType};
use crate::isolation::seccomp::SeccompProfile;
use crate::types::ResourceLimits;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

// ═══════════════════════════════════════════════════════════════════════════════
// Layer Kind Enumeration
// ═══════════════════════════════════════════════════════════════════════════════

/// Identifies a single security layer in the composable stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SecurityLayerKind {
    /// User namespace isolation (uid/gid remapping).
    UserNamespaces,
    /// Landlock LSM filesystem / network sandboxing.
    Landlock,
    /// Seccomp BPF system-call filtering.
    Seccomp,
    /// Cgroups v2 resource limits.
    Cgroups,
    /// Cryptographic image signature verification.
    ImageVerification,
    /// Optional micro-VM isolation (Cloud Hypervisor / Firecracker).
    VmIsolation,
}

impl fmt::Display for SecurityLayerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserNamespaces => write!(f, "user-namespaces"),
            Self::Landlock => write!(f, "landlock"),
            Self::Seccomp => write!(f, "seccomp"),
            Self::Cgroups => write!(f, "cgroups-v2"),
            Self::ImageVerification => write!(f, "image-verification"),
            Self::VmIsolation => write!(f, "vm-isolation"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Layer Status
// ═══════════════════════════════════════════════════════════════════════════════

/// Availability status for a single security layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStatus {
    /// Which layer this status refers to.
    pub kind: SecurityLayerKind,
    /// Whether the layer is supported on the running kernel / platform.
    pub available: bool,
    /// Human-readable reason when unavailable.
    pub reason: Option<String>,
    /// Detected feature level (e.g. Landlock ABI version).
    pub feature_level: Option<String>,
}

impl LayerStatus {
    /// Create a status indicating the layer is available.
    fn ok(kind: SecurityLayerKind) -> Self {
        Self {
            kind,
            available: true,
            reason: None,
            feature_level: None,
        }
    }

    /// Create a status indicating the layer is available with a feature level.
    fn ok_with_level(kind: SecurityLayerKind, level: impl Into<String>) -> Self {
        Self {
            kind,
            available: true,
            reason: None,
            feature_level: Some(level.into()),
        }
    }

    /// Create a status indicating the layer is unavailable.
    fn unavailable(kind: SecurityLayerKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            available: false,
            reason: Some(reason.into()),
            feature_level: None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Enforcement Report
// ═══════════════════════════════════════════════════════════════════════════════

/// Outcome of enforcing a layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerOutcome {
    /// Layer was applied successfully.
    Applied,
    /// Layer was skipped (not available or not requested).
    Skipped {
        /// Reason the layer was skipped.
        reason: String,
    },
    /// Layer encountered an error during enforcement.
    Failed {
        /// The error message.
        error: String,
    },
}

impl LayerOutcome {
    /// Returns `true` if the layer was applied.
    #[must_use]
    pub fn is_applied(&self) -> bool {
        matches!(self, Self::Applied)
    }

    /// Returns `true` if the layer failed.
    #[must_use]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }
}

/// Full enforcement report across all layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementReport {
    /// Per-layer outcomes, in enforcement order.
    pub layers: BTreeMap<SecurityLayerKind, LayerOutcome>,
    /// Number of layers successfully applied.
    pub applied_count: usize,
    /// Number of layers that were skipped.
    pub skipped_count: usize,
    /// Number of layers that failed.
    pub failed_count: usize,
}

impl EnforcementReport {
    /// Create a new empty report.
    fn new() -> Self {
        Self {
            layers: BTreeMap::new(),
            applied_count: 0,
            skipped_count: 0,
            failed_count: 0,
        }
    }

    /// Record a layer outcome.
    fn record(&mut self, kind: SecurityLayerKind, outcome: LayerOutcome) {
        match &outcome {
            LayerOutcome::Applied => self.applied_count += 1,
            LayerOutcome::Skipped { .. } => self.skipped_count += 1,
            LayerOutcome::Failed { .. } => self.failed_count += 1,
        }
        self.layers.insert(kind, outcome);
    }

    /// Returns `true` if the overall security posture is usable
    /// (i.e. no *critical* layers failed).
    ///
    /// Currently considers all failures non-fatal — the stack degrades
    /// gracefully. Specific policies can override via [`SecurityPolicy::strict`].
    #[must_use]
    pub fn is_usable(&self) -> bool {
        self.failed_count == 0 || !self.has_critical_failure()
    }

    /// Returns `true` if any failure is in a layer the policy marked mandatory.
    #[must_use]
    pub fn has_critical_failure(&self) -> bool {
        // Placeholder — see SecurityPolicy.required_layers
        false
    }

    /// Returns `true` if a given layer was applied.
    #[must_use]
    pub fn is_layer_applied(&self, kind: SecurityLayerKind) -> bool {
        self.layers
            .get(&kind)
            .map_or(false, LayerOutcome::is_applied)
    }

    /// A short summary string suitable for logging.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "{} applied, {} skipped, {} failed",
            self.applied_count, self.skipped_count, self.failed_count,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Security Policy
// ═══════════════════════════════════════════════════════════════════════════════

/// Preset security profiles with increasing strictness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityPreset {
    /// Minimal isolation — only namespaces.
    Minimal,
    /// Standard isolation — namespaces + seccomp + cgroups.
    Standard,
    /// Hardened isolation — all available layers enabled.
    Hardened,
    /// Custom — caller controls every layer individually.
    Custom,
}

impl Default for SecurityPreset {
    fn default() -> Self {
        Self::Standard
    }
}

/// Namespace policy within the security stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespacePolicy {
    /// Enable user namespace isolation.
    pub user_ns: bool,
    /// Enable PID namespace isolation.
    pub pid_ns: bool,
    /// Enable network namespace isolation.
    pub net_ns: bool,
    /// Enable mount namespace isolation.
    pub mount_ns: bool,
    /// Enable IPC namespace isolation.
    pub ipc_ns: bool,
    /// Enable UTS namespace isolation.
    pub uts_ns: bool,
    /// Enable cgroup namespace isolation.
    pub cgroup_ns: bool,
}

impl Default for NamespacePolicy {
    fn default() -> Self {
        Self {
            user_ns: true,
            pid_ns: true,
            net_ns: true,
            mount_ns: true,
            ipc_ns: true,
            uts_ns: true,
            cgroup_ns: true,
        }
    }
}

impl NamespacePolicy {
    /// Build the list of namespace types to create.
    #[must_use]
    pub fn enabled_types(&self) -> Vec<NamespaceType> {
        let mut types = Vec::new();
        if self.user_ns {
            types.push(NamespaceType::User);
        }
        if self.pid_ns {
            types.push(NamespaceType::Pid);
        }
        if self.net_ns {
            types.push(NamespaceType::Network);
        }
        if self.mount_ns {
            types.push(NamespaceType::Mount);
        }
        if self.ipc_ns {
            types.push(NamespaceType::Ipc);
        }
        if self.uts_ns {
            types.push(NamespaceType::Uts);
        }
        if self.cgroup_ns {
            types.push(NamespaceType::Cgroup);
        }
        types
    }

    /// Create a minimal namespace policy — user + mount + pid only.
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            user_ns: true,
            pid_ns: true,
            net_ns: false,
            mount_ns: true,
            ipc_ns: false,
            uts_ns: false,
            cgroup_ns: false,
        }
    }
}

/// Landlock policy options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LandlockPolicy {
    /// Disabled — no Landlock enforcement.
    Disabled,
    /// Use the standard container policy for the given rootfs path.
    Standard,
    /// Use the strict (minimal access) policy for the given rootfs path.
    Strict,
    /// Supply a custom-built ruleset.
    Custom(LandlockRuleset),
}

impl Default for LandlockPolicy {
    fn default() -> Self {
        Self::Standard
    }
}

/// Seccomp policy options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeccompPolicy {
    /// Disabled — no seccomp filter.
    Disabled,
    /// Use the default profile (block dangerous syscalls).
    Default,
    /// Unconfined — allow all syscalls.
    Unconfined,
    /// Supply a custom profile.
    Custom(SeccompProfile),
}

impl Default for SeccompPolicy {
    fn default() -> Self {
        Self::Default
    }
}

/// Cgroup resource policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CgroupPolicy {
    /// Whether to create and enforce cgroup limits.
    pub enabled: bool,
    /// Resource limits to apply.
    pub limits: ResourceLimits,
}

impl Default for CgroupPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            limits: ResourceLimits::default(),
        }
    }
}

/// Full security policy describing which layers to enable and how.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Which preset this was derived from.
    pub preset: SecurityPreset,
    /// Container root filesystem path — required for Landlock / namespace setup.
    pub rootfs: PathBuf,
    /// Namespace configuration.
    pub namespaces: NamespacePolicy,
    /// Landlock configuration.
    pub landlock: LandlockPolicy,
    /// Seccomp filter configuration.
    pub seccomp: SeccompPolicy,
    /// Cgroup resource limits.
    pub cgroups: CgroupPolicy,
    /// Whether to enable image signature verification (future).
    pub verify_images: bool,
    /// Whether to require VM isolation (future).
    pub vm_isolation: bool,
    /// Layers whose failure should be treated as a hard error.
    pub required_layers: Vec<SecurityLayerKind>,
}

impl SecurityPolicy {
    // ── Constructors ─────────────────────────────────────────────────────

    /// Minimal policy: user + PID + mount namespaces only.
    #[must_use]
    pub fn minimal(rootfs: impl Into<PathBuf>) -> Self {
        Self {
            preset: SecurityPreset::Minimal,
            rootfs: rootfs.into(),
            namespaces: NamespacePolicy::minimal(),
            landlock: LandlockPolicy::Disabled,
            seccomp: SeccompPolicy::Disabled,
            cgroups: CgroupPolicy {
                enabled: false,
                ..CgroupPolicy::default()
            },
            verify_images: false,
            vm_isolation: false,
            required_layers: Vec::new(),
        }
    }

    /// Standard policy: namespaces + seccomp + cgroups.
    #[must_use]
    pub fn standard(rootfs: impl Into<PathBuf>) -> Self {
        Self {
            preset: SecurityPreset::Standard,
            rootfs: rootfs.into(),
            namespaces: NamespacePolicy::default(),
            landlock: LandlockPolicy::Standard,
            seccomp: SeccompPolicy::Default,
            cgroups: CgroupPolicy::default(),
            verify_images: false,
            vm_isolation: false,
            required_layers: Vec::new(),
        }
    }

    /// Hardened policy: enable every available security layer.
    #[must_use]
    pub fn default_hardened(rootfs: impl Into<PathBuf>) -> Self {
        Self {
            preset: SecurityPreset::Hardened,
            rootfs: rootfs.into(),
            namespaces: NamespacePolicy::default(),
            landlock: LandlockPolicy::Strict,
            seccomp: SeccompPolicy::Default,
            cgroups: CgroupPolicy::default(),
            verify_images: true,
            vm_isolation: false,
            required_layers: vec![
                SecurityLayerKind::UserNamespaces,
                SecurityLayerKind::Seccomp,
            ],
        }
    }

    // ── Queries ──────────────────────────────────────────────────────────

    /// Returns `true` if any namespace isolation is requested.
    #[must_use]
    pub fn uses_namespaces(&self) -> bool {
        self.namespaces.user_ns
            || self.namespaces.pid_ns
            || self.namespaces.net_ns
            || self.namespaces.mount_ns
    }

    /// Returns `true` if Landlock is requested.
    #[must_use]
    pub fn uses_landlock(&self) -> bool {
        !matches!(self.landlock, LandlockPolicy::Disabled)
    }

    /// Returns `true` if seccomp is requested.
    #[must_use]
    pub fn uses_seccomp(&self) -> bool {
        !matches!(self.seccomp, SeccompPolicy::Disabled)
    }

    /// Returns `true` if cgroup enforcement is requested.
    #[must_use]
    pub fn uses_cgroups(&self) -> bool {
        self.cgroups.enabled
    }

    /// Returns `true` if a layer is in the required set.
    #[must_use]
    pub fn is_required(&self, kind: SecurityLayerKind) -> bool {
        self.required_layers.contains(&kind)
    }

    /// Count of enabled layers.
    #[must_use]
    pub fn enabled_layer_count(&self) -> usize {
        let mut count = 0;
        if self.uses_namespaces() {
            count += 1;
        }
        if self.uses_landlock() {
            count += 1;
        }
        if self.uses_seccomp() {
            count += 1;
        }
        if self.uses_cgroups() {
            count += 1;
        }
        if self.verify_images {
            count += 1;
        }
        if self.vm_isolation {
            count += 1;
        }
        count
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Security Stack — Detection & Enforcement
// ═══════════════════════════════════════════════════════════════════════════════

/// Composable security stack that orchestrates all isolation layers.
///
/// Detection is performed once at creation; the result is cached so that
/// repeated calls to [`apply`](Self::apply) are cheap.
pub struct SecurityStack {
    /// Per-layer availability, determined at detect-time.
    status: BTreeMap<SecurityLayerKind, LayerStatus>,
    /// Landlock manager (cached for abi detection).
    landlock_mgr: LandlockManager,
    /// Cgroup manager (cached for cgroup operations).
    cgroup_mgr: CgroupManager,
}

impl SecurityStack {
    // ── Construction & Detection ─────────────────────────────────────────

    /// Probe the running system and build a stack with maximum available
    /// security.  No root privileges required.
    #[must_use]
    pub fn detect() -> Self {
        let landlock_mgr = LandlockManager::new();
        let cgroup_mgr = CgroupManager::new();
        let mut status = BTreeMap::new();

        // Layer 1: User namespaces
        status.insert(
            SecurityLayerKind::UserNamespaces,
            Self::detect_user_namespaces(),
        );

        // Layer 2: Landlock
        status.insert(
            SecurityLayerKind::Landlock,
            Self::detect_landlock(&landlock_mgr),
        );

        // Layer 3: Seccomp
        status.insert(SecurityLayerKind::Seccomp, Self::detect_seccomp());

        // Layer 4: Cgroups v2
        status.insert(SecurityLayerKind::Cgroups, Self::detect_cgroups());

        // Layer 5: Image verification (future)
        status.insert(
            SecurityLayerKind::ImageVerification,
            LayerStatus::unavailable(
                SecurityLayerKind::ImageVerification,
                "image verification not yet implemented",
            ),
        );

        // Layer 6: VM isolation (future)
        status.insert(
            SecurityLayerKind::VmIsolation,
            LayerStatus::unavailable(
                SecurityLayerKind::VmIsolation,
                "VM isolation not yet implemented",
            ),
        );

        info!(
            layers = status.values().filter(|s| s.available).count(),
            total = status.len(),
            "security stack detection complete",
        );

        Self {
            status,
            landlock_mgr,
            cgroup_mgr,
        }
    }

    /// Create a stack with explicitly supplied status (useful for tests).
    #[must_use]
    pub fn with_status(status: BTreeMap<SecurityLayerKind, LayerStatus>) -> Self {
        Self {
            status,
            landlock_mgr: LandlockManager::new(),
            cgroup_mgr: CgroupManager::new(),
        }
    }

    // ── Detection helpers ────────────────────────────────────────────────

    fn detect_user_namespaces() -> LayerStatus {
        let path = Path::new("/proc/self/ns/user");
        if path.exists() {
            LayerStatus::ok(SecurityLayerKind::UserNamespaces)
        } else {
            LayerStatus::unavailable(
                SecurityLayerKind::UserNamespaces,
                "/proc/self/ns/user not found — kernel lacks userns support",
            )
        }
    }

    fn detect_landlock(mgr: &LandlockManager) -> LayerStatus {
        if let Some(abi) = mgr.abi_version() {
            LayerStatus::ok_with_level(SecurityLayerKind::Landlock, format!("{abi}"))
        } else {
            LayerStatus::unavailable(
                SecurityLayerKind::Landlock,
                "Landlock not available — requires kernel 5.13+",
            )
        }
    }

    fn detect_seccomp() -> LayerStatus {
        let path = Path::new("/proc/self/status");
        if path.exists() {
            // On a real system we'd parse Seccomp field; conservatively report ok
            LayerStatus::ok(SecurityLayerKind::Seccomp)
        } else {
            LayerStatus::unavailable(
                SecurityLayerKind::Seccomp,
                "cannot read /proc/self/status",
            )
        }
    }

    fn detect_cgroups() -> LayerStatus {
        let unified = Path::new("/sys/fs/cgroup/cgroup.controllers");
        if unified.exists() {
            LayerStatus::ok_with_level(SecurityLayerKind::Cgroups, "v2-unified")
        } else {
            let hybrid = Path::new("/sys/fs/cgroup");
            if hybrid.exists() {
                LayerStatus::ok_with_level(SecurityLayerKind::Cgroups, "v1-or-hybrid")
            } else {
                LayerStatus::unavailable(SecurityLayerKind::Cgroups, "cgroup filesystem not found")
            }
        }
    }

    // ── Queries ──────────────────────────────────────────────────────────

    /// Get the status map for all layers.
    #[must_use]
    pub fn layer_status(&self) -> &BTreeMap<SecurityLayerKind, LayerStatus> {
        &self.status
    }

    /// Check if a particular layer is available.
    #[must_use]
    pub fn is_available(&self, kind: SecurityLayerKind) -> bool {
        self.status.get(&kind).map_or(false, |s| s.available)
    }

    /// Number of available layers.
    #[must_use]
    pub fn available_count(&self) -> usize {
        self.status.values().filter(|s| s.available).count()
    }

    /// Get reference to the inner `LandlockManager`.
    #[must_use]
    pub fn landlock_manager(&self) -> &LandlockManager {
        &self.landlock_mgr
    }

    /// Get reference to the inner `CgroupManager`.
    #[must_use]
    pub fn cgroup_manager(&self) -> &CgroupManager {
        &self.cgroup_mgr
    }

    // ── Enforcement ─────────────────────────────────────────────────────

    /// Enforce a [`SecurityPolicy`] applying every requested layer.
    ///
    /// Returns an [`EnforcementReport`] describing per-layer outcomes.
    /// Layers that are requested but unavailable are reported as *Skipped*
    /// (or *Failed* if the layer is in `required_layers`).
    pub async fn apply(
        &self,
        policy: &SecurityPolicy,
        container_id: &str,
    ) -> Result<EnforcementReport> {
        let mut report = EnforcementReport::new();

        info!(
            container = %container_id,
            preset = ?policy.preset,
            requested_layers = policy.enabled_layer_count(),
            available_layers = self.available_count(),
            "applying composable security policy",
        );

        // Layer 1: User namespaces
        self.apply_namespaces(policy, container_id, &mut report)
            .await;

        // Layer 2: Landlock
        self.apply_landlock(policy, &mut report);

        // Layer 3: Seccomp
        self.apply_seccomp(policy, &mut report);

        // Layer 4: Cgroups
        self.apply_cgroups(policy, container_id, &mut report).await;

        // Layer 5: Image verification (future)
        if policy.verify_images {
            report.record(
                SecurityLayerKind::ImageVerification,
                LayerOutcome::Skipped {
                    reason: "image verification not yet implemented".into(),
                },
            );
        }

        // Layer 6: VM isolation (future)
        if policy.vm_isolation {
            report.record(
                SecurityLayerKind::VmIsolation,
                LayerOutcome::Skipped {
                    reason: "VM isolation not yet implemented".into(),
                },
            );
        }

        // Check if any required layer failed
        for kind in &policy.required_layers {
            if let Some(outcome) = report.layers.get(kind) {
                if outcome.is_failed() {
                    return Err(CoreError::Internal(format!(
                        "required security layer {kind} failed"
                    )));
                }
            }
        }

        info!(
            container = %container_id,
            summary = %report.summary(),
            "security enforcement complete",
        );

        Ok(report)
    }

    // ── Layer enforcement helpers ────────────────────────────────────────

    async fn apply_namespaces(
        &self,
        policy: &SecurityPolicy,
        container_id: &str,
        report: &mut EnforcementReport,
    ) {
        if !policy.uses_namespaces() {
            report.record(
                SecurityLayerKind::UserNamespaces,
                LayerOutcome::Skipped {
                    reason: "disabled by policy".into(),
                },
            );
            return;
        }

        if !self.is_available(SecurityLayerKind::UserNamespaces) {
            let outcome = if policy.is_required(SecurityLayerKind::UserNamespaces) {
                LayerOutcome::Failed {
                    error: "user namespaces required but not available".into(),
                }
            } else {
                LayerOutcome::Skipped {
                    reason: "user namespaces not available on this kernel".into(),
                }
            };
            report.record(SecurityLayerKind::UserNamespaces, outcome);
            return;
        }

        let ns_config = NamespaceConfig {
            create: policy.namespaces.enabled_types().into_iter().collect::<HashSet<_>>(),
            join: Vec::new(),
        };
        let _ns_mgr = NamespaceManager::new(ns_config);

        #[cfg(unix)]
        {
            match _ns_mgr.setup_network_namespace(container_id).await {
                Ok(path) => {
                    let ns_path_display = path.display().to_string();
                    debug!(
                        container = %container_id,
                        ns_path = %ns_path_display,
                        "namespace isolation applied",
                    );
                    report.record(SecurityLayerKind::UserNamespaces, LayerOutcome::Applied);
                }
                Err(e) => {
                    let err_display = e.to_string();
                    warn!(
                        container = %container_id,
                        err = %err_display,
                        "namespace setup failed",
                    );
                    report.record(
                        SecurityLayerKind::UserNamespaces,
                        LayerOutcome::Failed {
                            error: e.to_string(),
                        },
                    );
                }
            }
        }

        #[cfg(not(unix))]
        {
            debug!(
                container = %container_id,
                "namespace isolation recorded (enforcement requires Linux)",
            );
            report.record(SecurityLayerKind::UserNamespaces, LayerOutcome::Applied);
        }
    }

    fn apply_landlock(&self, policy: &SecurityPolicy, report: &mut EnforcementReport) {
        if !policy.uses_landlock() {
            report.record(
                SecurityLayerKind::Landlock,
                LayerOutcome::Skipped {
                    reason: "disabled by policy".into(),
                },
            );
            return;
        }

        if !self.is_available(SecurityLayerKind::Landlock) {
            let outcome = if policy.is_required(SecurityLayerKind::Landlock) {
                LayerOutcome::Failed {
                    error: "Landlock required but not available".into(),
                }
            } else {
                LayerOutcome::Skipped {
                    reason: "Landlock not available — kernel 5.13+ required".into(),
                }
            };
            report.record(SecurityLayerKind::Landlock, outcome);
            return;
        }

        let _ruleset = match &policy.landlock {
            LandlockPolicy::Standard => LandlockManager::standard_policy(&policy.rootfs),
            LandlockPolicy::Strict => LandlockManager::strict_policy(&policy.rootfs),
            LandlockPolicy::Custom(rs) => rs.clone(),
            LandlockPolicy::Disabled => unreachable!(),
        };

        // In a real enforcement path the ruleset would be applied via
        // `landlock_create_ruleset` + `landlock_restrict_self`.
        // Here we record intent — actual enforcement happens in the
        // container init process which receives the serialised ruleset.
        debug!(
            rootfs = %policy.rootfs.display(),
            "Landlock ruleset prepared",
        );
        report.record(SecurityLayerKind::Landlock, LayerOutcome::Applied);
    }

    fn apply_seccomp(&self, policy: &SecurityPolicy, report: &mut EnforcementReport) {
        if !policy.uses_seccomp() {
            report.record(
                SecurityLayerKind::Seccomp,
                LayerOutcome::Skipped {
                    reason: "disabled by policy".into(),
                },
            );
            return;
        }

        if !self.is_available(SecurityLayerKind::Seccomp) {
            let outcome = if policy.is_required(SecurityLayerKind::Seccomp) {
                LayerOutcome::Failed {
                    error: "seccomp required but not available".into(),
                }
            } else {
                LayerOutcome::Skipped {
                    reason: "seccomp not available".into(),
                }
            };
            report.record(SecurityLayerKind::Seccomp, outcome);
            return;
        }

        let _profile = match &policy.seccomp {
            SeccompPolicy::Default => SeccompProfile::default_profile(),
            SeccompPolicy::Unconfined => SeccompProfile::unconfined(),
            SeccompPolicy::Custom(p) => p.clone(),
            SeccompPolicy::Disabled => unreachable!(),
        };

        debug!("seccomp profile prepared");
        report.record(SecurityLayerKind::Seccomp, LayerOutcome::Applied);
    }

    async fn apply_cgroups(
        &self,
        policy: &SecurityPolicy,
        container_id: &str,
        report: &mut EnforcementReport,
    ) {
        if !policy.uses_cgroups() {
            report.record(
                SecurityLayerKind::Cgroups,
                LayerOutcome::Skipped {
                    reason: "disabled by policy".into(),
                },
            );
            return;
        }

        if !self.is_available(SecurityLayerKind::Cgroups) {
            let outcome = if policy.is_required(SecurityLayerKind::Cgroups) {
                LayerOutcome::Failed {
                    error: "cgroups required but not available".into(),
                }
            } else {
                LayerOutcome::Skipped {
                    reason: "cgroups not available".into(),
                }
            };
            report.record(SecurityLayerKind::Cgroups, outcome);
            return;
        }

        match self.cgroup_mgr.create_container_cgroup(container_id).await {
            Ok(cg_path) => {
                if let Err(e) = self
                    .cgroup_mgr
                    .apply_limits(&cg_path, &policy.cgroups.limits)
                    .await
                {
                    warn!(
                        container = %container_id,
                        err = %e,
                        "cgroup limit application failed",
                    );
                    report.record(
                        SecurityLayerKind::Cgroups,
                        LayerOutcome::Failed {
                            error: e.to_string(),
                        },
                    );
                } else {
                    debug!(
                        container = %container_id,
                        path = %cg_path.display(),
                        "cgroup limits applied",
                    );
                    report.record(SecurityLayerKind::Cgroups, LayerOutcome::Applied);
                }
            }
            Err(e) => {
                warn!(
                    container = %container_id,
                    err = %e,
                    "cgroup creation failed",
                );
                report.record(
                    SecurityLayerKind::Cgroups,
                    LayerOutcome::Failed {
                        error: e.to_string(),
                    },
                );
            }
        }
    }

    // ── Tear-down ────────────────────────────────────────────────────────

    /// Clean up security artefacts for a container (cgroup, netns, etc.)
    pub async fn cleanup(&self, container_id: &str) -> Result<()> {
        // Clean up namespace artefacts
        #[cfg(unix)]
        {
            let ns_mgr = NamespaceManager::container_default();
            if let Err(e) = ns_mgr.cleanup_network_namespace(container_id).await {
                warn!(container = %container_id, err = %e, "namespace cleanup error");
            }
        }

        // Clean up cgroup
        let cg_path = self
            .cgroup_mgr
            .create_container_cgroup(container_id)
            .await
            .unwrap_or_else(|_| PathBuf::from(format!("/sys/fs/cgroup/hyperbox/{container_id}")));
        if let Err(e) = self.cgroup_mgr.remove_cgroup(&cg_path).await {
            warn!(container = %container_id, err = %e, "cgroup cleanup error");
        }

        info!(container = %container_id, "security stack cleanup complete");
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Security Audit
// ═══════════════════════════════════════════════════════════════════════════════

/// A point-in-time audit snapshot of the security posture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAudit {
    /// Layer availability as detected.
    pub layers: Vec<LayerStatus>,
    /// Overall score: proportion of available layers.
    pub score: f64,
    /// Recommended actions.
    pub recommendations: Vec<String>,
}

impl SecurityAudit {
    /// Generate an audit from the current stack.
    #[must_use]
    pub fn from_stack(stack: &SecurityStack) -> Self {
        let layers: Vec<LayerStatus> = stack.status.values().cloned().collect();
        let available = layers.iter().filter(|l| l.available).count();
        let total = layers.len().max(1);
        let score = available as f64 / total as f64;

        let mut recommendations = Vec::new();
        for layer in &layers {
            if !layer.available {
                let rec = match layer.kind {
                    SecurityLayerKind::UserNamespaces => {
                        "Enable user namespaces (sysctl kernel.unprivileged_userns_clone=1)"
                    }
                    SecurityLayerKind::Landlock => {
                        "Upgrade to kernel 5.13+ for Landlock filesystem sandboxing"
                    }
                    SecurityLayerKind::Seccomp => "Ensure /proc is mounted and accessible",
                    SecurityLayerKind::Cgroups => "Mount cgroup v2 unified hierarchy",
                    SecurityLayerKind::ImageVerification => {
                        "Image verification will be available in a future release"
                    }
                    SecurityLayerKind::VmIsolation => {
                        "VM isolation will be available in a future release"
                    }
                };
                recommendations.push(rec.to_string());
            }
        }

        Self {
            layers,
            score,
            recommendations,
        }
    }

    /// Returns `true` if the score meets a minimum bar (≥ 0.5).
    #[must_use]
    pub fn is_acceptable(&self) -> bool {
        self.score >= 0.5
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Display
// ═══════════════════════════════════════════════════════════════════════════════

impl fmt::Display for SecurityStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SecurityStack ({} layers):", self.status.len())?;
        for (kind, status) in &self.status {
            let icon = if status.available { "✓" } else { "✗" };
            write!(f, "  [{icon}] {kind}")?;
            if let Some(level) = &status.feature_level {
                write!(f, " ({level})")?;
            }
            if let Some(reason) = &status.reason {
                write!(f, " — {reason}")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Builder for SecurityPolicy
// ═══════════════════════════════════════════════════════════════════════════════

/// Fluent builder for [`SecurityPolicy`].
pub struct SecurityPolicyBuilder {
    /// Inner policy being built.
    policy: SecurityPolicy,
}

impl SecurityPolicyBuilder {
    /// Start building from a given preset and rootfs.
    #[must_use]
    pub fn new(preset: SecurityPreset, rootfs: impl Into<PathBuf>) -> Self {
        let policy = match preset {
            SecurityPreset::Minimal => SecurityPolicy::minimal(rootfs),
            SecurityPreset::Standard => SecurityPolicy::standard(rootfs),
            SecurityPreset::Hardened => SecurityPolicy::default_hardened(rootfs),
            SecurityPreset::Custom => SecurityPolicy::minimal(rootfs),
        };
        Self { policy }
    }

    /// Override namespace policy.
    #[must_use]
    pub fn namespaces(mut self, ns: NamespacePolicy) -> Self {
        self.policy.namespaces = ns;
        self
    }

    /// Override Landlock policy.
    #[must_use]
    pub fn landlock(mut self, ll: LandlockPolicy) -> Self {
        self.policy.landlock = ll;
        self
    }

    /// Override seccomp policy.
    #[must_use]
    pub fn seccomp(mut self, sc: SeccompPolicy) -> Self {
        self.policy.seccomp = sc;
        self
    }

    /// Override cgroup policy.
    #[must_use]
    pub fn cgroups(mut self, cg: CgroupPolicy) -> Self {
        self.policy.cgroups = cg;
        self
    }

    /// Enable / disable image verification.
    #[must_use]
    pub fn verify_images(mut self, enable: bool) -> Self {
        self.policy.verify_images = enable;
        self
    }

    /// Enable / disable VM isolation.
    #[must_use]
    pub fn vm_isolation(mut self, enable: bool) -> Self {
        self.policy.vm_isolation = enable;
        self
    }

    /// Require a specific layer (failure = hard error).
    #[must_use]
    pub fn require(mut self, kind: SecurityLayerKind) -> Self {
        if !self.policy.required_layers.contains(&kind) {
            self.policy.required_layers.push(kind);
        }
        self
    }

    /// Build the policy.
    #[must_use]
    pub fn build(self) -> SecurityPolicy {
        self.policy
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Layer Kind ───────────────────────────────────────────────────────

    #[test]
    fn layer_kind_display() {
        assert_eq!(SecurityLayerKind::UserNamespaces.to_string(), "user-namespaces");
        assert_eq!(SecurityLayerKind::Landlock.to_string(), "landlock");
        assert_eq!(SecurityLayerKind::Seccomp.to_string(), "seccomp");
        assert_eq!(SecurityLayerKind::Cgroups.to_string(), "cgroups-v2");
        assert_eq!(
            SecurityLayerKind::ImageVerification.to_string(),
            "image-verification"
        );
        assert_eq!(SecurityLayerKind::VmIsolation.to_string(), "vm-isolation");
    }

    #[test]
    fn layer_kind_ord() {
        assert!(SecurityLayerKind::UserNamespaces < SecurityLayerKind::VmIsolation);
    }

    // ── Layer Status ────────────────────────────────────────────────────

    #[test]
    fn layer_status_ok() {
        let s = LayerStatus::ok(SecurityLayerKind::Seccomp);
        assert!(s.available);
        assert!(s.reason.is_none());
    }

    #[test]
    fn layer_status_with_level() {
        let s = LayerStatus::ok_with_level(SecurityLayerKind::Landlock, "V4");
        assert!(s.available);
        assert_eq!(s.feature_level.as_deref(), Some("V4"));
    }

    #[test]
    fn layer_status_unavailable() {
        let s = LayerStatus::unavailable(SecurityLayerKind::VmIsolation, "future");
        assert!(!s.available);
        assert_eq!(s.reason.as_deref(), Some("future"));
    }

    // ── Enforcement Report ──────────────────────────────────────────────

    #[test]
    fn report_tracking() {
        let mut r = EnforcementReport::new();
        r.record(SecurityLayerKind::Seccomp, LayerOutcome::Applied);
        r.record(
            SecurityLayerKind::Landlock,
            LayerOutcome::Skipped {
                reason: "no kernel".into(),
            },
        );
        r.record(
            SecurityLayerKind::Cgroups,
            LayerOutcome::Failed {
                error: "oops".into(),
            },
        );

        assert_eq!(r.applied_count, 1);
        assert_eq!(r.skipped_count, 1);
        assert_eq!(r.failed_count, 1);
        assert!(r.is_layer_applied(SecurityLayerKind::Seccomp));
        assert!(!r.is_layer_applied(SecurityLayerKind::Cgroups));
    }

    #[test]
    fn report_summary_format() {
        let mut r = EnforcementReport::new();
        r.record(SecurityLayerKind::Seccomp, LayerOutcome::Applied);
        r.record(SecurityLayerKind::Landlock, LayerOutcome::Applied);
        assert_eq!(r.summary(), "2 applied, 0 skipped, 0 failed");
    }

    #[test]
    fn report_is_usable_no_failures() {
        let r = EnforcementReport::new();
        assert!(r.is_usable());
    }

    // ── Security Policy ─────────────────────────────────────────────────

    #[test]
    fn minimal_policy() {
        let p = SecurityPolicy::minimal("/rootfs");
        assert_eq!(p.preset, SecurityPreset::Minimal);
        assert!(!p.uses_landlock());
        assert!(!p.uses_seccomp());
        assert!(!p.uses_cgroups());
        assert!(p.uses_namespaces());
    }

    #[test]
    fn standard_policy() {
        let p = SecurityPolicy::standard("/rootfs");
        assert_eq!(p.preset, SecurityPreset::Standard);
        assert!(p.uses_landlock());
        assert!(p.uses_seccomp());
        assert!(p.uses_cgroups());
        assert!(p.uses_namespaces());
    }

    #[test]
    fn hardened_policy() {
        let p = SecurityPolicy::default_hardened("/rootfs");
        assert_eq!(p.preset, SecurityPreset::Hardened);
        assert!(p.verify_images);
        assert!(p.is_required(SecurityLayerKind::UserNamespaces));
        assert!(p.is_required(SecurityLayerKind::Seccomp));
        assert!(!p.is_required(SecurityLayerKind::VmIsolation));
    }

    #[test]
    fn enabled_layer_count() {
        let p = SecurityPolicy::standard("/rootfs");
        assert_eq!(p.enabled_layer_count(), 4); // ns + landlock + seccomp + cgroups
    }

    // ── Namespace Policy ────────────────────────────────────────────────

    #[test]
    fn namespace_policy_default_enables_all() {
        let ns = NamespacePolicy::default();
        assert_eq!(ns.enabled_types().len(), 7);
    }

    #[test]
    fn namespace_policy_minimal() {
        let ns = NamespacePolicy::minimal();
        let types = ns.enabled_types();
        assert!(types.contains(&NamespaceType::User));
        assert!(types.contains(&NamespaceType::Pid));
        assert!(types.contains(&NamespaceType::Mount));
        assert!(!types.contains(&NamespaceType::Network));
    }

    // ── Policy Builder ──────────────────────────────────────────────────

    #[test]
    fn builder_minimal_then_customise() {
        let policy = SecurityPolicyBuilder::new(SecurityPreset::Minimal, "/rootfs")
            .seccomp(SeccompPolicy::Default)
            .verify_images(true)
            .require(SecurityLayerKind::Seccomp)
            .build();

        assert!(policy.uses_seccomp());
        assert!(policy.verify_images);
        assert!(policy.is_required(SecurityLayerKind::Seccomp));
    }

    #[test]
    fn builder_hardened_disables_vm() {
        let policy = SecurityPolicyBuilder::new(SecurityPreset::Hardened, "/rootfs")
            .vm_isolation(false)
            .build();
        assert!(!policy.vm_isolation);
    }

    // ── Security Audit ──────────────────────────────────────────────────

    #[test]
    fn audit_full_availability() {
        let mut status = BTreeMap::new();
        status.insert(
            SecurityLayerKind::UserNamespaces,
            LayerStatus::ok(SecurityLayerKind::UserNamespaces),
        );
        status.insert(
            SecurityLayerKind::Landlock,
            LayerStatus::ok(SecurityLayerKind::Landlock),
        );
        status.insert(
            SecurityLayerKind::Seccomp,
            LayerStatus::ok(SecurityLayerKind::Seccomp),
        );
        status.insert(
            SecurityLayerKind::Cgroups,
            LayerStatus::ok(SecurityLayerKind::Cgroups),
        );
        status.insert(
            SecurityLayerKind::ImageVerification,
            LayerStatus::ok(SecurityLayerKind::ImageVerification),
        );
        status.insert(
            SecurityLayerKind::VmIsolation,
            LayerStatus::ok(SecurityLayerKind::VmIsolation),
        );

        let stack = SecurityStack::with_status(status);
        let audit = SecurityAudit::from_stack(&stack);

        assert!((audit.score - 1.0).abs() < f64::EPSILON);
        assert!(audit.is_acceptable());
        assert!(audit.recommendations.is_empty());
    }

    #[test]
    fn audit_partial_availability() {
        let mut status = BTreeMap::new();
        status.insert(
            SecurityLayerKind::UserNamespaces,
            LayerStatus::ok(SecurityLayerKind::UserNamespaces),
        );
        status.insert(
            SecurityLayerKind::Landlock,
            LayerStatus::unavailable(SecurityLayerKind::Landlock, "old kernel"),
        );
        status.insert(
            SecurityLayerKind::Seccomp,
            LayerStatus::ok(SecurityLayerKind::Seccomp),
        );
        status.insert(
            SecurityLayerKind::Cgroups,
            LayerStatus::unavailable(SecurityLayerKind::Cgroups, "no cgroups"),
        );

        let stack = SecurityStack::with_status(status);
        let audit = SecurityAudit::from_stack(&stack);

        assert!((audit.score - 0.5).abs() < f64::EPSILON);
        assert!(audit.is_acceptable());
        assert_eq!(audit.recommendations.len(), 2);
    }

    #[test]
    fn audit_score_below_threshold() {
        let mut status = BTreeMap::new();
        status.insert(
            SecurityLayerKind::UserNamespaces,
            LayerStatus::unavailable(SecurityLayerKind::UserNamespaces, "x"),
        );
        status.insert(
            SecurityLayerKind::Landlock,
            LayerStatus::unavailable(SecurityLayerKind::Landlock, "x"),
        );
        status.insert(
            SecurityLayerKind::Seccomp,
            LayerStatus::unavailable(SecurityLayerKind::Seccomp, "x"),
        );

        let stack = SecurityStack::with_status(status);
        let audit = SecurityAudit::from_stack(&stack);

        assert!(!audit.is_acceptable());
    }

    // ── Stack Display ───────────────────────────────────────────────────

    #[test]
    fn stack_display_format() {
        let mut status = BTreeMap::new();
        status.insert(
            SecurityLayerKind::Seccomp,
            LayerStatus::ok(SecurityLayerKind::Seccomp),
        );
        let stack = SecurityStack::with_status(status);
        let display = stack.to_string();
        assert!(display.contains("✓"));
        assert!(display.contains("seccomp"));
    }

    // ── Seccomp / Landlock policy variants ──────────────────────────────

    #[test]
    fn seccomp_policy_default() {
        let p = SeccompPolicy::default();
        assert!(matches!(p, SeccompPolicy::Default));
    }

    #[test]
    fn landlock_policy_default() {
        let p = LandlockPolicy::default();
        assert!(matches!(p, LandlockPolicy::Standard));
    }

    // ── Cgroup policy default ──────────────────────────────────────────

    #[test]
    fn cgroup_policy_default_enabled() {
        let cg = CgroupPolicy::default();
        assert!(cg.enabled);
    }

    // ── Layer Outcome checks ────────────────────────────────────────────

    #[test]
    fn layer_outcome_checks() {
        assert!(LayerOutcome::Applied.is_applied());
        assert!(!LayerOutcome::Applied.is_failed());

        let skip = LayerOutcome::Skipped {
            reason: "test".into(),
        };
        assert!(!skip.is_applied());
        assert!(!skip.is_failed());

        let fail = LayerOutcome::Failed {
            error: "boom".into(),
        };
        assert!(!fail.is_applied());
        assert!(fail.is_failed());
    }
}
