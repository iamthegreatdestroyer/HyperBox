//! CNI (Container Network Interface) integration.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info};

/// CNI plugin paths.
pub const CNI_BIN_DIR: &str = "/opt/cni/bin";
pub const CNI_CONF_DIR: &str = "/etc/cni/net.d";

/// CNI configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CniConfig {
    /// CNI version
    pub cni_version: String,
    /// Network name
    pub name: String,
    /// Plugin type
    #[serde(rename = "type")]
    pub plugin_type: String,
    /// Bridge name (for bridge plugin)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge: Option<String>,
    /// Enable IP masquerade
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_gateway: Option<bool>,
    /// IP masquerade
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_masq: Option<bool>,
    /// IPAM configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipam: Option<IpamConfig>,
    /// Additional plugin-specific options
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// IPAM configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpamConfig {
    /// IPAM type (host-local, dhcp, static)
    #[serde(rename = "type")]
    pub ipam_type: String,
    /// Subnet
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet: Option<String>,
    /// Gateway
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    /// IP ranges
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranges: Option<Vec<Vec<IpRange>>>,
    /// Routes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routes: Option<Vec<Route>>,
}

/// IP range for IPAM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpRange {
    /// Subnet
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet: Option<String>,
    /// Gateway
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
}

/// Network route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// Destination (e.g., "0.0.0.0/0")
    pub dst: String,
    /// Gateway (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gw: Option<String>,
}

/// CNI result from ADD operation.
#[derive(Debug, Clone, Deserialize)]
pub struct CniResult {
    /// CNI version
    pub cni_version: Option<String>,
    /// Assigned IPs
    pub ips: Option<Vec<CniIp>>,
    /// Routes
    pub routes: Option<Vec<Route>>,
    /// DNS configuration
    pub dns: Option<CniDns>,
}

/// IP assignment from CNI.
#[derive(Debug, Clone, Deserialize)]
pub struct CniIp {
    /// IP address with prefix
    pub address: String,
    /// Gateway
    pub gateway: Option<String>,
    /// Interface index
    pub interface: Option<u32>,
}

/// DNS configuration from CNI.
#[derive(Debug, Clone, Deserialize)]
pub struct CniDns {
    /// Nameservers
    pub nameservers: Option<Vec<String>>,
    /// Search domains
    pub search: Option<Vec<String>>,
    /// Options
    pub options: Option<Vec<String>>,
}

/// CNI manager for container networking.
pub struct CniManager {
    bin_dir: PathBuf,
    conf_dir: PathBuf,
}

impl CniManager {
    /// Create a new CNI manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bin_dir: PathBuf::from(CNI_BIN_DIR),
            conf_dir: PathBuf::from(CNI_CONF_DIR),
        }
    }

    /// Create with custom paths.
    #[must_use]
    pub fn with_paths(bin_dir: impl Into<PathBuf>, conf_dir: impl Into<PathBuf>) -> Self {
        Self {
            bin_dir: bin_dir.into(),
            conf_dir: conf_dir.into(),
        }
    }

    /// Check if CNI is available.
    pub fn is_available(&self) -> bool {
        self.bin_dir.exists() && self.conf_dir.exists()
    }

    /// Get the default HyperBox CNI configuration.
    #[must_use]
    pub fn default_config() -> CniConfig {
        CniConfig {
            cni_version: "1.0.0".to_string(),
            name: "hyperbox".to_string(),
            plugin_type: "bridge".to_string(),
            bridge: Some("hyperbox0".to_string()),
            is_gateway: Some(true),
            ip_masq: Some(true),
            ipam: Some(IpamConfig {
                ipam_type: "host-local".to_string(),
                subnet: Some("172.20.0.0/16".to_string()),
                gateway: Some("172.20.0.1".to_string()),
                ranges: None,
                routes: Some(vec![Route {
                    dst: "0.0.0.0/0".to_string(),
                    gw: None,
                }]),
            }),
            extra: HashMap::new(),
        }
    }

    /// Write CNI configuration to file.
    pub async fn write_config(&self, config: &CniConfig) -> Result<PathBuf> {
        tokio::fs::create_dir_all(&self.conf_dir).await?;

        let config_path = self.conf_dir.join(format!("{}.conflist", config.name));
        let conflist = serde_json::json!({
            "cniVersion": config.cni_version,
            "name": config.name,
            "plugins": [config]
        });

        tokio::fs::write(&config_path, serde_json::to_string_pretty(&conflist)?).await?;

        debug!("Wrote CNI config to {:?}", config_path);
        Ok(config_path)
    }

    /// Add a container to a network.
    #[cfg(unix)]
    pub async fn add(
        &self,
        container_id: &str,
        netns_path: &Path,
        config: &CniConfig,
    ) -> Result<CniResult> {
        let plugin_path = self.bin_dir.join(&config.plugin_type);

        if !plugin_path.exists() {
            return Err(CoreError::NetworkConfiguration(format!(
                "CNI plugin {} not found",
                config.plugin_type
            )));
        }

        let config_json = serde_json::to_string(config)?;

        info!("Adding container {} to network {}", container_id, config.name);

        let output = Command::new(&plugin_path)
            .env("CNI_COMMAND", "ADD")
            .env("CNI_CONTAINERID", container_id)
            .env("CNI_NETNS", netns_path.to_string_lossy().as_ref())
            .env("CNI_IFNAME", "eth0")
            .env("CNI_PATH", self.bin_dir.to_string_lossy().as_ref())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        // Would write config_json to stdin and read result
        // Simplified for now
        let output = output
            .wait_with_output()
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        if !output.status.success() {
            return Err(CoreError::NetworkConfiguration(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let result: CniResult = serde_json::from_slice(&output.stdout).unwrap_or(CniResult {
            cni_version: Some("1.0.0".to_string()),
            ips: None,
            routes: None,
            dns: None,
        });

        Ok(result)
    }

    /// Remove a container from a network.
    #[cfg(unix)]
    pub async fn del(
        &self,
        container_id: &str,
        netns_path: &Path,
        config: &CniConfig,
    ) -> Result<()> {
        let plugin_path = self.bin_dir.join(&config.plugin_type);

        if !plugin_path.exists() {
            return Ok(()); // Plugin not found, nothing to clean up
        }

        info!("Removing container {} from network {}", container_id, config.name);

        let output = Command::new(&plugin_path)
            .env("CNI_COMMAND", "DEL")
            .env("CNI_CONTAINERID", container_id)
            .env("CNI_NETNS", netns_path.to_string_lossy().as_ref())
            .env("CNI_IFNAME", "eth0")
            .env("CNI_PATH", self.bin_dir.to_string_lossy().as_ref())
            .output()
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        if !output.status.success() {
            debug!("CNI DEL warning: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    /// List available CNI plugins.
    pub fn list_plugins(&self) -> Vec<String> {
        let mut plugins = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.bin_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    plugins.push(name.to_string());
                }
            }
        }

        plugins
    }
}

impl Default for CniManager {
    fn default() -> Self {
        Self::new()
    }
}
