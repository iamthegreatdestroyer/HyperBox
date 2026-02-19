# Phase E - Performance Benchmarking Plan

## Benchmark Overview

Each of the 4 Phase E streams has clear performance targets. This document defines baseline measurements, test procedures, and validation criteria to ensure targets are met.

**Core Principle:** Measure before and after, compare apples-to-apples, validate under realistic load.

---

## BENCHMARK SCHEDULE

### Week 2 (Mon Feb 24 - Fri Feb 28)
- **Mon:** Baseline measurement setup
- **Tue-Thu:** Continuous benchmarking during development
- **Fri:** Baseline report, performance targets confirmed

### Week 3 (Mon Mar 3 - Fri Mar 7)
- **Mon:** Performance optimization based on Week 2 results
- **Tue-Wed:** Stress testing and edge case validation
- **Thu-Fri:** Final performance validation

### Week 4 (Mon Mar 10 - Fri Mar 14)
- **Mon:** Release candidate benchmarking
- **Tue-Fri:** Staging environment validation

---

## STREAM A: PSI Memory Monitoring Benchmarks

### Target
- **Metric:** Memory pressure reduction
- **Goal:** 5-15% reduction in PSI metrics under high load
- **Measurement:** some_cpu, some_memory, full_memory pressure states

### Baseline Setup (Monday Week 2)

#### Workload Creation
```bash
# Create high-memory pressure environment
docker run --name mem-test-1 -m 1GB hyperbox:test /load-generator --memory 900MB &
docker run --name mem-test-2 -m 1GB hyperbox:test /load-generator --memory 900MB &
docker run --name mem-test-3 -m 1GB hyperbox:test /load-generator --memory 900MB &
docker run --name mem-test-4 -m 1GB hyperbox:test /load-generator --memory 900MB &
docker run --name mem-test-5 -m 1GB hyperbox:test /load-generator --memory 900MB &

# Total: 5GB memory pressure on 8GB system (heavy swap load expected)
```

#### Baseline Measurement (Without PSI)
```bash
# Run for 5 minutes without PSI optimization
for i in {1..300}; do
  # Sample every second
  cat /proc/pressure/memory >> baseline_psi.log
  sleep 1
done

# Parse results
grep "some memory" baseline_psi.log | awk '{sum+=$5} END {print "Avg some_memory: " sum/300 "%"}'
grep "full memory" baseline_psi.log | awk '{sum+=$5} END {print "Avg full_memory: " sum/300 "%"}'
```

**Expected Baseline:**
```
Without PSI:
  some_memory: 65-75% average
  full_memory: 45-55% average
  Swap usage: 3-4 GB
```

#### Measurement With PSI Optimization (Tuesday-Thursday)
```bash
# Enable PSI monitoring in daemon
HYPERBOX_ENABLE_PSI=1 hyperbox-daemon &

# Run same workload
for i in {1..300}; do
  # Sample PSI endpoint
  curl -s http://localhost:9999/metrics/memory/psi >> optimized_psi.log
  sleep 1
done

# Parse results
grep "some_memory" optimized_psi.log | awk '{sum+=$5} END {print "Avg some_memory: " sum/300 "%"}'
grep "full_memory" optimized_psi.log | awk '{sum+=$5} END {print "Avg full_memory: " sum/300 "%"}'
```

**Target Results:**
```
With PSI optimization:
  some_memory: 58-65% average (10% reduction)
  full_memory: 38-50% average (10% reduction)
  Swap usage: 2.5-3.5 GB (15% reduction)

Overall: 5-15% improvement ✓
```

### Validation Criteria
- [ ] Baseline measurements stable (3 runs, <5% variance)
- [ ] PSI-optimized runs show 5-15% improvement
- [ ] CPU overhead <1% during monitoring
- [ ] Memory overhead <20MB for PSI data
- [ ] Improvement consistent across different kernel versions

### Stress Test (Week 3)
```bash
# Sustained load: 10 containers, 8 hours
for i in {1..10}; do
  docker run -d --name mem-stress-$i -m 1GB \
    hyperbox:test /load-generator --memory 900MB --duration 8h
done

# Monitor PSI metrics continuously
while true; do
  curl -s http://localhost:9999/metrics/memory/psi >> stress_test.log
  sleep 5
done

# Verify: No crashes, consistent performance, swap handling correct
```

---

## STREAM B: EROFS + Fscache Benchmarks

### Target
- **Metric:** Image pull time reduction
- **Goal:** 30-50% faster images (Linux 5.19+)
- **Baseline:** composefs pull time
- **Optimized:** EROFS pull time

### Baseline Setup (Monday Week 2)

#### Test Images Creation
```bash
# Create 10 test images of increasing size
for size in 50 100 150 200 250 300 350 400 450 500; do
  dd if=/dev/urandom of=test-image-${size}mb.bin bs=1M count=$size
  docker build -t test-image:${size}mb \
    --build-arg BASE=ubuntu:22.04 \
    --build-arg IMAGE=test-image-${size}mb.bin \
    -f Dockerfile.test .
done

# Total: 50MB to 500MB images (2.75GB total)
```

#### Baseline Measurement (composefs)
```bash
# Warm up: pull once to eliminate cold cache effects
docker pull test-image:50mb

# Measure: 3 runs per image, record time and throughput
for image_size in 50 100 150 200 250 300 350 400 450 500; do
  for run in 1 2 3; do
    echo "Image: ${image_size}MB, Run: $run"
    time docker pull test-image:${image_size}mb 2>&1 | tee -a composefs_baseline.log
    docker image rm test-image:${image_size}mb  # Clear for next run
    sleep 5
  done
done

# Parse results: Calculate average pull time and throughput
# Formula: Throughput = Image_Size_MB / Pull_Time_seconds
```

**Expected Baseline (composefs):**
```
Image Size  | Avg Pull Time | Throughput
50 MB       | 8.5 sec       | 5.9 MB/s
100 MB      | 15.2 sec      | 6.6 MB/s
200 MB      | 28.3 sec      | 7.1 MB/s
500 MB      | 68.5 sec      | 7.3 MB/s

Overall average: 7.0 MB/s
```

#### Measurement With EROFS (Tuesday-Thursday)
```bash
# Enable EROFS in storage config
HYPERBOX_STORAGE_BACKEND=erofs hyperbox-daemon &

# Same test as composefs
for image_size in 50 100 150 200 250 300 350 400 450 500; do
  for run in 1 2 3; do
    echo "Image: ${image_size}MB, Run: $run"
    time docker pull test-image:${image_size}mb 2>&1 | tee -a erofs_optimized.log
    docker image rm test-image:${image_size}mb
    sleep 5
  done
done

# Parse results: Calculate improvement
```

**Target Results (EROFS on Linux 5.19+):**
```
Image Size  | Avg Pull Time | Throughput | Improvement
50 MB       | 4.2 sec       | 11.9 MB/s  | 50% faster
100 MB      | 7.6 sec       | 13.2 MB/s  | 50% faster
200 MB      | 14.2 sec      | 14.1 MB/s  | 50% faster
500 MB      | 34.3 sec      | 14.6 MB/s  | 50% faster

Overall average: 13.5 MB/s (93% improvement!)
Target: 30-50% minimum ✓
```

### Kernel Compatibility Testing (Week 2)

```bash
# Test on multiple kernel versions
for kernel in "5.10" "5.15" "5.19" "6.0" "6.1"; do
  # Boot with specific kernel or use VM
  uname -r  # Verify kernel version

  # Run quick benchmark
  docker pull test-image:100mb 2>&1 | tee erofs_kernel_${kernel}.log

  # Expected:
  # - 5.10, 5.15: Should fallback to composefs (~15 sec)
  # - 5.19+: Should use EROFS (~7.6 sec)
done
```

### Validation Criteria
- [ ] composefs baseline stable across 3 runs
- [ ] EROFS improves pull time by 30-50%
- [ ] EROFS available on Linux 5.19+
- [ ] Graceful fallback on Linux <5.19
- [ ] No data corruption (verify image checksums)
- [ ] Storage usage comparable to composefs

### Stress Test (Week 3)
```bash
# Concurrent pulls: 10 parallel image pulls
for i in {1..10}; do
  docker pull test-image:200mb &
done
wait

# Large image: 1GB image pull
time docker pull test-image:1gb

# Repeated access: Same image, 100 pulls
for i in {1..100}; do
  docker pull test-image:100mb --quiet
done

# Verify: No failures, consistent performance
```

---

## STREAM C: OpenTelemetry eBPF Benchmarks

### Target
- **Metric:** CPU overhead from tracing
- **Goal:** <2% CPU overhead in production workloads
- **Baseline:** No tracing
- **Optimized:** Full eBPF tracing with OpenTelemetry

### Baseline Setup (Monday Week 2)

#### Workload Setup
```bash
# Standard workload: CPU-bound task
docker run --name cpu-benchmark \
  --cpus 4 \
  hyperbox:test /load-generator --cpu 100 --duration 10m

# Monitor CPU during baseline (no tracing)
while docker stats --no-stream | grep cpu-benchmark; do
  sleep 2
done > cpu_baseline.log
```

**Expected Baseline (no tracing):**
```
Container: cpu-benchmark
CPU Usage: 99-100% (4 cores at ~25% each due to scheduling)
Memory: 150-200 MB
Context switches: ~100,000/sec
System calls: ~50,000/sec
```

#### Measurement With eBPF Tracing (Tuesday-Thursday)
```bash
# Enable eBPF tracing in daemon
HYPERBOX_ENABLE_EBPF=1 hyperbox-daemon &

# Run same workload with tracing enabled
docker run --name cpu-with-ebpf \
  --cpus 4 \
  hyperbox:test /load-generator --cpu 100 --duration 10m

# Monitor CPU with tracing
while docker stats --no-stream | grep cpu-with-ebpf; do
  sleep 2
done > cpu_with_ebpf.log

# Collect traces
curl -s http://localhost:9999/traces | tee ebpf_traces.log
```

**Target Results:**
```
Container: cpu-with-ebpf
CPU Usage: 101-102% (eBPF overhead <2%)
Memory: 180-230 MB (eBPF overhead ~50 MB)
Trace latency: <100ms
Spans collected: 95%+

Overhead calculation:
(102% - 99%) / 99% = 3% measured
Target: <2%
```

### Memory Overhead Test (Week 2)
```bash
# Baseline memory (no tracing)
free -h > memory_baseline.txt
docker stats --no-stream | grep -E "CONTAINER|test" >> memory_baseline.txt

# With eBPF tracing
HYPERBOX_ENABLE_EBPF=1 hyperbox-daemon &
sleep 30
free -h > memory_with_ebpf.txt
docker stats --no-stream | grep -E "CONTAINER|test" >> memory_with_ebpf.txt

# Expected overhead: <50 MB
```

### Tracing Coverage Test (Week 3)
```bash
# Verify eBPF captures 95%+ of syscalls
docker run --name trace-test hyperbox:test /load-generator --syscall-heavy

# Count syscalls from eBPF
curl -s http://localhost:9999/traces | jq '.spans | length' > ebpf_span_count.txt

# Compare to actual syscalls
strace -c docker run hyperbox:test /load-generator --syscall-heavy 2>&1 | \
  grep "total" > strace_count.txt

# Calculate coverage
# Formula: eBPF_spans / strace_syscalls * 100
```

### Validation Criteria
- [ ] CPU overhead <2% under production load
- [ ] Memory overhead <50 MB
- [ ] Trace latency <100ms
- [ ] Syscall coverage >95%
- [ ] Graceful degradation on non-eBPF systems
- [ ] No crashes under sustained load

---

## STREAM D: Seccomp Auto-generation Benchmarks

### Target
- **Metric:** Profile size reduction
- **Goal:** 50-80% smaller generated profiles vs default profiles
- **Baseline:** Default seccomp profiles
- **Optimized:** Auto-generated profiles

### Baseline Setup (Monday Week 2)

#### Default Profile Sizes
```bash
# Measure size of default seccomp profiles
ls -lah /etc/hyperbox/seccomp/default*.json

# Example expected sizes:
# default-container.json: 15.2 KB (600 syscalls)
# default-web-server.json: 14.8 KB (580 syscalls)
# default-db-server.json: 16.4 KB (650 syscalls)
# default-compute.json: 13.5 KB (540 syscalls)
```

**Expected Baseline:**
```
Default Profile Size: 13.5 - 16.4 KB
Average Syscall Count: 580-600 syscalls
```

#### Test Workload Profiling (Tuesday-Thursday)

```bash
# Run 10 different workload types
WORKLOADS="web-server db-server api-gateway cache-service compute-job batch-process worker message-queue load-balancer monitoring"

for workload in $WORKLOADS; do
  # Run workload in learn-mode
  docker run --name $workload \
    -e HYPERBOX_LEARN_SECCOMP=1 \
    hyperbox:test /apps/$workload --run-30s

  # Extract generated profile
  cp /var/lib/hyperbox/seccomp/$workload.json generated_profiles/

  # Measure size and syscalls
  ls -lh generated_profiles/$workload.json
  jq '.syscalls | length' generated_profiles/$workload.json
done
```

**Expected Generated Profiles:**
```
Workload            | Syscalls | Size (KB) | Reduction | Target
web-server          | 120      | 3.2       | 78%       | ✓ 50-80%
db-server           | 150      | 4.1       | 73%       | ✓
api-gateway         | 95       | 2.5       | 82%       | ✓
cache-service       | 80       | 2.1       | 84%       | ✓
compute-job         | 110      | 2.9       | 79%       | ✓
batch-process       | 130      | 3.5       | 77%       | ✓
worker              | 100      | 2.7       | 81%       | ✓
message-queue       | 140      | 3.8       | 75%       | ✓
load-balancer       | 105      | 2.8       | 81%       | ✓
monitoring          | 90       | 2.4       | 82%       | ✓

Average reduction: 80% ✓ (exceeds 50-80% target)
```

### False Negative Testing (Week 3)

```bash
# For each generated profile, run workload normally (not learn-mode)
# and verify it doesn't get blocked

for workload in $WORKLOADS; do
  # Apply generated profile
  docker run --security-opt seccomp=generated_profiles/$workload.json \
    --name $workload-validated \
    hyperbox:test /apps/$workload --run-30s

  # Check for seccomp violations
  dmesg | grep "seccomp" | wc -l
done

# Expected: 0 violations (false negatives = 0%)
```

### Profile Determinism Test (Week 3)

```bash
# Generate profile 5 times, verify output is identical
for run in {1..5}; do
  docker run --name web-server-gen-$run \
    -e HYPERBOX_LEARN_SECCOMP=1 \
    hyperbox:test /apps/web-server --run-30s

  cp /var/lib/hyperbox/seccomp/web-server.json profiles/run_$run.json
done

# Compare all profiles
diff profiles/run_1.json profiles/run_2.json
diff profiles/run_2.json profiles/run_3.json
# ... etc

# Expected: All identical ✓
```

### Validation Criteria
- [ ] Generated profiles 50-80% smaller than defaults
- [ ] Zero false negatives (workloads don't get blocked)
- [ ] Profile generation deterministic (same input = same output)
- [ ] All workload types supported
- [ ] CPU overhead of learn-mode <5%

---

## BENCHMARK REPORTING

### Weekly Benchmark Report (Friday EOD each week)

**Template:**

```markdown
# PHASE E Benchmark Report - Week [X]

## Summary
- [ ] All baselines established
- [ ] Targets on track: A [Y/N], B [Y/N], C [Y/N], D [Y/N]
- [ ] No regressions detected
- [ ] Ready for next week

## Stream A: PSI Memory Monitoring
Status: ON TRACK / AT RISK / FAILED

Baseline:
  - some_memory: 68% average
  - full_memory: 49% average
  - Swap usage: 3.2 GB

Current (Week 2 avg):
  - some_memory: 63% average (7% improvement)
  - full_memory: 44% average (10% improvement)
  - Swap usage: 2.8 GB (13% improvement)

Target: 5-15% improvement
Progress: [=========>] 85% ✓

## Stream B: EROFS + Fscache
Status: ON TRACK / AT RISK / FAILED

Baseline (composefs):
  - 100MB image: 15.2 sec
  - 200MB image: 28.3 sec
  - Avg throughput: 7.0 MB/s

Current (Week 2 avg):
  - 100MB image: 7.8 sec (49% improvement)
  - 200MB image: 14.5 sec (49% improvement)
  - Avg throughput: 13.5 MB/s

Target: 30-50% improvement
Progress: [===========] 98% ✓ (Exceeding target!)

Kernel compatibility:
  - Linux 5.19+: EROFS working ✓
  - Linux <5.19: Fallback to composefs ✓

## Stream C: OpenTelemetry eBPF
Status: ON TRACK / AT RISK / FAILED

Baseline (no tracing):
  - CPU usage: 99.5%
  - Memory: 180 MB
  - Context switches: 100k/sec

Current (Week 2 avg):
  - CPU usage: 100.8% (1.3% overhead)
  - Memory: 210 MB (30 MB overhead)
  - Trace latency: 85ms
  - Span coverage: 96%

Target: <2% CPU overhead
Progress: [===========] 65% ✓ (On track)

## Stream D: Seccomp Auto-generation
Status: ON TRACK / AT RISK / FAILED

Baseline (defaults):
  - Avg profile size: 15 KB
  - Avg syscalls: 590

Current (Week 2 avg):
  - Avg profile size: 3.0 KB (80% reduction)
  - Avg syscalls: 118
  - False negatives: 0

Target: 50-80% reduction
Progress: [===========] 100% ✓✓✓ (Exceeding target!)

## Overall Status
- [x] All streams on track
- [x] All targets achievable
- [x] No regressions
- [x] Ready for Week 3 integration
```

---

## BENCHMARK TOOLS

### Required Tools
```bash
# Performance measurement
- cargo bench (built-in Rust benchmarking)
- perf (Linux performance analysis)
- docker stats (container metrics)
- strace (syscall tracing)
- bpftrace (eBPF tracing)

# Result analysis
- jq (JSON parsing)
- awk (data processing)
- gnuplot (charting)
- spreadsheet (result compilation)
```

### Benchmark Commands

```bash
# CPU profiling
perf record -g docker run hyperbox:test /load-generator
perf report

# Memory profiling
valgrind --tool=massif docker run hyperbox:test /load-generator
ms_print massif.out.XXXX

# Syscall tracing
strace -c -f docker run hyperbox:test /app
```

---

## PERFORMANCE TARGETS SUMMARY

| Stream | Metric | Baseline | Target | Win |
|--------|--------|----------|--------|-----|
| A | Memory pressure | 68% | 53-63% | 5-15% |
| B | Pull time (100MB) | 15.2s | 7.6-10.6s | 30-50% |
| C | CPU overhead | 0% | <2% | <2% |
| D | Profile size | 15KB | 3-7.5KB | 50-80% |

All targets are achievable with proper optimization. Review weekly and adjust if needed.
