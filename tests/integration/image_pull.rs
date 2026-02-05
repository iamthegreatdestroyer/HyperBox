//! Tests for image pulling functionality.

use hyperbox_core::storage::registry::ImageRegistry;
use tempfile::TempDir;

#[tokio::test]
#[ignore = "requires network access and docker registry"]
async fn test_pull_alpine_image() {
    // Create temp directory for cache
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");

    // Create registry client
    let mut registry = ImageRegistry::new(&cache_dir).unwrap();

    // Pull alpine:latest
    let pulled = registry.pull("alpine:latest").await;

    assert!(pulled.is_ok(), "Failed to pull alpine:latest: {:?}", pulled.err());

    let pulled = pulled.unwrap();

    // Check that we have layers
    assert!(!pulled.layer_paths.is_empty(), "No layers downloaded");

    // Check that layer files exist
    for layer_path in &pulled.layer_paths {
        assert!(layer_path.exists(), "Layer file missing: {:?}", layer_path);
    }

    println!("Successfully pulled alpine:latest with {} layers", pulled.layer_paths.len());
}

#[tokio::test]
#[ignore = "requires network access and docker registry"]
async fn test_pull_and_extract_alpine() {
    // Create temp directories
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    let rootfs_dir = temp_dir.path().join("rootfs");

    // Create registry client
    let mut registry = ImageRegistry::new(&cache_dir).unwrap();

    // Pull alpine:latest
    let pulled = registry.pull("alpine:latest").await.unwrap();

    // Extract to rootfs
    let result = registry.extract_to_rootfs(&pulled, &rootfs_dir).await;

    assert!(result.is_ok(), "Failed to extract rootfs: {:?}", result.err());

    // Check that rootfs has expected directories
    assert!(rootfs_dir.join("bin").exists(), "Missing /bin directory");
    assert!(rootfs_dir.join("etc").exists(), "Missing /etc directory");
    assert!(
        rootfs_dir.join("lib").exists() || rootfs_dir.join("lib64").exists(),
        "Missing /lib directory"
    );

    // Check for alpine-specific files
    assert!(rootfs_dir.join("etc/alpine-release").exists(), "Missing alpine-release file");

    println!("Successfully extracted alpine:latest to {:?}", rootfs_dir);
}

#[tokio::test]
async fn test_parse_image_reference() {
    // Test Docker Hub official image
    let (registry, name, tag) = ImageRegistry::parse_ref("alpine");
    assert_eq!(registry, "https://registry-1.docker.io");
    assert_eq!(name, "library/alpine");
    assert_eq!(tag, "latest");

    // Test Docker Hub user image
    let (registry, name, tag) = ImageRegistry::parse_ref("nginx:1.21");
    assert_eq!(registry, "https://registry-1.docker.io");
    assert_eq!(name, "library/nginx");
    assert_eq!(tag, "1.21");

    // Test explicit registry
    let (registry, name, tag) = ImageRegistry::parse_ref("gcr.io/project/image:v1.0");
    assert_eq!(registry, "https://gcr.io");
    assert_eq!(name, "project/image");
    assert_eq!(tag, "v1.0");

    // Test with digest
    let (registry, name, tag) = ImageRegistry::parse_ref("alpine@sha256:abc123");
    assert_eq!(registry, "https://registry-1.docker.io");
    assert_eq!(name, "library/alpine");
    assert_eq!(tag, "sha256:abc123");
}

#[tokio::test]
async fn test_cache_directory_creation() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");

    let registry = ImageRegistry::new(&cache_dir);
    assert!(registry.is_ok());

    // Cache dir is created when actually pulling, not in constructor
    // Just verify the registry was created successfully
}
