# HyperBox v0.1.0 Build Guide

## Quick Build (Current Platform)

```bash
cargo build --release --bins
# Binaries will be in target/release/
```

## Multi-Platform Builds

### Windows x86_64 (MSVC)

```bash
# Install Visual Studio Build Tools or use pre-installed MSVC
cargo build --release --bins
```

**Output:**

- `target/release/hb.exe` - CLI tool
- `target/release/hyperboxd.exe` - Daemon

### Linux x86_64 (GNU)

Requires: Rust, pkg-config, libssl-dev, build-essential

```bash
cargo build --release --bins --target x86_64-unknown-linux-gnu
```

**Output:**

- `target/x86_64-unknown-linux-gnu/release/hb`
- `target/x86_64-unknown-linux-gnu/release/hyperboxd`

### Linux ARM64 (aarch64)

Requires: Cross-compilation toolchain

```bash
# Install cross (recommended)
cargo install cross

cross build --release --bins --target aarch64-unknown-linux-gnu
```

**Output:**

- `target/aarch64-unknown-linux-gnu/release/hb`
- `target/aarch64-unknown-linux-gnu/release/hyperboxd`

### macOS x86_64

Requires: macOS, Xcode Command Line Tools

```bash
# On macOS with M1/M2+, build for Intel:
cargo build --release --bins --target x86_64-apple-darwin
```

**Output:**

- `target/x86_64-apple-darwin/release/hb`
- `target/x86_64-apple-darwin/release/hyperboxd`

### macOS ARM64 (Apple Silicon)

Requires: macOS on Apple Silicon (M1/M2/M3+)

```bash
cargo build --release --bins --target aarch64-apple-darwin
```

**Output:**

- `target/aarch64-apple-darwin/release/hb`
- `target/aarch64-apple-darwin/release/hyperboxd`

## Docker Image Build

### Local Build

```bash
docker build -t hyperbox:0.1.0 .
docker tag hyperbox:0.1.0 ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0
docker tag hyperbox:0.1.0 ghcr.io/iamthegreatdestroyer/hyperbox:latest
```

### Push to Registry

```bash
# Requires: docker login to ghcr.io
docker push ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0
docker push ghcr.io/iamthegreatdestroyer/hyperbox:latest
```

### Multi-Platform with Buildx

```bash
docker buildx create --name hyperbox-builder
docker buildx use hyperbox-builder

docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0 \
  -t ghcr.io/iamthegreatdestroyer/hyperbox:latest \
  --push .
```

## Creating Artifacts

### Create Source Archives

```bash
# Source tar.gz
tar --exclude='target' --exclude='.git' --exclude='build-artifacts' \
    -czf hyperbox-0.1.0-source.tar.gz .

# Source zip
7z a -r hyperbox-0.1.0-source.zip . -x !target -x !.git -x !build-artifacts
```

### Generate SHA256 Checksums

```bash
# Linux/macOS
sha256sum hyperbox-*.tar.gz hyperbox-*.exe hyperbox-* > SHA256SUMS

# PowerShell (Windows)
Get-ChildItem hyperbox-* | ForEach-Object {
    Write-Output "$((Get-FileHash $_ -Algorithm SHA256).Hash)  $_"
} | Out-File -FilePath SHA256SUMS -Encoding ASCII
```

## Automated CI/CD Build

For true multi-platform builds, use GitHub Actions (see `.github/workflows/build.yml`):

```yaml
name: Build Release Artifacts

on:
    push:
        tags:
            - "v*"

jobs:
    build-matrix:
        strategy:
            matrix:
                include:
                    - target: x86_64-unknown-linux-gnu
                      os: ubuntu-latest
                    - target: aarch64-unknown-linux-gnu
                      os: ubuntu-latest
                    - target: x86_64-apple-darwin
                      os: macos-latest
                    - target: aarch64-apple-darwin
                      os: macos-latest
                    - target: x86_64-pc-windows-msvc
                      os: windows-latest
        # ... CI job definition
```

## Version Information

- **Version:** 0.1.0 (Alpha)
- **Edition:** Rust 2021
- **License:** MIT OR Apache-2.0

## Build Validation

After building, verify binaries:

```bash
# Windows
./target/release/hb.exe --version
./target/release/hyperboxd.exe --version

# Linux/macOS
./target/release/hb --version
./target/release/hyperboxd --version

# Help command
./target/release/hb --help
./target/release/hb health
```

## Troubleshooting

### Build fails with linker errors

**Windows:** Ensure Visual Studio Build Tools with MSVC 14.0+ installed

**Linux:** Install build-essential:

```bash
sudo apt-get install build-essential pkg-config libssl-dev
```

**macOS:** Install Xcode Command Line Tools:

```bash
xcode-select --install
```

### Cross-compilation issues

Use the `cross` tool for reliable cross-compilation:

```bash
cargo install cross
cross build --release --bins --target <target-triple>
```

### Docker build fails

Ensure Docker daemon is running and sufficient disk space available:

```bash
docker system df
docker system prune -a
```
