# D5: GitHub Release Setup Guide

## Overview

This guide walks through creating the v0.1.0-alpha release on GitHub with all artifacts.

## Prerequisites

- GitHub CLI (`gh`) installed and authenticated
- Build artifacts ready in `build-artifacts/v0.1.0/`
- GitHub account with push access to repository
- Docker image pushed to ghcr.io (optional for v0.1.0-alpha)

## Step 1: Prepare Release Information

### Release Notes Template

````markdown
# HyperBox v0.1.0-alpha

**Initial Alpha Release** - Containerized process isolation and workload optimization

## What's New

### Core Features

- **Container Isolation**: Isolate processes and resources using crun/runc
- **Layer Deduplication**: Reduce image sizes by detecting duplicate blocks
- **Daemon Socket**: Background service for long-running operations
- **CLI Tools**: Comprehensive command-line interface for all operations
- **Health Checks**: Monitor daemon and system dependency health

### CLI Commands

- `hb project create` - Create new HyperBox project
- `hb container isolate` - Create isolated environments
- `hb image analyze` - Analyze container images
- `hb system info` - View system information
- `hb health` - Check daemon health

### Platforms

- ✅ Windows x86_64
- ✅ Linux x86_64 (GNU)
- ✅ Linux aarch64 (ARM64)
- ⚠️ macOS (not yet built, pending Xcode setup)
- 🐳 Docker image available: `ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0`

## Known Limitations

- macOS binary builds require macOS host (Apple silicon support pending)
- Docker image currently linux/amd64 only (multi-arch planned for v0.1.1)
- No Windows Hyper-V integration yet (planned)
- No Kubernetes integration (planned)

## Installation

See [INSTALLATION_GUIDE.md](INSTALLATION_GUIDE.md)

## Quick Start

```bash
# Extract archive or use installer
hb --version
hb health

# Create a project
hb project create my-project
```
````

See [QUICKSTART.md](QUICKSTART.md) for more examples.

## Contributors

- Built with Rust and Tokio
- Thanks to the open-source community

## SHA256 Checksums

See `checksums/SHA256SUMS` for artifact verification.

````

## Step 2: Create Release via GitHub CLI

### Option A: Using gh CLI (Recommended)

```bash
# Navigate to repo
cd s:\HyperBox

# Create draft release
gh release create v0.1.0-alpha `
  --title "HyperBox v0.1.0-alpha" `
  --draft `
  --notes-file RELEASE_NOTES.md

# Upload artifacts
gh release upload v0.1.0-alpha `
  build-artifacts/v0.1.0/windows-x86_64/hb.exe `
  build-artifacts/v0.1.0/windows-x86_64/hyperboxd.exe `
  build-artifacts/v0.1.0/linux-x86_64/hb `
  build-artifacts/v0.1.0/linux-x86_64/hyperboxd `
  build-artifacts/v0.1.0/checksums/SHA256SUMS

# Publish release
gh release edit v0.1.0-alpha --draft=false
````

### Option B: Using REST API

```bash
# Get GitHub token
$token = $env:GITHUB_TOKEN

# Create release
$headers = @{
    Authorization = "Bearer $token"
    Accept = "application/vnd.github+json"
    "X-GitHub-Api-Version" = "2022-11-28"
}

$body = @{
    tag_name = "v0.1.0-alpha"
    name = "HyperBox v0.1.0-alpha"
    body = Get-Content RELEASE_NOTES.md -Raw
    draft = $true
    prerelease = $true
} | ConvertTo-Json

$release = Invoke-RestMethod `
    -Uri "https://api.github.com/repos/iamthegreatdestroyer/hyperbox/releases" `
    -Method POST `
    -Headers $headers `
    -Body $body

# Upload each artifact
$artifacts = @(
    "build-artifacts/v0.1.0/windows-x86_64/hb.exe",
    "build-artifacts/v0.1.0/windows-x86_64/hyperboxd.exe",
    "build-artifacts/v0.1.0/linux-x86_64/hb",
    "build-artifacts/v0.1.0/linux-x86_64/hyperboxd",
    "build-artifacts/v0.1.0/checksums/SHA256SUMS"
)

foreach ($artifact in $artifacts) {
    $filename = Split-Path -Leaf $artifact
    $fileContent = [System.IO.File]::ReadAllBytes($artifact)

    Invoke-RestMethod `
        -Uri "$($release.upload_url -replace '{?name,label}', "?name=$filename")" `
        -Method POST `
        -Headers @{
            Authorization = "Bearer $token"
            "Content-Type" = "application/octet-stream"
        } `
        -Body $fileContent
}
```

## Step 3: Configure Release Settings

### Page Layout

The release page should include:

1. **Title & Description**
    - "HyperBox v0.1.0-alpha: Initial Release"
    - Highlight key features

2. **Download Section**
    - Windows (hb.exe, hyperboxd.exe)
    - Linux x86_64 (hb, hyperboxd)
    - Docker image reference

3. **Documentation Links**
    - Installation Guide
    - Quick Start
    - [API Reference](API_REFERENCE.md)

4. **Checksums**
    - SHA256SUMS file
    - Instructions for verification

### Release Settings

```yaml
Release Type: Pre-release (alpha)
Tag: v0.1.0-alpha
Target: main branch
Draft: No (release publicly)
Latest Release: Yes (if no other releases)
```

## Step 4: Post-Release Tasks

### Create Release Discussion

Create a GitHub Discussion for feedback:

```
Title: HyperBox v0.1.0-alpha Feedback & Testing
Category: Announcements
Body:
This is the initial alpha release. Please:
1. Download and test on your platform
2. Report bugs via Issues
3. Share feedback in comments
4. Help us improve!
```

### Update Repository

Update main branch:

```bash
# Update version in Cargo.toml
sed -i 's/version = "0.1.0"/version = "0.1.1-dev"/' Cargo.toml

# Commit version bump
git add Cargo.toml
git commit -m "chore: bump version to 0.1.1-dev"
git push origin main
```

### Announce Release

- [ ] Post on GitHub Releases page
- [ ] Create GitHub Discussion
- [ ] Update README.md with latest version
- [ ] Tag in Discord/community channels

## Step 5: Docker Image Release

### Push to GitHub Container Registry

```bash
# Authenticate
echo $env:CR_PAT | docker login ghcr.io -u USERNAME --password-stdin

# Tag image
docker tag hyperbox:0.1.0 ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0
docker tag hyperbox:0.1.0 ghcr.io/iamthegreatdestroyer/hyperbox:latest

# Push
docker push ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0
docker push ghcr.io/iamthegreatdestroyer/hyperbox:latest
```

### Document Image Usage

Create `DOCKER_USAGE.md`:

```markdown
# HyperBox Docker Usage

## Pull Image

\`\`\`bash
docker pull ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0
\`\`\`

## Run Container

\`\`\`bash

# Interactive shell

docker run -it ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0

# Run command

docker run --rm ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0 hb --version

# Check health

docker run --rm ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0 hb health
\`\`\`

## Health Check

The image includes a built-in health check that runs \`hb health\` command.

Monitor with:
\`\`\`bash
docker inspect --format='{{.State.Health.Status}}' <container-id>
\`\`\`
```

## Step 6: Release Verification Checklist

- [ ] GitHub release created with correct tag (v0.1.0-alpha)
- [ ] All binaries uploaded:
    - [ ] Windows hb.exe
    - [ ] Windows hyperboxd.exe
    - [ ] Linux x86_64 hb
    - [ ] Linux x86_64 hyperboxd
    - [ ] Linux aarch64 binaries (if built)
- [ ] SHA256SUMS file attached
- [ ] Release notes include:
    - [ ] Feature list
    - [ ] Known limitations
    - [ ] Installation instructions
    - [ ] Platform support matrix
- [ ] Docker image pushed to ghcr.io
- [ ] Release marked as pre-release
- [ ] Discussion created for feedback
- [ ] README.md updated with download links

## Troubleshooting

### gh CLI Authentication

```bash
# Login to GitHub
gh auth login

# Verify authentication
gh auth status
```

### Failed Upload

```bash
# Retry failed artifacts
gh release upload v0.1.0-alpha \
  build-artifacts/v0.1.0/windows-x86_64/hb.exe \
  --clobber
```

### Release Already Exists

```bash
# Delete draft release and recreate
gh release delete v0.1.0-alpha --yes
# Then recreate with correct information
```

## Next Steps

After release:

- [ ] Complete D4 (Release Notes)
- [ ] Complete D6 (API Reference)
- [ ] Complete D7 (Troubleshooting Guide)
- [ ] Complete D8 (Beta Program Setup)
- [ ] Complete D9 (Examples)

## Files to Reference

- [INSTALLATION_GUIDE.md](INSTALLATION_GUIDE.md) - Installation instructions
- [QUICKSTART.md](QUICKSTART.md) - Quick start guide
- [BUILD_GUIDE.md](BUILD_GUIDE.md) - Build instructions
- [API_REFERENCE.md](API_REFERENCE.md) - API documentation (pending D6)
- [RELEASE_NOTES.md](RELEASE_NOTES.md) - Form release notes (pending D4)
