//! Container image registry client.
//!
//! Supports pulling images from OCI-compliant registries.

use crate::error::{CoreError, Result};
use crate::storage::{ImageConfig, ImageManifest};
use flate2::read::GzDecoder;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

/// Docker Hub registry URL.
pub const DOCKER_HUB_REGISTRY: &str = "https://registry-1.docker.io";
/// Docker Hub auth service.
pub const DOCKER_HUB_AUTH: &str = "https://auth.docker.io";

/// Image registry client.
pub struct ImageRegistry {
    /// HTTP client
    client: Client,
    /// Cache directory
    cache_dir: PathBuf,
    /// Authentication tokens
    tokens: HashMap<String, String>,
}

/// Token response from registry auth.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: String,
    #[allow(dead_code)]
    expires_in: Option<u64>,
}

impl ImageRegistry {
    /// Create a new registry client.
    pub fn new(cache_dir: impl Into<PathBuf>) -> Result<Self> {
        let client = Client::builder()
            .user_agent(format!("hyperbox/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        Ok(Self {
            client,
            cache_dir: cache_dir.into(),
            tokens: HashMap::new(),
        })
    }

    /// Parse an image reference.
    pub fn parse_ref(image: &str) -> (String, String, String) {
        let mut registry = DOCKER_HUB_REGISTRY.to_string();
        let mut name = image.to_string();
        let mut tag = "latest".to_string();

        // Check for explicit registry
        if image.contains('/')
            && (image.split('/').next().unwrap().contains('.')
                || image.split('/').next().unwrap().contains(':'))
        {
            let parts: Vec<&str> = image.splitn(2, '/').collect();
            registry = format!("https://{}", parts[0]);
            name = parts[1].to_string();
        }

        // Check for digest (@ takes precedence over :)
        if let Some(idx) = name.find('@') {
            tag = name[idx + 1..].to_string(); // sha256:abc123
            name = name[..idx].to_string(); // alpine
        } else if let Some(idx) = name.rfind(':') {
            // Check for tag (not port in registry hostname)
            if !name[idx..].contains('/') {
                tag = name[idx + 1..].to_string();
                name = name[..idx].to_string();
            }
        }

        // Add library/ prefix for Docker Hub official images
        if registry.contains("docker.io") && !name.contains('/') {
            name = format!("library/{name}");
        }

        (registry, name, tag)
    }

    /// Get authentication token for a registry.
    async fn get_token(&mut self, registry: &str, repo: &str) -> Result<String> {
        let cache_key = format!("{registry}/{repo}");

        if let Some(token) = self.tokens.get(&cache_key) {
            return Ok(token.clone());
        }

        // For Docker Hub, use auth service
        if registry.contains("docker.io") {
            let auth_url = format!(
                "{}/token?service=registry.docker.io&scope=repository:{}:pull",
                DOCKER_HUB_AUTH, repo
            );

            debug!("Fetching token from {}", auth_url);

            let response: TokenResponse = self
                .client
                .get(&auth_url)
                .send()
                .await
                .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?
                .json()
                .await
                .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

            self.tokens.insert(cache_key, response.token.clone());
            return Ok(response.token);
        }

        // For other registries, return empty token (anonymous)
        Ok(String::new())
    }

    /// Fetch image manifest.
    pub async fn get_manifest(
        &mut self,
        registry: &str,
        name: &str,
        reference: &str,
    ) -> Result<ImageManifest> {
        let token = self.get_token(registry, name).await?;

        let url = format!("{}/v2/{}/manifests/{}", registry, name, reference);

        info!("Fetching manifest from {}", url);

        let mut request = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.oci.image.manifest.v1+json")
            .header("Accept", "application/vnd.docker.distribution.manifest.v2+json");

        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let response = request
            .send()
            .await
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CoreError::ImageNotFound(format!("{}/{}", name, reference)));
        }

        let manifest: ImageManifest = response
            .json()
            .await
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        Ok(manifest)
    }

    /// Fetch image config.
    pub async fn get_config(
        &mut self,
        registry: &str,
        name: &str,
        config_digest: &str,
    ) -> Result<ImageConfig> {
        let token = self.get_token(registry, name).await?;

        let url = format!("{}/v2/{}/blobs/{}", registry, name, config_digest);

        debug!("Fetching config from {}", url);

        let mut request = self.client.get(&url);

        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let response = request
            .send()
            .await
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CoreError::StorageOperation(format!(
                "fetch config: Status {}",
                response.status()
            )));
        }

        let config: ImageConfig = response
            .json()
            .await
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        Ok(config)
    }

    /// Download a blob (layer).
    pub async fn download_blob(
        &mut self,
        registry: &str,
        name: &str,
        digest: &str,
        output: &Path,
    ) -> Result<u64> {
        let token = self.get_token(registry, name).await?;

        let url = format!("{}/v2/{}/blobs/{}", registry, name, digest);

        info!("Downloading blob {} to {:?}", digest, output);

        let mut request = self.client.get(&url);

        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let response = request
            .send()
            .await
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CoreError::StorageOperation(format!(
                "download blob: Status {}",
                response.status()
            )));
        }

        // Stream to file
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(output).await?;
        let bytes = response
            .bytes()
            .await
            .map_err(|e| CoreError::NetworkConfiguration(e.to_string()))?;

        file.write_all(&bytes).await?;

        Ok(bytes.len() as u64)
    }

    /// Pull a complete image.
    pub async fn pull(&mut self, image: &str) -> Result<PulledImage> {
        let (registry, name, tag) = Self::parse_ref(image);

        info!("Pulling image {} from {}", image, registry);

        // Get manifest
        let manifest = self.get_manifest(&registry, &name, &tag).await?;

        // Get config
        let config = self
            .get_config(&registry, &name, &manifest.config.digest)
            .await?;

        // Download layers
        let mut layer_paths = Vec::new();
        for layer in &manifest.layers {
            let layer_path = self.cache_dir.join("blobs").join(&layer.digest);

            if !layer_path.exists() {
                self.download_blob(&registry, &name, &layer.digest, &layer_path)
                    .await?;
            } else {
                debug!("Layer {} already cached", layer.digest);
            }

            layer_paths.push(layer_path);
        }

        info!("Successfully pulled {}", image);

        Ok(PulledImage {
            manifest,
            config,
            layer_paths,
        })
    }

    /// Extract image layers to a rootfs directory.
    pub async fn extract_to_rootfs(&self, pulled: &PulledImage, rootfs: &Path) -> Result<()> {
        fs::create_dir_all(rootfs).await?;

        info!("Extracting {} layers to {:?}", pulled.layer_paths.len(), rootfs);

        for (idx, layer_path) in pulled.layer_paths.iter().enumerate() {
            debug!("Extracting layer {}/{}: {:?}", idx + 1, pulled.layer_paths.len(), layer_path);

            // Read the layer blob
            let data = fs::read(layer_path).await?;

            // Decompress if gzipped
            let decompressed = if data.starts_with(&[0x1f, 0x8b]) {
                // GZip magic bytes
                let mut decoder = GzDecoder::new(&data[..]);
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(|e| CoreError::StorageOperation(format!("decompress layer: {}", e)))?;
                decompressed
            } else {
                data
            };

            // Extract tar archive
            let mut archive = Archive::new(&decompressed[..]);

            archive.unpack(rootfs).map_err(|e| {
                CoreError::StorageOperation(format!("extract tar layer {}: {}", idx, e))
            })?;

            debug!("Extracted layer {}/{}", idx + 1, pulled.layer_paths.len());
        }

        info!("Successfully extracted image to {:?}", rootfs);
        Ok(())
    }
}

/// Result of pulling an image.
#[derive(Debug)]
pub struct PulledImage {
    /// Image manifest
    pub manifest: ImageManifest,
    /// Image config
    pub config: ImageConfig,
    /// Paths to downloaded layers
    pub layer_paths: Vec<PathBuf>,
}
