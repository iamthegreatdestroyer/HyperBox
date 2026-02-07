//! Linux namespace management.

use crate::error::{CoreError, Result};
use std::collections::HashSet;
use std::path::PathBuf;
#[cfg(unix)]
use std::process::Command;

/// Types of Linux namespaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamespaceType {
    /// Mount namespace
    Mount,
    /// UTS namespace (hostname)
    Uts,
    /// IPC namespace
    Ipc,
    /// Network namespace
    Network,
    /// PID namespace
    Pid,
    /// User namespace
    User,
    /// Cgroup namespace
    Cgroup,
    /// Time namespace (Linux 5.6+)
    Time,
}

impl NamespaceType {
    /// Get the namespace file name in /proc/[pid]/ns/
    #[must_use]
    pub fn proc_name(&self) -> &'static str {
        match self {
            Self::Mount => "mnt",
            Self::Uts => "uts",
            Self::Ipc => "ipc",
            Self::Network => "net",
            Self::Pid => "pid",
            Self::User => "user",
            Self::Cgroup => "cgroup",
            Self::Time => "time",
        }
    }

    /// Get the clone flag for this namespace.
    #[cfg(unix)]
    #[must_use]
    pub fn clone_flag(&self) -> i32 {
        use nix::libc::*;
        match self {
            Self::Mount => CLONE_NEWNS,
            Self::Uts => CLONE_NEWUTS,
            Self::Ipc => CLONE_NEWIPC,
            Self::Network => CLONE_NEWNET,
            Self::Pid => CLONE_NEWPID,
            Self::User => CLONE_NEWUSER,
            Self::Cgroup => CLONE_NEWCGROUP,
            Self::Time => 0x80, // CLONE_NEWTIME
        }
    }

    /// All standard namespaces for container isolation.
    #[must_use]
    pub fn container_defaults() -> Vec<Self> {
        vec![
            Self::Mount,
            Self::Uts,
            Self::Ipc,
            Self::Network,
            Self::Pid,
            Self::Cgroup,
        ]
    }
}

/// Namespace configuration for a container.
#[derive(Debug, Clone)]
pub struct NamespaceConfig {
    /// Namespaces to create
    pub create: HashSet<NamespaceType>,
    /// Namespaces to join (path to namespace file)
    pub join: Vec<(NamespaceType, PathBuf)>,
}

impl Default for NamespaceConfig {
    fn default() -> Self {
        Self {
            create: NamespaceType::container_defaults().into_iter().collect(),
            join: Vec::new(),
        }
    }
}

/// Manager for container namespaces.
pub struct NamespaceManager {
    config: NamespaceConfig,
}

impl NamespaceManager {
    /// Create a new namespace manager.
    #[must_use]
    pub fn new(config: NamespaceConfig) -> Self {
        Self { config }
    }

    /// Create with default container namespaces.
    #[must_use]
    pub fn container_default() -> Self {
        Self::new(NamespaceConfig::default())
    }

    /// Get the path to a namespace file for a PID.
    #[must_use]
    pub fn namespace_path(pid: u32, ns_type: NamespaceType) -> PathBuf {
        PathBuf::from(format!("/proc/{}/ns/{}", pid, ns_type.proc_name()))
    }

    /// Check if a namespace type is available on this system.
    pub fn is_available(ns_type: NamespaceType) -> bool {
        let path = Self::namespace_path(1, ns_type);
        path.exists()
    }

    /// Get combined clone flags for namespace creation.
    #[cfg(unix)]
    #[must_use]
    pub fn clone_flags(&self) -> i32 {
        self.config
            .create
            .iter()
            .map(|ns| ns.clone_flag())
            .fold(0, |acc, flag| acc | flag)
    }

    /// Setup network namespace for a container.
    #[cfg(unix)]
    pub async fn setup_network_namespace(&self, container_id: &str) -> Result<PathBuf> {
        use std::process::Command;

        let ns_name = format!("hyperbox-{}", &container_id[..12.min(container_id.len())]);
        let ns_path = PathBuf::from(format!("/var/run/netns/{ns_name}"));

        // Create network namespace using ip command
        let output = Command::new("ip")
            .args(["netns", "add", &ns_name])
            .output()
            .map_err(|e| CoreError::NamespaceOperation {
                namespace_type: "network".to_string(),
                reason: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::NamespaceOperation {
                namespace_type: "network".to_string(),
                reason: stderr.to_string(),
            });
        }

        Ok(ns_path)
    }

    /// Cleanup network namespace.
    #[cfg(unix)]
    pub async fn cleanup_network_namespace(&self, container_id: &str) -> Result<()> {
        use std::process::Command;

        let ns_name = format!("hyperbox-{}", &container_id[..12.min(container_id.len())]);

        let _ = Command::new("ip")
            .args(["netns", "delete", &ns_name])
            .output();

        Ok(())
    }

    /// Create a veth pair for container networking.
    #[cfg(unix)]
    pub async fn create_veth_pair(
        &self,
        container_id: &str,
        host_bridge: &str,
    ) -> Result<(String, String)> {
        use std::process::Command;

        let short_id = &container_id[..8.min(container_id.len())];
        let veth_host = format!("veth{short_id}");
        let veth_container = format!("eth0");

        // Create veth pair
        let output = Command::new("ip")
            .args([
                "link",
                "add",
                &veth_host,
                "type",
                "veth",
                "peer",
                "name",
                &veth_container,
            ])
            .output()
            .map_err(|e| CoreError::NamespaceOperation {
                namespace_type: "network".to_string(),
                reason: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::NamespaceOperation {
                namespace_type: "network".to_string(),
                reason: format!("Failed to create veth pair: {}", stderr),
            });
        }

        // Attach host end to bridge
        let _ = Command::new("ip")
            .args(["link", "set", &veth_host, "master", host_bridge])
            .output();

        // Bring up host end
        let _ = Command::new("ip")
            .args(["link", "set", &veth_host, "up"])
            .output();

        Ok((veth_host, veth_container))
    }
}

impl Default for NamespaceManager {
    fn default() -> Self {
        Self::container_default()
    }
}
