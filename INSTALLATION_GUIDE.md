# HyperBox Installation Guide

**Version:** 0.1.0-alpha  
**Last Updated:** February 2026  
**Status:** Production Ready

---

## 📋 Table of Contents

- [Windows Installation](#windows-installation)
- [Linux Installation](#linux-installation)
- [macOS Installation](#macos-installation)
- [Verification](#verification)
- [Troubleshooting](#troubleshooting)
- [Uninstallation](#uninstallation)

---

## Windows Installation

### Prerequisites

#### Required

- **Windows 10 (Build 19041) or later** OR **Windows 11** (any version)
- **Docker Desktop for Windows** (4.10 or later)
    - Download from: https://www.docker.com/products/docker-desktop
    - WSL 2 backend MUST be enabled
    - 4GB RAM minimum allocation to Docker
- **PowerShell 7+** (recommended) or Windows Command Prompt

#### Optional

- **Git for Windows** (if building from source)
- **Rust toolchain** (if building from source): https://rustup.rs/

### Step 1: Download HyperBox

**Option A: Download Pre-Built Binary (Recommended)**

1. Visit the [HyperBox Releases](https://github.com/iamthegreatdestroyer/HyperBox/releases) page
2. Find the latest **v0.1.0-alpha** release
3. Download: `hyperbox-0.1.0-windows-x86_64.zip`
4. Extract to a folder (e.g., `C:\Program Files\HyperBox`)

**Option B: Build from Source**

```powershell
git clone https://github.com/iamthegreatdestroyer/HyperBox.git
cd HyperBox
cargo build --release
# Binary at: ./target/release/hb.exe
```

### Step 2: Add to System PATH

**PowerShell Method (Recommended):**

```powershell
# Open PowerShell as Administrator
$HyperBoxPath = "C:\Program Files\HyperBox"
[Environment]::SetEnvironmentVariable(
    "Path",
    "$env:Path;$HyperBoxPath",
    [EnvironmentVariableTarget]::User
)

# Refresh environment
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
```

**Manual Method:**

1. Open "Environment Variables" (search in Start menu)
2. Click "Edit the system environment variables"
3. Click "Environment Variables" button
4. Under "User variables", click "Path" → "Edit"
5. Click "New" and add: `C:\Program Files\HyperBox`
6. Click OK and restart PowerShell

### Step 3: Verify Docker is Running

```powershell
docker --version
# Output: Docker version 24.0.6, build ed62f53922

docker ps
# Output: CONTAINER ID   IMAGE     COMMAND   CREATED   STATUS    PORTS     NAMES
```

If Docker is not running:

1. Open "Docker Desktop" application
2. Wait for it to finish starting
3. Retry the command above

### Step 4: Install HyperBox Daemon (Windows Service)

```powershell
# Download and install as Windows service
hb install-service

# Verify service installed
Get-Service Hyperbox

# Start the service
Start-Service Hyperbox

# Verify it's running
Get-Service Hyperbox | Select-Object Status
# Output: Status
#         ------
#         Running
```

### Step 5: Verify Installation

```powershell
hb --version
# Output: HyperBox v0.1.0-alpha

hb health
# Output: HyperBox Health Check:
#           Daemon:     ✅
#           Docker:     ✅
```

### Troubleshooting Windows Installation

**Problem: `hb: The term 'hb' is not recognized`**

Solution: PATH not updated. Try:

1. Restart PowerShell (close all windows)
2. Run: `$env:Path` and verify `C:\Program Files\HyperBox` is listed
3. If not listed, repeat Step 2 above

**Problem: `Docker daemon is not running`**

Solution:

```powershell
Start-Service Docker  # or open Docker Desktop
docker ps  # verify it works
hb health  # should now show Docker as ✅
```

**Problem: `Service installation failed: Access denied`**

Solution: Run PowerShell as Administrator

```powershell
# Open PowerShell as Admin, then:
hb install-service
```

---

## Linux Installation

### Prerequisites

#### Required

- **Ubuntu 20.04+, Debian 11+, or RHEL 8+**
- **crun** runtime (required for OCI containers)
- **Docker** or **Podman** (for image pulling)
- **glibc 2.31+** (usually pre-installed)

#### Optional

- **Systemd** (for service management)

### Step 1: Install crun (Required)

**Ubuntu/Debian:**

```bash
# Method 1: From system repositories (recommended if available)
sudo apt-get update
sudo apt-get install -y crun

# Method 2: Download pre-built binary
wget https://github.com/containers/crun/releases/download/1.8.4/crun-1.8.4-linux-amd64
chmod +x crun-1.8.4-linux-amd64
sudo mv crun-1.8.4-linux-amd64 /usr/local/bin/crun

# Verify
crun --version
# Output: crun version 1.8.4
```

**RHEL/CentOS:**

```bash
sudo dnf install -y crun
crun --version
```

### Step 2: Install Docker/Podman

**Docker (Ubuntu/Debian):**

```bash
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Add user to docker group (optional, to run without sudo)
sudo usermod -aG docker $USER
newgrp docker

docker --version
```

**Podman (Alternative):**

```bash
sudo apt-get install -y podman
podman --version
```

### Step 3: Download HyperBox

**Option A: Pre-Built Binary (Recommended)**

```bash
cd /tmp
wget https://github.com/iamthegreatdestroyer/HyperBox/releases/download/v0.1.0-alpha/hyperbox-0.1.0-linux-x86_64.tar.gz

# Verify checksum (optional but recommended)
wget https://github.com/iamthegreatdestroyer/HyperBox/releases/download/v0.1.0-alpha/SHA256SUMS
sha256sum -c SHA256SUMS 2>&1 | grep "hyperbox-0.1.0-linux-x86_64.tar.gz"
# Output: hyperbox-0.1.0-linux-x86_64.tar.gz: OK

# Extract
tar -xzf hyperbox-0.1.0-linux-x86_64.tar.gz
sudo cp hb /usr/local/bin/
sudo chmod +x /usr/local/bin/hb
```

**Option B: Build from Source**

```bash
git clone https://github.com/iamthegreatdestroyer/HyperBox.git
cd HyperBox
cargo build --release
sudo cp target/release/hb /usr/local/bin/
```

### Step 4: Setup Daemon (with systemd)

```bash
# Create daemon configuration directory
sudo mkdir -p /etc/hyperbox

# Generate default config
cat | sudo tee /etc/hyperbox/hyperbox.conf <<EOF
[daemon]
socket = /run/hyperbox/hyperbox.sock
crun_path = /usr/local/bin/crun
docker_socket = /var/run/docker.sock
runtime_dir = /var/lib/hyperbox
EOF

# Create systemd service
sudo tee /etc/systemd/system/hyperboxd.service >/dev/null <<EOF
[Unit]
Description=HyperBox Daemon
After=docker.service
Wants=docker.service

[Service]
Type=simple
ExecStart=/usr/local/bin/hyperboxd
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable hyperboxd
sudo systemctl start hyperboxd
```

### Step 5: Verify Installation

```bash
hb --version
# Output: HyperBox v0.1.0-alpha

hb health
# Output: HyperBox Health Check:
#           Daemon:     ✅
#           crun:       ✅
#           Docker:     ✅
```

### Troubleshooting Linux Installation

**Problem: `crun: command not found`**

Solution:

```bash
sudo apt-get install -y crun
# or download from: https://github.com/containers/crun/releases
```

**Problem: `Permission denied when creating containers`**

Solution:

```bash
# Add user to docker group (Linux only)
sudo usermod -aG docker $USER
newgrp docker

# Verify
docker ps
```

**Problem: `Daemon socket not found: /run/hyperbox/hyperbox.sock`**

Solution:

```bash
sudo systemctl start hyperboxd
sudo systemctl status hyperboxd  # check for errors
sudo journalctl -u hyperboxd -n 20  # view daemon logs
```

---

## macOS Installation

### Prerequisites

#### Required

- **macOS 11 (Big Sur) or later**
- **Docker Desktop for Mac** (4.10 or later)
    - Download from: https://www.docker.com/products/docker-desktop
    - Apple Silicon (M1/M2) or Intel versions available
- **Homebrew** (recommended for dependency management)

#### Optional

- **Xcode Command Line Tools**: `xcode-select --install`
- **Rust toolchain** (if building from source): https://rustup.rs/

### Step 1: Install crun via Homebrew

```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install crun
brew install crun

# Verify
crun --version
# Output: crun version 1.8.4
```

### Step 2: Download HyperBox

**Option A: Pre-Built Binary (Recommended)**

```bash
cd /tmp

# Download (choose your architecture)
# For Apple Silicon (M1/M2):
wget https://github.com/iamthegreatdestroyer/HyperBox/releases/download/v0.1.0-alpha/hyperbox-0.1.0-macos-aarch64.tar.gz

# For Intel Macs:
wget https://github.com/iamthegreatdestroyer/HyperBox/releases/download/v0.1.0-alpha/hyperbox-0.1.0-macos-x86_64.tar.gz

# Extract and install
tar -xzf hyperbox-0.1.0-macos-*.tar.gz
sudo cp hb /usr/local/bin/
sudo chmod +x /usr/local/bin/hb
```

**Option B: Build from Source**

```bash
git clone https://github.com/iamthegreatdestroyer/HyperBox.git
cd HyperBox
cargo build --release
sudo cp target/release/hb /usr/local/bin/
```

### Step 3: Setup Daemon (with launchd)

```bash
# Create daemon plist for macOS
mkdir -p $HOME/Library/LaunchAgents

cat > $HOME/Library/LaunchAgents/com.hyperbox.daemon.plist <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.hyperbox.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/hyperboxd</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
EOF

# Load the daemon
launchctl load $HOME/Library/LaunchAgents/com.hyperbox.daemon.plist

# Verify
launchctl list | grep hyperbox
```

### Step 4: Verify Installation

```bash
hb --version
# Output: HyperBox v0.1.0-alpha

# Note: macOS Docker Desktop runs Linux kernel in a VM
# Some features may behave differently than native Linux
hb health
# Output: HyperBox Health Check:
#           Daemon:     ✅
#           Docker:     ✅
```

### Troubleshooting macOS Installation

**Problem: `crun: command not found`**

Solution:

```bash
brew install crun
which crun  # verify
```

**Problem: Docker Desktop not running**

Solution:

1. Open Finder → Applications → Docker.app
2. Wait for Docker menu icon to appear in menu bar
3. Retry commands

**Problem: Cannot open `/usr/local/bin/hb`**

Solution (Intel Macs 10.15+):

```bash
# macOS requires code signing approval
xattr -d com.apple.quarantine /usr/local/bin/hb

# Try again
hb --version
```

---

## Verification

### All Platforms: Health Check

```bash
$ hb health

HyperBox Health Check:
  Core System:    ✅ Ready
  Daemon:         ✅ Connected
  Docker:         ✅ Socket found
```

### All Platforms: First Container

```bash
# Create a project
$ hb project create test-app

# List projects
$ hb project list
# Output: test-app (created: 2026-02-07)

# Pull a small test image
$ hb image pull alpine:latest
# Output: Pulling alpine:latest...
#         Pulling layer sha256:abc123def456...
#         Downloaded: 7.3 MB

# Create a container
$ hb container create test-app --image alpine:latest --name test
# Output: Created container: test-app/test (id: abc123def456)

# List containers
$ hb container list test-app
# Output: CONTAINER ID  NAME   STATUS    IMAGE
#         abc123def456   test   CREATED   alpine:latest

# Start the container
$ hb container start test-app test
# Output: Started container: test-app/test

# Execute a command
$ hb container exec test-app test -- echo "Hello from HyperBox!"
# Output: Hello from HyperBox!

# Stop the container
$ hb container stop test-app test
# Output: Stopped container: test-app/test

# Remove the container
$ hb container remove test-app test
# Output: Removed container: test-app/test
```

---

## Uninstallation

### Windows

```powershell
# Stop the service
Stop-Service Hyperbox

# Remove the service
sc delete Hyperbox

# Remove from PATH (see installation step 2)

# Delete the installation directory
Remove-Item -Path "C:\Program Files\HyperBox" -Recurse -Force
```

### Linux

```bash
# Stop the daemon
sudo systemctl stop hyperboxd
sudo systemctl disable hyperboxd

# Remove service
sudo rm /etc/systemd/system/hyperboxd.service
sudo systemctl daemon-reload

# Remove binary
sudo rm /usr/local/bin/hb

# Remove configuration (optional)
sudo rm -rf /etc/hyperbox
sudo rm -rf /var/lib/hyperbox
```

### macOS

```bash
# Unload the daemon
launchctl unload $HOME/Library/LaunchAgents/com.hyperbox.daemon.plist
rm $HOME/Library/LaunchAgents/com.hyperbox.daemon.plist

# Remove binary
sudo rm /usr/local/bin/hb
```

---

## Getting Help

**Installation Issues:**

- See: [Troubleshooting Guide](TROUBLESHOOTING.md)
- GitHub Issues: https://github.com/iamthegreatdestroyer/HyperBox/issues

**Next Steps:**

- Read: [Quick Start Guide](QUICKSTART.md)
- Read: [CLI Reference](API_AND_CLI_REFERENCE.md)
- Examples: [Feature Examples](EXAMPLES.md)

---

**Document Version:** 1.0  
**Last Updated:** February 2026  
**Maintained by:** HyperBox Development Team
