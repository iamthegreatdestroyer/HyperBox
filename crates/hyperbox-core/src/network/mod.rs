//! Network management for containers.
//!
//! Provides CNI integration, eBPF-based networking, and port management.

pub mod bridge;
pub mod cni;
pub mod ports;

pub use bridge::BridgeNetwork;
pub use cni::CniManager;
pub use ports::PortAllocator;

use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Network mode for containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMode {
    /// Bridge network (default)
    Bridge,
    /// Host network (share host namespace)
    Host,
    /// No networking
    None,
    /// Container network (share with another container)
    Container,
    /// Custom CNI network
    Custom,
}

impl Default for NetworkMode {
    fn default() -> Self {
        Self::Bridge
    }
}

/// Network configuration for a container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network mode
    pub mode: NetworkMode,
    /// Network name (for bridge or custom)
    pub network_name: Option<String>,
    /// Static IP address
    pub ip_address: Option<IpAddr>,
    /// Gateway address
    pub gateway: Option<IpAddr>,
    /// DNS servers
    pub dns: Vec<IpAddr>,
    /// Extra hosts (/etc/hosts entries)
    pub extra_hosts: Vec<(String, IpAddr)>,
    /// Port mappings
    pub ports: Vec<crate::types::PortMapping>,
    /// MAC address
    pub mac_address: Option<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mode: NetworkMode::Bridge,
            network_name: Some("hyperbox0".to_string()),
            ip_address: None,
            gateway: None,
            dns: vec!["8.8.8.8".parse().unwrap(), "8.8.4.4".parse().unwrap()],
            extra_hosts: Vec::new(),
            ports: Vec::new(),
            mac_address: None,
        }
    }
}

/// Network interface information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name
    pub name: String,
    /// MAC address
    pub mac_address: String,
    /// IPv4 addresses
    pub ipv4_addresses: Vec<String>,
    /// IPv6 addresses
    pub ipv6_addresses: Vec<String>,
    /// MTU
    pub mtu: u32,
}
