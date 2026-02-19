# Phase E Architecture Review
## 4 Parallel Work Streams (Weeks 2-5)

---

## A) FEATURE BREAKDOWN

### STREAM A: PSI Memory Monitoring
**Assigned Team:** @APEX (Primary), @VELOCITY (Support), @PULSE (Monitoring)

```
Files: crates/hyperbox-core/src/memory/psi.rs
LOC: 400 (350 impl + 100 tests)
Target: 5-15% memory pressure reduction
Dependencies: None (new module)
Blockers: None anticipated
Success Criteria: Benchmark shows improvement
```

**Scope:**
- Implement Linux PSI (Pressure Stall Information) metrics reader
- Track memory pressure: some/full states
- Integrate with swap tuning logic
- Expose via `/metrics/memory/psi` endpoint
- Build integration tests with synthetic memory load

**Acceptance Criteria:**
- Accurate PSI readings on Linux systems
- <1% CPU overhead for monitoring
- 5-15% reduction in memory pressure under high load
- All unit tests passing (15+)
- Backward compatible (no breaking changes)

---

### STREAM B: EROFS + Fscache Integration
**Assigned Team:** @VELOCITY (Primary), @APEX (Support), @CIPHER (Security)

```
Files: crates/hyperbox-optimize/src/storage/erofs.rs
LOC: 600 (450 impl + 200 tests)
Target: 30-50% faster images (Linux 5.19+)
Dependencies: Linux kernel 5.19+
Blockers: Kernel version, EROFS tools availability
Success Criteria: Benchmark shows 30-50% improvement on 5.19+
```

**Scope:**
- Implement EROFS (Enhanced Read-Only File System) support
- Integrate Fscache for lazy-loading capability
- Automatic fallback to composefs for older kernels
- Lazy loading pipeline optimization
- Comprehensive benchmarking vs composefs

**Acceptance Criteria:**
- 30-50% faster image pulls on Linux 5.19+
- Graceful degradation on older kernels
- Automatic format detection and fallback
- All integration tests passing (20+)
- Zero data corruption in stress tests

---

### STREAM C: OpenTelemetry eBPF Integration
**Assigned Team:** @QUANTUM (Primary), @NEURAL (Support), @PULSE (Monitoring)

```
Files: crates/hyperbox-daemon/src/observability/ebpf.rs
LOC: 500 (400 impl + 150 tests)
Target: Zero-code observability (<2% CPU overhead)
Dependencies: Linux kernel 5.1+ (eBPF support)
Blockers: Kernel eBPF support availability
Success Criteria: Automatic tracing on all platforms
```

**Scope:**
- Implement eBPF-based system call tracing
- Automatic OpenTelemetry span generation
- Zero-code instrumentation (no app changes needed)
- Expose traces via `/traces/*` endpoints
- Send to OpenTelemetry collector
- CPU overhead tracking and optimization

**Acceptance Criteria:**
- <2% CPU overhead in production workloads
- Automatic tracing of 95%+ of system calls
- OpenTelemetry spec compliance
- Graceful fallback on systems without eBPF
- All integration tests passing (18+)

---

### STREAM D: Seccomp Auto-generation
**Assigned Team:** @CIPHER (Primary), @FORTRESS (Support), @APEX (Core)

```
Files: crates/hyperbox-core/src/isolation/seccomp_gen.rs
LOC: 300 (250 impl + 100 tests)
Target: 50-80% smaller default profiles
Dependencies: None (new module)
Blockers: None anticipated
Success Criteria: Generated profiles 50-80% smaller than default
```

**Scope:**
- Implement dynamic seccomp profile generation
- Learn-mode tracing of actual syscall usage
- Automatic profile generation with --learn-seccomp flag
- Store profiles in `/var/lib/hyperbox/seccomp/`
- Validation against known workload patterns
- Profile optimization and cleanup

**Acceptance Criteria:**
- 50-80% smaller profiles than defaults
- <5% false negatives in profile generation
- Zero security regressions vs default
- All unit tests passing (12+)
- Deterministic profile generation

---

## B) INTEGRATION POINTS

### PSI Memory Monitoring Integration
```
Hook into: crates/hyperbox-daemon/src/state.rs
├─ Add PSI metrics collection to state struct
├─ Polling interval: 5 seconds (configurable)
└─ Storage: In-memory circular buffer (last 100 samples)

API Endpoint: /metrics/memory/psi
├─ Response format: JSON with pressure metrics
├─ Fields: some_cpu_us, some_memory_us, full_memory_us
└─ Sampling: 10-second average

Integrate with: swap tuning logic
├─ Auto-adjust swap parameters when pressure >80%
├─ Trigger kernel page cache optimization
└─ Update daemon swap settings

Success Metrics:
├─ Memory pressure <50% under normal load
├─ Memory pressure <75% under high load
├─ Swap usage reduced by 5-15%
└─ No performance degradation
```

### EROFS + Fscache Integration
```
Hook into: crates/hyperbox-optimize/src/storage/mod.rs
├─ Add erofs module to storage subsystem
├─ Detection: kernel version check at startup
└─ Configuration: erofs_enabled flag (default: auto)

Fallback Strategy: composefs (existing)
├─ Try EROFS on Linux 5.19+
├─ Automatic fallback if unavailable
├─ Transparent to user

Integrate with: lazy loading pipeline
├─ EROFS enables on-demand loading
├─ Fscache provides persistence
├─ Reduces initial pull time

Success Metrics:
├─ Image pull time: 30-50% faster
├─ Storage usage: 5-10% reduction
├─ First-use latency: 20-30% improvement
└─ Zero data corruption
```

### OpenTelemetry eBPF Integration
```
Hook into: crates/hyperbox-daemon/src/main.rs
├─ Initialize eBPF programs on startup
├─ Check kernel version (5.1+)
└─ Graceful failure if unsupported

Export to: /traces/* endpoints
├─ Spans exposed via HTTP/gRPC
├─ Real-time streaming available
└─ Batching for efficiency

Send to: OpenTelemetry collector
├─ Default: localhost:4317
├─ Configurable endpoint
└─ Fallback: in-process storage

Success Metrics:
├─ CPU overhead: <2%
├─ Memory overhead: <50MB
├─ Trace latency: <100ms
└─ Span count: 95%+ coverage
```

### Seccomp Auto-generation Integration
```
Hook into: crates/hyperbox-core/src/isolation/mod.rs
├─ Add seccomp_gen module
├─ Export functions: generate(), validate(), apply()
└─ Integration point: container startup

Add flag: --learn-seccomp
├─ Enables learning mode
├─ Traces all syscalls
├─ Generates optimized profile at end

Store: /var/lib/hyperbox/seccomp/
├─ Generated profiles per workload
├─ Profile metadata (generation time, stats)
├─ Backup of default profiles

Success Metrics:
├─ Profile size: 50-80% smaller
├─ False negatives: <5%
├─ Security: zero regressions
└─ Performance: 2-5% improvement
```

---

## C) MILESTONE TIMELINE

### Week 2 (Parallel Development)
- **Day 1-2:** Architecture finalization, environment setup
- **Day 3-4:** Core implementation (all streams)
- **Day 5:** Automated testing, integration checkpoint

### Week 3-4 (Integration & Optimization)
- **Week 3:** Cross-stream integration, performance tuning
- **Week 4:** Final testing, documentation completion

### Week 5 (Validation & Release)
- **Day 1-2:** Final validation, merge preparation
- **Day 3-4:** Staging environment testing
- **Day 5:** Production release readiness

---

## D) DEPENDENCIES & RISK MITIGATION

| Stream | Dependency | Risk | Mitigation |
|--------|-----------|------|-----------|
| A (PSI) | Linux kernel | Low | Graceful degradation on non-Linux |
| B (EROFS) | Linux 5.19+ | Medium | Automatic composefs fallback |
| C (eBPF) | Linux 5.1+ | Medium | Skip tracing on unsupported systems |
| D (Seccomp) | libc/libseccomp | Low | Vendored dependencies |

---

## E) QUALITY GATES

All streams must meet these before merge:

1. **Compilation:** `cargo build -p <crate>` - zero warnings
2. **Tests:** `cargo test -p <crate>` - 100% passing
3. **Linting:** `cargo clippy -p <crate> -- -D warnings` - zero warnings
4. **Formatting:** `cargo fmt --all -- --check` - verified
5. **Documentation:** All public APIs documented
6. **Performance:** Benchmarks meet targets (5-15%, 30-50%, <2%, 50-80%)
7. **Security:** Security review passed (especially streams B, C, D)
8. **Integration:** Cross-stream integration tests passing

---

## F) SUCCESS CRITERIA

### Overall Phase E Success
- [ ] All 4 streams merged to develop
- [ ] All acceptance criteria met
- [ ] Performance benchmarks validated
- [ ] Zero critical bugs
- [ ] Documentation complete
- [ ] Ready for production release
