//! Landlock LSM sandboxing for container filesystem and network access control.
//!
//! Landlock is a Linux security module that enables unprivileged processes to
//! restrict their own access rights. Unlike SELinux or AppArmor, Landlock
//! doesn't require system-wide configuration or root privileges — any process
//! can sandbox itself at runtime.
//!
//! ## Kernel Requirements
//!
//! | ABI | Kernel | Features                          |
//! |-----|--------|-----------------------------------|
//! | V1  | 5.13   | Filesystem access control         |
//! | V2  | 5.19   | File referring/linking             |
//! | V3  | 6.2    | File truncation                   |
//! | V4  | 6.7    | TCP network access control        |
//! | V5  | 6.10   | Device ioctl control              |
//!
//! ## Container Integration
//!
//! HyperBox applies Landlock as an additional defense-in-depth layer
//! beyond namespaces and seccomp, restricting filesystem and network
//! access for container processes.
//!
//! ```rust,no_run
//! use hyperbox_core::isolation::landlock::{LandlockManager, RulesetBuilder, AccessFs};
//!
//! let manager = LandlockManager::new();
//! if manager.is_supported() {
//!     let ruleset = LandlockManager::standard_policy("/var/lib/hyperbox/containers/abc");
//!     // Apply inside the container init process:
//!     // manager.enforce(&ruleset).unwrap();
//! }
//! ```

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::debug;

// =============================================================================
// Filesystem Access Flags
// =============================================================================

/// Filesystem access rights for Landlock rules.
///
/// Flags can be combined with `|` to create compound access sets:
/// ```rust
/// # use hyperbox_core::isolation::landlock::AccessFs;
/// let access = AccessFs::READ_FILE | AccessFs::READ_DIR;
/// assert!(access.contains(AccessFs::READ_FILE));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccessFs(u64);

impl AccessFs {
    /// No access rights.
    pub const EMPTY: Self = Self(0);

    // ---- ABI v1 (kernel 5.13) ----

    /// Execute a file.
    pub const EXECUTE: Self = Self(1 << 0);
    /// Open a file with write access.
    pub const WRITE_FILE: Self = Self(1 << 1);
    /// Open a file with read access.
    pub const READ_FILE: Self = Self(1 << 2);
    /// Open a directory or list its content.
    pub const READ_DIR: Self = Self(1 << 3);
    /// Remove an empty directory or a directory tree.
    pub const REMOVE_DIR: Self = Self(1 << 4);
    /// Unlink (remove) a file.
    pub const REMOVE_FILE: Self = Self(1 << 5);
    /// Create a character device.
    pub const MAKE_CHAR: Self = Self(1 << 6);
    /// Create a directory.
    pub const MAKE_DIR: Self = Self(1 << 7);
    /// Create a regular file.
    pub const MAKE_REG: Self = Self(1 << 8);
    /// Create a UNIX domain socket.
    pub const MAKE_SOCK: Self = Self(1 << 9);
    /// Create a named pipe (FIFO).
    pub const MAKE_FIFO: Self = Self(1 << 10);
    /// Create a block device.
    pub const MAKE_BLOCK: Self = Self(1 << 11);
    /// Create a symbolic link.
    pub const MAKE_SYM: Self = Self(1 << 12);

    // ---- ABI v2 (kernel 5.19) ----

    /// Link or rename a file from or to a different directory.
    pub const REFER: Self = Self(1 << 13);

    // ---- ABI v3 (kernel 6.2) ----

    /// Truncate a file via `truncate(2)`, `ftruncate(2)`, `creat(2)`,
    /// or `open` with `O_TRUNC`.
    pub const TRUNCATE: Self = Self(1 << 14);

    // ---- ABI v5 (kernel 6.10) ----

    /// Perform ioctl on a device file.
    pub const IOCTL_DEV: Self = Self(1 << 15);

    // ---- Compound sets ----

    /// Read-only access: read files, list directories, execute.
    pub const READ_ONLY: Self = Self(Self::READ_FILE.0 | Self::READ_DIR.0 | Self::EXECUTE.0);

    /// Standard read-write for container workloads.
    pub const READ_WRITE: Self = Self(
        Self::READ_FILE.0
            | Self::WRITE_FILE.0
            | Self::READ_DIR.0
            | Self::EXECUTE.0
            | Self::REMOVE_DIR.0
            | Self::REMOVE_FILE.0
            | Self::MAKE_DIR.0
            | Self::MAKE_REG.0
            | Self::MAKE_SYM.0
            | Self::MAKE_FIFO.0
            | Self::TRUNCATE.0
            | Self::REFER.0,
    );

    /// Full access including device creation.
    pub const FULL: Self = Self(
        Self::READ_FILE.0
            | Self::WRITE_FILE.0
            | Self::READ_DIR.0
            | Self::EXECUTE.0
            | Self::REMOVE_DIR.0
            | Self::REMOVE_FILE.0
            | Self::MAKE_CHAR.0
            | Self::MAKE_DIR.0
            | Self::MAKE_REG.0
            | Self::MAKE_SOCK.0
            | Self::MAKE_FIFO.0
            | Self::MAKE_BLOCK.0
            | Self::MAKE_SYM.0
            | Self::REFER.0
            | Self::TRUNCATE.0
            | Self::IOCTL_DEV.0,
    );

    /// All flags from ABI v1 (kernel 5.13).
    pub const ALL_V1: Self = Self((1 << 13) - 1);

    /// Raw bits value.
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Create from raw bit value.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Returns `true` if `self` contains every flag in `other`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns `true` if no flags are set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Union of two access sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersection of two access sets.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Maximum access flags supported by a given ABI version.
    #[must_use]
    pub fn for_abi(abi: LandlockAbi) -> Self {
        match abi {
            LandlockAbi::V1 => Self::ALL_V1,
            LandlockAbi::V2 => Self::from_bits(Self::ALL_V1.0 | Self::REFER.0),
            LandlockAbi::V3 => Self::from_bits(Self::ALL_V1.0 | Self::REFER.0 | Self::TRUNCATE.0),
            // V4 adds network, not filesystem flags
            LandlockAbi::V4 => Self::from_bits(Self::ALL_V1.0 | Self::REFER.0 | Self::TRUNCATE.0),
            LandlockAbi::V5 => Self::FULL,
        }
    }

    /// Number of individual flags set.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }
}

impl std::ops::BitOr for AccessFs {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for AccessFs {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitOrAssign for AccessFs {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

// =============================================================================
// Network Access Flags
// =============================================================================

/// Network access rights for Landlock rules (ABI v4+, kernel 6.7+).
///
/// Controls TCP socket operations. Only available when the kernel
/// supports Landlock ABI v4 or later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccessNet(u64);

impl AccessNet {
    /// No network access.
    pub const EMPTY: Self = Self(0);
    /// Bind a TCP socket to a local port.
    pub const BIND_TCP: Self = Self(1 << 0);
    /// Connect a TCP socket to a remote port.
    pub const CONNECT_TCP: Self = Self(1 << 1);
    /// All network access flags.
    pub const ALL: Self = Self(Self::BIND_TCP.0 | Self::CONNECT_TCP.0);

    /// Raw bits value.
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Create from raw bit value.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Returns `true` if `self` contains every flag in `other`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns `true` if no flags are set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for AccessNet {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

// =============================================================================
// Landlock ABI Version
// =============================================================================

/// Landlock ABI version detected from the running kernel.
///
/// Higher versions are a strict superset of lower versions — a kernel
/// supporting V3 also supports all V1 and V2 features.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LandlockAbi {
    /// ABI v1 (kernel 5.13): filesystem access control.
    V1 = 1,
    /// ABI v2 (kernel 5.19): file referring/linking across directories.
    V2 = 2,
    /// ABI v3 (kernel 6.2): file truncation control.
    V3 = 3,
    /// ABI v4 (kernel 6.7): TCP network access control.
    V4 = 4,
    /// ABI v5 (kernel 6.10): device ioctl control.
    V5 = 5,
}

impl LandlockAbi {
    /// Parse from a raw version number returned by the kernel.
    #[must_use]
    pub fn from_version(version: i32) -> Option<Self> {
        match version {
            1 => Some(Self::V1),
            2 => Some(Self::V2),
            3 => Some(Self::V3),
            4 => Some(Self::V4),
            v if v >= 5 => Some(Self::V5), // forward-compatible
            _ => None,
        }
    }

    /// Whether this ABI supports TCP network rules.
    #[must_use]
    pub fn supports_network(self) -> bool {
        self >= Self::V4
    }

    /// Whether this ABI supports file truncation control.
    #[must_use]
    pub fn supports_truncate(self) -> bool {
        self >= Self::V3
    }

    /// Whether this ABI supports file refer (cross-directory rename/link).
    #[must_use]
    pub fn supports_refer(self) -> bool {
        self >= Self::V2
    }

    /// Minimum kernel version string for this ABI.
    #[must_use]
    pub fn min_kernel_version(self) -> &'static str {
        match self {
            Self::V1 => "5.13",
            Self::V2 => "5.19",
            Self::V3 => "6.2",
            Self::V4 => "6.7",
            Self::V5 => "6.10",
        }
    }
}

impl std::fmt::Display for LandlockAbi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ABI v{} (kernel {}+)", *self as u32, self.min_kernel_version())
    }
}

// =============================================================================
// Rule Types
// =============================================================================

/// A Landlock rule restricting filesystem access beneath a path.
///
/// When a path rule is added to a ruleset, the `access` specifies which
/// *handled* operations are **allowed** under that path. Handled operations
/// not granted by any rule are denied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRule {
    /// Directory or file path to which this rule applies.
    pub path: PathBuf,
    /// Filesystem access rights allowed under this path.
    pub access: AccessFs,
}

/// A Landlock rule restricting TCP operations on a port (ABI v4+).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetRule {
    /// TCP port number.
    pub port: u16,
    /// Network access rights allowed for this port.
    pub access: AccessNet,
}

// =============================================================================
// Landlock Ruleset
// =============================================================================

/// A complete Landlock ruleset ready for enforcement.
///
/// The ruleset declares which access types are *handled* (i.e., subject to
/// restriction). Any handled access that is **not** granted by a path or
/// network rule is implicitly **denied**.
///
/// Example: if `handled_fs` includes `WRITE_FILE`, and no path rule grants
/// `WRITE_FILE` for `/data`, then writing to `/data` is denied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandlockRuleset {
    /// Filesystem access types restricted by this ruleset.
    pub handled_fs: AccessFs,
    /// Network access types restricted by this ruleset (ABI v4+).
    pub handled_net: AccessNet,
    /// Filesystem path rules (exceptions that *allow* access).
    pub path_rules: Vec<PathRule>,
    /// Network port rules (exceptions that *allow* access).
    pub net_rules: Vec<NetRule>,
}

impl LandlockRuleset {
    /// Total number of rules (path + network).
    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.path_rules.len() + self.net_rules.len()
    }

    /// Returns `true` if no access types are handled.
    #[must_use]
    pub fn is_restrictive(&self) -> bool {
        !self.handled_fs.is_empty() || !self.handled_net.is_empty()
    }
}

// =============================================================================
// Ruleset Builder
// =============================================================================

/// Ergonomic builder for constructing [`LandlockRuleset`] values.
///
/// # Example
///
/// ```rust
/// use hyperbox_core::isolation::landlock::{RulesetBuilder, AccessFs, AccessNet};
///
/// let ruleset = RulesetBuilder::new()
///     .handle_fs(AccessFs::FULL)
///     .allow_path("/app", AccessFs::READ_WRITE)
///     .allow_path("/tmp", AccessFs::READ_WRITE)
///     .allow_path("/usr", AccessFs::READ_ONLY)
///     .handle_net(AccessNet::ALL)
///     .allow_connect(443)
///     .allow_connect(80)
///     .allow_bind(8080)
///     .build();
///
/// assert_eq!(ruleset.path_rules.len(), 3);
/// assert_eq!(ruleset.net_rules.len(), 3);
/// ```
pub struct RulesetBuilder {
    handled_fs: AccessFs,
    handled_net: AccessNet,
    path_rules: Vec<PathRule>,
    net_rules: Vec<NetRule>,
}

impl RulesetBuilder {
    /// Create a new empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            handled_fs: AccessFs::EMPTY,
            handled_net: AccessNet::EMPTY,
            path_rules: Vec::new(),
            net_rules: Vec::new(),
        }
    }

    /// Set which filesystem access types are handled (restricted).
    #[must_use]
    pub fn handle_fs(mut self, access: AccessFs) -> Self {
        self.handled_fs = access;
        self
    }

    /// Set which network access types are handled (restricted).
    #[must_use]
    pub fn handle_net(mut self, access: AccessNet) -> Self {
        self.handled_net = access;
        self
    }

    /// Allow specific filesystem access beneath a path.
    #[must_use]
    pub fn allow_path(mut self, path: impl Into<PathBuf>, access: AccessFs) -> Self {
        self.path_rules.push(PathRule {
            path: path.into(),
            access,
        });
        self
    }

    /// Allow network operations on a port.
    #[must_use]
    pub fn allow_port(mut self, port: u16, access: AccessNet) -> Self {
        self.net_rules.push(NetRule { port, access });
        self
    }

    /// Allow outgoing TCP connections to a specific port.
    #[must_use]
    pub fn allow_connect(mut self, port: u16) -> Self {
        self.net_rules.push(NetRule {
            port,
            access: AccessNet::CONNECT_TCP,
        });
        self
    }

    /// Allow a TCP server to bind on a specific port.
    #[must_use]
    pub fn allow_bind(mut self, port: u16) -> Self {
        self.net_rules.push(NetRule {
            port,
            access: AccessNet::BIND_TCP,
        });
        self
    }

    /// Consume the builder and produce a [`LandlockRuleset`].
    #[must_use]
    pub fn build(self) -> LandlockRuleset {
        LandlockRuleset {
            handled_fs: self.handled_fs,
            handled_net: self.handled_net,
            path_rules: self.path_rules,
            net_rules: self.net_rules,
        }
    }
}

impl Default for RulesetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Landlock Manager
// =============================================================================

/// Manages Landlock sandboxing for HyperBox containers.
///
/// Provides ABI detection, pre-built security policies, and enforcement
/// of Landlock rulesets on the current process.
pub struct LandlockManager {
    /// Detected ABI version (`None` if Landlock is unsupported).
    abi_version: Option<LandlockAbi>,
}

impl LandlockManager {
    /// Create a new Landlock manager, auto-detecting kernel support.
    #[must_use]
    pub fn new() -> Self {
        let abi_version = Self::detect_abi();
        if let Some(abi) = abi_version {
            debug!("Landlock {} detected", abi);
        } else {
            debug!("Landlock not supported on this system");
        }
        Self { abi_version }
    }

    /// Returns `true` if the running kernel supports Landlock.
    #[must_use]
    pub fn is_supported(&self) -> bool {
        self.abi_version.is_some()
    }

    /// Get the detected ABI version.
    #[must_use]
    pub fn abi_version(&self) -> Option<LandlockAbi> {
        self.abi_version
    }

    /// Whether the kernel supports Landlock network rules (ABI v4+).
    #[must_use]
    pub fn supports_network(&self) -> bool {
        self.abi_version.map_or(false, |abi| abi.supports_network())
    }

    // =========================================================================
    // Pre-built Policies
    // =========================================================================

    /// Strict sandbox policy for a container.
    ///
    /// - **Rootfs**: read + write + execute (no device creation)
    /// - **`/proc`**: read-only
    /// - **`/dev/null`, `/dev/zero`, `/dev/urandom`**: limited access
    /// - **Network**: fully restricted (no rules ⇒ all handled access denied)
    pub fn strict_policy(rootfs: impl AsRef<Path>) -> LandlockRuleset {
        let rootfs = rootfs.as_ref();
        RulesetBuilder::new()
            .handle_fs(AccessFs::FULL)
            .handle_net(AccessNet::ALL)
            .allow_path(rootfs, AccessFs::READ_WRITE)
            .allow_path("/proc", AccessFs::READ_ONLY)
            .allow_path("/dev/null", AccessFs::READ_FILE | AccessFs::WRITE_FILE)
            .allow_path("/dev/zero", AccessFs::READ_FILE)
            .allow_path("/dev/urandom", AccessFs::READ_FILE)
            .build()
    }

    /// Standard sandbox policy for a typical container.
    ///
    /// Allows common container filesystem access patterns, shared libraries,
    /// temp directories, and standard system paths. Network is not restricted.
    pub fn standard_policy(rootfs: impl AsRef<Path>) -> LandlockRuleset {
        let rootfs = rootfs.as_ref();
        RulesetBuilder::new()
            .handle_fs(AccessFs::FULL)
            .allow_path(rootfs, AccessFs::READ_WRITE)
            .allow_path("/proc", AccessFs::READ_ONLY)
            .allow_path("/sys", AccessFs::READ_ONLY)
            .allow_path("/dev", AccessFs::READ_FILE | AccessFs::WRITE_FILE)
            .allow_path("/tmp", AccessFs::READ_WRITE)
            .allow_path("/var/tmp", AccessFs::READ_WRITE)
            .allow_path("/lib", AccessFs::READ_ONLY)
            .allow_path("/lib64", AccessFs::READ_ONLY)
            .allow_path("/usr/lib", AccessFs::READ_ONLY)
            .allow_path("/usr/lib64", AccessFs::READ_ONLY)
            .allow_path("/usr/share", AccessFs::READ_ONLY)
            .allow_path("/etc", AccessFs::READ_ONLY)
            .build()
    }

    /// Permissive policy restricting only dangerous operations.
    ///
    /// Only blocks character and block device creation.
    /// Suitable for trusted workloads that need broad access.
    pub fn permissive_policy(rootfs: impl AsRef<Path>) -> LandlockRuleset {
        let rootfs = rootfs.as_ref();
        RulesetBuilder::new()
            .handle_fs(AccessFs::MAKE_CHAR | AccessFs::MAKE_BLOCK)
            .allow_path(rootfs, AccessFs::EMPTY)
            .build()
    }

    /// Build a custom container ruleset with additional mount points.
    ///
    /// Starts from a standard policy and adds user-specified mounts
    /// with their individual access levels.
    pub fn container_ruleset(
        rootfs: impl AsRef<Path>,
        mounts: &[(PathBuf, AccessFs)],
    ) -> LandlockRuleset {
        let rootfs = rootfs.as_ref();
        let mut builder = RulesetBuilder::new()
            .handle_fs(AccessFs::FULL)
            .allow_path(rootfs, AccessFs::READ_WRITE)
            .allow_path("/proc", AccessFs::READ_ONLY)
            .allow_path("/sys", AccessFs::READ_ONLY)
            .allow_path("/dev/null", AccessFs::READ_FILE | AccessFs::WRITE_FILE)
            .allow_path("/dev/zero", AccessFs::READ_FILE)
            .allow_path("/dev/urandom", AccessFs::READ_FILE);

        for (path, access) in mounts {
            builder = builder.allow_path(path, *access);
        }

        builder.build()
    }

    /// Serialize a ruleset to JSON for container runtime configuration.
    ///
    /// The JSON can be passed to runtimes that support Landlock
    /// configuration as part of the container spec.
    pub fn to_config_json(ruleset: &LandlockRuleset) -> serde_json::Value {
        serde_json::json!({
            "landlock": {
                "handled_access_fs": ruleset.handled_fs.bits(),
                "handled_access_net": ruleset.handled_net.bits(),
                "path_rules": ruleset.path_rules.iter().map(|r| {
                    serde_json::json!({
                        "path": r.path.to_string_lossy(),
                        "access_fs": r.access.bits()
                    })
                }).collect::<Vec<_>>(),
                "net_rules": ruleset.net_rules.iter().map(|r| {
                    serde_json::json!({
                        "port": r.port,
                        "access_net": r.access.bits()
                    })
                }).collect::<Vec<_>>()
            }
        })
    }
}

// =============================================================================
// Linux Syscall Implementation
// =============================================================================

#[cfg(target_os = "linux")]
impl LandlockManager {
    /// Landlock syscall numbers (stable across architectures since 5.13).
    const SYS_LANDLOCK_CREATE_RULESET: i64 = 444;
    const SYS_LANDLOCK_ADD_RULE: i64 = 445;
    const SYS_LANDLOCK_RESTRICT_SELF: i64 = 446;

    /// Flag for querying the supported ABI version.
    const CREATE_RULESET_VERSION: u32 = 1 << 0;

    /// Rule type: path beneath.
    const RULE_PATH_BENEATH: u32 = 1;
    /// Rule type: network port.
    const RULE_NET_PORT: u32 = 2;

    /// Detect the Landlock ABI version supported by the running kernel.
    #[must_use]
    pub fn detect_abi() -> Option<LandlockAbi> {
        let version = unsafe {
            nix::libc::syscall(
                Self::SYS_LANDLOCK_CREATE_RULESET,
                std::ptr::null::<u8>(),
                0_usize,
                Self::CREATE_RULESET_VERSION,
            )
        };

        if version < 0 {
            None
        } else {
            LandlockAbi::from_version(version as i32)
        }
    }

    /// Enforce a Landlock ruleset on the current process.
    ///
    /// After enforcement, the current process and all future children
    /// are permanently restricted by the ruleset. This is irreversible.
    ///
    /// ## Prerequisites
    ///
    /// - Linux kernel 5.13+ with Landlock enabled
    /// - The process must be able to set `PR_SET_NO_NEW_PRIVS`
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::PermissionDenied`] if Landlock is unsupported
    /// or the privilege restriction cannot be set. Returns
    /// [`CoreError::Internal`] on syscall failures.
    pub fn enforce(&self, ruleset: &LandlockRuleset) -> Result<()> {
        let abi = self
            .abi_version
            .ok_or_else(|| CoreError::PermissionDenied {
                operation: "landlock_enforce".to_string(),
                required: "Linux 5.13+ with Landlock enabled".to_string(),
            })?;

        // Clamp handled access to what the current ABI actually supports
        let handled_fs = ruleset.handled_fs.intersection(AccessFs::for_abi(abi));
        let handled_net = if abi.supports_network() {
            ruleset.handled_net
        } else {
            AccessNet::EMPTY
        };

        // 1. Create ruleset file descriptor
        let ruleset_fd = self.sys_create_ruleset(handled_fs, handled_net)?;

        // Guard: ensure the FD is closed on any error path
        let result = (|| -> Result<()> {
            // 2. Add path rules
            for rule in &ruleset.path_rules {
                // Clamp each rule's access to handled flags
                let clamped = rule.access.intersection(handled_fs);
                if !clamped.is_empty() {
                    self.sys_add_path_rule(ruleset_fd, &rule.path, clamped)?;
                }
            }

            // 3. Add network rules (if supported)
            if abi.supports_network() {
                for rule in &ruleset.net_rules {
                    self.sys_add_net_rule(ruleset_fd, rule.port, rule.access)?;
                }
            }

            // 4. Set no-new-privs (required before landlock_restrict_self)
            let nnp = unsafe { nix::libc::prctl(nix::libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
            if nnp != 0 {
                return Err(CoreError::PermissionDenied {
                    operation: "prctl(PR_SET_NO_NEW_PRIVS)".to_string(),
                    required: "Unprivileged process or CAP_SYS_ADMIN".to_string(),
                });
            }

            // 5. Restrict self — this is the point of no return
            let ret =
                unsafe { nix::libc::syscall(Self::SYS_LANDLOCK_RESTRICT_SELF, ruleset_fd, 0_u32) };
            if ret < 0 {
                let err = std::io::Error::last_os_error();
                return Err(CoreError::Internal(format!("landlock_restrict_self failed: {err}")));
            }

            Ok(())
        })();

        // Always close the ruleset FD
        unsafe {
            nix::libc::close(ruleset_fd);
        }

        if result.is_ok() {
            debug!(
                "Landlock enforced: {} path rules, {} net rules",
                ruleset.path_rules.len(),
                ruleset.net_rules.len()
            );
        }

        result
    }

    /// Create a Landlock ruleset file descriptor via syscall.
    fn sys_create_ruleset(&self, handled_fs: AccessFs, handled_net: AccessNet) -> Result<i32> {
        #[repr(C)]
        struct RulesetAttr {
            handled_access_fs: u64,
            handled_access_net: u64,
        }

        let attr = RulesetAttr {
            handled_access_fs: handled_fs.bits(),
            handled_access_net: handled_net.bits(),
        };

        let fd = unsafe {
            nix::libc::syscall(
                Self::SYS_LANDLOCK_CREATE_RULESET,
                &attr as *const RulesetAttr,
                std::mem::size_of::<RulesetAttr>(),
                0_u32,
            )
        };

        if fd < 0 {
            let err = std::io::Error::last_os_error();
            return Err(CoreError::Internal(format!("landlock_create_ruleset failed: {err}")));
        }

        Ok(fd as i32)
    }

    /// Add a "path beneath" rule to an open ruleset.
    fn sys_add_path_rule(&self, ruleset_fd: i32, path: &Path, access: AccessFs) -> Result<()> {
        use std::os::unix::io::AsRawFd;

        // Open the path to get a file descriptor for the kernel
        let file = std::fs::File::open(path).map_err(|e| {
            CoreError::Internal(format!(
                "Cannot open path for Landlock rule '{}': {e}",
                path.display()
            ))
        })?;

        #[repr(C)]
        struct PathBeneathAttr {
            allowed_access: u64,
            parent_fd: i32,
        }

        let attr = PathBeneathAttr {
            allowed_access: access.bits(),
            parent_fd: file.as_raw_fd(),
        };

        let ret = unsafe {
            nix::libc::syscall(
                Self::SYS_LANDLOCK_ADD_RULE,
                ruleset_fd,
                Self::RULE_PATH_BENEATH,
                &attr as *const PathBeneathAttr,
                0_u32,
            )
        };

        if ret < 0 {
            let err = std::io::Error::last_os_error();
            return Err(CoreError::Internal(format!(
                "landlock_add_rule(path='{}') failed: {err}",
                path.display()
            )));
        }

        Ok(())
    }

    /// Add a network port rule to an open ruleset.
    fn sys_add_net_rule(&self, ruleset_fd: i32, port: u16, access: AccessNet) -> Result<()> {
        #[repr(C)]
        struct NetPortAttr {
            allowed_access: u64,
            port: u64,
        }

        let attr = NetPortAttr {
            allowed_access: access.bits(),
            port: u64::from(port),
        };

        let ret = unsafe {
            nix::libc::syscall(
                Self::SYS_LANDLOCK_ADD_RULE,
                ruleset_fd,
                Self::RULE_NET_PORT,
                &attr as *const NetPortAttr,
                0_u32,
            )
        };

        if ret < 0 {
            let err = std::io::Error::last_os_error();
            return Err(CoreError::Internal(format!(
                "landlock_add_rule(net port={port}) failed: {err}"
            )));
        }

        Ok(())
    }
}

// =============================================================================
// Non-Linux Stubs
// =============================================================================

#[cfg(not(target_os = "linux"))]
impl LandlockManager {
    /// Landlock is Linux-only. Always returns `None` on other platforms.
    #[must_use]
    pub fn detect_abi() -> Option<LandlockAbi> {
        None
    }

    /// Landlock enforcement is not available on non-Linux platforms.
    ///
    /// # Errors
    ///
    /// Always returns [`CoreError::PermissionDenied`].
    pub fn enforce(&self, _ruleset: &LandlockRuleset) -> Result<()> {
        Err(CoreError::PermissionDenied {
            operation: "landlock_enforce".to_string(),
            required: "Linux 5.13+ with Landlock enabled".to_string(),
        })
    }
}

impl Default for LandlockManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- AccessFs tests ----

    #[test]
    fn test_access_fs_individual_flags() {
        assert_eq!(AccessFs::EXECUTE.bits(), 1);
        assert_eq!(AccessFs::WRITE_FILE.bits(), 2);
        assert_eq!(AccessFs::READ_FILE.bits(), 4);
        assert_eq!(AccessFs::READ_DIR.bits(), 8);
        assert_eq!(AccessFs::MAKE_SYM.bits(), 1 << 12);
        assert_eq!(AccessFs::REFER.bits(), 1 << 13);
        assert_eq!(AccessFs::TRUNCATE.bits(), 1 << 14);
        assert_eq!(AccessFs::IOCTL_DEV.bits(), 1 << 15);
    }

    #[test]
    fn test_access_fs_empty() {
        assert!(AccessFs::EMPTY.is_empty());
        assert!(!AccessFs::READ_FILE.is_empty());
        assert_eq!(AccessFs::EMPTY.count(), 0);
    }

    #[test]
    fn test_access_fs_bitor() {
        let combined = AccessFs::READ_FILE | AccessFs::WRITE_FILE;
        assert!(combined.contains(AccessFs::READ_FILE));
        assert!(combined.contains(AccessFs::WRITE_FILE));
        assert!(!combined.contains(AccessFs::EXECUTE));
        assert_eq!(combined.count(), 2);
    }

    #[test]
    fn test_access_fs_bitand() {
        let a = AccessFs::READ_FILE | AccessFs::WRITE_FILE | AccessFs::EXECUTE;
        let b = AccessFs::READ_FILE | AccessFs::EXECUTE;
        let inter = a & b;
        assert!(inter.contains(AccessFs::READ_FILE));
        assert!(inter.contains(AccessFs::EXECUTE));
        assert!(!inter.contains(AccessFs::WRITE_FILE));
    }

    #[test]
    fn test_access_fs_bitor_assign() {
        let mut access = AccessFs::READ_FILE;
        access |= AccessFs::WRITE_FILE;
        assert!(access.contains(AccessFs::READ_FILE));
        assert!(access.contains(AccessFs::WRITE_FILE));
    }

    #[test]
    fn test_access_fs_compound_read_only() {
        assert!(AccessFs::READ_ONLY.contains(AccessFs::READ_FILE));
        assert!(AccessFs::READ_ONLY.contains(AccessFs::READ_DIR));
        assert!(AccessFs::READ_ONLY.contains(AccessFs::EXECUTE));
        assert!(!AccessFs::READ_ONLY.contains(AccessFs::WRITE_FILE));
        assert!(!AccessFs::READ_ONLY.contains(AccessFs::REMOVE_FILE));
    }

    #[test]
    fn test_access_fs_compound_read_write() {
        assert!(AccessFs::READ_WRITE.contains(AccessFs::READ_FILE));
        assert!(AccessFs::READ_WRITE.contains(AccessFs::WRITE_FILE));
        assert!(AccessFs::READ_WRITE.contains(AccessFs::MAKE_DIR));
        assert!(AccessFs::READ_WRITE.contains(AccessFs::MAKE_REG));
        assert!(AccessFs::READ_WRITE.contains(AccessFs::REMOVE_FILE));
        assert!(!AccessFs::READ_WRITE.contains(AccessFs::MAKE_CHAR));
        assert!(!AccessFs::READ_WRITE.contains(AccessFs::MAKE_BLOCK));
    }

    #[test]
    fn test_access_fs_full_contains_all_individual() {
        assert!(AccessFs::FULL.contains(AccessFs::EXECUTE));
        assert!(AccessFs::FULL.contains(AccessFs::WRITE_FILE));
        assert!(AccessFs::FULL.contains(AccessFs::READ_FILE));
        assert!(AccessFs::FULL.contains(AccessFs::MAKE_CHAR));
        assert!(AccessFs::FULL.contains(AccessFs::MAKE_BLOCK));
        assert!(AccessFs::FULL.contains(AccessFs::REFER));
        assert!(AccessFs::FULL.contains(AccessFs::TRUNCATE));
        assert!(AccessFs::FULL.contains(AccessFs::IOCTL_DEV));
    }

    #[test]
    fn test_access_fs_for_abi() {
        let v1 = AccessFs::for_abi(LandlockAbi::V1);
        assert!(v1.contains(AccessFs::EXECUTE));
        assert!(v1.contains(AccessFs::MAKE_SYM));
        assert!(!v1.contains(AccessFs::REFER));
        assert!(!v1.contains(AccessFs::TRUNCATE));

        let v2 = AccessFs::for_abi(LandlockAbi::V2);
        assert!(v2.contains(AccessFs::REFER));
        assert!(!v2.contains(AccessFs::TRUNCATE));

        let v3 = AccessFs::for_abi(LandlockAbi::V3);
        assert!(v3.contains(AccessFs::TRUNCATE));
        assert!(!v3.contains(AccessFs::IOCTL_DEV));

        let v5 = AccessFs::for_abi(LandlockAbi::V5);
        assert!(v5.contains(AccessFs::IOCTL_DEV));
    }

    #[test]
    fn test_access_fs_from_bits_roundtrip() {
        let original = AccessFs::READ_WRITE;
        let roundtripped = AccessFs::from_bits(original.bits());
        assert_eq!(original, roundtripped);
    }

    #[test]
    fn test_access_fs_serialization() {
        let access = AccessFs::READ_FILE | AccessFs::WRITE_FILE;
        let json = serde_json::to_string(&access).unwrap();
        let deserialized: AccessFs = serde_json::from_str(&json).unwrap();
        assert_eq!(access, deserialized);
    }

    // ---- AccessNet tests ----

    #[test]
    fn test_access_net_flags() {
        assert_eq!(AccessNet::BIND_TCP.bits(), 1);
        assert_eq!(AccessNet::CONNECT_TCP.bits(), 2);
        assert!(AccessNet::ALL.contains(AccessNet::BIND_TCP));
        assert!(AccessNet::ALL.contains(AccessNet::CONNECT_TCP));
        assert!(AccessNet::EMPTY.is_empty());
    }

    #[test]
    fn test_access_net_bitor() {
        let combined = AccessNet::BIND_TCP | AccessNet::CONNECT_TCP;
        assert_eq!(combined, AccessNet::ALL);
    }

    // ---- LandlockAbi tests ----

    #[test]
    fn test_abi_from_version() {
        assert_eq!(LandlockAbi::from_version(1), Some(LandlockAbi::V1));
        assert_eq!(LandlockAbi::from_version(2), Some(LandlockAbi::V2));
        assert_eq!(LandlockAbi::from_version(3), Some(LandlockAbi::V3));
        assert_eq!(LandlockAbi::from_version(4), Some(LandlockAbi::V4));
        assert_eq!(LandlockAbi::from_version(5), Some(LandlockAbi::V5));
        // Forward-compatible: unknown future versions map to V5
        assert_eq!(LandlockAbi::from_version(6), Some(LandlockAbi::V5));
        assert_eq!(LandlockAbi::from_version(99), Some(LandlockAbi::V5));
        // Invalid
        assert_eq!(LandlockAbi::from_version(0), None);
        assert_eq!(LandlockAbi::from_version(-1), None);
    }

    #[test]
    fn test_abi_ordering() {
        assert!(LandlockAbi::V1 < LandlockAbi::V2);
        assert!(LandlockAbi::V2 < LandlockAbi::V3);
        assert!(LandlockAbi::V3 < LandlockAbi::V4);
        assert!(LandlockAbi::V4 < LandlockAbi::V5);
    }

    #[test]
    fn test_abi_feature_support() {
        assert!(!LandlockAbi::V1.supports_refer());
        assert!(LandlockAbi::V2.supports_refer());
        assert!(!LandlockAbi::V2.supports_truncate());
        assert!(LandlockAbi::V3.supports_truncate());
        assert!(!LandlockAbi::V3.supports_network());
        assert!(LandlockAbi::V4.supports_network());
    }

    #[test]
    fn test_abi_min_kernel() {
        assert_eq!(LandlockAbi::V1.min_kernel_version(), "5.13");
        assert_eq!(LandlockAbi::V4.min_kernel_version(), "6.7");
    }

    #[test]
    fn test_abi_display() {
        let s = format!("{}", LandlockAbi::V3);
        assert!(s.contains("v3"));
        assert!(s.contains("6.2"));
    }

    // ---- RulesetBuilder tests ----

    #[test]
    fn test_builder_empty() {
        let ruleset = RulesetBuilder::new().build();
        assert!(ruleset.handled_fs.is_empty());
        assert!(ruleset.handled_net.is_empty());
        assert_eq!(ruleset.rule_count(), 0);
        assert!(!ruleset.is_restrictive());
    }

    #[test]
    fn test_builder_fs_only() {
        let ruleset = RulesetBuilder::new()
            .handle_fs(AccessFs::FULL)
            .allow_path("/app", AccessFs::READ_WRITE)
            .allow_path("/tmp", AccessFs::READ_WRITE)
            .build();

        assert_eq!(ruleset.handled_fs, AccessFs::FULL);
        assert!(ruleset.handled_net.is_empty());
        assert_eq!(ruleset.path_rules.len(), 2);
        assert_eq!(ruleset.net_rules.len(), 0);
        assert!(ruleset.is_restrictive());
    }

    #[test]
    fn test_builder_with_network() {
        let ruleset = RulesetBuilder::new()
            .handle_fs(AccessFs::READ_ONLY)
            .handle_net(AccessNet::ALL)
            .allow_path("/app", AccessFs::READ_ONLY)
            .allow_connect(443)
            .allow_connect(80)
            .allow_bind(8080)
            .build();

        assert_eq!(ruleset.path_rules.len(), 1);
        assert_eq!(ruleset.net_rules.len(), 3);
        assert_eq!(ruleset.net_rules[0].port, 443);
        assert_eq!(ruleset.net_rules[0].access, AccessNet::CONNECT_TCP);
        assert_eq!(ruleset.net_rules[2].port, 8080);
        assert_eq!(ruleset.net_rules[2].access, AccessNet::BIND_TCP);
    }

    #[test]
    fn test_builder_allow_port() {
        let ruleset = RulesetBuilder::new()
            .handle_net(AccessNet::ALL)
            .allow_port(9090, AccessNet::ALL)
            .build();

        assert_eq!(ruleset.net_rules.len(), 1);
        assert!(ruleset.net_rules[0].access.contains(AccessNet::BIND_TCP));
        assert!(ruleset.net_rules[0].access.contains(AccessNet::CONNECT_TCP));
    }

    // ---- Policy tests ----

    #[test]
    fn test_strict_policy() {
        let ruleset = LandlockManager::strict_policy("/rootfs");
        assert_eq!(ruleset.handled_fs, AccessFs::FULL);
        assert_eq!(ruleset.handled_net, AccessNet::ALL);
        assert!(!ruleset.path_rules.is_empty());
        // First rule should be the rootfs
        assert_eq!(ruleset.path_rules[0].path, PathBuf::from("/rootfs"));
        // Should have rules for /proc and /dev/*
        assert!(ruleset.path_rules.len() >= 4);
        // Network is handled but no rules ⇒ all network access denied
        assert!(ruleset.net_rules.is_empty());
    }

    #[test]
    fn test_standard_policy() {
        let ruleset = LandlockManager::standard_policy("/container/root");
        assert_eq!(ruleset.handled_fs, AccessFs::FULL);
        assert!(ruleset.handled_net.is_empty()); // network not restricted
        assert!(ruleset.path_rules.len() >= 10); // rootfs + system paths
                                                 // Includes /lib, /usr/lib, /etc, /tmp, etc.
        let paths: Vec<_> = ruleset
            .path_rules
            .iter()
            .map(|r| r.path.to_string_lossy().to_string())
            .collect();
        assert!(paths.contains(&"/tmp".to_string()));
        assert!(paths.contains(&"/etc".to_string()));
        assert!(paths.contains(&"/proc".to_string()));
    }

    #[test]
    fn test_permissive_policy() {
        let ruleset = LandlockManager::permissive_policy("/rootfs");
        // Only blocks device creation
        assert!(ruleset.handled_fs.contains(AccessFs::MAKE_CHAR));
        assert!(ruleset.handled_fs.contains(AccessFs::MAKE_BLOCK));
        assert!(!ruleset.handled_fs.contains(AccessFs::READ_FILE));
        assert!(!ruleset.handled_fs.contains(AccessFs::WRITE_FILE));
    }

    #[test]
    fn test_container_ruleset_with_mounts() {
        let mounts = vec![
            (PathBuf::from("/data"), AccessFs::READ_WRITE),
            (PathBuf::from("/config"), AccessFs::READ_ONLY),
        ];

        let ruleset = LandlockManager::container_ruleset("/rootfs", &mounts);
        assert!(ruleset.is_restrictive());

        let paths: Vec<_> = ruleset
            .path_rules
            .iter()
            .map(|r| r.path.to_string_lossy().to_string())
            .collect();
        assert!(paths.contains(&"/rootfs".to_string()));
        assert!(paths.contains(&"/data".to_string()));
        assert!(paths.contains(&"/config".to_string()));

        // Find mount with read-only access
        let config_rule = ruleset
            .path_rules
            .iter()
            .find(|r| r.path == PathBuf::from("/config"))
            .unwrap();
        assert_eq!(config_rule.access, AccessFs::READ_ONLY);
    }

    // ---- LandlockManager tests ----

    #[test]
    fn test_manager_creation() {
        let manager = LandlockManager::new();
        // On non-Linux, always unsupported
        #[cfg(not(target_os = "linux"))]
        {
            assert!(!manager.is_supported());
            assert!(manager.abi_version().is_none());
            assert!(!manager.supports_network());
        }
    }

    #[test]
    fn test_manager_default() {
        let manager = LandlockManager::default();
        assert_eq!(manager.abi_version(), LandlockManager::detect_abi());
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn test_enforce_fails_on_non_linux() {
        let manager = LandlockManager::new();
        let ruleset = LandlockManager::strict_policy("/rootfs");
        let result = manager.enforce(&ruleset);
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            CoreError::PermissionDenied { operation, .. } => {
                assert!(operation.contains("landlock"));
            }
            other => panic!("Expected PermissionDenied, got: {other:?}"),
        }
    }

    // ---- JSON config tests ----

    #[test]
    fn test_config_json_structure() {
        let ruleset = RulesetBuilder::new()
            .handle_fs(AccessFs::READ_ONLY)
            .handle_net(AccessNet::ALL)
            .allow_path("/app", AccessFs::READ_ONLY)
            .allow_connect(443)
            .build();

        let json = LandlockManager::to_config_json(&ruleset);
        let landlock = &json["landlock"];

        assert_eq!(landlock["handled_access_fs"], AccessFs::READ_ONLY.bits());
        assert_eq!(landlock["handled_access_net"], AccessNet::ALL.bits());

        let path_rules = landlock["path_rules"].as_array().unwrap();
        assert_eq!(path_rules.len(), 1);
        assert_eq!(path_rules[0]["path"], "/app");

        let net_rules = landlock["net_rules"].as_array().unwrap();
        assert_eq!(net_rules.len(), 1);
        assert_eq!(net_rules[0]["port"], 443);
    }

    #[test]
    fn test_config_json_empty_ruleset() {
        let ruleset = RulesetBuilder::new().build();
        let json = LandlockManager::to_config_json(&ruleset);
        let landlock = &json["landlock"];

        assert_eq!(landlock["handled_access_fs"], 0);
        assert_eq!(landlock["handled_access_net"], 0);
        assert!(landlock["path_rules"].as_array().unwrap().is_empty());
        assert!(landlock["net_rules"].as_array().unwrap().is_empty());
    }

    // ---- Serialization roundtrip ----

    #[test]
    fn test_ruleset_serialization_roundtrip() {
        let ruleset = RulesetBuilder::new()
            .handle_fs(AccessFs::FULL)
            .handle_net(AccessNet::ALL)
            .allow_path("/app", AccessFs::READ_WRITE)
            .allow_connect(443)
            .build();

        let json = serde_json::to_string(&ruleset).unwrap();
        let deserialized: LandlockRuleset = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.handled_fs, AccessFs::FULL);
        assert_eq!(deserialized.handled_net, AccessNet::ALL);
        assert_eq!(deserialized.path_rules.len(), 1);
        assert_eq!(deserialized.net_rules.len(), 1);
        assert_eq!(deserialized.net_rules[0].port, 443);
    }

    #[test]
    fn test_abi_serialization() {
        let abi = LandlockAbi::V4;
        let json = serde_json::to_string(&abi).unwrap();
        let deserialized: LandlockAbi = serde_json::from_str(&json).unwrap();
        assert_eq!(abi, deserialized);
    }
}
