//! DevContainers support for HyperBox.
//!
//! Implements the [Development Containers specification](https://containers.dev/implementors/json_reference/)
//! to fill the gap left by Docker Desktop removing Dev Environments (v4.42+).
//!
//! This module provides:
//! - Parsing of `.devcontainer/devcontainer.json` and `.devcontainer.json`
//! - OCI-based Feature installation
//! - Template support
//! - Automatic port forwarding from devcontainer configuration
//! - Integration with [`crate::ProjectManager`]
//!
//! # Locating DevContainer Config
//!
//! The spec defines the following search order:
//! 1. `.devcontainer/devcontainer.json`
//! 2. `.devcontainer.json` (root)
//! 3. `.devcontainer/<folder>/devcontainer.json` (multi-config)

use crate::config::{
    BuildConfig, ContainerDef, DevConfig, NetworkDef, PortDef, ProjectConfig, ProjectType,
    SyncMode, VolumeDef,
};
use crate::error::{ProjectError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// DevContainer JSON Schema
// ---------------------------------------------------------------------------

/// Top-level devcontainer.json representation.
///
/// Based on the [containers.dev JSON reference](https://containers.dev/implementors/json_reference/).
/// Only widely-used fields are included; unknown fields are captured via
/// `#[serde(flatten)]` into `extra`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevContainerConfig {
    // -- Metadata ----------------------------------------------------------

    /// An optional display name for the configuration.
    #[serde(default)]
    pub name: Option<String>,

    // -- Image source (one of three) ----------------------------------------

    /// Base image for the container.
    #[serde(default)]
    pub image: Option<String>,

    /// Dockerfile-based build configuration.
    #[serde(default)]
    pub build: Option<DevContainerBuild>,

    /// Docker Compose configuration.
    #[serde(default, alias = "dockerComposeFile")]
    pub docker_compose_file: Option<StringOrVec>,

    /// Service name within the compose file.
    #[serde(default)]
    pub service: Option<String>,

    // -- Features -----------------------------------------------------------

    /// OCI-based Dev Container Features (key = OCI reference, value = options).
    #[serde(default)]
    pub features: HashMap<String, serde_json::Value>,

    // -- Lifecycle ----------------------------------------------------------

    /// Command to run after the container is created (first time only).
    #[serde(default)]
    pub on_create_command: Option<LifecycleCommand>,

    /// Command to run after `onCreateCommand` and after rebuilds.
    #[serde(default)]
    pub update_content_command: Option<LifecycleCommand>,

    /// Command to run every time the container starts.
    #[serde(default)]
    pub post_create_command: Option<LifecycleCommand>,

    /// Command to run after `postCreateCommand`.
    #[serde(default)]
    pub post_start_command: Option<LifecycleCommand>,

    /// Command to run when a tool connects. Runs after `postStartCommand`.
    #[serde(default)]
    pub post_attach_command: Option<LifecycleCommand>,

    // -- Runtime configuration ----------------------------------------------

    /// Ports to forward from the container.
    #[serde(default)]
    pub forward_ports: Vec<PortOrString>,

    /// Port attributes keyed by port number (as string).
    #[serde(default)]
    pub ports_attributes: HashMap<String, PortAttributes>,

    /// Default port attributes applied to all forwarded ports.
    #[serde(default)]
    pub other_ports_attributes: Option<PortAttributes>,

    /// Environment variables set inside the container.
    #[serde(default)]
    pub container_env: HashMap<String, String>,

    /// Environment variables set for the remote user.
    #[serde(default)]
    pub remote_env: HashMap<String, String>,

    /// Remote user to run as inside the container.
    #[serde(default)]
    pub remote_user: Option<String>,

    /// Whether to override the container's default command.
    #[serde(default)]
    pub override_command: Option<bool>,

    /// Whether to shut down the container action on tool window close.
    #[serde(default)]
    pub shutdown_action: Option<ShutdownAction>,

    /// Mounts to add to the container.
    #[serde(default)]
    pub mounts: Vec<MountOrString>,

    /// Run arguments passed to `docker run` / container creation.
    #[serde(default)]
    pub run_args: Vec<String>,

    /// Working directory inside the container.
    #[serde(default)]
    pub workspace_folder: Option<String>,

    /// Mount the workspace.
    #[serde(default)]
    pub workspace_mount: Option<String>,

    // -- Tool customization -------------------------------------------------

    /// Per-tool customizations (e.g. `vscode`, `jetbrains`).
    #[serde(default)]
    pub customizations: HashMap<String, serde_json::Value>,

    // -- Resource limits ----------------------------------------------------

    /// Container user (for `USER` directive override).
    #[serde(default)]
    pub container_user: Option<String>,

    /// Privileged mode.
    #[serde(default)]
    pub privileged: Option<bool>,

    /// Init process.
    #[serde(default)]
    pub init: Option<bool>,

    /// Capabilities to add.
    #[serde(default)]
    pub cap_add: Vec<String>,

    /// Security options.
    #[serde(default)]
    pub security_opt: Vec<String>,

    // -- Extra fields (forward-compatible) ----------------------------------

    /// Unknown fields captured for round-tripping.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Dockerfile-based build configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevContainerBuild {
    /// Dockerfile path (relative to context).
    #[serde(default)]
    pub dockerfile: Option<String>,

    /// Build context directory.
    #[serde(default)]
    pub context: Option<String>,

    /// Build arguments.
    #[serde(default)]
    pub args: HashMap<String, String>,

    /// Target stage for multi-stage builds.
    #[serde(default)]
    pub target: Option<String>,

    /// Cache-from images.
    #[serde(default)]
    pub cache_from: Option<StringOrVec>,
}

/// A value that can be either a single string or a vector of strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrVec {
    /// Single string value.
    Single(String),
    /// Multiple string values.
    Multiple(Vec<String>),
}

impl StringOrVec {
    /// Convert to a `Vec<String>`.
    #[must_use]
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s],
            Self::Multiple(v) => v,
        }
    }
}

/// Lifecycle command — can be a string, array, or object of named commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LifecycleCommand {
    /// Simple shell string.
    Simple(String),
    /// Command and arguments.
    Array(Vec<String>),
    /// Named parallel commands.
    Object(HashMap<String, StringOrVec>),
}

impl LifecycleCommand {
    /// Flatten to a list of shell commands to execute sequentially.
    #[must_use]
    pub fn to_shell_commands(&self) -> Vec<String> {
        match self {
            Self::Simple(s) => vec![s.clone()],
            Self::Array(a) => vec![a.join(" ")],
            Self::Object(map) => map
                .values()
                .map(|v| match v {
                    StringOrVec::Single(s) => s.clone(),
                    StringOrVec::Multiple(a) => a.join(" "),
                })
                .collect(),
        }
    }
}

/// Port forwarding value — either a number or `"host:container"` style string.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PortOrString {
    /// Numeric port (both host and container).
    Number(u16),
    /// String form, e.g. `"8080:80"` or `"label:8080"`.
    Named(String),
}

/// Per-port attributes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortAttributes {
    /// Human-readable label.
    #[serde(default)]
    pub label: Option<String>,

    /// What happens when the port is auto-detected.
    #[serde(default)]
    pub on_auto_forward: Option<String>,

    /// Whether the port requires elevation.
    #[serde(default)]
    pub elevate_if_needed: Option<bool>,

    /// Protocol (default `"http"`).
    #[serde(default)]
    pub protocol: Option<String>,
}

/// Action taken when the tool window that opened the container is closed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShutdownAction {
    /// Do nothing — leave the container running.
    None,
    /// Stop the container gracefully.
    StopContainer,
    /// Stop the compose project.
    StopCompose,
}

/// Mount definition — either a structured object or a Docker-style string.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MountOrString {
    /// Docker-style bind/volume string (e.g. `"type=bind,source=...,target=..."`).
    String(String),
    /// Structured mount.
    Structured(MountDef),
}

/// Structured mount definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountDef {
    /// Mount type (`bind`, `volume`, `tmpfs`).
    #[serde(rename = "type")]
    pub mount_type: String,
    /// Source path or volume name.
    pub source: Option<String>,
    /// Target path inside container.
    pub target: String,
}

// ---------------------------------------------------------------------------
// DevContainer Feature
// ---------------------------------------------------------------------------

/// Metadata for an OCI-based Dev Container Feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureMetadata {
    /// Feature identifier (OCI reference).
    pub id: String,
    /// Version constraint.
    pub version: Option<String>,
    /// Feature name.
    pub name: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Options schema.
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
    /// Install script (relative path).
    pub install_sh: Option<String>,
}

// ---------------------------------------------------------------------------
// DevContainer Template
// ---------------------------------------------------------------------------

/// Metadata for a Dev Container Template.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateMetadata {
    /// Template identifier (OCI reference).
    pub id: String,
    /// Display name.
    pub name: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Base devcontainer.json included in the template.
    pub devcontainer: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// DevContainerManager
// ---------------------------------------------------------------------------

/// Manages Dev Container configuration lifecycle.
///
/// Responsibilities:
/// - Locate and parse `devcontainer.json`
/// - Resolve Features referenced in the configuration
/// - Convert the parsed configuration to [`ProjectConfig`]
/// - Generate build instructions for the container image
pub struct DevContainerManager {
    /// Resolved configuration (after parsing).
    config: Option<DevContainerConfig>,
    /// Path to the devcontainer.json file that was loaded.
    config_path: Option<PathBuf>,
    /// Project root directory.
    project_root: PathBuf,
}

impl DevContainerManager {
    /// Create a new manager for the given project root.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            config: None,
            config_path: None,
            project_root: project_root.into(),
        }
    }

    /// Locate and load the devcontainer configuration.
    ///
    /// Search order:
    /// 1. `.devcontainer/devcontainer.json`
    /// 2. `.devcontainer.json` (root)
    /// 3. `.devcontainer/<folder>/devcontainer.json` (first match)
    pub async fn load(&mut self) -> Result<&DevContainerConfig> {
        let path = Self::find_config(&self.project_root).await?;
        let content = fs::read_to_string(&path).await.map_err(|e| {
            ProjectError::ConfigError {
                path: path.clone(),
                message: format!("Failed to read devcontainer.json: {e}"),
            }
        })?;

        // Strip JSON comments (// and /* */) — devcontainer.json allows JSONC.
        let cleaned = strip_jsonc_comments(&content);

        let config: DevContainerConfig = serde_json::from_str(&cleaned).map_err(|e| {
            ProjectError::ConfigError {
                path: path.clone(),
                message: format!("Failed to parse devcontainer.json: {e}"),
            }
        })?;

        info!(
            name = config.name.as_deref().unwrap_or("<unnamed>"),
            features = config.features.len(),
            forward_ports = config.forward_ports.len(),
            "Loaded devcontainer config from {path:?}"
        );

        self.config_path = Some(path);
        self.config = Some(config);
        Ok(self.config.as_ref().expect("just set"))
    }

    /// Return the loaded configuration, if any.
    #[must_use]
    pub fn config(&self) -> Option<&DevContainerConfig> {
        self.config.as_ref()
    }

    /// Return the path from which the config was loaded.
    #[must_use]
    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }

    /// Check whether the project root has a devcontainer configuration.
    pub async fn has_config(project_root: &Path) -> bool {
        Self::find_config(project_root).await.is_ok()
    }

    /// List available multi-root configurations under `.devcontainer/`.
    ///
    /// Returns `(folder_name, path)` pairs for each sub-directory that
    /// contains a `devcontainer.json`.
    pub async fn list_configs(project_root: &Path) -> Result<Vec<(String, PathBuf)>> {
        let devcontainer_dir = project_root.join(".devcontainer");
        let mut configs = Vec::new();

        // Primary config.
        let primary = devcontainer_dir.join("devcontainer.json");
        if primary.exists() {
            configs.push(("default".to_string(), primary));
        }

        // Root-level config.
        let root_config = project_root.join(".devcontainer.json");
        if root_config.exists() {
            configs.push(("root".to_string(), root_config));
        }

        // Sub-folder configs.
        if devcontainer_dir.is_dir() {
            if let Ok(mut entries) = fs::read_dir(&devcontainer_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_dir() {
                        let child_config = path.join("devcontainer.json");
                        if child_config.exists() {
                            if let Some(name) = path.file_name() {
                                configs.push((
                                    name.to_string_lossy().to_string(),
                                    child_config,
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(configs)
    }

    /// Convert the loaded devcontainer config to a [`ProjectConfig`].
    ///
    /// This bridges the devcontainer specification to HyperBox's internal
    /// project configuration model.
    pub fn to_project_config(&self) -> Result<ProjectConfig> {
        let config = self.config.as_ref().ok_or_else(|| {
            ProjectError::InvalidConfig("No devcontainer config loaded".into())
        })?;

        let container_def = self.build_container_def(config)?;

        let volumes = self.build_volume_defs(config);

        let network = NetworkDef {
            name: None,
            isolated: true,
            ipv6: false,
            subnet: None,
        };

        let build = self.build_build_config(config);

        let dev = DevConfig {
            hot_reload: true,
            watch_paths: vec![PathBuf::from(".")],
            ignore_paths: default_ignore_paths(),
            sync_mode: SyncMode::Bind,
        };

        // Merge container env + remote env.
        let mut environment = config.container_env.clone();
        for (k, v) in &config.remote_env {
            environment.entry(k.clone()).or_insert_with(|| v.clone());
        }

        Ok(ProjectConfig {
            project_type: ProjectType::Generic,
            containers: vec![container_def],
            volumes,
            network,
            environment,
            build,
            dev,
        })
    }

    /// Resolve the list of Feature OCI references into [`FeatureRef`] entries.
    ///
    /// Each feature is represented as a key-value pair where the key is a
    /// GitHub short-hand (e.g. `ghcr.io/devcontainers/features/node:1`) or
    /// a local path, and the value contains option overrides.
    #[must_use]
    pub fn resolve_features(&self) -> Vec<FeatureRef> {
        let Some(config) = &self.config else {
            return Vec::new();
        };

        config
            .features
            .iter()
            .map(|(reference, options)| {
                let (id, version) = parse_feature_reference(reference);
                FeatureRef {
                    reference: reference.clone(),
                    id,
                    version,
                    options: options.clone(),
                }
            })
            .collect()
    }

    /// Generate Dockerfile instructions that install the resolved features.
    ///
    /// Features are typically installed by downloading the feature archive,
    /// extracting it, and running `install.sh`. This method produces the
    /// `RUN` directives needed.
    #[must_use]
    pub fn feature_install_instructions(&self) -> Vec<String> {
        let features = self.resolve_features();
        if features.is_empty() {
            return Vec::new();
        }

        let mut instructions = Vec::new();
        instructions.push("# Dev Container Features".to_string());

        for feature in &features {
            let env_args: Vec<String> = if let Some(obj) = feature.options.as_object() {
                obj.iter()
                    .filter_map(|(k, v)| {
                        let val = match v {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            serde_json::Value::Number(n) => n.to_string(),
                            _ => return None,
                        };
                        // Feature options are exposed as env vars with prefix.
                        Some(format!("{k}={val}"))
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let env_prefix = if env_args.is_empty() {
                String::new()
            } else {
                format!("{} ", env_args.join(" "))
            };

            instructions.push(format!(
                "# Feature: {} ({})",
                feature.id,
                feature.version.as_deref().unwrap_or("latest")
            ));
            instructions.push(format!(
                "RUN {env_prefix}bash -c \"$(curl -fsSL https://raw.githubusercontent.com/devcontainers/features/main/src/{}/install.sh)\"",
                feature.id
            ));
        }

        instructions
    }

    // -- Private helpers ----------------------------------------------------

    /// Locate the devcontainer configuration file.
    async fn find_config(project_root: &Path) -> Result<PathBuf> {
        // 1. .devcontainer/devcontainer.json
        let primary = project_root
            .join(".devcontainer")
            .join("devcontainer.json");
        if primary.exists() {
            debug!("Found devcontainer config at {primary:?}");
            return Ok(primary);
        }

        // 2. .devcontainer.json (root level)
        let root_config = project_root.join(".devcontainer.json");
        if root_config.exists() {
            debug!("Found root devcontainer config at {root_config:?}");
            return Ok(root_config);
        }

        // 3. .devcontainer/<folder>/devcontainer.json — first match
        let devcontainer_dir = project_root.join(".devcontainer");
        if devcontainer_dir.is_dir() {
            if let Ok(mut entries) = fs::read_dir(&devcontainer_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_dir() {
                        let child = path.join("devcontainer.json");
                        if child.exists() {
                            debug!("Found sub-folder devcontainer config at {child:?}");
                            return Ok(child);
                        }
                    }
                }
            }
        }

        Err(ProjectError::ConfigError {
            path: project_root.to_path_buf(),
            message: "No devcontainer.json found".into(),
        })
    }

    /// Build the primary [`ContainerDef`] from the devcontainer config.
    fn build_container_def(&self, config: &DevContainerConfig) -> Result<ContainerDef> {
        let name = config
            .name
            .clone()
            .unwrap_or_else(|| {
                self.project_root
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "devcontainer".to_string())
            });

        let image = config
            .image
            .clone()
            .or_else(|| {
                // If no image specified and there is a build section, use a
                // placeholder that will be replaced by the built image.
                config.build.as_ref().map(|_| format!("{name}:devcontainer"))
            })
            .unwrap_or_else(|| "mcr.microsoft.com/devcontainers/base:ubuntu".to_string());

        let dockerfile = config.build.as_ref().and_then(|b| {
            b.dockerfile.as_ref().map(PathBuf::from)
        });

        let context = config.build.as_ref().and_then(|b| {
            b.context.as_ref().map(PathBuf::from)
        });

        let command = config
            .on_create_command
            .as_ref()
            .map(|lc| lc.to_shell_commands())
            .or_else(|| {
                config.override_command.and_then(|ov| {
                    if ov {
                        Some(vec!["sleep".into(), "infinity".into()])
                    } else {
                        None
                    }
                })
            });

        let ports = self.extract_ports(config);

        let mut environment = config.container_env.clone();
        for (k, v) in &config.remote_env {
            environment.entry(k.clone()).or_insert_with(|| v.clone());
        }

        let volumes = self.extract_volume_strings(config);

        Ok(ContainerDef {
            name,
            image,
            dockerfile,
            context,
            command,
            ports,
            volumes,
            environment,
            depends_on: Vec::new(),
            healthcheck: None,
            resources: None,
        })
    }

    /// Extract forwarded ports from the devcontainer configuration.
    fn extract_ports(&self, config: &DevContainerConfig) -> Vec<PortDef> {
        config
            .forward_ports
            .iter()
            .filter_map(|p| match p {
                PortOrString::Number(port) => Some(PortDef {
                    container: *port,
                    host: None,
                    protocol: "tcp".into(),
                }),
                PortOrString::Named(s) => parse_port_string(s),
            })
            .collect()
    }

    /// Extract mount/volume strings from the devcontainer configuration.
    fn extract_volume_strings(&self, config: &DevContainerConfig) -> Vec<String> {
        config
            .mounts
            .iter()
            .map(|m| match m {
                MountOrString::String(s) => s.clone(),
                MountOrString::Structured(def) => {
                    let src = def.source.as_deref().unwrap_or("");
                    format!("type={},source={src},target={}", def.mount_type, def.target)
                }
            })
            .collect()
    }

    /// Build volume definitions from the mounts.
    fn build_volume_defs(&self, config: &DevContainerConfig) -> Vec<VolumeDef> {
        config
            .mounts
            .iter()
            .filter_map(|m| match m {
                MountOrString::Structured(def) if def.mount_type == "volume" => {
                    Some(VolumeDef {
                        name: def
                            .source
                            .clone()
                            .unwrap_or_else(|| def.target.replace('/', "_")),
                        source: def.source.as_ref().map(PathBuf::from),
                        driver_opts: HashMap::new(),
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// Build a [`BuildConfig`] from the devcontainer build section.
    fn build_build_config(&self, config: &DevContainerConfig) -> BuildConfig {
        match &config.build {
            Some(build) => BuildConfig {
                buildkit: true,
                args: build.args.clone(),
                target: build.target.clone(),
                cache_from: build
                    .cache_from
                    .as_ref()
                    .map(|c| c.clone().into_vec())
                    .unwrap_or_default(),
                platform: None,
            },
            None => BuildConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Resolved Feature Reference
// ---------------------------------------------------------------------------

/// A resolved Dev Container Feature reference.
#[derive(Debug, Clone)]
pub struct FeatureRef {
    /// Original OCI reference string.
    pub reference: String,
    /// Extracted feature ID (e.g. `node`, `python`, `docker-in-docker`).
    pub id: String,
    /// Version constraint (from `@version` or `:version` suffix).
    pub version: Option<String>,
    /// Option overrides from `devcontainer.json`.
    pub options: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Parse a feature reference into `(id, version)`.
///
/// Supports:
/// - `ghcr.io/devcontainers/features/node:1`  → (`node`, Some(`1`))
/// - `./local-feature`                          → (`local-feature`, None)
/// - `my-feature@1.2.3`                         → (`my-feature`, Some(`1.2.3`))
fn parse_feature_reference(reference: &str) -> (String, Option<String>) {
    // Handle OCI references with ':' version.
    if let Some(last_slash) = reference.rfind('/') {
        let name_ver = &reference[last_slash + 1..];
        if let Some(colon) = name_ver.rfind(':') {
            let id = &name_ver[..colon];
            let version = &name_ver[colon + 1..];
            return (id.to_string(), Some(version.to_string()));
        }
        return (name_ver.to_string(), None);
    }

    // Handle `name@version`.
    if let Some(at) = reference.rfind('@') {
        let id = &reference[..at];
        let version = &reference[at + 1..];
        return (id.to_string(), Some(version.to_string()));
    }

    // Handle `./path` or bare name.
    let name = reference
        .trim_start_matches("./")
        .trim_start_matches("../");
    (name.to_string(), None)
}

/// Parse a port string like `"8080:80"`, `"8080"`, or `"label:8080"`.
fn parse_port_string(s: &str) -> Option<PortDef> {
    // Try "host:container" form.
    if let Some(colon) = s.find(':') {
        let left = &s[..colon];
        let right = &s[colon + 1..];

        if let (Ok(host), Ok(container)) = (left.parse::<u16>(), right.parse::<u16>()) {
            return Some(PortDef {
                container,
                host: Some(host),
                protocol: "tcp".into(),
            });
        }

        // "label:port" form.
        if let Ok(port) = right.parse::<u16>() {
            return Some(PortDef {
                container: port,
                host: None,
                protocol: "tcp".into(),
            });
        }
    }

    // Plain number.
    if let Ok(port) = s.parse::<u16>() {
        return Some(PortDef {
            container: port,
            host: None,
            protocol: "tcp".into(),
        });
    }

    warn!("Could not parse port string: {s}");
    None
}

/// Default ignore paths for dev containers.
fn default_ignore_paths() -> Vec<String> {
    vec![
        "**/node_modules/**".into(),
        "**/.git/**".into(),
        "**/target/**".into(),
        "**/__pycache__/**".into(),
        "**/.venv/**".into(),
    ]
}

/// Strip JSONC-style comments from input.
///
/// DevContainer config files officially support JSON with Comments (JSONC).
/// This function removes `//` line comments and `/* */` block comments while
/// preserving strings (so `"//"` inside a string is not stripped).
fn strip_jsonc_comments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Inside a string literal — copy verbatim.
        if bytes[i] == b'"' {
            output.push('"');
            i += 1;
            while i < len {
                if bytes[i] == b'\\' && i + 1 < len {
                    output.push(bytes[i] as char);
                    output.push(bytes[i + 1] as char);
                    i += 2;
                } else if bytes[i] == b'"' {
                    output.push('"');
                    i += 1;
                    break;
                } else {
                    output.push(bytes[i] as char);
                    i += 1;
                }
            }
            continue;
        }

        // Line comment.
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            // Skip to end of line.
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Block comment.
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2; // skip */
            }
            continue;
        }

        output.push(bytes[i] as char);
        i += 1;
    }

    output
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_devcontainer_json() -> &'static str {
        r#"{
            // This is a JSONC comment
            "name": "Rust Development",
            "image": "mcr.microsoft.com/devcontainers/rust:1",
            "features": {
                "ghcr.io/devcontainers/features/node:1": {
                    "version": "20"
                },
                "ghcr.io/devcontainers/features/docker-in-docker:2": {}
            },
            "forwardPorts": [8080, 3000],
            "containerEnv": {
                "CARGO_HOME": "/usr/local/cargo",
                "RUST_LOG": "debug"
            },
            "remoteUser": "vscode",
            "postCreateCommand": "cargo build",
            "customizations": {
                "vscode": {
                    "extensions": [
                        "rust-lang.rust-analyzer",
                        "tamasfe.even-better-toml"
                    ]
                }
            }
        }"#
    }

    fn sample_build_devcontainer_json() -> &'static str {
        r#"{
            "name": "Custom Build",
            "build": {
                "dockerfile": "Dockerfile.dev",
                "context": ".",
                "args": {
                    "RUST_VERSION": "1.78"
                }
            },
            "forwardPorts": [8080],
            "mounts": [
                {
                    "type": "volume",
                    "source": "cargo-cache",
                    "target": "/usr/local/cargo/registry"
                },
                "type=bind,source=${localWorkspaceFolder}/.cargo,target=/home/vscode/.cargo"
            ]
        }"#
    }

    #[test]
    fn test_parse_image_based_config() {
        let config: DevContainerConfig =
            serde_json::from_str(sample_devcontainer_json()).expect("parse");

        assert_eq!(config.name.as_deref(), Some("Rust Development"));
        assert_eq!(
            config.image.as_deref(),
            Some("mcr.microsoft.com/devcontainers/rust:1")
        );
        assert_eq!(config.features.len(), 2);
        assert_eq!(config.forward_ports.len(), 2);
        assert_eq!(config.container_env.len(), 2);
        assert_eq!(config.remote_user.as_deref(), Some("vscode"));
    }

    #[test]
    fn test_parse_build_based_config() {
        let config: DevContainerConfig =
            serde_json::from_str(sample_build_devcontainer_json()).expect("parse");

        assert_eq!(config.name.as_deref(), Some("Custom Build"));
        assert!(config.image.is_none());
        let build = config.build.as_ref().expect("build section");
        assert_eq!(build.dockerfile.as_deref(), Some("Dockerfile.dev"));
        assert_eq!(build.args.get("RUST_VERSION").map(String::as_str), Some("1.78"));
        assert_eq!(config.mounts.len(), 2);
    }

    #[test]
    fn test_strip_jsonc_comments() {
        let input = r#"{
            // line comment
            "key": "value", // trailing
            /* block
               comment */
            "url": "https://example.com" // with slashes in string
        }"#;
        let cleaned = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&cleaned).expect("valid json");
        assert_eq!(parsed["key"], "value");
        assert_eq!(parsed["url"], "https://example.com");
    }

    #[test]
    fn test_strip_jsonc_preserves_string_slashes() {
        let input = r#"{"path": "//server/share", "url": "http://x"}"#;
        let cleaned = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&cleaned).expect("valid json");
        assert_eq!(parsed["path"], "//server/share");
    }

    #[test]
    fn test_parse_feature_reference_oci() {
        let (id, ver) =
            parse_feature_reference("ghcr.io/devcontainers/features/node:1");
        assert_eq!(id, "node");
        assert_eq!(ver.as_deref(), Some("1"));
    }

    #[test]
    fn test_parse_feature_reference_at_version() {
        let (id, ver) = parse_feature_reference("my-feature@1.2.3");
        assert_eq!(id, "my-feature");
        assert_eq!(ver.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn test_parse_feature_reference_local() {
        let (id, ver) = parse_feature_reference("./local-feature");
        assert_eq!(id, "local-feature");
        assert!(ver.is_none());
    }

    #[test]
    fn test_parse_port_string_host_container() {
        let port = parse_port_string("8080:80").expect("valid");
        assert_eq!(port.container, 80);
        assert_eq!(port.host, Some(8080));
    }

    #[test]
    fn test_parse_port_string_plain() {
        let port = parse_port_string("3000").expect("valid");
        assert_eq!(port.container, 3000);
        assert!(port.host.is_none());
    }

    #[test]
    fn test_parse_port_string_labeled() {
        let port = parse_port_string("api:8080").expect("valid");
        assert_eq!(port.container, 8080);
        assert!(port.host.is_none());
    }

    #[test]
    fn test_lifecycle_command_variants() {
        let simple: LifecycleCommand =
            serde_json::from_str(r#""npm install""#).expect("simple");
        assert_eq!(simple.to_shell_commands(), vec!["npm install"]);

        let array: LifecycleCommand =
            serde_json::from_str(r#"["npm", "install"]"#).expect("array");
        assert_eq!(array.to_shell_commands(), vec!["npm install"]);

        let obj: LifecycleCommand =
            serde_json::from_str(r#"{"backend": "cargo build", "frontend": "npm install"}"#)
                .expect("object");
        let cmds = obj.to_shell_commands();
        assert_eq!(cmds.len(), 2);
        assert!(cmds.contains(&"cargo build".to_string()));
        assert!(cmds.contains(&"npm install".to_string()));
    }

    #[tokio::test]
    async fn test_find_config_primary() {
        let dir = TempDir::new().expect("tempdir");
        let dc_dir = dir.path().join(".devcontainer");
        std::fs::create_dir_all(&dc_dir).expect("mkdir");
        std::fs::write(dc_dir.join("devcontainer.json"), "{}").expect("write");

        let path = DevContainerManager::find_config(dir.path())
            .await
            .expect("found");
        assert!(path.ends_with("devcontainer.json"));
    }

    #[tokio::test]
    async fn test_find_config_root_level() {
        let dir = TempDir::new().expect("tempdir");
        std::fs::write(dir.path().join(".devcontainer.json"), "{}").expect("write");

        let path = DevContainerManager::find_config(dir.path())
            .await
            .expect("found");
        assert!(path.ends_with(".devcontainer.json"));
    }

    #[tokio::test]
    async fn test_find_config_subfolder() {
        let dir = TempDir::new().expect("tempdir");
        let sub = dir.path().join(".devcontainer").join("rust");
        std::fs::create_dir_all(&sub).expect("mkdir");
        std::fs::write(sub.join("devcontainer.json"), "{}").expect("write");

        let path = DevContainerManager::find_config(dir.path())
            .await
            .expect("found");
        assert!(path.to_string_lossy().contains("rust"));
    }

    #[tokio::test]
    async fn test_find_config_not_found() {
        let dir = TempDir::new().expect("tempdir");
        let result = DevContainerManager::find_config(dir.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_and_convert() {
        let dir = TempDir::new().expect("tempdir");
        let dc_dir = dir.path().join(".devcontainer");
        std::fs::create_dir_all(&dc_dir).expect("mkdir");
        std::fs::write(
            dc_dir.join("devcontainer.json"),
            sample_devcontainer_json(),
        )
        .expect("write");

        let mut mgr = DevContainerManager::new(dir.path());
        mgr.load().await.expect("load");

        let project_config = mgr.to_project_config().expect("convert");
        assert_eq!(project_config.containers.len(), 1);
        assert_eq!(
            project_config.containers[0].image,
            "mcr.microsoft.com/devcontainers/rust:1"
        );
        assert_eq!(project_config.containers[0].ports.len(), 2);
        assert_eq!(project_config.environment.len(), 2);
    }

    #[test]
    fn test_resolve_features() {
        let config: DevContainerConfig =
            serde_json::from_str(sample_devcontainer_json()).expect("parse");

        let mut mgr = DevContainerManager::new("/tmp/test");
        mgr.config = Some(config);

        let features = mgr.resolve_features();
        assert_eq!(features.len(), 2);

        let node = features.iter().find(|f| f.id == "node").expect("node");
        assert_eq!(node.version.as_deref(), Some("1"));
    }

    #[test]
    fn test_feature_install_instructions() {
        let config: DevContainerConfig =
            serde_json::from_str(sample_devcontainer_json()).expect("parse");

        let mut mgr = DevContainerManager::new("/tmp/test");
        mgr.config = Some(config);

        let instructions = mgr.feature_install_instructions();
        assert!(!instructions.is_empty());
        assert!(instructions.iter().any(|i| i.contains("RUN")));
    }

    #[tokio::test]
    async fn test_list_configs() {
        let dir = TempDir::new().expect("tempdir");
        let dc_dir = dir.path().join(".devcontainer");
        std::fs::create_dir_all(&dc_dir).expect("mkdir");
        std::fs::write(dc_dir.join("devcontainer.json"), "{}").expect("write");

        let rust_dir = dc_dir.join("rust");
        std::fs::create_dir_all(&rust_dir).expect("mkdir");
        std::fs::write(rust_dir.join("devcontainer.json"), "{}").expect("write");

        let configs = DevContainerManager::list_configs(dir.path())
            .await
            .expect("list");
        assert_eq!(configs.len(), 2); // default + rust
    }

    #[test]
    fn test_has_no_extra_fields_lost() {
        let input = r#"{
            "image": "ubuntu",
            "unknownField": true,
            "anotherUnknown": { "nested": 1 }
        }"#;
        let config: DevContainerConfig = serde_json::from_str(input).expect("parse");
        assert_eq!(config.extra.len(), 2);
        assert!(config.extra.contains_key("unknownField"));
    }

    #[test]
    fn test_to_project_config_with_build() {
        let config: DevContainerConfig =
            serde_json::from_str(sample_build_devcontainer_json()).expect("parse");

        let mut mgr = DevContainerManager::new("/tmp/test");
        mgr.config = Some(config);

        let project_config = mgr.to_project_config().expect("convert");
        assert!(project_config.build.buildkit);
        assert_eq!(
            project_config.build.args.get("RUST_VERSION").map(String::as_str),
            Some("1.78")
        );
        assert_eq!(project_config.volumes.len(), 1); // only the volume mount
    }
}
