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
```
