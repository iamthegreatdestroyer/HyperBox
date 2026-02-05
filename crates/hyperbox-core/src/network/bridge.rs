//! Bridge network management.

use crate::error::{CoreError, Result};
use std::net::Ipv4Addr;
use std::process::Command;
use tracing::{debug, info};

/// Default HyperBox bridge name.
pub const DEFAULT_BRIDGE: &str = "hyperbox0";
/// Default bridge subnet.
pub const DEFAULT_SUBNET: &str = "172.20.0.0/16";
/// Default gateway.
pub const DEFAULT_GATEWAY: &str = "172.20.0.1";

/// Bridge network manager.
pub struct BridgeNetwork {
    name: String,
    subnet: String,
    gateway: Ipv4Addr,
}

impl BridgeNetwork {
    /// Create a new bridge network manager.
    #[must_use]
    pub fn new(name: impl Into<String>, subnet: impl Into<String>, gateway: Ipv4Addr) -> Self {
        Self {
            name: name.into(),
            subnet: subnet.into(),
            gateway,
        }
    }

    /// Create the default HyperBox bridge.
    #[must_use]
    pub fn default_bridge() -> Self {
        Self {
            name: DEFAULT_BRIDGE.to_string(),
            subnet: DEFAULT_SUBNET.to_string(),
            gateway: DEFAULT_GATEWAY.parse().unwrap(),
        }
    }

    /// Check if the bridge exists.
    pub fn exists(&self) -> bool {
        let output = Command::new("ip")
            .args(["link", "show", &self.name])
            .output();

        matches!(output, Ok(o) if o.status.success())
    }

    /// Create the bridge interface.
    #[cfg(unix)]
    pub fn create(&self) -> Result<()> {
        if self.exists() {
            debug!("Bridge {} already exists", self.name);
            return Ok(());
        }

        info!("Creating bridge network {}", self.name);

        // Create bridge
        let output = Command::new("ip")
            .args(["link", "add", "name", &self.name, "type", "bridge"])
            .output()
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        if !output.status.success() {
            return Err(CoreError::NetworkConfiguration(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        // Set IP address
        let gateway_cidr = format!("{}/16", self.gateway);
        let output = Command::new("ip")
            .args(["addr", "add", &gateway_cidr, "dev", &self.name])
            .output()
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore if address already exists
            if !stderr.contains("RTNETLINK answers: File exists") {
                return Err(CoreError::NetworkConfiguration(stderr.to_string()));
            }
        }

        // Bring up the bridge
        let output = Command::new("ip")
            .args(["link", "set", &self.name, "up"])
            .output()
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        if !output.status.success() {
            return Err(CoreError::NetworkConfiguration(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        // Enable IP forwarding
        let _ = std::fs::write("/proc/sys/net/ipv4/ip_forward", "1");

        // Setup NAT
        self.setup_nat()?;

        info!("Bridge {} created successfully", self.name);
        Ok(())
    }

    /// Setup NAT for container internet access.
    #[cfg(unix)]
    fn setup_nat(&self) -> Result<()> {
        // Add MASQUERADE rule for the subnet
        let output = Command::new("iptables")
            .args([
                "-t",
                "nat",
                "-A",
                "POSTROUTING",
                "-s",
                &self.subnet,
                "!",
                "-o",
                &self.name,
                "-j",
                "MASQUERADE",
            ])
            .output()
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore if rule already exists
            if !stderr.contains("already exists") {
                debug!("NAT setup warning: {}", stderr);
            }
        }

        // Allow forwarding to/from the bridge
        let _ = Command::new("iptables")
            .args(["-A", "FORWARD", "-i", &self.name, "-j", "ACCEPT"])
            .output();

        let _ = Command::new("iptables")
            .args(["-A", "FORWARD", "-o", &self.name, "-j", "ACCEPT"])
            .output();

        Ok(())
    }

    /// Delete the bridge.
    #[cfg(unix)]
    pub fn delete(&self) -> Result<()> {
        if !self.exists() {
            return Ok(());
        }

        info!("Deleting bridge {}", self.name);

        // Remove NAT rules
        let _ = Command::new("iptables")
            .args([
                "-t",
                "nat",
                "-D",
                "POSTROUTING",
                "-s",
                &self.subnet,
                "!",
                "-o",
                &self.name,
                "-j",
                "MASQUERADE",
            ])
            .output();

        // Bring down and delete bridge
        let _ = Command::new("ip")
            .args(["link", "set", &self.name, "down"])
            .output();

        let output = Command::new("ip")
            .args(["link", "delete", &self.name, "type", "bridge"])
            .output()
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        if !output.status.success() {
            return Err(CoreError::NetworkConfiguration(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Allocate an IP address for a container.
    pub fn allocate_ip(&self) -> Result<Ipv4Addr> {
        // Simple sequential allocation
        // Production would use IPAM with persistence
        static NEXT_IP: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(2);

        let host_part = NEXT_IP.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        if host_part > 65534 {
            return Err(CoreError::NetworkConfiguration("IP address pool exhausted".to_string()));
        }

        // 172.20.x.y
        let ip = Ipv4Addr::new(172, 20, (host_part >> 8) as u8, (host_part & 0xFF) as u8);
        Ok(ip)
    }

    /// Get the gateway address.
    #[must_use]
    pub fn gateway(&self) -> Ipv4Addr {
        self.gateway
    }

    /// Get the bridge name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Default for BridgeNetwork {
    fn default() -> Self {
        Self::default_bridge()
    }
}
