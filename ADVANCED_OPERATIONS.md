# HyperBox Advanced Operations Guide

## Overview

This guide addresses advanced operational patterns for production HyperBox deployments across multiple environments (Docker Compose, Kubernetes, Systemd).

---

## Cluster Management

### Kubernetes Multi-Node Deployment

#### Rolling Updates (Zero-Downtime)

```bash
# Strategy 1: Update DaemonSet image (all nodes)
kubectl set image daemonset/hyperboxd \
  hyperboxd=myregistry.azurecr.io/hyperbox:v2.0 \
  -n hyperbox

# Monitor rollout
kubectl rollout status daemonset/hyperboxd -n hyperbox --timeout=10m

# Rollback if needed
kubectl rollout undo daemonset/hyperboxd -n hyperbox
```

#### Selective Node Updates

```bash
# Drain node (evict pods, mark unschedulable)
kubectl drain node-01 --ignore-daemonsets --delete-emptydir-data

# Update or upgrade node
# (e.g., kernel patch, docker upgrade)

# Uncordon node (make schedulable again)
kubectl uncordon node-01

# Verify pod redistribution
kubectl get pods -n hyperbox -o wide
```

#### Canary Deployments

```bash
# 1. Deploy new version alongside old
kubectl set image daemonset/hyperboxd \
  hyperboxd=myregistry.azurecr.io/hyperbox:v2.0 \
  -n hyperbox \
  --record

# 2. Monitor metrics (30 minutes)
watch -n 5 'curl http://localhost:8888/metrics'

# 3. If issues, rollback
kubectl rollout undo daemonset/hyperboxd -n hyperbox

# 4. If healthy, complete rollout (already complete for DaemonSet)
```

### Docker Compose Multi-Container Management

#### Blue-Green Deployment

```yaml
# docker-compose.current.yml
version: "3.8"
services:
    hyperboxd-blue:
        image: hyperbox:v1.0
        ports:
            - "9999:9999"
        networks:
            - hyperbox_network

    hyperboxd-green:
        image: hyperbox:v2.0
        ports:
            - "9998:9999" # Different port
        networks:
            - hyperbox_network
```

```bash
# Deploy green (new version)
docker-compose -f docker-compose.yml up -d hyperboxd-green

# Test green
curl http://localhost:9998/health

# Switch traffic (update load balancer or reverse proxy)
# nginx.conf: upstream hyperbox { server hyperboxd-green:9999; }
dockerexec nginx nginx -s reload

# Monitor green (30 min)
# If healthy, remove blue
docker-compose down hyperboxd-blue

# Rename green to blue for next rotation
docker-compose rename hyperboxd-green hyperboxd-blue
```

### Systemd Fleet Management

For managing multiple HyperBox instances across servers:

```ini
# /etc/systemd/system/hyperboxd@.service
[Unit]
Description=HyperBox Daemon Instance %i
After=network-online.target

[Service]
Type=notify
ExecStart=/usr/local/bin/hyperboxd --config=/etc/hyperbox/hyperboxd.%i.toml
User=hyperbox
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

```bash
# Start multiple instances
systemctl start hyperboxd@prod.service
systemctl start hyperboxd@staging.service
systemctl start hyperboxd@dev.service

# Check status
systemctl status 'hyperboxd@*'

# Restart specific instance
systemctl restart hyperboxd@prod.service
```

---

## Data Management

### Backup Strategy

#### Volume Snapshots (Kubernetes)

```yaml
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshot
metadata:
    name: hyperbox-backup-$(date +%Y%m%d)
    namespace: hyperbox
spec:
    volumeSnapshotClassName: csi-hostpath-snapclass
    source:
        persistentVolumeClaimName: hyperbox-pvc
```

```bash
# Create snapshot
kubectl apply -f hyperbox-snapshot.yaml

# List snapshots
kubectl get volumesnapshot -n hyperbox

# Restore from snapshot
kubectl describe volumesnapshot hyperbox-backup-20240115 -n hyperbox
# Use snapshotHandle to restore
```

#### File-Based Backups

```bash
#!/bin/bash
# Daily backup script
backup_dir="/backup/hyperbox"
mkdir -p "$backup_dir"

# Backup cache and config
tar --exclude='*.lock' \
    -czf "$backup_dir/hyperbox-$(date +%Y%m%d-%H%M%S).tar.gz" \
    /var/lib/hyperbox \
    /etc/hyperbox

# Retain only 7 days
find "$backup_dir" -name "hyperbox-*.tar.gz" -mtime +7 -delete

# Verify backup
tar -tzf "$backup_dir"/hyperbox-latest.tar.gz | head -20
```

#### Database Backups (PostgreSQL)

```bash
# For deployments with PostgreSQL backend
docker-compose exec postgres \
  pg_dump -U hyperbox hyperbox_db > /backup/hyperbox-db-$(date +%Y%m%d).sql

# Restore
docker-compose exec postgres \
  psql -U hyperbox hyperbox_db < /backup/hyperbox-db-20240115.sql
```

### Data Recovery

#### Generic Volume Recovery (Kubernetes)

```bash
# 1. Create recovery pod with snapshot
cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: Pod
metadata:
  name: recovery-pod
  namespace: hyperbox
spec:
  containers:
  - name: recovery
    image: busybox
    volumeMounts:
    - name: snapshot
      mountPath: /mnt/snapshot
  volumes:
  - name: snapshot
    emptyDir: {}
EOF

# 2. Mount snapshot (depends on CSI driver)
# 3. Copy data out
kubectl cp recovery-pod:/mnt/snapshot /local/recovery/

# 4. Cleanup
kubectl delete pod recovery-pod -n hyperbox
```

#### File Recovery (Any Platform)

```bash
# Stop HyperBox
# (Kubernetes: kubectl delete pod -l app=hyperboxd -n hyperbox)
# (Docker: docker-compose down)
# (Systemd: systemctl stop hyperboxd)

# Restore from backup
tar -xzf /backup/hyperbox-20240115.tar.gz -C /

# Start HyperBox
# (restart respective platform)

# Verify
curl http://localhost:9999/health
curl http://localhost:8888/metrics | grep version
```

---

## Disaster Recovery

### RTO/RPO Planning

| Scenario            | RTO      | RPO      | Strategy                          |
| ------------------- | -------- | -------- | --------------------------------- |
| Single node failure | 5 min    | 1 day    | Automated restart + daily backups |
| Regional outage     | 30 min   | 1 hour   | Multi-region replication          |
| Data corruption     | 4 hours  | 24 hours | Daily off-site backup             |
| Complete loss       | 48 hours | 24 hours | Full infrastructure rebuild       |

### Single-Node Failure Recovery

```bash
# Automatic (configured in systemd/K8s)
systemctl restart hyperboxd  # Restarts automatically on failure

# Manual recovery
systemctl status hyperboxd
systemctl restart hyperboxd
systemctl status hyperboxd

# If restart fails
journalctl -u hyperboxd -n 100 | tail -50
# Diagnose from TROUBLESHOOTING_GUIDE.md
```

### Multi-Node Failure Scenario

```bash
# Kubernetes: DaemonSet automatically redistributes to healthy nodes
# Systemd: Manual intervention required for each failed node

# Check failed nodes
kubectl get nodes
kubectl describe node <failed-node>

# Option 1: Drain and replace node
kubectl drain <failed-node> --ignore-daemonsets --delete-emptydir-data
# Replace node (cloud provider steps)
kubectl delete node <failed-node>

# Option 2: Scale down if node permanently lost
kubectl scale daemonset hyperboxd --replicas=7 -n hyperbox
# (not typical for DaemonSets, which auto-scale to node count)
```

### Regional Outage (Multi-Region Setup)

```bash
# Assuming replication between regions
# Region 1 (primary) fails completely

# Kubernetes multi-region with etcd replication
# 1. Verify Region 2 is healthy
kubectl --context=region2 get nodes
kubectl --context=region2 get pods -n hyperbox

# 2. Update DNS to point to Region 2
aws route53 change-resource-record-sets \
  --hosted-zone-id Z123ABC \
  --change-batch '{
    "Changes": [{
      "Action": "MODIFY",
      "ResourceRecordSet": {
        "Name": "hyperbox.example.com",
        "Type": "CNAME",
        "TTL": 60,
        "ResourceRecords": [{"Value": "hyperbox-region2.example.com"}]
      }
    }]
  }'

# 3. Verify Region 2 is serving requests
curl http://hyperbox.example.com/health

# 4. Rebuild Region 1 from backups (in parallel)
# 5. Update DNS back to load-balanced setup
```

---

## Monitoring & Alerting

### Key Metrics to Monitor

```bash
# Essential metrics from /metrics endpoint
curl http://localhost:8888/metrics | grep -E "^hyperbox_" | head -20
```

**Critical metrics:**

```
hyperbox_daemon_up                      # 1 if healthy, 0 if down
hyperbox_cache_hit_ratio                # Target: >60%
hyperbox_operation_latency_ms_bucket    # p95: <100ms, p99: <500ms
hyperbox_memory_usage_bytes             # Monitor for leaks
hyperbox_prediction_accuracy_percent    # Target: >70%
hyperbox_dedup_effective_ratio          # Target: >2.0x
```

### Alert Rules (Prometheus)

```yaml
# /etc/prometheus/rules/hyperbox-alerts.yml
groups:
    - name: hyperbox_sla
      interval: 30s
      rules:
          # Critical: Daemon down
          - alert: HyperBoxDaemonDown
            expr: hyperbox_daemon_up == 0
            for: 1m
            labels:
                severity: critical
            annotations:
                summary: "HyperBox daemon is down on {{ $labels.instance }}"
                action: "Restart daemon, check logs, contact on-call"

          # Critical: Cache hit ratio <50%
          - alert: HyperBoxCacheHitRatioDegraded
            expr: hyperbox_cache_hit_ratio < 50
            for: 10m
            labels:
                severity: critical
            annotations:
                summary: "Cache hit ratio degraded to {{ $value }}%"
                action: "Increase cache size, check workload pattern"

          # Warning: Latency p99 >500ms
          - alert: HyperBoxLatencyHigh
            expr: histogram_quantile(0.99, hyperbox_operation_latency_ms_bucket) > 500
            for: 5m
            labels:
                severity: warning
            annotations:
                summary: "p99 latency is {{ $value }}ms"
                action: "Check CPU/memory/I/O, consider optimization"

          # Warning: Memory usage >80%
          - alert: HyperBoxMemoryHigh
            expr: hyperbox_memory_usage_bytes / hyperbox_memory_limit_bytes > 0.8
            for: 5m
            labels:
                severity: warning
            annotations:
                summary: "Memory usage {{ $value }}%"
                action: "Check for memory leak, increase limit"

          # Info: Prediction accuracy <60%
          - alert: HyperBoxPredictionAccuracyLow
            expr: hyperbox_prediction_accuracy_percent < 60
            for: 30m
            labels:
                severity: info
            annotations:
                summary: "Prediction accuracy {{ $value }}%"
                action: "Check workload stability, adjust thresholds"
```

### AlertManager Configuration

```yaml
# /etc/alertmanager/config.yml
global:
    resolve_timeout: 5m

route:
    receiver: default
    group_by: ["alertname", "instance"]
    routes:
        # Critical alerts -> PagerDuty + SMS
        - match:
              severity: critical
          receiver: pagerduty_critical
          group_wait: 0s
          group_interval: 1m

        # Warnings -> Slack
        - match:
              severity: warning
          receiver: slack_warnings
          group_wait: 1m
          group_interval: 5m

receivers:
    - name: pagerduty_critical
      pagerduty_configs:
          - service_key: xxx
            description: "{{ .GroupLabels.alertname }}"

    - name: slack_warnings
      slack_configs:
          - api_url: https://hooks.slack.com/services/xxx
            channel: "#hyperbox-ops"

    - name: default
      # Catch-all receiver
```

---

## Security Hardening (Production)

### Network Segmentation (Kubernetes)

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
    name: hyperbox-network-policy
    namespace: hyperbox
spec:
    podSelector:
        matchLabels:
            app: hyperboxd

    policyTypes:
        - Ingress
        - Egress

    ingress:
        # Allow from Prometheus
        - from:
              - namespaceSelector:
                    matchLabels:
                        name: monitoring
          ports:
              - protocol: TCP
                port: 8888 # Metrics

        # Allow from local clients
        - from:
              - podSelector: {}
          ports:
              - protocol: TCP
                port: 9999 # API

    egress:
        # Allow to Docker socket
        - to:
              - namespaceSelector: {}
          ports:
              - protocol: TCP
                port: 2375

        # Allow DNS
        - to:
              - namespaceSelector:
                    matchLabels:
                        name: kube-system
          protocols:
              - UDP
          ports:
              - port: 53
```

### RBAC (Role-Based Access Control)

```yaml
# /hyperbox-k8s.yaml (already included)
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
    name: hyperboxd
rules:
    - apiGroups: [""]
      resources: ["pods"]
      verbs: ["get", "list", "watch"]
    - apiGroups: [""]
      resources: ["pods/logs"]
      verbs: ["get"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
    name: hyperboxd
roleRef:
    apiGroup: rbac.authorization.k8s.io
    kind: ClusterRole
    name: hyperboxd
subjects:
    - kind: ServiceAccount
      name: hyperboxd
      namespace: hyperbox
```

### Secrets Management

```bash
# Kubernetes: Use sealed-secrets or external-secrets
kubectl create secret docker-registry regcred \
  --docker-server=myregistry.azurecr.io \
  --docker-username=<username> \
  --docker-password=<password> \
  -n hyperbox

# Systemd: Use systemd-credential or HashiCorp Vault
systemd-creds encrypt /etc/hyperbox/secrets.conf /etc/hyperbox/secrets.conf.enc

# Docker Compose: Use .env file (not committed)
cat > .env << EOF
HYPERBOX_APIKEY=secret-key-xxx
REGISTRY_USERNAME=user
REGISTRY_PASSWORD=pass
EOF
```

### TLS/SSL Configuration

```bash
# Generate self-signed cert for HTTPS
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365

# Kubernetes secret
kubectl create secret tls hyperbox-tls \
  --cert=cert.pem \
  --key=key.pem \
  -n hyperbox

# Docker Compose
volumes:
  hyperboxd:
    driver: local
  certs:
    driver: local

services:
  hyperboxd:
    volumes:
      - certs:/etc/hyperbox/certs
```

---

## Scaling Strategies

### Horizontal Scaling (Kubernetes)

```bash
# DaemonSet automatically scales to node count
# For StatefulSet usage (not recommended, but possible):

# Scale up
kubectl scale statefulset hyperboxd --replicas=10 -n hyperbox

# Scale down
kubectl scale statefulset hyperboxd --replicas=5 -n hyperbox

# Watch scaling progress
kubectl rollout status statefulset/hyperboxd -n hyperbox
```

### Vertical Scaling

```yaml
# Kubernetes: Increase resource limits
kubectl patch daemonset hyperboxd --type='json' \
  -p='[{"op": "replace", "path": "/spec/template/spec/containers/0/resources/limits/memory", "value":"2Gi"}]' \
  -n hyperbox

# Docker Compose: Update in docker-compose.yml
services:
  hyperboxd:
    mem_limit: 2g  # Increase from 1g
    cpus: '2.0'    # Increase from 1.0
```

### Load Balancing

```bash
# Kubernetes: Service automatically load-balances across pods
# Systemd: Use reverse proxy (nginx recommended)

# /etc/nginx/sites-available/hyperbox
upstream hyperbox_backend {
  server 127.0.0.1:9999;
  server 127.0.0.1:9998;  # Second instance
  keepalive 32;
}

server {
  listen 80;
  server_name hyperbox.local;

  location / {
    proxy_pass http://hyperbox_backend;
    proxy_http_version 1.1;
    proxy_set_header Connection "";
  }
}
```

---

## Cost Optimization

### Kubernetes Cost Reduction

```bash
# 1. Use spot instances for non-critical workloads
kubectl label nodes spot-node workload-type=batch

# 2. Set resource limits to prevent overuse
kubectl set resources daemonset hyperboxd \
  --limits=cpu=1000m,memory=512Mi \
  --requests=cpu=500m,memory=256Mi \
  -n hyperbox

# 3. Enable cluster autoscaling
# (Cloud provider configuration)

# 4. Monitor actual usage
kubectl top nodes
kubectl top pods -n hyperbox
```

### Docker Compose Cost Reduction

```yaml
# Use lighter base images
services:
    hyperboxd:
        image: alpine:latest # vs ubuntu:latest


# Reduce unused services
# Keep only: hyperboxd, postgres (if needed)
# Remove: prometheus, grafana if monitoring via cloud provider
```

### Monitoring Spend

```bash
# Kubernetes on AWS
aws ce get-cost-and-usage \
  --time-period Start=2024-01-01,End=2024-01-31 \
  --metrics BlendedCost \
  --filter "file:Dimensions:Resource:hyperbox" \
  --granularity DAILY
```

---

## Compliance & Audit

### Audit Logging

```bash
# Kubernetes audit
kubectl api-server admits logs to:
/var/log/kubernetes/audit.log

# Query audit logs
grep hyperboxd /var/log/kubernetes/audit.log | head -20

# Systemd audit
auditctl -a exit,always -F exe=/usr/local/bin/hyperboxd

# View audit logs
ausearch -i | grep hyperboxd
```

### Log Retention

```bash
# Kubernetes: Configure apiserver audit policy
cat > /etc/kubernetes/audit-policy.yaml << EOF
apiVersion: audit.k8s.io/v1
kind: Policy

rules:
# Log all requests at Metadata level
- level: Metadata
  omitStages:
  - RequestReceived
EOF

# Docker Compose: Configure logrotate
cat > /etc/logrotate.d/hyperbox << EOF
/var/log/hyperbox/*.log {
  daily
  rotate 30
  compress
  delaycompress
  notifempty
  create 0640 hyperbox hyperbox
}
EOF
```

### Compliance Reporting

```bash
#!/bin/bash
# Generate monthly compliance report

cat > /tmp/hyperbox-compliance-report.md << EOF
# HyperBox Compliance Report - $(date +%B%Y)

## Uptime
Uptime: $(systemctl show hyperboxd -p Result)
Incidents: [source from incident tracking]

## Security
- Logs retained: 30 days
- TLS/SSL: Enabled
- RBAC: Configured
- Network policies: Active
- Latest patches: Applied on [date]

## Performance
- Avg latency: $(curl -s http://localhost:8888/metrics | grep operation_latency | tail -1)
- Cache hit ratio: $(curl -s http://localhost:8888/metrics | grep cache_hit_ratio | head -1)
- Dedup ratio: $(curl -s http://localhost:8888/metrics | grep dedup_ratio | head -1)

## Backups
- Last backup: [automated daily]
- Backup retention: 30 days
- Backup test date: [monthly]
EOF

echo "Report: /tmp/hyperbox-compliance-report.md"
```

---

## Quick Command Reference

### Kubernetes

```bash
# View all HyperBox resources
kubectl get all -n hyperbox

# Check daemon status
kubectl get daemonset -n hyperbox
kubectl get pods -n hyperbox
kubectl logs -n hyperbox -l app=hyperboxd --tail=50 -f

# Scale
kubectl scale daemonset hyperboxd --replicas=3 -n hyperbox  # Not typical

# Update
kubectl set image daemonset/hyperboxd hyperboxd=hyperbox:v2 -n hyperbox
kubectl rollout status daemonset/hyperboxd -n hyperbox

# Delete/Recreate
kubectl delete daemonset hyperboxd -n hyperbox
kubectl apply -f hyperbox-k8s.yaml
```

### Docker Compose

```bash
# Start stack
docker-compose up -d

# Check health
docker-compose ps
docker-compose exec hyperboxd curl http://localhost:9999/health

# View logs
docker-compose logs -f hyperboxd

# Update
docker-compose pull
docker-compose down
docker-compose up -d

# Backup data
docker-compose exec hyperboxd tar -czf /backup/data.tar.gz /var/lib/hyperbox
```

### Systemd

```bash
# Start/stop/restart
systemctl start hyperboxd
systemctl stop hyperboxd
systemctl restart hyperboxd

# Enable auto-start
systemctl enable hyperboxd

# Check status
systemctl status hyperboxd
systemctl is-active hyperboxd
systemctl is-enabled hyperboxd

# View logs
journalctl -u hyperboxd -n 100 -f --no-pager

# Resource limits
systemctl set-property hyperboxd MemoryLimit=2G CPUQuota=200%
```
