# HyperBox v0.1.0 Release Build Automation Script
# Builds multi-platform artifacts and creates release package

param(
    [string]$Version = "0.1.0",
    [string]$OutputDir = "build-artifacts",
    [switch]$BuildWindows = $true,
    [switch]$BuildLinux = $false,
    [switch]$BuildMacOS = $false,
    [switch]$BuildDocker = $true,
    [switch]$SkipTests = $false,
    [switch]$DryRun = $false
)

# Color output helper
function Write-ColorOutput {
    param(
        [string]$Message,
        [ConsoleColor]$Color = [ConsoleColor]::White
    )
    Write-Host $Message -ForegroundColor $Color
}

function Write-Header {
    param([string]$Message)
    Write-ColorOutput "`n========================================" Green
    Write-ColorOutput $Message Green
    Write-ColorOutput "========================================`n" Green
}

function Write-Success {
    param([string]$Message)
    Write-ColorOutput "✓ $Message" Green
}

function Write-Error {
    param([string]$Message)
    Write-ColorOutput "✗ $Message" Red
}

function Write-Info {
    param([string]$Message)
    Write-ColorOutput "ℹ $Message" Cyan
}

# Main execution
Write-Header "HyperBox v${Version} Release Builder"

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$ArtifactRoot = Join-Path $RootDir $OutputDir $Version

Write-Info "Root Directory: $RootDir"
Write-Info "Artifact Directory: $ArtifactRoot"

# Create artifact directories
Write-Info "Creating artifact directories..."
@(
    "windows-x86_64",
    "linux-x86_64",
    "linux-aarch64",
    "macos-x86_64",
    "macos-aarch64",
    "docker",
    "checksums"
) | ForEach-Object {
    $dir = Join-Path $ArtifactRoot $_
    if (-not (Test-Path $dir)) {
        New-Item -Path $dir -ItemType Directory -Force | Out-Null
        Write-Success "Created: $dir"
    }
}

# Test phase
if (-not $SkipTests) {
    Write-Header "Running Tests"
    Write-Info "Running: cargo test --workspace"
    if (-not $DryRun) {
        Push-Location $RootDir
        cargo test --workspace 2>&1 | Tee-Object -FilePath (Join-Path $ArtifactRoot "test-results.log")
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Tests failed! Check test-results.log"
            Pop-Location
            exit 1
        }
        Pop-Location
        Write-Success "All tests passed"
    }
    else {
        Write-Info "[DRY RUN] Would run tests"
    }
}

# Build Windows binaries
if ($BuildWindows) {
    Write-Header "Building Windows x86_64 Binaries"
    Write-Info "Target: x86_64-pc-windows-msvc"

    if (-not $DryRun) {
        Push-Location $RootDir
        cargo build --release --bins 2>&1 | Tee-Object -FilePath (Join-Path $ArtifactRoot "build-windows.log")
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Windows build failed! Check build-windows.log"
            Pop-Location
            exit 1
        }
        Pop-Location

        # Copy artifacts
        $windowsDir = Join-Path $ArtifactRoot "windows-x86_64"
        Copy-Item -Path "$RootDir\target\release\hb.exe" -Destination $windowsDir -Force
        Copy-Item -Path "$RootDir\target\release\hyperboxd.exe" -Destination $windowsDir -Force

        Write-Success "Windows binaries built and copied"
        Get-ChildItem $windowsDir | ForEach-Object { Write-Info "  - $($_.Name) ($('{0:N2}' -f ($_.Length / 1MB)) MB)" }
    }
    else {
        Write-Info "[DRY RUN] Would build Windows x86_64"
    }
}

# Build Linux binaries (requires cross)
if ($BuildLinux) {
    Write-Header "Building Linux Binaries"

    # Check for cross tool
    if (-not (Get-Command cross -ErrorAction SilentlyContinue)) {
        Write-Info "Installing 'cross' tool for cross-compilation..."
        if (-not $DryRun) {
            cargo install cross
        }
    }

    $linuxTargets = @(
        @{ Target = "x86_64-unknown-linux-gnu"; Dir = "linux-x86_64" },
        @{ Target = "aarch64-unknown-linux-gnu"; Dir = "linux-aarch64" }
    )

    foreach ($target in $linuxTargets) {
        Write-Info "Building for: $($target.Target)"

        if (-not $DryRun) {
            Push-Location $RootDir
            cross build --release --bins --target $target.Target 2>&1 | Tee-Object -FilePath (Join-Path $ArtifactRoot "build-$($target.Dir).log")
            if ($LASTEXITCODE -ne 0) {
                Write-Error "Linux $($target.Target) build failed!"
                Pop-Location
                continue
            }
            Pop-Location

            # Copy artifacts
            $artifactDir = Join-Path $ArtifactRoot $target.Dir
            Copy-Item -Path "$RootDir\target\$($target.Target)\release\hb" -Destination $artifactDir -Force
            Copy-Item -Path "$RootDir\target\$($target.Target)\release\hyperboxd" -Destination $artifactDir -Force

            Write-Success "Linux $($target.Target) binaries built"
        }
        else {
            Write-Info "[DRY RUN] Would build $($target.Target)"
        }
    }
}

# Build Docker image
if ($BuildDocker) {
    Write-Header "Building Docker Image"

    if (-not $DryRun) {
        Push-Location $RootDir

        # Build image
        Write-Info "Building docker image: hyperbox:${Version}"
        docker build -t "hyperbox:${Version}" -t "ghcr.io/iamthegreatdestroyer/hyperbox:${Version}" . 2>&1 | Tee-Object -FilePath (Join-Path $ArtifactRoot "docker-build.log")

        if ($LASTEXITCODE -ne 0) {
            Write-Error "Docker build failed!"
            Pop-Location
            continue
        }

        # Get image info
        $imageInfo = docker images --filter "reference=hyperbox:${Version}" --format "{{.Repository}} {{.Size}}" 2>&1
        Write-Success "Docker image built: $imageInfo"

        # Save image reference
        @"
# HyperBox Docker Image - v${Version}

## Image Information
- Registry: ghcr.io/iamthegreatdestroyer/hyperbox
- Tags: ${Version}, latest
- Architecture: linux/amd64 (initial)

## Build Date
$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss UTC')

## Usage
\`\`\`bash
docker pull ghcr.io/iamthegreatdestroyer/hyperbox:${Version}
docker run --rm ghcr.io/iamthegreatdestroyer/hyperbox:${Version} hb --version
\`\`\`

## Health Check
\`\`\`bash
docker run --rm ghcr.io/iamthegreatdestroyer/hyperbox:${Version} hb health
\`\`\`
"@ | Out-File -FilePath (Join-Path $ArtifactRoot "docker" "IMAGE_INFO.md")

        Pop-Location
    }
    else {
        Write-Info "[DRY RUN] Would build Docker image"
    }
}

# Generate checksums
Write-Header "Generating SHA256 Checksums"

$checksumFile = Join-Path $ArtifactRoot "checksums" "SHA256SUMS"
$checksumContent = @()

Get-ChildItem -Path $ArtifactRoot -Recurse -File | Where-Object {
    $_.Extension -in @(".exe", ".log", ".md") -or
    ($_.Name -in @("hb", "hyperboxd"))
} | ForEach-Object {
    $hash = (Get-FileHash -Path $_.FullName -Algorithm SHA256).Hash
    $relativePath = $_.FullName.Replace("$ArtifactRoot\", "")
    $checksumContent += "$hash  $relativePath"
    Write-Info "  $($_.Name): $($hash.Substring(0, 16))..."
}

if (-not $DryRun) {
    $checksumContent | Out-File -FilePath $checksumFile -Encoding ASCII
    Write-Success "Checksums written to: $checksumFile"
}
else {
    Write-Info "[DRY RUN] Would write checksums"
}

# Create release summary
Write-Header "Release Summary"

$summary = @"
# HyperBox v${Version} Release

## Build Date
$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss UTC')

## Artifacts

### Binaries
"@

# Count built binaries
$binCount = 0
Get-ChildItem -Path $ArtifactRoot -Recurse -File | Where-Object {
    $_.Name -in @("hb.exe", "hyperboxd.exe", "hb", "hyperboxd")
} | ForEach-Object {
    $binCount++
    $size = '{0:N2}' -f ($_.Length / 1MB)
    $summary += "`n- $($_.Directory.Name)/$($_.Name) ($size MB)"
}

$summary += @"


### Docker Image
- ghcr.io/iamthegreatdestroyer/hyperbox:${Version}
- ghcr.io/iamthegreatdestroyer/hyperbox:latest

### Documentation
- BUILD_GUIDE.md: Detailed build instructions
- INSTALLATION_GUIDE.md: Installation instructions
- QUICKSTART.md: Quick start guide
- API_REFERENCE.md: API and CLI reference

## Statistics
- Binaries Built: $binCount
- Platforms: $(if ($BuildWindows) { "Windows " })$(if ($BuildLinux) { "Linux " })$(if ($BuildMacOS) { "macOS " })
- Docker Image: $(if ($BuildDocker) { "Yes" } else { "No" })
- Build Time: $(Get-Date -Format 'HH:mm:ss')

## Next Steps
1. Review checksums: $(Test-Path $checksumFile ? "✓ Available" : "✗ Not generated")
2. Test binaries: \`hb --version\` and \`hb health\`
3. Create GitHub release with artifacts
4. Push Docker image to registry
5. Update CHANGELOG.md with release notes

## Troubleshooting
See BUILD_GUIDE.md for detailed troubleshooting steps.
"@

$summaryFile = Join-Path $ArtifactRoot "RELEASE_SUMMARY.md"
$summary | Out-File -FilePath $summaryFile

Write-Success "Release summary written to: $summaryFile"
Write-Header "Build Complete!"

Write-ColorOutput "`nArtifacts Location: $ArtifactRoot" Cyan
Write-ColorOutput "Next: Review artifacts and run: hb --version`n" Yellow

