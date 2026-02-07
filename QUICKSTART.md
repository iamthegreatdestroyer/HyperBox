# HyperBox Quick Start Guide

**Version:** 0.1.0-alpha  
**Setup Time:** ~5 minutes  
**Target Audience:** First-time users

Welcome to HyperBox! Get your first container running in 5 minutes.

---

## ⚡ TL;DR (30 seconds)

```bash
# 1. Install (if not already done)
# Windows: Download from releases and add to PATH
# Linux: wget + tar + /usr/local/bin/
# macOS: brew install hyperbox

# 2. Start daemon
hyperboxd &           # Background on Linux/macOS
Start-Service Hyperbox  # Windows (service auto-starts)

# 3. Create and run a container
hb project create demo
hb container create demo --image alpine:latest --name hello
hb container start demo hello
hb container exec demo hello -- echo "Hello HyperBox!"

# 4. Clean up
hb container stop demo hello
hb container remove demo hello
```

---

## 📋 Prerequisites Check (1 minute)

Before starting, verify everything is installed:

**Windows:**

```powershell
docker --version        # Should show Docker 24.0+
hb --version            # Should show HyperBox v0.1.0-alpha
Get-Service Hyperbox    # Should show Status: Running
```

**Linux/macOS:**

```bash
docker --version        # Should show Docker 24.0+
crun --version          # Should show crun 1.8.4+
hb --version            # Should show HyperBox v0.1.0-alpha
hb health               # All checks should show ✅
```

**Not working?** See [Installation Guide](INSTALLATION_GUIDE.md)

---

## 🚀 Step-by-Step (5 minutes)

### Step 1: Create a Project (1 min)

A "project" is a container namespace - think of it as a folder for your containers.

```bash
$ hb project create quickstart-demo

// Output:
// Created project: quickstart-demo
// Location: $HOME/.hyperbox/projects/quickstart-demo
```

**What this did:**

- ✅ Created a new project directory
- ✅ Initialized project config
- ✅ Ready to add containers

**Verify:**

```bash
$ hb project list
// Output:
// quickstart-demo (created: 2026-02-07T10:30:00Z)
```

---

### Step 2: Pull an Image (1 min)

Images are the "templates" for containers. Let's pull a lightweight Alpine Linux image.

```bash
$ hb image pull alpine:latest

// Output:
// Pulling alpine:latest...
// Resolving image metadata...
// Downloading layers [████████████████████] 100%
// Downloaded: 7.3 MB
// Image ready: alpine@sha256:abc123def456...
```

**What this did:**

- ✅ Connected to Docker Hub
- ✅ Downloaded the Alpine Linux image layers
- ✅ Extracted and cached locally
- ✅ Ready to create containers from

**Speed up future pulls:**

```bash
# Just list cached images
$ hb image list
// Output:
// REPOSITORY  TAG      DIGEST                          SIZE
// alpine      latest   sha256:abc123def456...          7.3 MB
```

---

### Step 3: Create a Container (1 min)

Containers are **instances** of images - like running processes.

```bash
$ hb container create quickstart-demo \
    --image alpine:latest \
    --name demo-container

// Output:
// Created container: quickstart-demo/demo-container
// ID: abc123def456
// Status: CREATED
```

**What this did:**

- ✅ Allocated container filesystem (copy-on-write from Alpine image)
- ✅ Created container configuration
- ✅ Assigned networking (if needed)
- ✅ NOT yet running (that's next step)

**Verify:**

```bash
$ hb container list quickstart-demo

// Output:
// CONTAINER ID     NAME               STATUS     IMAGE
// abc123def456     demo-container     CREATED    alpine:latest
```

---

### Step 4: Start and Run (1 min)

Now let's bring the container to life!

```bash
$ hb container start quickstart-demo demo-container

// Output:
// Started container: quickstart-demo/demo-container
// PID: 5432
// Status: RUNNING
```

**Verify it's running:**

```bash
$ hb container ps quickstart-demo

// Output:
// CONTAINER ID     NAME               STATUS    UPTIME       MEMORY
// abc123def456     demo-container     RUNNING   3 seconds    2 MB
```

---

### Step 5: Execute a Command (1 min)

Let's run something inside the container!

```bash
$ hb container exec quickstart-demo demo-container -- \
    sh -c "echo 'Hello from HyperBox!' && uname -a"

// Output:
// Hello from HyperBox!
// Linux abc123def456 5.10.0 #1 SMP x86_64 GNU/Linux
```

**Try more commands:**

```bash
# List files in container
$ hb container exec quickstart-demo demo-container -- ls -la /
// Output: bin, lib, usr, etc, ...

# Install a package (Alpine uses apk)
$ hb container exec quickstart-demo demo-container -- apk add curl
// Output: Installed curl successfully

# Use the package
$ hb container exec quickstart-demo demo-container -- curl --version
// Output: curl 8.5.0 ...
```

---

### Step 6: Stop and Clean Up

When done, gracefully stop the container:

```bash
$ hb container stop quickstart-demo demo-container

// Output:
// Stopped container: quickstart-demo/demo-container
// Graceful shutdown: 3 seconds
```

**Verify it stopped:**

```bash
$ hb container ps quickstart-demo
// Output: (empty - no running containers)

$ hb container list quickstart-demo
// Output:
// CONTAINER ID     NAME               STATUS    IMAGE
// abc123def456     demo-container     STOPPED   alpine:latest
```

**Remove when done:**

```bash
$ hb container remove quickstart-demo demo-container

// Output:
// Removed container: quickstart-demo/demo-container
// Freed space: 2 MB

# Clean up the entire project (optional)
$ hb project remove quickstart-demo
// Output: Removed project: quickstart-demo
```

---

## 📚 Next Steps

Congratulations! 🎉 You've mastered the basics. Here's where to go next:

### Basic Usage

- **All CLI commands:** [CLI Reference](API_AND_CLI_REFERENCE.md)
- **Troubleshooting:** [Troubleshooting Guide](TROUBLESHOOTING.md)

### Real-World Examples

- **Web server example:** [Examples](EXAMPLES.md#running-a-web-server)
- **Database example:** [Examples](EXAMPLES.md#running-a-database)
- **Multi-container setup:** [Examples](EXAMPLES.md#multi-container-application)

### Advanced Features

- **Networking:** Link containers together
- **Volumes:** Persist data between runs
- **Resource limits:** Constrain CPU/Memory
- **Health checks:** Monitor container status

### Getting Help

**Common Issues:**

```bash
# Check daemon status
hb health

# View daemon logs (Linux/macOS)
sudo journalctl -u hyperboxd -n 50

# View daemon logs (Windows PowerShell)
Get-EventLog -LogName Application -Source hyperboxd -Newest 50
```

**Find us online:**

- **GitHub Issues:** https://github.com/iamthegreatdestroyer/HyperBox/issues
- **GitHub Discussions:** https://github.com/iamthegreatdestroyer/HyperBox/discussions
- **Documentation:** https://github.com/iamthegreatdestroyer/HyperBox

---

## 💡 Pro Tips

**Tip 1: Tab completion**

```bash
# Enable bash/zsh completion
eval "$(hb completion bash)"  # bash
eval "$(hb completion zsh)"   # zsh
```

**Tip 2: Reuse containers**

```bash
# Create once, restart many times
hb container create myproj --image alpine:latest --name dev
hb container start myproj dev    # first time
hb container exec myproj dev -- sh
# Exit from shell...
hb container start myproj dev    # start again later
```

**Tip 3: Image caching**

```bash
# First pull takes time, but layers are cached
hb image pull ubuntu:22.04       # ~80 MB, takes 10s
hb image pull ubuntu:22.04       # instant! (cached)
```

**Tip 4: Check resource usage**

```bash
$ hb container stats quickstart-demo

// Output:
// CONTAINER ID     NAME          MEMORY    CPU%   PID MEMORY
// abc123def456     demo-cont     2 MB      0.1    5432
```

---

## 🎯 Common Tasks in 2 Minutes

### Run a Python Script

```bash
# Create container with Python
hb container create proj --image python:3.11 --name py-app

# Create a script
hb container exec proj py-app -- sh -c 'cat > /tmp/hello.py <<EOF
#!/usr/bin/env python3
print("Hello from Python in HyperBox!")
for i in range(3):
    print(f"  Count: {i+1}")
EOF'

# Run it
hb container exec proj py-app -- python3 /tmp/hello.py
// Output:
// Hello from Python in HyperBox!
//   Count: 1
//   Count: 2
//   Count: 3
```

### Run a Web Server

```bash
# Create container with nginx
hb container create proj --image nginx:alpine --name webserver

# Start it
hb container start proj webserver

# Test it (from host)
curl http://localhost:80
// Output: (nginx default page HTML)

# View logs
hb container logs proj webserver
```

### Run a Database

```bash
# PostgreSQL container
hb container create proj --image postgres:15 --name db \
    --env POSTGRES_PASSWORD=securepass

hb container start proj db

# Give it time to initialize (10 seconds)
sleep 10

# Connect from inside container
hb container exec proj db -- psql -U postgres -c "SELECT version();"
// Output: PostgreSQL 15.1 on ...
```

---

## ❓ FAQ

**Q: Can I have multiple containers running?**
A: Yes! Create as many as you want:

```bash
hb container create myproj --image alpine --name app1
hb container create myproj --image nginx --name webserver
hb container create myproj --image postgres --name database
hb container start myproj app1
hb container start myproj webserver
hb container start myproj database
```

**Q: How do I persist data between container runs?**
A: Refer to [Advanced Features](API_AND_CLI_REFERENCE.md#volumes) in the CLI reference.

**Q: Can I run GUI applications in HyperBox containers?**
A: Not recommended for v0.1.0-alpha. Focus on server/CLI applications first.

**Q: What images can I use?**
A: Any OCI-compatible image from Docker Hub:

- `alpine:latest` - Tiny (~7 MB)
- `ubuntu:22.04` - Full Linux distro
- `python:3.11` - Python pre-installed
- `node:20` - Node.js pre-installed
- `nginx:latest` - Web server
- `postgres:15` - Database
- Custom images (build with Docker, run in HyperBox)

**Q: How do I update HyperBox?**
A: Download the new release and replace the binary:

```bash
# Windows: Download new .zip, extract, copy hb.exe to install folder
# Linux: wget new binary, cp to /usr/local/bin, chmod +x
# macOS: brew upgrade hyperbox
```

---

## 📖 Complete Documentation

- **Installation:** [Installation Guide](INSTALLATION_GUIDE.md)
- **Full CLI Reference:** [CLI Reference](API_AND_CLI_REFERENCE.md)
- **Examples:** [Feature Examples](EXAMPLES.md)
- **Troubleshooting:** [Troubleshooting Guide](TROUBLESHOOTING.md)
- **Release Notes:** [Release Notes](RELEASE_NOTES_0.1.0.md)

---

**You're all set!** 🚀

Questions? Visit: https://github.com/iamthegreatdestroyer/HyperBox/discussions

---

**Document Version:** 1.0  
**Last Updated:** February 2026  
**Estimated Time:** 5 minutes
