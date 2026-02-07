# HyperBox Performance Tuning Guide

## Philosophy

Performance tuning should be **metrics-driven**. Always:

1. Measure baseline performance
2. Change one parameter at a time
3. Re-measure to verify improvement
4. Document results

---

## CPU Optimization

### Understanding CPU Bottlenecks

```bash
# Identify if CPU is the bottleneck
top -p $(pgrep -f hyperboxd)

# Key metric: %CPU should be < 80% for normal operations
# If > 90%, CPU is likely bottleneck
```

### Parameter Tuning

```toml
[prewarming]
# Set to number of CPU cores
# Each thread handles one parallel worming task
parallel_ops = 8  # For 8-core system

# For high-core systems (16+ cores), reduce batching delay
batch_delay_ms = 5  # Default: 10ms
```

### CPU-Specific Optimization

```bash
# Check CPU count
nproc
cat /proc/cpuinfo | grep "physical id" | sort -u | wc -l

# Check if hyperthreading is enabled
grep "siblings" /proc/cpuinfo | head -1

# Check CPU frequency
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq
```

**Configuration per CPU count:**

| CPU Cores | parallel_ops | batch_delay_ms | Expected Throughput |
| --------- | ------------ | -------------- | ------------------- |
| 2         | 1            | 20             | 50 ops/sec          |
| 4         | 2            | 15             | 100 ops/sec         |
| 8         | 4            | 10             | 200 ops/sec         |
| 16        | 8            | 5              | 400 ops/sec         |
| 32+       | 16           | 2              | 800+ ops/sec        |

### Measuring CPU Impact

```bash
#!/bin/bash
# Baseline CPU usage (idle)
baseline=$(top -b -n 1 -p $(pgrep -f hyperboxd) | grep hyperboxd | awk '{print $9}')
echo "Idle CPU: $baseline%"

# Load test (generate 1000 container operations)
for i in {1..1000}; do
  docker run --rm alpine:latest /bin/true &
done
wait

# Peak CPU usage
peak=$(top -b -n 1 -p $(pgrep -f hyperboxd) | grep hyperboxd | awk '{print $9}')
echo "Peak CPU: $peak%"
echo "Overhead: $(echo "$peak - $baseline" | bc)%"
```

---

## Memory Optimization

### Memory Budget Analysis

```bash
# Available memory
free -h

# System memory map
cat /proc/meminfo

# HyperBox memory usage
pmap -x $(pgrep -f hyperboxd)
```

**Typical memory usage:**

| Component             | Baseline | Per GB Cache | Notes                 |
| --------------------- | -------- | ------------ | --------------------- |
| Daemon                | 50MB     | -            | Fixed overhead        |
| Cache                 | 0        | 1x           | 1GB config = ~1GB RAM |
| Prediction models     | 20MB     | -            | Fixed size            |
| Metrics buffer        | 10MB     | -            | Rolling buffer        |
| **Total @ 1GB cache** | **80MB** | -            | For 1024MB configured |

### Cache Size Tuning

```toml
[cache]
# Rule of thumb: Use 40-60% of available RAM
# Example: 16GB available RAM → 6-10GB cache
size_mb = 1024

# Enable memory-mapped I/O for large caches
use_mmap = true
```

**Cache vs Memory Trade-off:**

```
Cache Size    Memory Usage    Hit Ratio    Latency
512MB         ~520MB          35%          150ms
1024MB        ~1030MB         60%          80ms
2048MB        ~2060MB         80%          20ms
4096MB        ~4100MB         92%          5ms
```

### Compression Tuning

```toml
[cache]
compression = true
compression_level = 6  # 1-9, higher = slower compression

# For memory-constrained systems
compression_level = 9  # Slower but smaller

# For performance-critical systems
compression_level = 1  # Faster but larger
```

**Compression Statistics:**

```bash
# Monitor compression effectiveness
curl http://localhost:8888/metrics | grep compression

# Key metrics:
# hyperbox_compression_ratio     (target: >2.0x)
# hyperbox_compression_time_ms   (target: <50ms)
```

### Measuring Memory Impact

```bash
#!/bin/bash
# Memory baseline
baseline_rss=$(pmap -x $(pgrep -f hyperboxd) | tail -1 | awk '{print $3}')
echo "Baseline RSS: ${baseline_rss}MB"

# Run prewarming ops
for i in {1..100}; do
  docker run --rm alpine:latest /bin/true &
done
wait

# Peak memory
peak_rss=$(pmap -x $(pgrep -f hyperboxd) | tail -1 | awk '{print $3}')
echo "Peak RSS: ${peak_rss}MB"
echo "Growth: $((peak_rss - baseline_rss))MB"

# Memory efficiency
echo "Ops per MB: $(echo "100 / ($peak_rss - $baseline_rss)" | bc)"
```

---

## I/O Optimization

### Disk Path Selection

```bash
# Identify available disks
lsblk
df -h

# Check disk speed (IOPS, throughput)
fio --name=randrw --ioengine=libaio --iodepth=16 \
    --rw=randrw --bs=4k --direct=1 --size=1G \
    --filename=/var/lib/hyperbox/test.dat --runtime=60
```

**Recommended configurations:**

| Storage Type  | Expected IOPS | Cache Config | Use Case         |
| ------------- | ------------- | ------------ | ---------------- |
| NVMe SSD      | 100k+         | 4096MB       | High-performance |
| SATA SSD      | 50k           | 2048MB       | Production       |
| HDD           | 500           | 512MB        | Legacy/budget    |
| Network (NFS) | 5k            | 256MB        | Distributed      |

### Batch Write Tuning

```toml
[cache]
# Batch multiple writes for efficiency
batch_writes = true
batch_size = 100  # Objects per batch
batch_timeout_ms = 100  # Max wait time

# For fast SSDs, reduce batching
batch_size = 50
batch_timeout_ms = 10

# For slow storage, increase batching
batch_size = 500
batch_timeout_ms = 500
```

### Measuring I/O Impact

```bash
#!/bin/bash
# Monitor disk I/O
iostat -x 1 30 | grep -E "rkB/s|wkB/s"

# Per-process I/O
iotop -p $(pgrep -f hyperboxd) -n 10

# Latency monitoring
curl http://localhost:8888/metrics | grep "io_latency"
```

---

## Network Optimization

### Connection Management

```toml
[networking]
# Maximum concurrent connections
max_connections = 1000

# Connection timeout (seconds)
connection_timeout_secs = 30

# TCP keepalive (seconds)
tcp_keepalive_secs = 60

# For high-throughput scenarios
max_connections = 10000
connection_timeout_secs = 60
tcp_keepalive_secs = 30
```

### Buffer Tuning

For systemd socket:

```ini
[Socket]
# TCP buffer sizes (2M recommended)
ReceiveBuffer=2M
SendBuffer=2M

# For very high throughput
ReceiveBuffer=8M
SendBuffer=8M
```

For Docker Compose:

```yaml
# TCP options in docker-compose.yml
sysctls:
    - net.ipv4.tcp_rmem=4096 2097152 8388608
    - net.ipv4.tcp_wmem=4096 2097152 8388608
```

### Measuring Network Performance

```bash
#!/bin/bash
# Network throughput
iperf3 -c <hyperbox-host> -p 9999 -t 30

# Packet loss and latency
ping -c 100 <hyperbox-host> | grep -E "min/avg/max|loss"

# Per-connection statistics
curl http://localhost:8888/metrics | grep "network_connections"
```

---

## Prediction Optimization

### Accuracy vs Performance Trade-off

```toml
[prediction]
enabled = true

# Update frequency (seconds)
update_interval_secs = 300  # 5 minutes

# Confidence threshold (0-100%)
confidence_threshold = 65

# For high-accuracy scenarios
update_interval_secs = 60  # More frequent updates
confidence_threshold = 80  # More selective

# For fast-changing workloads
update_interval_secs = 600  # Less frequent
confidence_threshold = 50  # More aggressive
```

### Monitoring Prediction Quality

```bash
# Check prediction accuracy
curl http://localhost:8888/metrics | grep prediction_accuracy

# Historical trend
curl 'http://localhost:8888/metrics?interval=1h' | grep prediction_accuracy

# Impact on cache hits
curl http://localhost:8888/metrics | grep cache_hit_ratio
```

---

## Deduplication Optimization

### Concurrency Tuning

```toml
[deduplication]
enabled = true

# Number of concurrent dedup operations
parallel_ops = 4

# For CPU-heavy workloads
parallel_ops = 2  # Reduce CPU contention

# For high-throughput
parallel_ops = 8  # Match CPU cores
```

### Hash Algorithm Selection

```toml
[deduplication]
# Available: sha256 (default), blake3, xxhash
hash_algorithm = "blake3"  # Faster than SHA256

# Result metrics for different algorithms
# SHA256: 100ms per 100MB, highly secure
# BLAKE3: 20ms per 100MB, fast, secure
# XXHASH: 5ms per 100MB, fast, collision-prone
```

---

## Prewarming Optimization

### Strategy Selection

```toml
[prewarming]
enabled = true

# Technique: predictive, frequency, statistical
technique = "predictive"

# Parallel operations matching CPU cores
parallel_ops = 8

# Batch processing
batch_size_layers = 10
batch_delay_ms = 10

# For aggressive prewarming
parallel_ops = 16
batch_size_layers = 50
batch_delay_ms = 1  # Minimal delay

# For conservative prewarming
parallel_ops = 2
batch_size_layers = 5
batch_delay_ms = 100  # More delay for fairness
```

---

## Comprehensive Tuning Playbook

### Step 1: Baseline Collection (60 iterations, 1 hour)

```bash
#!/bin/bash
# Collect 60 baseline measurements over 60 minutes
output_file="/tmp/hyperbox-baseline-$(date +%s).csv"
echo "timestamp,rss_mb,cache_hit_ratio,avg_latency_ms,cpu_percent,dedup_ops,prediction_accuracy" > "$output_file"

for i in {1..60}; do
  timestamp=$(date +%s)
  rss=$(pmap -x $(pgrep -f hyperboxd) 2>/dev/null | tail -1 | awk '{print $3}' || echo "0")
  metrics=$(curl -s http://localhost:8888/metrics)
  cache_hits=$(echo "$metrics" | grep "cache_hit_ratio" | head -1 | awk '{print $2}' || echo "0")
  latency=$(echo "$metrics" | grep "operation_latency" | tail -1 | awk '{print $2}' || echo "0")
  cpu=$(top -b -n 1 -p $(pgrep -f hyperboxd) 2>/dev/null | grep hyperboxd | awk '{print $9}' || echo "0")
  dedup=$(echo "$metrics" | grep "dedup_operations_total" | awk '{print $2}' || echo "0")
  pred=$(echo "$metrics" | grep "prediction_accuracy" | head -1 | awk '{print $2}' || echo "0")

  echo "$timestamp,$rss,$cache_hits,$latency,$cpu,$dedup,$pred" >> "$output_file"

  echo "Iteration $i/60 - RSS: ${rss}MB, Cache hits: $cache_hits, Latency: ${latency}ms"

  if [ $i -lt 60 ]; then
    sleep 60
  fi
done

echo "Baseline complete: $output_file"
```

### Step 2: Identify Bottleneck

```bash
# Analyze baseline
awk -F',' 'NR>1 {
  rss_sum+=$2; cpu_sum+=$5; latency_sum+=$4; hits_sum+=$3
  rss_max=($2>rss_max)?$2:rss_max
  cpu_max=($5>cpu_max)?$5:cpu_max
  latency_max=($4>latency_max)?$4:latency_max
}
END {
  printf "Averages:\n"
  printf "  Memory: %.0fMB (max: %.0fMB)\n", rss_sum/NR, rss_max
  printf "  CPU: %.1f%% (max: %.1f%%)\n", cpu_sum/NR, cpu_max
  printf "  Latency: %.1fms (max: %.1fms)\n", latency_sum/NR, latency_max
  printf "  Cache hits: %.1f%%\n", hits_sum/NR
}' "$output_file"
```

### Step 3: Apply Targeted Tuning

**If CPU-bound:**

```toml
[prewarming]
parallel_ops = 16  # Increase
batch_delay_ms = 2  # Minimize
```

**If memory-bound:**

```toml
[cache]
size_mb = 512  # Reduce
compression = true
compression_level = 9
```

**If I/O-bound:**

```toml
[cache]
batch_size = 500  # Increase
batch_timeout_ms = 500
use_mmap = true
```

**If cache miss-heavy:**

```toml
[cache]
size_mb = 2048  # Increase
[prediction]
enabled = true
confidence_threshold = 50
```

### Step 4: Measure Impact

```bash
# Re-run baseline after one parameter change
# Compare metrics side-by-side
diff <(tail -5 baseline1.csv) <(tail -5 baseline2.csv)
```

---

## Scenario-Based Tuning

### High-Throughput Scenario (100+ ops/sec)

```toml
[cache]
size_mb = 4096
compression = true
compression_level = 6
use_mmap = true
batch_size = 500
batch_timeout_ms = 50

[prewarming]
parallel_ops = 16
technique = "statistical"

[deduplication]
parallel_ops = 8
hash_algorithm = "blake3"

[networking]
max_connections = 10000
tcp_keepalive_secs = 30
```

### Memory-Constrained Scenario (512MB available)

```toml
[cache]
size_mb = 128
compression = true
compression_level = 9
use_mmap = true

[prediction]
enabled = false

[prewarming]
enabled = false

[deduplication]
parallel_ops = 1
```

### Latency-Critical Scenario (<10ms target)

```toml
[cache]
size_mb = 4096
compression = false  # Disable for speed
use_mmap = true

[prediction]
enabled = true
confidence_threshold = 80
update_interval_secs = 60

[prewarming]
parallel_ops = 8
technique = "predictive"

[networking]
tcp_keepalive_secs = 10  # More frequent
```

### Balanced Scenario (production default)

```toml
[cache]
size_mb = 2048
compression = true
compression_level = 6

[prewarming]
parallel_ops = 8

[deduplication]
parallel_ops = 4

[networking]
max_connections = 1000
```

---

## Monitoring Performance Over Time

```bash
#!/bin/bash
# Daily performance tracking
cron_script="/etc/cron.daily/hyperbox-perf-track"

cat > "$cron_script" << 'EOF'
#!/bin/bash
date=$(date +%Y-%m-%d)
perf_dir="/var/lib/hyperbox/performance"
mkdir -p "$perf_dir"

curl -s http://localhost:8888/metrics > "$perf_dir/metrics-$date.txt"

# Extract key values
echo "$date: $(grep cache_hit_ratio "$perf_dir/metrics-$date.txt" | head -1 | awk '{print $2}')" |\
  tee -a "$perf_dir/hit-ratio.log"

echo "$date: $(grep operation_latency "$perf_dir/metrics-$date.txt" | tail -1 | awk '{print $2}')" |\
  tee -a "$perf_dir/latency.log"
EOF

chmod +x "$cron_script"
```

---

## Performance Regression Detection

```bash
#!/bin/bash
# Alert if performance drops >10%
current_latency=$(curl -s http://localhost:8888/metrics | grep operation_latency | tail -1 | awk '{print $2}')
baseline_latency=50  # ms, from initial baseline

threshold=$(echo "$baseline_latency * 1.1" | bc)
if (( $(echo "$current_latency > $threshold" | bc -l) )); then
  echo "ALERT: Latency regression detected! $current_latency ms > $threshold ms"
  systemctl restart hyperboxd  # Auto-restart to clear state
fi
```

---

## Quick Reference

### Common Bottleneck Solutions

| Bottleneck | Symptom                  | Solution                                     |
| ---------- | ------------------------ | -------------------------------------------- |
| CPU        | `top` shows >90%         | Increase `parallel_ops`                      |
| Memory     | RSS > 80% of limit       | Increase `size_mb` or enable compression     |
| I/O        | High `iostat` write time | Increase `batch_size` and `batch_timeout_ms` |
| Network    | High connection count    | Increase `max_connections`                   |
| Cache      | Low hit ratio            | Increase `size_mb` or enable `prediction`    |

### Parameter Ranges

| Parameter         | Min | Default | Max   | Tuning                      |
| ----------------- | --- | ------- | ----- | --------------------------- |
| cache/size_mb     | 128 | 1024    | 16000 | Up = faster, more memory    |
| parallel_ops      | 1   | 4       | 64    | Up = faster, more CPU       |
| compression_level | 1   | 6       | 9     | Up = smaller, slower        |
| batch_size        | 1   | 100     | 1000  | Up = faster, higher latency |
