# HyperBox Troubleshooting Guide

## Quick Diagnostics

### Health Check Commands

```bash
# Test daemon connectivity
curl -v http://localhost:9999/health

# Get daemon metrics
curl http://localhost:8888/metrics

# Check daemon process
ps aux | grep hyperboxd

# View recent logs
journalctl -u hyperboxd -n 50
# Or for Docker:
docker logs hyperboxd --tail=50
```

---

## Startup Issues

### Problem: Daemon fails to start

**Symptoms:**

- `systemctl status hyperboxd` shows failed state
- `docker-compose up` exits immediately
- Error: "Address already in use"

**Debugging:**

```bash
# 1. Check if port is in use
sudo lsof -i :9999
sudo netstat -tulpn | grep 9999

# 2. Free the port
sudo kill -9 <PID>
# Or change port in config:
sed -i 's/listen_addr = "0.0.0.0:9999"/listen_addr = "0.0.0.0:9998"/' /etc/hyperbox/hyperbox.toml

# 3. Validate configuration
/usr/local/bin/hyperboxd --config=/etc/hyperbox/hyperbox.toml --dry-run

# 4. Check file permissions
ls -la /etc/hyperbox/
ls -la /var/lib/hyperbox/

# 5. Review full logs
journalctl -u hyperboxd -n 100 --no-pager
```

**Solutions:**

| Issue                 | Solution                                               |
| --------------------- | ------------------------------------------------------ |
| Port already in use   | Kill existing process or change port                   |
| Config file not found | Verify path in ExecStart command                       |
| Permission denied     | `sudo chown hyperbox:hyperbox /var/lib/hyperbox`       |
| Invalid TOML syntax   | Validate with `toml-lint` or `cargo install tomli-cli` |

---

### Problem: Out of memory on startup

**Symptoms:**

- OOMKilled in Kubernetes
- "Killed" message in systemd logs
- Daemon exits unexpectedly

**Debugging:**

```bash
# Check memory availability
free -h

# Check memory limits in systemd
systemctl show hyperboxd | grep Memory

# Check memory limits in Kubernetes
kubectl describe pod <pod-name> -n hyperbox | grep -A 5 "Limits:"

# Monitor memory during startup
watch -n 1 'ps aux | grep hyperboxd | grep -v grep || echo "Process not running"'
```

**Solutions:**

```toml
# Reduce cache size in /etc/hyperbox/hyperbox.toml
[cache]
size_mb = 256  # Reduce from 512

# Disable compression memory overhead
compression = false

# Disable prediction models
[prediction]
enabled = false
```

For Kubernetes:

```yaml
resources:
    requests:
        memory: "512Mi"
    limits:
        memory: "2Gi"
```

---

## Runtime Issues

### Problem: High CPU usage

**Symptoms:**

- `top` shows hyperboxd using >80% CPU
- High system load
- Slow container operations

**Debugging:**

```bash
# Check which threads are busy
ps -eLf | grep hyperboxd

# Get CPU details
lscpu

# Check current load
uptime
cat /proc/loadavg

# Monitor in real-time
top -p $(pgrep -f hyperboxd)

# Check prewarming activity
curl http://localhost:8888/metrics | grep prewarming
```

**Solutions:**

```toml
# Reduce parallel operations
[prewarming]
parallel_ops = 2  # Reduce from 8

# Lower prediction update frequency
[prediction]
update_interval_secs = 600  # Increase from 300

# Disable expensive features
deduplication = false  # Temporarily disable
prewarming = false
```

Or systemd limits:

```ini
CPUQuota=100%  # Limit to 1 core
```

---

### Problem: High memory usage

**Symptoms:**

- Memory usage grows over time
- OOMKilled after hours of operation
- `free -h` shows low available memory

**Debugging:**

```bash
# Check memory map
pmap -x $(pgrep -f hyperboxd) | tail -1

# Get detailed memory from metrics
curl http://localhost:8888/metrics | grep memory

# Check for memory leaks with valgrind
valgrind --leak-check=full /usr/local/bin/hyperboxd

# Monitor memory over time
watch -n 5 'ps aux | grep hyperboxd | grep -v grep | awk "{print \$6}"'
```

**Solutions:**

```toml
# Reduce cache size
[cache]
size_mb = 128  # Reduce from 512

# Enable compression
compression = true

# Reduce history retention
[prediction]
history_days = 7  # Reduce from 30
max_predictions = 5  # Reduce from 20
```

For systemd:

```ini
MemoryLimit=512M
```

---

### Problem: Slow performance

**Symptoms:**

- Container operations take >5 seconds
- Low cache hit ratio
- High latency in metrics

**Debugging:**

```bash
# Check cache hit ratio
curl http://localhost:8888/metrics | grep cache_hit_ratio

# Check deduplication effectiveness
curl http://localhost:8888/metrics | grep dedup_

# Check prediction accuracy
curl http://localhost:8888/metrics | grep prediction_accuracy

# Check startup times
curl http://localhost:8888/metrics | grep startup_time

# Monitor disk I/O
iostat -x 1 10

# Check network latency
ping <docker-daemon-host>
```

**Solutions:**

```toml
# Increase cache size
[cache]
size_mb = 2048  # Increase from 512

# Enable all optimizations
[deduplication]
enabled = true

[prediction]
enabled = true

[prewarming]
enabled = true
parallel_ops = 8
```

---

## Docker Issues

### Problem: "docker.sock: Permission denied"

**Symptoms:**

```
Error: permission denied while trying to connect to Docker daemon
```

**Debugging:**

```bash
# Check socket permissions
ls -la /var/run/docker.sock

# Check user group membership
groups hyperbox

# Verify container capabilities
docker inspect hyperboxd | grep -A 10 CapAdd
```

**Solutions:**

```bash
# Add user to docker group
sudo usermod -aG docker hyperbox

# Or modify Docker socket permissions
sudo chmod 666 /var/run/docker.sock  # ⚠️ Security risk in production

# Better: Use Docker daemon socket in compose
volumes:
  - /var/run/docker.sock:/var/run/docker.sock:ro
```

---

### Problem: "Failed to pull image"

**Symptoms:**

```
Failed to pull image 'hyperbox:latest': pull access denied
```

**Debugging:**

```bash
# Check Docker daemon connectivity
docker ps

# Verify image exists
docker images | grep hyperbox

# Check registry connectivity
curl https://registry.example.com/v2/  # For private registry

# Check authentication
cat /root/.docker/config.json | grep -A 5 auths
```

**Solutions:**

```bash
# Build image locally
docker build -t hyperbox:latest .

# Or push to accessible registry
docker tag hyperbox:latest myregistry.azurecr.io/hyperbox:latest
docker push myregistry.azurecr.io/hyperbox:latest

# Update docker-compose reference
sed -i 's|image: hyperbox:latest|image: myregistry.azurecr.io/hyperbox:latest|' docker-compose.yml
```

---

## Kubernetes Issues

### Problem: Pod CrashLoopBackOff

**Symptoms:**

```
0/1       CrashLoopBackOff   5          2m
```

**Debugging:**

```bash
# Get pod status
kubectl get pods -n hyperbox -o wide

# Check pod events
kubectl describe pod <pod-name> -n hyperbox

# View logs
kubectl logs <pod-name> -n hyperbox
kubectl logs <pod-name> -n hyperbox --previous  # Previous run

# Check resource constraints
kubectl top pod <pod-name> -n hyperbox
kubectl describe node <node-name>
```

**Solutions:**

```yaml
# Increase resource limits
resources:
    requests:
        memory: "512Mi"
        cpu: "500m"
    limits:
        memory: "2Gi"
        cpu: "2000m"

# Add startup wait time
startupProbe:
    httpGet:
        path: /health
        port: 8888
    failureThreshold: 30
    periodSeconds: 10
```

---

### Problem: ImagePullBackOff

**Symptoms:**

```
0/1       ImagePullBackOff   0          2m
```

**Debugging:**

```bash
# Check image pull details
kubectl describe pod <pod-name> -n hyperbox | grep -A 5 "Events:"

# Verify image exists in registry
kubectl auth can-i get secrets --as=system:serviceaccount:hyperbox:hyperbox
```

**Solutions:**

```bash
# Create image pull secret
kubectl create secret docker-registry regcred \
  --docker-server=myregistry.azurecr.io \
  --docker-username=<username> \
  --docker-password=<password> \
  -n hyperbox

# Add to deployment
imagePullSecrets:
  - name: regcred
```

---

### Problem: Pod stuck in Pending

**Symptoms:**

```
0/3       Pending   0          5m
```

**Debugging:**

```bash
# Check node availability
kubectl get nodes
kubectl describe nodes

# Check resource availability
kubectl top nodes
kubectl describe node <node-name>

# Check pod requirements
kubectl describe pod <pod-name> -n hyperbox | grep -A 5 "Requests:"
```

**Solutions:**

```bash
# Add more nodes to cluster
kubectl scale nodes --min=3 --max=10

# Reduce pod resource requests
resources:
  requests:
    memory: "256Mi"
    cpu: "250m"

# Drain node and let pods reschedule
kubectl drain <node-name> --ignore-daemonsets --delete-emptydir-data
```

---

### Problem: Service not accessible

**Symptoms:**

```
curl: (7) Failed to connect to service
Connection refused
```

**Debugging:**

```bash
# Check service exists
kubectl get svc -n hyperbox

# Check service has endpoints
kubectl get endpoints -n hyperbox

# Check network policy
kubectl get networkpolicies -n hyperbox

# Test connectivity from pod
kubectl run -it --rm debug --image=busybox --restart=Never -- /bin/sh
# Inside: nc -zv hyperboxd.hyperbox.svc 9999
```

**Solutions:**

```bash
# Fix service selector
kubectl get pods -n hyperbox --show-labels
# Verify labels match service selector

# Check network policy
kubectl describe networkpolicies -n hyperbox

# Allow traffic temporarily
kubectl delete networkpolicies -n hyperbox --all
```

---

## Systemd Issues

### Problem: Socket activation not working

**Symptoms:**

```
Failed to get D-Bus connection
Socket unit not found
```

**Debugging:**

```bash
# Check socket status
systemctl status hyperboxd.socket

# Check socket binding
ss -ltn | grep 9999

# Check socket unit
systemctl cat hyperboxd.socket

# Check service unit
systemctl cat hyperboxd.service
```

**Solutions:**

```bash
# Reload units
systemctl daemon-reload

# Ensure both units exist
ls -la /etc/systemd/system/hyperboxd.*

# Restart socket
systemctl restart hyperboxd.socket
systemctl restart hyperboxd.service

# Verify
systemctl status hyperboxd.socket
systemctl status hyperboxd.service
```

---

### Problem: Service doesn't auto-restart

**Symptoms:**

```
Restart=on-failure not working
Service runs once then stops
```

**Debugging:**

```bash
# Check restart policy
systemctl cat hyperboxd.service | grep -i restart

# Monitor restarts
while true; do
  systemctl is-active hyperboxd && echo "Running" || echo "Stopped"
  sleep 1
done

# Check journal for failures
journalctl -u hyperboxd | grep -i "exited"
```

**Solutions:**

```ini
# Update /etc/systemd/system/hyperboxd.service
Restart=on-failure
RestartSec=5
StartLimitBurst=0
StartLimitIntervalSec=0
```

Then reload:

```bash
systemctl daemon-reload
systemctl restart hyperboxd.service
```

---

## Monitoring Issues

### Problem: Metrics not collected

**Symptoms:**

```
curl http://localhost:8888/metrics: Connection refused
No metrics in Prometheus
```

**Debugging:**

```bash
# Check if metrics port is open
netstat -tulpn | grep 8888
lsof -i :8888

# Check daemon config
grep -A 5 "\[monitoring\]" /etc/hyperbox/hyperbox.toml

# View prometheus config
cat /etc/prometheus/prometheus.yml | grep -A 10 "hyperbox"

# Check prometheus targets
curl http://localhost:9090/api/v1/targets | jq .
```

**Solutions:**

```toml
# Enable in config
[monitoring]
metrics_enabled = true
metrics_port = 8888
```

For Prometheus config:

```yaml
scrape_configs:
    - job_name: "hyperbox"
      static_configs:
          - targets: ["localhost:8888"]
      scrape_interval: 30s
```

---

### Problem: Missing alerts

**Symptoms:**

```
No firing alerts in Alertmanager
AlertManager queue empty
```

**Debugging:**

```bash
# Check Prometheus alerts
curl http://localhost:9090/api/v1/rules | jq '.data.groups[].rules'

# Check AlertManager
curl http://localhost:9093/file

# View alert history
curl http://localhost:9090/api/v1/query_range?query=ALERTS&start=...&end=...
```

**Solutions:**

```bash
# Reload prometheus rules
curl -X POST http://localhost:9090/-/reload

# Restart alert manager
systemctl restart alertmanager

# Check rule syntax
promtool check rules /etc/prometheus/rules.yml
```

---

## Configuration Issues

### Problem: Invalid TOML syntax

**Symptoms:**

```
Error: expected key
Thread 'main' panicked at 'Failed to parse configuration'
```

**Debugging:**

```bash
# Validate TOML
cat /etc/hyperbox/hyperbox.toml | toml-lint

# Or with cargo
cargo run --example validate_config -- /etc/hyperbox/hyperbox.toml

# Python fallback
python3 -c "import toml; toml.load(open('/etc/hyperbox/hyperbox.toml'))"
```

**Solutions:**

```bash
# Check for common mistakes
grep '= ""' /etc/hyperbox/hyperbox.toml  # Missing quotes
grep '\t' /etc/hyperbox/hyperbox.toml  # Tabs instead of spaces
grep -E '^[^[]' /etc/hyperbox/hyperbox.toml | head  # Missing section headers

# Use provided templates
cp config/hyperbox-config-templates.conf /etc/hyperbox/hyperbox.toml
nano /etc/hyperbox/hyperbox.toml
```

---

### Problem: Settings not being applied

**Symptoms:**

```
Config changes ignored
Settings not reflected in behavior
Cache size doesn't change
```

**Debugging:**

```bash
# Verify config file modified
stat /etc/hyperbox/hyperbox.toml
ls -la /etc/hyperbox/hyperbox.toml

# Check daemon is reading new config
grep "config\|Config" /proc/$(pgrep -f hyperboxd)/fd/* 2>/dev/null

# See actual running config
cat /proc/$(pgrep -f hyperboxd)/environ | tr '\0' '\n' | grep HYPERBOX
```

**Solutions:**

```bash
# Restart daemon for config reload
systemctl restart hyperboxd

# Or for docker-compose
docker-compose restart hyperboxd

# Or for Kubernetes
kubectl rollout restart daemonset/hyperboxd -n hyperbox

# Verify new settings applied
journalctl -u hyperboxd -n 20 | grep -i "config\|loaded"
```

---

## Performance Analysis

### Find Performance Bottleneck

```bash
#!/bin/bash
# Comprehensive performance diagnosis

echo "=== System Resources ==="
free -h
df -h /var/lib/hyperbox
top -b -n 1 | head -5

echo "=== HyperBox Metrics ==="
curl -s http://localhost:8888/metrics | grep -E "hyperbox_(memory|cache|startup|dedup|prediction)"

echo "=== I/O Performance ==="
iostat -x 1 3

echo "=== Network ==="
netstat -an | grep 9999 | wc -l
```

---

## Getting Help

```bash
# Collect diagnostics
cat > /tmp/hyperbox-diagnostics.sh << 'EOF'
#!/bin/bash
echo "=== System Info ==="
uname -a
echo ""

echo "=== HyperBox Version ==="
hyperboxd --version
echo ""

echo "=== Configuration ==="
cat /etc/hyperbox/hyperbox.toml
echo ""

echo "=== Recent Logs (last 100 lines) ==="
journalctl -u hyperboxd -n 100
echo ""

echo "=== Current Metrics ==="
curl -s http://localhost:8888/metrics
echo ""

echo "=== Health Status ==="
curl -s http://localhost:9999/health
echo ""

echo "=== Process Info ==="
ps aux | grep hyperboxd | grep -v grep
echo ""

echo "=== Port Status ==="
netstat -tulpn | grep 9999
echo ""
EOF

chmod +x /tmp/hyperbox-diagnostics.sh
/tmp/hyperbox-diagnostics.sh > /tmp/hyperbox-diagnostics.log

# Share the log
cat /tmp/hyperbox-diagnostics.log
```

---

## Common Error Messages

| Error                    | Meaning                  | Solution                       |
| ------------------------ | ------------------------ | ------------------------------ |
| `Address already in use` | Port 9999 occupied       | Change port or kill process    |
| `Permission denied`      | Insufficient permissions | Run as root or add to group    |
| `Out of memory`          | Memory exhausted         | Reduce cache, disable features |
| `Connection refused`     | Daemon not running       | Start daemon, check logs       |
| `Too many open files`    | File descriptor limit    | Increase ulimit                |
| `Stream closed`          | Client disconnect        | Normal, check if persistent    |

---

## Network Issues

### Problem: Container can't reach external network

**Symptoms:**
```
Failed to resolve DNS
curl: (7) Failed to connect to remote host
Network unreachable
```

**Debugging:**

```bash
# Test DNS resolution from container
hb container exec myapp nslookup google.com

# Check network connectivity
hb container exec myapp ping 8.8.8.8

# Check routing
hb container exec myapp ip route

# Check iptables rules
sudo iptables -t nat -L -n | grep DOCKER
```

**Solutions:**

```bash
# Enable IP forwarding (Linux)
sudo sysctl -w net.ipv4.ip_forward=1

# Persist the change
echo "net.ipv4.ip_forward=1" | sudo tee /etc/sysctl.d/99-hyperbox.conf

# Reload
sudo sysctl -p /etc/sysctl.d/99-hyperbox.conf

# Check DNS in container
hb container exec myapp cat /etc/resolv.conf
```

---

### Problem: Port mapping not working

**Symptoms:**

```
curl http://localhost:8080: Connection refused
Service accessible inside container but not from host
```

**Debugging:**

```bash
# Check port is actually mapped
hb container inspect myapp | grep -A 20 "PortBindings"

# Check if port is listening
netstat -tulpn | grep 8080
ss -tulpn | grep 8080

# Test from inside container
hb container exec myapp curl http://localhost:80

# Check firewall
sudo firewall-cmd --list-all | grep 8080
sudo iptables -L -n | grep 8080
```

**Solutions:**

```bash
# Expose port when running
hb container run -p 8080:80 nginx

# Or modify network settings
docker network ls
docker inspect bridge | grep "Gateway"

# On Windows, check Hyper-V
Get-NetNat

# On macOS, check network setup
ifconfig | grep -A 5 docker
```

---

### Problem: Container DNS resolution fails

**Symptoms:**

```
host: Name or service not known
getaddrinfo failed
nslookup: command not found
```

**Debugging:**

```bash
# Check if DNS server is running
systemctl status systemd-resolved

# Check container DNS config
hb container exec myapp cat /etc/resolv.conf

# Test DNS queries
hb container exec myapp dig google.com

# Check daemon DNS settings
grep -i dns /etc/hyperbox/hyperbox.toml
```

**Solutions:**

```bash
# Start systemd-resolved
sudo systemctl start systemd-resolved

# Or specify custom DNS when running
hb container run \
  --env "RUN_OPTIONS=--dns 8.8.8.8" \
  myapp

# Update daemon config
cat >> /etc/hyperbox/hyperbox.toml << 'EOF'
[dns]
servers = ["8.8.8.8", "1.1.1.1"]
EOF

sudo systemctl restart hyperboxd
```

---

## Volume Issues

### Problem: Volume mount permission denied

**Symptoms:**

```
Permission denied (os error 13)
Cannot write to mounted volume
```

**Debugging:**

```bash
# Check mount points
hb container inspect myapp | grep "Mounts" -A 20

# Check permissions on host
ls -la /host/path/

# Check inside container
hb container exec myapp ls -la /app/

# Check user ID mapping
hb container exec myapp id
```

**Solutions:**

```bash
# Fix permissions on host
sudo chown 1000:1000 /host/path/
chmod 755 /host/path/

# Or run container with specific user
hb container run \
  --user 1000:1000 \
  -v /host/path:/app \
  myapp

# Or use SELinux context
hb container run \
  -v /host/path:/app:z \
  myapp  # z = shared, Z = private
```

---

### Problem: Volume data not persisting

**Symptoms:**

```
Data lost after container stops
Volume appears empty
```

**Debugging:**

```bash
# List volumes
hb container inspect myapp | grep Volumes

# Check actual volume location
docker volume ls
docker volume inspect myapp_data

# Check mount point
mount | grep docker

# Verify data exists
ls -la /var/lib/docker/volumes/myapp_data/_data/
```

**Solutions:**

```bash
# Use named volumes properly
hb container run \
  -v myapp_data:/app/data \
  myapp

# Or verify bind mount
hb container run \
  -v /absolute/path:/app/data \
  myapp

# Check if container has write permissions
hb container exec myapp touch /app/data/test.txt
```

---

## Image Issues

### Problem: Image build failures

**Symptoms:**

```
Build failed: command not found
RUN step exited with code 127
```

**Debugging:**

```bash
# Build with verbose output
hb image build --build-arg BUILDKIT_INLINE_CACHE=1 -t myapp:debug .

# Check Dockerfile syntax
docker run --rm -i hadolint/hadolint < Dockerfile

# Test base image
hb container run alpine echo "test"
```

**Solutions:**

```bash
# Ensure base image exists
hb image pull alpine:latest

# Update Dockerfile with proper paths
RUN which python3 || apt-get install -y python3

# Or use specific base image version
FROM python:3.11-slim  # Instead of python:latest
```

---

### Problem: Image pull timeout

**Symptoms:**

```
timeout waiting for connection
context deadline exceeded
```

**Debugging:**

```bash
# Check network connectivity
curl https://registry.hub.docker.com/

# Check Docker daemon
ps aux | grep docker

# Monitor pull progress
hb image pull --verbose nginx
```

**Solutions:**

```bash
# Build locally instead
hb image build -t myapp:latest .

# Use private registry with shorter timeout
timeout 300 hb image pull myregistry/image:tag

# Increase daemon timeout
cat >> /etc/hyperbox/hyperbox.toml << 'EOF'
[registry]
timeout_secs = 600  # 10 minutes
EOF
```

---

## Logging Issues

### Problem: Logs not showing

**Symptoms:**

```
hb container logs: empty output
No container output
```

**Debugging:**

```bash
# Check if container is running
hb container list

# Check logs are being captured
docker logs myapp

# Check log driver
hb container inspect myapp | grep LogDriver

# Check file system for logs
find /var/lib/docker -name "*.log" | head -5
```

**Solutions:**

```bash
# Ensure application writes to stdout
hb container run \
  -e LOG_LEVEL=debug \
  myapp

# Check if logs are disabled
docker inspect myapp | grep -A 10 "LogConfig"

# Use different log driver
docker run --log-driver json-file myapp
```

---

## Daemon Communication

### Problem: CLI can't reach daemon

**Symptoms:**

```
Error: Cannot connect to hyperbox daemon
dial unix /run/hyperbox/hyperbox.sock: no such file or directory
```

**Debugging:**

```bash
# Check if daemon running
ps aux | grep hyperboxd

# Check socket file
ls -la /run/hyperbox/hyperbox.sock
ls -la /tmp/hyperbox/hyperbox.sock
ls -la ~/.hyperbox/

# Test socket connectivity
socat - UNIX-CONNECT:/run/hyperbox/hyperbox.sock

# Check permissions
stat /run/hyperbox/hyperbox.sock
```

**Solutions:**

```bash
# Start daemon
hb system daemon start

# Or as service
sudo systemctl start hyperboxd

# Fix socket permissions
sudo chmod 666 /run/hyperbox/hyperbox.sock

# Set socket path explicitly
export HYPERBOX_SOCKET=/run/hyperbox/hyperbox.sock
hb project list
```

---

## Environment & Variables

### Problem: Environment variables not set

**Symptoms:**

```
Application can't find DATABASE_URL
echo $DATABASE_URL returns empty
```

**Debugging:**

```bash
# Check env vars in container
hb container exec myapp env | sort

# Check what was passed
hb container inspect myapp | grep -A 20 "Env"

# Verify in shell
hb container exec -it myapp /bin/sh
# Inside: echo $DATABASE_URL
```

**Solutions:**

```bash
# Set when running
hb container run \
  -e DATABASE_URL=postgres://... \
  -e NODE_ENV=production \
  myapp

# Or in Dockerfile
ENV DATABASE_URL=default_value
ENV NODE_ENV=production

# Or via file
cat > .env << 'EOF'
DATABASE_URL=postgres://user:pass@localhost/db
NODE_ENV=production
EOF

hb container run --env-file .env myapp
```

---

## Resource Issues

### Problem: Container OOMKilled

**Symptoms:**

```
Killed (exit code 137)
Out of memory: Kill process
Memory limit exceeded
```

**Debugging:**

```bash
# Check memory usage before it crashes
hb container stats myapp

# Check memory limit
hb container inspect myapp | grep Memory

# Monitor memory over time
watch -n 1 'hb container stats myapp --no-stream'
```

**Solutions:**

```bash
# Increase memory limit
hb container run \
  -m 2g \  # 2 gigabytes
  myapp

# Or reduce memory usage
# - Use smaller base image (alpine instead of ubuntu)
# - Optimize application code
# - Enable garbage collection
```

---

## CLI/Command Issues

### Problem: Command not found

**Symptoms:**

```
hb: command not found
bash: hb: No such file or directory
```

**Debugging:**

```bash
# Check if binary exists
which hb
ls -la /usr/local/bin/hb

# Check PATH
echo $PATH

# Try with full path
/usr/local/bin/hb --version
```

**Solutions:**

```bash
# Add to PATH
export PATH="/usr/local/bin:$PATH"

# Or create symlink
sudo ln -s /usr/local/bin/hb /usr/bin/hb

# Or install properly
curl -L https://releases.hyperbox.io/install.sh | bash
```

---

### Problem: Permission denied running hb

**Symptoms:**

```
Permission denied: /usr/local/bin/hb
Cannot execute binary file
```

**Debugging:**

```bash
# Check permissions
ls -la /usr/local/bin/hb

# Check architecture
file /usr/local/bin/hb
uname -m
```

**Solutions:**

```bash
# Make executable
sudo chmod +x /usr/local/bin/hb

# Or reinstall for correct architecture
uname -m  # Check if x86_64, arm64, etc
# Download matching binary
```

---

## Quick Fixes

```bash
# Restart everything
systemctl restart hyperboxd.socket hyperboxd.service

# Clear cache
rm -rf /var/lib/hyperbox/*
systemctl restart hyperboxd

# Reset to defaults
cp config/hyperbox-config-templates.conf /etc/hyperbox/hyperbox.toml
systemctl restart hyperboxd

# Emergency: Run with minimal config
hyperboxd --config=/dev/null  # Use all defaults

# Full system reset (careful!)
hb system prune --all --volumes --force
hb system daemon restart
```

---

## Escalation Checklist

Before opening a GitHub issue, verify:

- [ ] Daemon is running: `hb health`
- [ ] Latest version: `hb system version`
- [ ] Config is valid: `cat /etc/hyperbox/hyperbox.toml | toml-lint`
- [ ] System has resources: `free -h` and `df -h`
- [ ] No port conflicts: `sudo lsof -i :9999`
- [ ] Logs collected: Run diagnostics script above
- [ ] Reproducible: Can you repeat the issue?

When opening issue, include:
- Full error message
- Steps to reproduce
- Output of `/tmp/hyperbox-diagnostics.log`
- HyperBox version and OS
