# HyperBox Production Deployment Guide

## Overview

This guide covers deploying HyperBox in production environments across multiple deployment scenarios:

- Docker Compose (Single host)
- Kubernetes (Cloud-native)
- Systemd (Bare metal)
- Linux container orchestration

## Table of Contents

1. [Pre-Deployment Requirements](#pre-deployment-requirements)
2. [Docker Compose Deployment](#docker-compose-deployment)
3. [Kubernetes Deployment](#kubernetes-deployment)
4. [Systemd Deployment](#systemd-deployment)
5. [Configuration Management](#configuration-management)
6. [Monitoring & Observability](#monitoring--observability)
7. [Troubleshooting](#troubleshooting)
8. [Performance Tuning](#performance-tuning)

---

## Pre-Deployment Requirements

### System Requirements

**Minimum:**

- CPU: 2 cores (4+ cores recommended)
- Memory: 512 MB (1 GB+ recommended)
- Disk: 10 GB free space
- OS: Linux (kernel 4.15+) or Windows with WSL2

**Recommended for Production:**

- CPU: 4+ cores
- Memory: 2-4 GB
- Disk: 50+ GB SSD
- Network: Gigabit or higher
- Multi-node setup for high availability

### Network Requirements

- Inbound: Port 9999 (daemon API)
- Inbound: Port 8888 (metrics/health)
- Outbound: Docker/containerd socket access
- For Kubernetes: CNI network plugin support

### Dependencies

```bash
# Docker Compose
docker >= 20.10
docker-compose >= 1.29

# Kubernetes
kubectl >= 1.21
helm >= 3.0 (optional)

# Systemd
systemd >= 230
```

### Pre-Deployment Checklist

```yaml
Setup:
  - [ ] Review and customize hyperbox-config-templates.conf for your environment
  - [ ] Ensure sufficient disk space on all nodes
  - [ ] Validate DNS resolution for your domain
  - [ ] Configure firewall rules to allow ports 9999 and 8888
  - [ ] Set up log aggregation (optional but recommended)
  - [ ] Configure backup strategy for /var/lib/hyperbox

Credentials:
  - [ ] Generate TLS certificates (if using HTTPS)
  - [ ] Create service account credentials
  - [ ] Set up registry credentials for container pull

Monitoring:
  - [ ] Verify Prometheus is accessible
  - [ ] Verify Grafana dashboards are imported
  - [ ] Test alerting rules
  - [ ] Configure log shipping if using centralized logging
```

---

## Docker Compose Deployment

### Quick Start (Development/Testing)

```bash
# Clone repository
git clone https://github.com/sgbillings/HyperBox.git
cd HyperBox

# Build HyperBox image
docker build -t hyperbox:latest -f Dockerfile .

# Start stack (all services)
docker-compose up -d

# Verify services are running
docker-compose ps

# Check daemon health
curl http://localhost:9999/health

# View logs
docker-compose logs -f hyperboxd
```

### Production Deployment

#### Step 1: Create directories and configuration

```bash
# Create configuration directory
mkdir -p /opt/hyperbox/config
mkdir -p /opt/hyperbox/data

# Copy configuration template
cp config/hyperbox-config-templates.conf /opt/hyperbox/config/hyperbox.toml

# Edit for your environment
nano /opt/hyperbox/config/hyperbox.toml
```

#### Step 2: Create docker-compose.yml override

```bash
# Create local override file
cat > /opt/hyperbox/docker-compose.override.yml << 'EOF'
version: '3.8'

services:
  hyperboxd:
    environment:
      HYPERBOX_LISTEN_ADDR: '0.0.0.0:9999'
      HYPERBOX_DATA_DIR: '/var/lib/hyperbox'
      RUST_LOG: 'warn,hyperbox_core=info'
    volumes:
      - /opt/hyperbox/config:/etc/hyperbox:ro
      - /opt/hyperbox/data:/var/lib/hyperbox
    restart: always

  prometheus:
    volumes:
      - /opt/hyperbox/monitoring/prometheus.yml:/etc/prometheus/prometheus.yml:ro
EOF
```

#### Step 3: Deploy and verify

```bash
# Navigate to deployment directory
cd /opt/hyperbox

# Start services
docker-compose -f docker-compose.yml -f docker-compose.override.yml up -d

# Verify all services started
docker-compose ps

# Check daemon is responding
sleep 5
curl -v http://localhost:9999/health

# Check metrics
curl http://localhost:8888/metrics | head -20

# View daemon logs
docker-compose logs -f hyperboxd --tail=100
```

#### Step 4: Enable auto-start

```bash
# Install Docker Compose as systemd service
sudo systemctl enable docker
sudo systemctl start docker

# Create systemd unit for docker-compose
sudo tee /etc/systemd/system/hyperbox-compose.service << 'EOF'
[Unit]
Description=HyperBox Docker Compose Stack
Requires=docker.service
After=docker.service
BindsTo=docker.service

[Service]
Type=exec
WorkingDirectory=/opt/hyperbox
ExecStart=/usr/bin/docker-compose -f docker-compose.yml -f docker-compose.override.yml up
ExecStop=/usr/bin/docker-compose -f docker-compose.yml -f docker-compose.override.yml down
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable hyperbox-compose
sudo systemctl start hyperbox-compose
```

---

## Kubernetes Deployment

### Prerequisites

```bash
# Verify kubectl connection
kubectl cluster-info

# Install Prometheus Operator (optional but recommended)
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm install kube-prometheus prometheus-community/kube-prometheus-stack \
  --namespace monitoring \
  --create-namespace
```

### Deployment Steps

#### Step 1: Namespace and configuration

```bash
# Create namespace
kubectl create namespace hyperbox

# Label namespace for Prometheus ServiceMonitor
kubectl label namespace hyperbox prometheus=enabled

# Create ConfigMap from template
kubectl create configmap hyperbox-config \
  --from-file=hyperbox.toml=config/hyperbox.toml \
  --from-file=optimization_policy.yaml=config/optimization_policy.yaml \
  --namespace=hyperbox
```

#### Step 2: Build and push Docker image

```bash
# Build image
docker build -t your-registry/hyperbox:latest .

# Push to registry
docker push your-registry/hyperbox:latest

# Update image reference in hyperbox-k8s.yaml
sed -i 's|image: hyperbox:latest|image: your-registry/hyperbox:latest|g' hyperbox-k8s.yaml
```

#### Step 3: Deploy manifests

```bash
# Apply all manifests
kubectl apply -f hyperbox-k8s.yaml

# Verify resources created
kubectl get all -n hyperbox

# Check DaemonSet status
kubectl get daemonset -n hyperbox
kubectl describe daemonset hyperboxd -n hyperbox

# Watch pod rollout
kubectl rollout status daemonset/hyperboxd -n hyperbox
```

#### Step 4: Verify deployment

```bash
# Get daemon pods
kubectl get pods -n hyperbox -o wide

# Check logs
kubectl logs -n hyperbox -l app=hyperbox --tail=50 -f

# Port forward to test locally
kubectl port-forward -n hyperbox svc/hyperboxd 9999:9999 &

# Test health endpoint
curl http://localhost:9999/health

# Kill port forward
jobs
kill %1
```

#### Step 5: Configure ServiceMonitor (if using Prometheus Operator)

```bash
# The ServiceMonitor is already in hyperbox-k8s.yaml
# Verify it's working
kubectl get servicemonitor -n hyperbox
kubectl describe servicemonitor hyperbox -n hyperbox

# Check Prometheus targets
kubectl port-forward -n monitoring svc/kube-prometheus-prometheus 9090:9090 &
# Visit http://localhost:9090/targets
```

#### Step 6: Set up ingress (optional)

```bash
# Create ingress for API access
kubectl apply -f - << 'EOF'
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: hyperbox-api
  namespace: hyperbox
spec:
  ingressClassName: nginx
  rules:
    - host: hyperbox-api.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: hyperboxd
                port:
                  number: 9999
EOF
```

---

## Systemd Deployment

### Prerequisites

```bash
# Install Rust and build tools
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install system dependencies
sudo apt-get install -y build-essential pkg-config libssl-dev

# Create hyperbox user
sudo useradd -m -s /bin/bash hyperbox
```

### Installation Steps

#### Step 1: Build and install binaries

```bash
# Clone and build
git clone https://github.com/sgbillings/HyperBox.git
cd HyperBox

cargo build --release

# Install binaries
sudo install -m755 target/release/hyperboxd /usr/local/bin/
sudo install -m755 target/release/hb /usr/local/bin/

# Create directories
sudo mkdir -p /var/lib/hyperbox
sudo mkdir -p /etc/hyperbox
sudo chown hyperbox:hyperbox /var/lib/hyperbox /etc/hyperbox
```

#### Step 2: Install systemd units

```bash
# Copy units
sudo cp hyperboxd.service /etc/systemd/system/
sudo cp hyperboxd.socket /etc/systemd/system/

# Set permissions
sudo chmod 644 /etc/systemd/system/hyperboxd.*

# Reload systemd
sudo systemctl daemon-reload
```

#### Step 3: Configure and start

```bash
# Copy configuration
sudo cp config/hyperbox.toml /etc/hyperbox/
sudo chown hyperbox:hyperbox /etc/hyperbox/hyperbox.toml

# Edit for your environment
sudo nano /etc/hyperbox/hyperbox.toml

# Enable and start service
sudo systemctl enable hyperboxd.service
sudo systemctl start hyperboxd.service
sudo systemctl start hyperboxd.socket

# Verify status
sudo systemctl status hyperboxd.service
sudo systemctl status hyperboxd.socket

# Check logs
sudo journalctl -u hyperboxd -f
```

#### Step 4: Configure logging

```bash
# Create rsyslog configuration
sudo tee /etc/rsyslog.d/hyperbox.conf << 'EOF'
:programname, isequal, "hyperboxd" /var/log/hyperbox/daemon.log
& stop
EOF

# Create log directory
sudo mkdir -p /var/log/hyperbox
sudo chown syslog:adm /var/log/hyperbox

# Reload rsyslog
sudo systemctl restart rsyslog

# View logs
sudo tail -f /var/log/hyperbox/daemon.log
```

#### Step 5: Set up log rotation

```bash
# Create logrotate configuration
sudo tee /etc/logrotate.d/hyperbox << 'EOF'
/var/log/hyperbox/*.log {
    daily
    rotate 7
    compress
    delaycompress
    notifempty
    create 0640 syslog adm
    sharedscripts
    postrotate
        systemctl reload rsyslog > /dev/null 2>&1 || true
    endscript
}
EOF
```

---

## Configuration Management

### Environment Variables

```bash
# Override variables without editing config file
export HYPERBOX_DATA_DIR=/var/lib/hyperbox
export HYPERBOX_CONFIG_DIR=/etc/hyperbox
export HYPERBOX_LISTEN_ADDR=0.0.0.0:9999
export RUST_LOG=info,hyperbox_core=debug

# Pass to daemon
/usr/local/bin/hyperboxd --config=/etc/hyperbox/hyperbox.toml
```

### Configuration Templates

Use templates from `config/hyperbox-config-templates.conf`:

```bash
# Production template
cp config/hyperbox-config-templates.conf /etc/hyperbox/hyperbox.toml
# Then customize for your environment

# Development template
sed -n '/Development Configuration/,/---/p' config/hyperbox-config-templates.conf | tail -n +2 > /tmp/hyperbox.toml
```

### Secrets Management

#### Using Kubernetes Secrets

```bash
# Create secrets
kubectl create secret generic hyperbox-credentials \
  --from-literal=db-password=your-secure-password \
  --namespace=hyperbox

# Reference in pod
env:
  - name: DB_PASSWORD
    valueFrom:
      secretKeyRef:
        name: hyperbox-credentials
        key: db-password
```

#### Using HashiCorp Vault

```bash
# Install Vault agent injector
helm repo add hashicorp https://helm.releases.hashicorp.com
helm install vault hashicorp/vault --namespace vault --create-namespace

# Annotate pods for secret injection
annotations:
  vault.hashicorp.com/agent-inject: "true"
  vault.hashicorp.com/role: "hyperbox"
  vault.hashicorp.com/agent-inject-secret-config: "secret/data/hyperbox/config"
```

---

## Monitoring & Observability

### Prometheus Metrics

HyperBox exposes metrics at `http://localhost:8888/metrics`:

```bash
# Scrape metrics
curl http://localhost:8888/metrics

# Key metrics to monitor:
# - hyperbox_memory_usage_bytes: Current memory usage
# - hyperbox_startup_time_ms: Container startup time
# - hyperbox_dedup_effectiveness: Deduplication percentage
# - hyperbox_prediction_accuracy: Model accuracy
# - hyperbox_cache_hit_ratio: Cache effectiveness
```

### Grafana Dashboard

```bash
# Import dashboard
curl -s https://raw.githubusercontent.com/sgbillings/HyperBox/main/monitoring/grafana/dashboards/hyperbox.json \
  | curl -X POST http://admin:admin@localhost:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @-
```

### Alert Rules

```yaml
groups:
    - name: hyperbox
      rules:
          - alert: HyperBoxDaemonDown
            expr: up{job="hyperbox"} == 0
            for: 2m
            annotations:
                summary: "HyperBox daemon is down"

          - alert: HighMemoryUsage
            expr: hyperbox_memory_usage_bytes > 900000000 # 900MB
            for: 5m
            annotations:
                summary: "HyperBox using high memory"

          - alert: LowCacheHitRatio
            expr: hyperbox_cache_hit_ratio < 0.5
            for: 10m
            annotations:
                summary: "Cache hit ratio below 50%"
```

### Distributed Tracing

```bash
# Enable tracing in config
[monitoring]
tracing_enabled = true
jaeger_agent_host = "localhost"
jaeger_agent_port = 6831

# Deploy Jaeger
docker run -d --name jaeger \
  -e COLLECTOR_ZIPKIN_HTTP_PORT=9411 \
  -p 6831:6831/udp \
  -p 16686:16686 \
  jaegertracing/all-in-one:latest
```

---

## Troubleshooting

### Common Issues

#### 1. Daemon won't start

```bash
# Check logs
journalctl -u hyperboxd -n 50

# Check configuration syntax
/usr/local/bin/hyperboxd --config=/etc/hyperbox/hyperbox.toml --validate-config

# Verify permissions
ls -la /var/lib/hyperbox
ls -la /etc/hyperbox

# Check port availability
sudo lsof -i :9999
sudo netstat -tulpn | grep 9999
```

#### 2. Docker socket access denied

```bash
# Add user to docker group
sudo usermod -aG docker hyperbox

# For Kubernetes, verify hostPath volumes
kubectl describe pod <pod-name> -n hyperbox | grep -A 5 "Mounts:"

# Check socket permissions
ls -la /var/run/docker.sock
ls -la /var/run/containerd
```

#### 3. High memory usage

```bash
# Check current metrics
curl http://localhost:8888/metrics | grep memory_usage

# Reduce cache size in config
sed -i 's/size_mb = 2048/size_mb = 512/' /etc/hyperbox/hyperbox.toml

# Restart daemon
systemctl restart hyperboxd
```

#### 4. Slow performance

```bash
# Check CPU usage
hostnamectl; cat /proc/cpuinfo | grep processor | wc -l

# Check disk I/O
iostat -x 1 10

# Increase parallel operations
sed -i 's/parallel_ops = 4/parallel_ops = 8/' /etc/hyperbox/hyperbox.toml

# Check for network bottlenecks
iftop -n
```

### Debug Commands

```bash
# Health check
curl -v http://localhost:9999/health

# Detailed metrics
curl http://localhost:8888/metrics | grep hyperbox

# Check daemon process
ps aux | grep hyperboxd

# System resource usage
top -p $(pgrep -f hyperboxd)

# Network connections
netstat -an | grep 9999

# File descriptors
lsof -p $(pgrep -f hyperboxd) | wc -l

# Memory usage
pmap -x $(pgrep -f hyperboxd) | tail -1
```

---

## Performance Tuning

### CPU Optimization

```toml
[prewarming]
# Match CPU core count
parallel_ops = 8  # For 8-core system

# Enable SIMD optimizations
vectorize = true
```

### Memory Optimization

```toml
[cache]
# Balance between cache size and memory
size_mb = 1024

# Enable compression for memory savings
compression = true
compression_level = 6  # 1-9, higher = more compression

# Use mmap for large datasets
use_mmap = true
```

### Network Optimization

```toml
[daemon]
# Adjust buffer sizes based on network
max_connections = 1000
connection_timeout_secs = 30

# Enable TCP keepalive
tcp_keepalive = true
tcp_keepalive_interval_secs = 60
```

### I/O Optimization

```toml
[cache]
# Use faster storage for cache
cache_path = /data/fast-ssd/hyperbox

# Batch writes for throughput
batch_writes = true
batch_size = 100
```

### Monitoring Performance

```bash
# Collect baseline metrics
for i in {1..60}; do
  echo "Iteration $i"
  curl -s http://localhost:8888/metrics | grep hyperbox_
  sleep 60
done > /tmp/baseline.txt

# Analyze results
cat /tmp/baseline.txt | grep memory_usage
cat /tmp/baseline.txt | grep cache_hit_ratio
```

---

## Next Steps

1. Complete deployment in your environment
2. Run integration tests: `cargo test --test container_scenarios`
3. Run benchmarks: `cargo bench -p hyperbox-optimize`
4. Monitor performance for 24-48 hours
5. Fine-tune configuration based on workload
6. Document custom configuration for your team
7. Set up backup and disaster recovery

## Support & Resources

- **Documentation**: https://github.com/sgbillings/HyperBox
- **Issues**: https://github.com/sgbillings/HyperBox/issues
- **Community**: https://github.com/sgbillings/HyperBox/discussions
