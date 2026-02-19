# HyperBox CLI Reference

## Overview

HyperBox is a revolutionary container management platform designed for developers. The `hb` CLI provides a powerful, project-centric interface for managing containers, images, projects, and the system.

### Key Concepts

- **Projects**: Isolated container environments - each project gets its own namespace
- **Containers**: Running instances of images within a project
- **Images**: Container images from registries or built locally
- **Services**: Named containers within a project that can be managed together

### Quick Facts

- **Tool Name**: `hb` (or `hb.exe` on Windows)
- **Config Location**: `~/.hyperbox/` (Linux/macOS), `%APPDATA%\HyperBox\` (Windows)
- **Socket**: Unix socket or Windows named pipe for daemon communication
- **Output Formats**: Text (default) or JSON via `--output json`
- **Verbosity**: Control with `-v`, `-vv`, `-vvv` flags

---

## Global Options

These options work with all `hb` commands:

```bash
hb [GLOBAL_OPTIONS] <COMMAND>
```

| Option | Short | Description | Example |
|--------|-------|-------------|---------|
| `--verbose` | `-v` | Increase verbosity (stack: -v, -vv, -vvv) | `hb -vvv project list` |
| `--output` | `-o` | Output format (text, json) | `hb -o json container list` |
| `--help` | `-h` | Show help information | `hb --help` |
| `--version` | `-V` | Show version | `hb --version` |

---

## Command Index

### Project Commands

Manage projects (container namespaces):

- [project open](#hb-project-open) - Open/activate a project
- [project start](#hb-project-start) - Start containers
- [project stop](#hb-project-stop) - Stop containers
- [project restart](#hb-project-restart) - Restart containers
- [project status](#hb-project-status) - Show status
- [project list](#hb-project-list) - List all projects
- [project logs](#hb-project-logs) - View logs
- [project close](#hb-project-close) - Close/cleanup project
- [project init](#hb-project-init) - Initialize configuration

### Container Commands

Manage individual containers:

- [container list](#hb-container-list) - List containers
- [container run](#hb-container-run) - Run a new container
- [container start](#hb-container-start) - Start stopped container
- [container stop](#hb-container-stop) - Stop running container
- [container restart](#hb-container-restart) - Restart container
- [container remove](#hb-container-remove) - Remove container
- [container exec](#hb-container-exec) - Execute command in container
- [container logs](#hb-container-logs) - View container logs
- [container inspect](#hb-container-inspect) - Show container details
- [container stats](#hb-container-stats) - Show container statistics
- [container cp](#hb-container-cp) - Copy files

### Image Commands

Manage container images:

- [image list](#hb-image-list) - List images
- [image pull](#hb-image-pull) - Pull from registry
- [image push](#hb-image-push) - Push to registry
- [image build](#hb-image-build) - Build from Dockerfile
- [image remove](#hb-image-remove) - Remove images
- [image inspect](#hb-image-inspect) - Show image details
- [image history](#hb-image-history) - Show image layers
- [image tag](#hb-image-tag) - Tag an image
- [image prune](#hb-image-prune) - Remove unused images

### System Commands

System and daemon management:

- [system info](#hb-system-info) - Show system information
- [system version](#hb-system-version) - Show version
- [system disk-usage](#hb-system-disk-usage) - Show disk usage
- [system prune](#hb-system-prune) - Clean up unused data
- [system daemon](#hb-system-daemon) - Manage daemon
- [system events](#hb-system-events) - Real-time events
- [system benchmark](#hb-system-benchmark) - Run benchmarks
- [system health](#hb-system-health) - Health check

### Other Commands

- [health](#hb-health) - Check health status
- [completion](#hb-completion) - Shell completions
- [docker](#hb-docker) - Docker compatibility mode

---

## Detailed Reference

### hb project open

**Open/activate a project directory**

```bash
hb project open [OPTIONS] [PATH]
```

**Arguments:**
- `[PATH]` - Path to project directory (default: `.`)

**Options:**
- `-n, --name <NAME>` - Project name (defaults to directory name)

**Examples:**

```bash
# Open current directory as project
hb project open

# Open specific directory
hb project open /path/to/project

# Open with custom name
hb project open . --name my-app
```

**What it does:**
1. Scans the directory for `docker-compose.yml`, `Dockerfile`, or `.hyperbox` config
2. Creates a project entry in HyperBox
3. Makes the project the "current" project for subsequent commands

---

### hb project start

**Start a project's containers**

```bash
hb project start [OPTIONS] [SERVICES]...
```

**Arguments:**
- `[SERVICES]...` - Specific services to start (optional, starts all if not specified)

**Options:**
- `-b, --build` - Build images before starting
- `--force-recreate` - Force recreation of containers
- `-d, --detach` - Run in detached mode (background)

**Examples:**

```bash
# Start all containers in current project
hb project start

# Start specific service
hb project start web

# Start multiple services
hb project start web db cache

# Build and start
hb project start --build

# Start in background
hb project start --detach

# Force recreate (useful for config changes)
hb project start --force-recreate
```

**Output:**
```
Starting project 'myapp'...
  ✅ web (port 8080)
  ✅ db (port 5432)
  ✅ cache (port 6379)
Started 3 containers in 0.234s
```

---

### hb project stop

**Stop a project's containers**

```bash
hb project stop [OPTIONS] [SERVICES]...
```

**Arguments:**
- `[SERVICES]...` - Specific services to stop (optional, stops all if not specified)

**Options:**
- `-t, --timeout <TIMEOUT>` - Timeout in seconds (default: 10)

**Examples:**

```bash
# Stop all containers
hb project stop

# Stop specific service
hb project stop web

# Stop with 30s timeout
hb project stop --timeout 30
```

---

### hb project restart

**Restart a project's containers**

```bash
hb project restart [OPTIONS] [SERVICES]...
```

**Arguments:**
- `[SERVICES]...` - Specific services to restart

**Examples:**

```bash
# Restart all containers
hb project restart

# Restart specific service
hb project restart web
```

---

### hb project status

**Show project status**

```bash
hb project status [OPTIONS]
```

**Options:**
- `-d, --detailed` - Show detailed status including resource usage

**Examples:**

```bash
# Quick status
hb project status

# Detailed status
hb project status --detailed
```

**Output:**
```
Project: myapp
Status: Running
Containers: 3/3 running
Memory: 245.3 MB
CPU: 2.5%
```

---

### hb project list

**List all projects**

```bash
hb project list [OPTIONS]
```

**Options:**
- `-a, --all` - Show all projects (including stopped)

**Examples:**

```bash
# List running projects
hb project list

# List all projects
hb project list --all
```

**Output:**
```
NAME        STATUS    CONTAINERS  MEMORY     CPU
myapp       Running   3/3         245.3 MB   2.5%
api         Running   2/2         156.7 MB   1.2%
worker      Stopped   2           -          -
```

---

### hb project logs

**Show project logs**

```bash
hb project logs [OPTIONS] [SERVICES]...
```

**Arguments:**
- `[SERVICES]...` - Services to show logs for (optional, shows all if not specified)

**Options:**
- `-f, --follow` - Follow log output (like `tail -f`)
- `-n, --tail <TAIL>` - Number of lines to show (default: 100)
- `-t, --timestamps` - Show timestamps

**Examples:**

```bash
# Show last 100 lines of all logs
hb project logs

# Follow logs in real-time
hb project logs --follow

# Show logs with timestamps
hb project logs --timestamps

# Show last 50 lines
hb project logs --tail 50

# Follow specific service
hb project logs --follow web
```

---

### hb project close

**Close and cleanup a project**

```bash
hb project close [OPTIONS]
```

**Options:**
- `--networks` - Remove networks created by the project

**Examples:**

```bash
# Stop and cleanup
hb project close

# Stop, cleanup, and remove networks
hb project close --networks
```

---

### hb project init

**Initialize a new project configuration**

```bash
hb project init [OPTIONS] [PATH]
```

**Arguments:**
- `[PATH]` - Path to initialize (default: `.`)

**Options:**
- `-t, --template <TEMPLATE>` - Use a project template (e.g., `node`, `python`, `rust`)

**Examples:**

```bash
# Initialize current directory
hb project init

# Initialize with Node.js template
hb project init --template node

# Initialize different directory
hb project init /path/to/project
```

---

### hb container list

**List containers**

```bash
hb container list [OPTIONS]
```

**Options:**
- `-a, --all` - Show all containers (including stopped)
- `-p, --project <PROJECT>` - Filter by project name
- `-q, --quiet` - Show only container IDs

**Examples:**

```bash
# List running containers
hb container list

# List all containers
hb container list --all

# Filter by project
hb container list --project myapp

# Quiet mode (just IDs)
hb container list --quiet
```

---

### hb container run

**Run a new container**

```bash
hb container run [OPTIONS] <IMAGE> [-- <COMMAND>...]
```

**Arguments:**
- `<IMAGE>` - Image to run (e.g., `nginx:latest`, `postgres:15`)
- `[COMMAND]...` - Command to run (optional)

**Options:**
- `--name <NAME>` - Container name
- `-p, --port <PORT>` - Port mappings (`host:container`)
- `-v, --volume <VOLUME>` - Volume mounts (`host:container`)
- `-e, --env <ENV>` - Environment variables (`KEY=VALUE`)
- `-d, --detach` - Run in background
- `--rm` - Remove container when it exits
- `-i, --interactive` - Keep stdin open
- `-t, --tty` - Allocate TTY
- `-w, --workdir <WORKDIR>` - Working directory in container

**Examples:**

```bash
# Run simple container
hb container run nginx

# Run with name
hb container run --name web nginx

# Run with port mapping
hb container run -p 8080:80 nginx

# Run with volume
hb container run -v /host/data:/data postgres

# Run with environment variable
hb container run -e DATABASE_URL=postgres://... myapp

# Run with multiple mappings
hb container run \
  --name api \
  -p 8000:8000 \
  -e NODE_ENV=production \
  -v /app:/app \
  node:20

# Run and remove on exit
hb container run --rm alpine echo "hello"

# Interactive shell
hb container run -it ubuntu bash
```

---

### hb container start

**Start a stopped container**

```bash
hb container start <CONTAINER>
```

**Arguments:**
- `<CONTAINER>` - Container ID or name

**Examples:**

```bash
hb container start myapp
hb container start abc123def456
```

---

### hb container stop

**Stop a running container**

```bash
hb container stop [OPTIONS] <CONTAINER>
```

**Arguments:**
- `<CONTAINER>` - Container ID or name

**Options:**
- `-t, --timeout <TIMEOUT>` - Timeout in seconds (default: 10)

**Examples:**

```bash
# Stop container (10s timeout)
hb container stop myapp

# Stop with 30s timeout
hb container stop --timeout 30 myapp

# Force stop (use --timeout 0)
hb container stop --timeout 0 myapp
```

---

### hb container restart

**Restart a container**

```bash
hb container restart [OPTIONS] <CONTAINER>
```

**Arguments:**
- `<CONTAINER>` - Container ID or name

**Options:**
- `-t, --timeout <TIMEOUT>` - Timeout in seconds (default: 10)

**Examples:**

```bash
hb container restart myapp
hb container restart --timeout 30 myapp
```

---

### hb container remove

**Remove a container**

```bash
hb container remove [OPTIONS] <CONTAINERS>...
```

**Arguments:**
- `<CONTAINERS>...` - Container IDs or names

**Options:**
- `-f, --force` - Force removal
- `-v, --volumes` - Remove associated volumes

**Examples:**

```bash
# Remove container
hb container remove myapp

# Force remove running container
hb container remove --force myapp

# Remove and delete volumes
hb container remove --volumes myapp

# Remove multiple containers
hb container remove app1 app2 app3
```

---

### hb container exec

**Execute a command in a running container**

```bash
hb container exec [OPTIONS] <CONTAINER> [COMMAND]...
```

**Arguments:**
- `<CONTAINER>` - Container ID or name
- `[COMMAND]...` - Command to execute

**Options:**
- `-i, --interactive` - Keep stdin open
- `-t, --tty` - Allocate TTY
- `-w, --workdir <WORKDIR>` - Working directory
- `-e, --env <ENV>` - Environment variables

**Examples:**

```bash
# Run command in container
hb container exec myapp ls -la

# Interactive shell
hb container exec -it myapp bash

# Execute as specific user (if in COMMAND)
hb container exec myapp su - postgres -c psql

# With environment variable
hb container exec -e DEBUG=1 myapp npm test
```

---

### hb container logs

**Show container logs**

```bash
hb container logs [OPTIONS] <CONTAINER>
```

**Arguments:**
- `<CONTAINER>` - Container ID or name

**Options:**
- `-f, --follow` - Follow output
- `-n, --tail <TAIL>` - Lines to show (default: 100)
- `-t, --timestamps` - Show timestamps

**Examples:**

```bash
# Show last 100 lines
hb container logs myapp

# Follow logs
hb container logs --follow myapp

# Show last 50 lines with timestamps
hb container logs --tail 50 --timestamps myapp
```

---

### hb container inspect

**Show container details**

```bash
hb container inspect <CONTAINER>
```

**Arguments:**
- `<CONTAINER>` - Container ID or name

**Examples:**

```bash
hb container inspect myapp
hb container inspect abc123 --output json
```

**Output includes:**
- Container ID and name
- State (running, stopped, etc.)
- Port mappings
- Volumes
- Network settings
- Environment variables

---

### hb container stats

**Show container statistics**

```bash
hb container stats [OPTIONS] [CONTAINERS]...
```

**Arguments:**
- `[CONTAINERS]...` - Container IDs or names (all if empty)

**Options:**
- `--no-stream` - Show snapshot instead of streaming

**Examples:**

```bash
# Stream stats for all containers
hb container stats

# Stream stats for specific container
hb container stats myapp

# Take snapshot
hb container stats --no-stream

# Multiple containers
hb container stats app1 app2 db
```

**Output:**
```
CONTAINER     CPU%    MEM         MEM%    NET I/O        BLOCK I/O
myapp         2.5%    245.3 MB    12%     156.7 KB       50.2 MB
```

---

### hb container cp

**Copy files between host and container**

```bash
hb container cp <SOURCE> <DEST>
```

**Arguments:**
- `<SOURCE>` - Source path (`container:path` or `/host/path`)
- `<DEST>` - Destination path

**Examples:**

```bash
# Copy from host to container
hb container cp /host/file.txt myapp:/app/

# Copy from container to host
hb container cp myapp:/app/file.txt /host/

# Copy directories
hb container cp /host/data myapp:/app/data
```

---

### hb image list

**List images**

```bash
hb image list [OPTIONS]
```

**Options:**
- `-a, --all` - Show all images (including intermediate)
- `-q, --quiet` - Show only image IDs

**Examples:**

```bash
# List local images
hb image list

# List all including intermediate
hb image list --all

# Quiet mode
hb image list --quiet
```

**Output:**
```
REPOSITORY          TAG         IMAGE ID        SIZE
nginx               latest      abc123def456    141.4 MB
postgres            15          xyz789abc123    314.8 MB
node                20          def456xyz789    1.1 GB
```

---

### hb image pull

**Pull an image from a registry**

```bash
hb image pull [OPTIONS] <IMAGE>
```

**Arguments:**
- `<IMAGE>` - Image name (e.g., `nginx:latest`, `postgres:15`)

**Options:**
- `-a, --all-tags` - Pull all tags
- `--platform <PLATFORM>` - Platform (e.g., `linux/amd64`, `linux/arm64`)

**Examples:**

```bash
# Pull latest image
hb image pull nginx

# Pull specific version
hb image pull postgres:15

# Pull for specific platform
hb image pull --platform linux/arm64 node:20

# Pull all tags
hb image pull --all-tags myregistry.com/myimage
```

---

### hb image push

**Push an image to a registry**

```bash
hb image push [OPTIONS] <IMAGE>
```

**Arguments:**
- `<IMAGE>` - Image name

**Options:**
- `-a, --all-tags` - Push all tags

**Examples:**

```bash
# Push image
hb image push myregistry.com/myapp:v1.0

# Push all tags
hb image push --all-tags myregistry.com/myapp
```

---

### hb image build

**Build an image from a Dockerfile**

```bash
hb image build [OPTIONS] [PATH]
```

**Arguments:**
- `[PATH]` - Build context (default: `.`)

**Options:**
- `-t, --tag <TAG>` - Image tag (can be used multiple times)
- `-f, --file <FILE>` - Dockerfile path (default: `Dockerfile`)
- `--build-arg <BUILD_ARG>` - Build arguments (`KEY=VALUE`)
- `--target <TARGET>` - Target build stage
- `--no-cache` - Don't use build cache
- `--pull` - Always pull base images

**Examples:**

```bash
# Build with default settings
hb image build

# Build with tag
hb image build -t myapp:v1.0

# Multiple tags
hb image build -t myapp:v1.0 -t myapp:latest

# Custom Dockerfile
hb image build -f Dockerfile.prod

# Build arguments
hb image build --build-arg NODE_ENV=production

# Multi-stage target
hb image build --target production

# No cache
hb image build --no-cache

# Complex build
hb image build \
  -t myapp:v1.0 \
  -f Dockerfile.prod \
  --build-arg NODE_ENV=production \
  --target production \
  --pull \
  ./
```

---

### hb image remove

**Remove images**

```bash
hb image remove [OPTIONS] [IMAGES]...
```

**Arguments:**
- `[IMAGES]...` - Image names or IDs

**Options:**
- `-f, --force` - Force removal

**Examples:**

```bash
# Remove image
hb image remove myapp:v1.0

# Force remove
hb image remove --force myapp

# Remove multiple
hb image remove nginx postgres redis
```

---

### hb image inspect

**Show image details**

```bash
hb image inspect <IMAGE>
```

**Arguments:**
- `<IMAGE>` - Image name or ID

**Examples:**

```bash
hb image inspect nginx:latest
hb image inspect myapp:v1.0 --output json
```

---

### hb image history

**Show image layers**

```bash
hb image history [OPTIONS] <IMAGE>
```

**Arguments:**
- `<IMAGE>` - Image name or ID

**Options:**
- `--no-trunc` - Don't truncate output

**Examples:**

```bash
hb image history nginx
hb image history --no-trunc myapp
```

---

### hb image tag

**Tag an image**

```bash
hb image tag <SOURCE> <TARGET>
```

**Arguments:**
- `<SOURCE>` - Source image
- `<TARGET>` - Target image name

**Examples:**

```bash
# Tag with new name
hb image tag myapp:v1.0 myapp:latest

# Tag for registry
hb image tag myapp myregistry.com/myapp:v1.0
```

---

### hb image prune

**Remove unused images**

```bash
hb image prune [OPTIONS]
```

**Options:**
- `-a, --all` - Remove all unused images
- `-f, --force` - Don't prompt for confirmation

**Examples:**

```bash
# Remove dangling images
hb image prune

# Remove all unused
hb image prune --all

# Force without confirmation
hb image prune --all --force
```

---

### hb system info

**Show system information**

```bash
hb system info
```

**Output includes:**
- HyperBox version
- Daemon status
- OS and kernel info
- Container runtime info
- Storage backend

---

### hb system version

**Show HyperBox version**

```bash
hb system version
```

---

### hb system disk-usage

**Show disk usage**

```bash
hb system disk-usage [OPTIONS]
```

**Options:**
- `-v, --verbose` - Verbose output

**Output:**
```
Total HyperBox storage: 3.5 GB

  Images:     1.2 GB
  Containers: 845 MB
  Volumes:    1.3 GB
  Temp:       178 MB
```

---

### hb system prune

**Clean up unused data**

```bash
hb system prune [OPTIONS]
```

**Options:**
- `-a, --all` - Remove all unused data
- `--volumes` - Remove unused volumes
- `-f, --force` - Skip confirmation

**Examples:**

```bash
# Prune dangling containers/images
hb system prune

# Prune everything
hb system prune --all

# Prune including volumes
hb system prune --all --volumes

# Force without confirmation
hb system prune --force
```

---

### hb system daemon

**Manage the HyperBox daemon**

```bash
hb system daemon <COMMAND>
```

**Commands:**
- `start` - Start the daemon
- `stop` - Stop the daemon
- `restart` - Restart the daemon
- `status` - Show daemon status

**Examples:**

```bash
# Start daemon
hb system daemon start

# Check status
hb system daemon status

# Restart
hb system daemon restart
```

---

### hb system events

**Show real-time events**

```bash
hb system events [OPTIONS]
```

**Options:**
- `-f, --filter <FILTER>` - Filter by event type
- `--since <SINCE>` - Show events since timestamp

**Examples:**

```bash
# Stream all events
hb system events

# Filter by type
hb system events --filter container

# Recent events (no stream)
hb system events --since 5m
```

---

### hb system benchmark

**Run performance benchmarks**

```bash
hb system benchmark [OPTIONS]
```

**Options:**
- `-a, --all` - Run all benchmarks
- `--compare-docker` - Compare with Docker

**Examples:**

```bash
# Run default benchmarks
hb system benchmark

# Run all benchmarks
hb system benchmark --all

# Compare with Docker
hb system benchmark --compare-docker
```

---

### hb system health

**Health check**

```bash
hb system health
```

**Checks:**
- Daemon connectivity
- Runtime installation (crun)
- Docker/Podman availability

---

### hb health

**Check HyperBox health status**

```bash
hb health
```

Same as `hb system health`. Shortcut command.

---

### hb completion

**Generate shell completions**

```bash
hb completion
```

Generates shell completions for bash, zsh, fish, etc.

---

### hb docker

**Docker CLI compatibility mode**

HyperBox can act as a drop-in replacement for Docker CLI:

```bash
hb docker run nginx
hb docker ps
hb docker build -t myapp .
hb docker pull postgres
```

**Supported commands:**
- `run` - Run a container
- `ps` - List containers
- `start`, `stop`, `restart` - Manage containers
- `rm` - Remove containers
- `images` - List images
- `pull`, `push`, `build` - Image operations
- `exec` - Execute in container
- `logs` - View logs
- `inspect` - Inspect container/image
- `info` - System info
- `version` - Version info

---

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `HYPERBOX_HOME` | HyperBox config directory | `~/.hyperbox` |
| `HYPERBOX_SOCKET` | Daemon socket path | `/run/hyperbox/hyperbox.sock` |
| `HYPERBOX_DEBUG` | Enable debug logging | `1` |
| `HYPERBOX_LOG_LEVEL` | Log level (trace, debug, info, warn, error) | `debug` |
| `HYPERBOX_DAEMON_ADDR` | Daemon address | `localhost:9999` |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error |
| `2` | Command line usage error |
| `125` | Daemon error |
| `126` | Command invocation error |
| `127` | Command not found |
| `130` | Terminated by Ctrl+C |

---

## Configuration Files

### Project Configuration (`.hyperbox/config.toml`)

```toml
[project]
name = "myapp"
version = "1.0.0"

[containers.web]
image = "nginx:latest"
ports = ["8080:80"]

[containers.db]
image = "postgres:15"
environment = { POSTGRES_PASSWORD = "secret" }
volumes = ["/data:/var/lib/postgresql/data"]
```

### Daemon Configuration (`~/.hyperbox/daemon.toml`)

```toml
[daemon]
listen_addr = "127.0.0.1:9999"
sock_path = "/run/hyperbox/hyperbox.sock"

[storage]
backend = "overlayfs"
data_root = "/var/lib/hyperbox"

[runtime]
runtime = "crun"
cgroup_parent = "/hyperbox"
```

---

## Common Use Patterns

### Development Workflow

```bash
# Initialize project
hb project init --template node

# Start services
hb project start --build

# View logs
hb project logs --follow

# Execute tests
hb container exec myapp npm test

# Cleanup
hb project close
```

### CI/CD Integration

```bash
# Build image
hb image build -t myapp:$CI_COMMIT_SHA

# Push to registry
hb image push myregistry.com/myapp:$CI_COMMIT_SHA

# Run tests in container
hb container run --rm myapp:$CI_COMMIT_SHA npm test
```

### Local Testing

```bash
# Run specific container
hb container run -it myapp bash

# Check logs
hb container logs myapp

# Monitor resources
hb container stats myapp

# Cleanup
hb container remove myapp
```

---

## Performance Tips

1. **Use `--detach`**: Start containers in background for faster CLI response
2. **Use tags wisely**: Tag images for quick identification and version management
3. **Manage volumes**: Remove old volumes with `hb system prune --volumes`
4. **Monitor resources**: Use `hb container stats` to track memory/CPU
5. **Build caching**: Use `hb image build` (leverages Docker build cache)

---

## Troubleshooting Commands

```bash
# Check daemon status
hb health

# View system info
hb system info

# Check disk usage
hb system disk-usage

# View recent logs
hb container logs -n 50 myapp

# Get verbose output
hb -vvv project start

# Output as JSON for parsing
hb -o json container list
```

---

## Related Documentation

- [Quick Start Guide](QUICKSTART.md) - Get started in 5 minutes
- [Installation Guide](INSTALLATION_GUIDE.md) - Setup instructions
- [Advanced Operations](ADVANCED_OPERATIONS.md) - Complex scenarios
- [Troubleshooting Guide](TROUBLESHOOTING_GUIDE.md) - Problem solutions
