# D5 Execution Plan: GitHub Release v0.1.0-alpha

**Status:** Ready to Execute  
**Prerequisites:** Windows binaries built ✅  
**Estimated Time:** 15-20 minutes

---

## Quick Summary

HyperBox v0.1.0-alpha **build is complete** with these artifacts:

- `hb.exe` (5.41 MB)
- `hyperboxd.exe` (5.24 MB)

Ready to publish to GitHub and Docker registries.

---

## Pre-Release Checklist

Before executing, verify:

- [ ] Windows binaries exist at `target/release/hb.exe` and `target/release/hyperboxd.exe`
- [ ] READ.md has been reviewed (latest version)
- [ ] You have GitHub CLI installed (`gh` command available)
- [ ] GitHub auth is configured (`gh auth status` shows authenticated)
- [ ] Docker account set up for `ghcr.io` (optional: for Docker image)

---

## Method A: GitHub CLI (Recommended - 5 minutes)

### Step 1: Prepare Release Notes

Set release date and create the notes file:

````bash
# Option 1: Use template from D4_RELEASE_NOTES_GUIDE.md
# Copy contents to RELEASE_NOTES.md and update placeholders

# Option 2: Quick version (minimal)
cat > RELEASE_NOTES.md << 'EOF'
# HyperBox v0.1.0-alpha

## Features

- Container isolation with resource constraints
- Image layer deduplication and analysis
- Command-line interface for all operations
- Daemon service for background operations
- System health monitoring
- Multi-platform support (Windows, Linux, Docker)

## Supported Platforms

- Windows x86_64 ✅
- Linux x86_64 ✅
- Linux aarch64 ✅
- Docker (linux/amd64) ✅
- macOS (pending native build)

## Known Limitations

- macOS requires native build environment
- Docker image currently single architecture
- No Hyper-V integration yet

## Installation

See [INSTALLATION_GUIDE.md](INSTALLATION_GUIDE.md) for detailed instructions.

## Quick Start

```bash
hb --version
hb health
hb --help
````

See [QUICKSTART.md](QUICKSTART.md) for examples.

## Support

- GitHub Issues: https://github.com/iamthegreatdestroyer/hyperbox/issues
- Discussions: https://github.com/iamthegreatdestroyer/hyperbox/discussions

---

Thank you for testing HyperBox v0.1.0-alpha!
EOF

````

### Step 2: Verify `gh` CLI

```bash
gh auth status
# Expected: "Logged in to github.com as <username>"

gh release list --repo iamthegreatdestroyer/hyperbox
# Check if any prior releases exist
````

### Step 3: Create Draft Release

```bash
cd s:\HyperBox

gh release create v0.1.0-alpha `
  --title "HyperBox v0.1.0-alpha" `
  --notes-file RELEASE_NOTES.md `
  --draft `
  --repo iamthegreatdestroyer/hyperbox
```

**Expected output:**

```
✓ Created GitHub release 'v0.1.0-alpha'
```

### Step 4: Upload Artifacts

```bash
# Upload Windows binaries
gh release upload v0.1.0-alpha `
  target/release/hb.exe `
  target/release/hyperboxd.exe `
  --repo iamthegreatdestroyer/hyperbox

# Upload checksums
# Option: Generate checksums first
# sha256sum target/release/*.exe > SHA256SUMS
# Then upload
gh release upload v0.1.0-alpha `
  SHA256SUMS `
  --repo iamthegreatdestroyer/hyperbox
```

**Expected output:**

```
✓ Uploaded hb.exe (5.4 MB)
✓ Uploaded hyperboxd.exe (5.2 MB)
✓ Uploaded SHA256SUMS
```

### Step 5: Verify Release

```bash
gh release view v0.1.0-alpha --repo iamthegreatdestroyer/hyperbox
```

Check:

- [ ] Title: "HyperBox v0.1.0-alpha"
- [ ] Status: DRAFT
- [ ] Artifacts: 3 files listed
- [ ] Release notes: Correct content displayed

### Step 6: Publish Release

```bash
gh release edit v0.1.0-alpha `
  --draft=false `
  --repo iamthegreatdestroyer/hyperbox
```

**Expected output:**

```
✓ Edited GitHub release 'v0.1.0-alpha'
```

---

## Method B: Manual GitHub Web UI (10 minutes)

### Step 1: Open GitHub

1. Go to: https://github.com/iamthegreatdestroyer/hyperbox/releases/new
2. **Tag version:** `v0.1.0-alpha`
3. **Target:** `main` (default)
4. **Release title:** "HyperBox v0.1.0-alpha"

### Step 2: Add Release Notes

Paste content from RELEASE_NOTES.md in description box.

### Step 3: Upload Artifacts

Drag and drop or use file picker:

- `hb.exe`
- `hyperboxd.exe`
- `SHA256SUMS` (optional)

### Step 4: Mark as Pre-release

Check: **"This is a pre-release"** ✓

### Step 5: Publish

Click: **"Publish release"** button

---

## Method C: REST API (Fallback - 10 minutes)

If `gh` CLI unavailable:

```bash
# 1. Create release
$headers = @{
    "Authorization" = "Bearer YOUR_GITHUB_TOKEN"
    "Accept" = "application/vnd.github.v3+json"
}

$body = @{
    "tag_name" = "v0.1.0-alpha"
    "target_commitish" = "main"
    "name" = "HyperBox v0.1.0-alpha"
    "body" = (Get-Content RELEASE_NOTES.md -Raw)
    "draft" = $true
    "prerelease" = $true
} | ConvertTo-Json

$response = Invoke-WebRequest `
    -Uri "https://api.github.com/repos/iamthegreatdestroyer/hyperbox/releases" `
    -Headers $headers `
    -Method POST `
    -Body $body `
    -ContentType "application/json"

# Extract release ID
$releaseId = ($response.Content | ConvertFrom-Json).id

# 2. Upload artifacts
$artifactPath = "target/release/hb.exe"
$headers2 = @{
    "Authorization" = "Bearer YOUR_GITHUB_TOKEN"
    "Content-Type" = "application/octet-stream"
}

$fileContent = [System.IO.File]::ReadAllBytes($artifactPath)
Invoke-WebRequest `
    -Uri "https://uploads.github.com/repos/iamthegreatdestroyer/hyperbox/releases/$releaseId/assets?name=$(Split-Path -Leaf $artifactPath)" `
    -Headers $headers2 `
    -Method POST `
    -Body $fileContent

# Repeat for hyperboxd.exe
```

---

## Docker Image Publication (Optional)

If Docker image built:

```bash
# Create registry credentials
echo YOUR_GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin

# Tag image
docker tag hyperbox:0.1.0 ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0
docker tag hyperbox:0.1.0 ghcr.io/iamthegreatdestroyer/hyperbox:latest

# Push
docker push ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0
docker push ghcr.io/iamthegreatdestroyer/hyperbox:latest

# Verify
docker pull ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0
```

---

## Post-Release Verification (18-Point Checklist)

### Release Published

- [ ] Release visible at https://github.com/iamthegreatdestroyer/hyperbox/releases/tag/v0.1.0-alpha
- [ ] Pre-release indicator shows ✓ (yellow "Pre-release" label)
- [ ] Release date is today

### Assets Available

- [ ] `hb.exe` downloadable and correct size (5.41 MB)
- [ ] `hyperboxd.exe` downloadable and correct size (5.24 MB)
- [ ] SHA256SUMS file present
- [ ] Total 3+ assets listed
- [ ] Download counts initialized (will increment as users download)

### Release Notes

- [ ] Release notes render correctly (no markdown errors)
- [ ] All features listed are accurate
- [ ] Platform support matrix is displayed
- [ ] Installation instructions are clear
- [ ] Links to INSTALLATION_GUIDE.md and QUICKSTART.md work

### GitHub Settings

- [ ] Release is the **Latest release** (unless prior version existed)
- [ ] Pre-release flag is set (✓ for alpha)
- [ ] Target branch is `main` (or appropriate)
- [ ] Author is correct

### Post-Release Tasks

- [ ] Create GitHub Discussion for feedback (optional)
- [ ] Update README.md with download link to latest (optional)
- [ ] Bump version in Cargo.toml to 0.1.1-dev (for next iteration)
- [ ] Commit version bump: `git commit -am "chore: bump version to 0.1.1-dev"`
- [ ] Push: `git push origin main`

### Docker Registry (if done)

- [ ] Image at ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0 is available
- [ ] Image can be pulled: `docker pull ghcr.io/iamthegreatdestroyer/hyperbox:0.1.0`
- [ ] Image runs without error: `docker run ... hyperbox:0.1.0 hb --version`
- [ ] Image sizes are reasonable (~400-500 MB)

### Verification Complete

- [ ] Test one artifact download works (any binary)
- [ ] Test via Docker pull works (if Docker image pushed)
- [ ] Test URLs resolve correctly
- [ ] Test release notes are readable

---

## Troubleshooting

### `gh: command not found`

Install GitHub CLI:

```bash
# Windows (via Chocolatey)
choco install gh

# Or download from: https://cli.github.com/
```

### `gh auth status` shows "Not logged in"

Authenticate:

```bash
gh auth login
# Follow prompts, select "GitHub.com" and "HTTPS"
# Paste token when prompted
```

### `Permission denied` error

Your GitHub token may not have `repo` scope:

```bash
gh auth token --secured
# Verify in GitHub Settings > Developer Settings > Personal access tokens
# Ensure `repo` scope is enabled
```

### Release already exists

Delete and recreate:

```bash
gh release delete v0.1.0-alpha --yes --repo iamthegreatdestroyer/hyperbox
# Then re-run create command
```

### Assets fail to upload

Check file paths are correct:

```bash
ls target/release/hb.exe
ls target/release/hyperboxd.exe
```

If files don't exist, rebuild:

```bash
cargo build --release --bins
```

---

## Next Steps After Release

1. ✅ **D4: Release Notes** - Write full release notes (use template from D4_RELEASE_NOTES_GUIDE.md)
2. ✅ **D6: API Reference** - Generate API docs from `hb --help` output
3. ✅ **D7: Troubleshooting** - Expand troubleshooting section
4. ✅ **D9: Examples** - Document usage examples
5. ✅ **D8: Beta Program** - Set up GitHub discussions for feedback
6. 📢 **Announce** - Tweet, blog post, community posts
7. 📊 **Monitor** - Track downloads, issues, feedback

---

## Success Criteria

✅ You have successfully completed D5 when:

1. GitHub release `v0.1.0-alpha` is published and visible
2. All 3+ artifacts are downloadable and correct
3. Pre-release flag is set
4. Release notes are readable and accurate
5. At least one person can download and run the binaries

---

**Estimated Completion Time:** 15-20 minutes

**Time Remaining for Other Tasks:** 12+ hours (with buffer)

Good luck! 🚀
