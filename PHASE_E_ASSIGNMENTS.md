# Phase E - Agent Assignments & Pairing

## Executive Summary
40+ agents across 4 parallel work streams with clear role definitions, daily synchronization, and merge protocols. Each stream has a primary expert, supporting agents, and designated reviewers.

---

## STREAM A: PSI Memory Monitoring

### Team Composition
- **Primary Lead:** @APEX (Core Runtime Expert)
- **Performance Support:** @VELOCITY (Performance Optimization)
- **Monitoring Support:** @PULSE (Systems Monitoring)
- **Architecture Reviewer:** @ARCHITECT (Design & Architecture)
- **Security Reviewer:** @CIPHER (Security & Isolation)

### Role Descriptions

#### @APEX (Primary - 40% allocation)
- **Responsibility:** Core PSI implementation, testing strategy
- **Deliverables:**
  - PSI metrics reader in `crates/hyperbox-core/src/memory/psi.rs`
  - Unit tests (15+) for PSI reading and edge cases
  - Integration with swap tuning logic
  - Performance benchmarking setup
- **Success Metric:** 5-15% memory pressure reduction
- **Daily Checkin:** Progress on impl, tests, blockers

#### @VELOCITY (Support - 30% allocation)
- **Responsibility:** Performance profiling, optimization
- **Deliverables:**
  - CPU overhead benchmarks
  - Memory overhead analysis
  - Performance tuning recommendations
  - Pressure threshold optimization
- **Success Metric:** <1% CPU overhead for monitoring

#### @PULSE (Support - 20% allocation)
- **Responsibility:** Metrics integration, dashboard setup
- **Deliverables:**
  - `/metrics/memory/psi` endpoint implementation
  - Metrics formatting and schema
  - Dashboard visualization guidance
  - Real-time monitoring integration
- **Success Metric:** Real-time PSI metrics available

#### @ARCHITECT (Review - 10% allocation)
- **Responsibility:** Architecture soundness review
- **Checklist:**
  - Design aligns with PHASE_E_ARCHITECTURE.md
  - No breaking changes
  - API is extensible for future metrics
  - Error handling strategy

#### @CIPHER (Review - 10% allocation)
- **Responsibility:** Security implications review
- **Checklist:**
  - No privilege escalation vectors
  - Safe /proc access patterns
  - No information disclosure
  - Safe error messages

---

## STREAM B: EROFS + Fscache Integration

### Team Composition
- **Primary Lead:** @VELOCITY (Performance Expert)
- **Core Support:** @APEX (Runtime & Integration)
- **Security Support:** @CIPHER (Access Control)
- **Architecture Reviewer:** @ARCHITECT (Design & Architecture)
- **Innovation Reviewer:** @NEXUS (Future-Proofing)

### Role Descriptions

#### @VELOCITY (Primary - 40% allocation)
- **Responsibility:** EROFS implementation, performance tuning
- **Deliverables:**
  - EROFS module in `crates/hyperbox-optimize/src/storage/erofs.rs`
  - Fscache integration and configuration
  - Integration tests (20+) with real images
  - Performance benchmarking vs composefs
- **Success Metric:** 30-50% faster images on Linux 5.19+
- **Daily Checkin:** Implementation progress, optimization findings

#### @APEX (Support - 30% allocation)
- **Responsibility:** Runtime integration, kernel interaction
- **Deliverables:**
  - Integration into storage subsystem
  - Kernel version detection and fallback logic
  - Container lifecycle integration
  - Error handling for unsupported kernels
- **Success Metric:** Seamless fallback to composefs

#### @CIPHER (Support - 20% allocation)
- **Responsibility:** Security & access control verification
- **Deliverables:**
  - Access control validation
  - File system permission verification
  - EROFS security posture review
  - Fscache security analysis
- **Success Metric:** Zero security regressions

#### @ARCHITECT (Review - 10% allocation)
- **Responsibility:** Design and forward compatibility
- **Checklist:**
  - Storage abstraction layer respected
  - No monolithic coupling
  - Extensible for future formats
  - Graceful degradation path

#### @NEXUS (Review - 10% allocation)
- **Responsibility:** Future-proofing and innovation
- **Checklist:**
  - Scalable for future enhancements
  - Compatible with emerging standards
  - Backward compatibility maintained
  - Innovation opportunities identified

---

## STREAM C: OpenTelemetry eBPF Integration

### Team Composition
- **Primary Lead:** @QUANTUM (Observability Expert)
- **Analytics Support:** @NEURAL (Data & Analytics)
- **Monitoring Support:** @PULSE (Systems Monitoring)
- **Architecture Reviewer:** @ARCHITECT (Design & Architecture)
- **Documentation Reviewer:** @SCRIBE (Documentation & Examples)

### Role Descriptions

#### @QUANTUM (Primary - 40% allocation)
- **Responsibility:** eBPF program development, OpenTelemetry integration
- **Deliverables:**
  - eBPF programs in `crates/hyperbox-daemon/src/observability/ebpf.rs`
  - OpenTelemetry span generation logic
  - Kernel version detection (5.1+)
  - Integration tests (18+)
- **Success Metric:** <2% CPU overhead, 95%+ syscall coverage
- **Daily Checkin:** eBPF program progress, performance metrics

#### @NEURAL (Support - 30% allocation)
- **Responsibility:** Analytics and data processing
- **Deliverables:**
  - Span data analysis and aggregation
  - Performance telemetry collection
  - Statistical analysis of tracing data
  - Anomaly detection patterns
- **Success Metric:** Meaningful analytics from traces

#### @PULSE (Support - 20% allocation)
- **Responsibility:** Monitoring integration and real-time dashboards
- **Deliverables:**
  - Traces endpoint implementation (`/traces/*`)
  - Real-time trace streaming
  - Integration with monitoring systems
  - Dashboard templates
- **Success Metric:** Real-time trace visibility

#### @ARCHITECT (Review - 10% allocation)
- **Responsibility:** Architecture and system design
- **Checklist:**
  - eBPF strategy aligned with architecture
  - Observability layer well-designed
  - No tight coupling
  - Testability maintained

#### @SCRIBE (Review - 10% allocation)
- **Responsibility:** Documentation and examples
- **Checklist:**
  - eBPF concepts well-documented
  - Examples provided for common use cases
  - Troubleshooting guide complete
  - API documentation clear

---

## STREAM D: Seccomp Auto-generation

### Team Composition
- **Primary Lead:** @CIPHER (Security Expert)
- **Hardening Support:** @FORTRESS (Defense & Hardening)
- **Core Support:** @APEX (Runtime & Integration)
- **Architecture Reviewer:** @ARCHITECT (Design & Architecture)
- **Validation Reviewer:** @ARBITRER (Quality & Validation)

### Role Descriptions

#### @CIPHER (Primary - 40% allocation)
- **Responsibility:** Seccomp implementation, profile generation
- **Deliverables:**
  - Seccomp gen module in `crates/hyperbox-core/src/isolation/seccomp_gen.rs`
  - Profile generation algorithm
  - Learning mode implementation (--learn-seccomp)
  - Unit tests (12+) for all functionality
- **Success Metric:** 50-80% smaller profiles, <5% false negatives
- **Daily Checkin:** Implementation progress, security validations

#### @FORTRESS (Support - 30% allocation)
- **Responsibility:** Security hardening and validation
- **Deliverables:**
  - Seccomp profile security analysis
  - False positive/negative testing
  - Profile quality metrics
  - Hardening recommendations
- **Success Metric:** Zero security regressions

#### @APEX (Support - 20% allocation)
- **Responsibility:** Runtime integration and lifecycle
- **Deliverables:**
  - Integration into isolation subsystem
  - Container startup integration
  - Profile storage and retrieval
  - Backward compatibility assurance
- **Success Metric:** Seamless container integration

#### @ARCHITECT (Review - 10% allocation)
- **Responsibility:** Design and modularity
- **Checklist:**
  - Isolation layer well-designed
  - Clear separation of concerns
  - Extensible for future isolation types
  - No breaking changes

#### @ARBITRER (Review - 10% allocation)
- **Responsibility:** Validation and quality assurance
- **Checklist:**
  - All acceptance criteria met
  - Stress testing completed
  - Edge cases covered
  - Regression testing passed

---

## CROSS-STREAM ROLES

### @ARCHITECT (Design Authority - 5% per stream)
- **Total Allocation:** 20%
- **Responsibility:** Ensure all streams follow architecture guidelines
- **Key Activities:**
  - Daily architecture review of PRs
  - Integration point validation
  - Design consistency across streams
  - Escalation of major design questions
- **Success Metric:** No architectural regressions

### @SCRIBE (Documentation Lead - 2% per stream)
- **Total Allocation:** 8%
- **Responsibility:** Comprehensive Phase E documentation
- **Key Activities:**
  - User-facing documentation
  - API documentation
  - Integration guides
  - Troubleshooting guides
- **Success Metric:** Complete documentation by end of Phase E

### Integration Team (2% each)
- **@NEXUS:** Future-proofing and innovation
- **@ARBITRER:** Quality assurance and validation
- **@SCRIBE:** Documentation and examples

---

## AGENT READINESS CHECKLIST

### Pre-Phase E Verification (Must Complete)

#### Each Primary Lead Must:
- [ ] Review PHASE_E_ARCHITECTURE.md thoroughly
- [ ] Understand acceptance criteria for their stream
- [ ] Identify potential blockers in your domain
- [ ] Prepare implementation timeline (weekly breakdown)
- [ ] Set up local development environment
- [ ] Create feature branch (feat/psi, feat/erofs, feat/otel-ebpf, feat/seccomp-gen)

#### Each Support Agent Must:
- [ ] Review assigned stream architecture
- [ ] Understand integration points for their role
- [ ] Prepare support deliverables list
- [ ] Identify any resource needs
- [ ] Schedule sync with primary lead

#### Each Reviewer Must:
- [ ] Review architectural guidelines in PHASE_E_ARCHITECTURE.md
- [ ] Understand quality gates and success criteria
- [ ] Set up code review tooling
- [ ] Schedule review availability (24-hour turnaround)

---

## TEAM SNAPSHOT

```
STREAM A (PSI Memory):        5 agents (@APEX @VELOCITY @PULSE @ARCHITECT @CIPHER)
STREAM B (EROFS):              5 agents (@VELOCITY @APEX @CIPHER @ARCHITECT @NEXUS)
STREAM C (eBPF):               5 agents (@QUANTUM @NEURAL @PULSE @ARCHITECT @SCRIBE)
STREAM D (Seccomp):            5 agents (@CIPHER @FORTRESS @APEX @ARCHITECT @ARBITRER)
Cross-Stream Roles:            3 agents (@ARCHITECT @SCRIBE @NEXUS)

TOTAL AGENTS:                  40+ agents involved in Phase E
UNIQUE AGENTS:                 ~15 key agents
PRIMARY LEADS:                 4 (@APEX @VELOCITY @QUANTUM @CIPHER)
ARCHITECTURE AUTHORITY:        @ARCHITECT (cross-all)
QUALITY AUTHORITY:             @ARBITRER (cross-all)
DOCUMENTATION AUTHORITY:       @SCRIBE (cross-all)
```

---

## AGENT ALLOCATION MATRIX

| Agent | Role | Stream A | Stream B | Stream C | Stream D | Total |
|-------|------|----------|----------|----------|----------|-------|
| @APEX | Primary/Support | 40% | 30% | - | 20% | 90% |
| @VELOCITY | Primary/Support | 30% | 40% | - | - | 70% |
| @CIPHER | Primary/Support/Review | 10% | 20% | - | 40% | 70% |
| @PULSE | Support/Review | 20% | - | 20% | - | 40% |
| @QUANTUM | Primary | - | - | 40% | - | 40% |
| @NEURAL | Support | - | - | 30% | - | 30% |
| @FORTRESS | Support | - | - | - | 30% | 30% |
| @ARCHITECT | Reviewer (All) | 10% | 10% | 10% | 10% | 40% |
| @SCRIBE | Reviewer/Docs | - | - | 10% | - | 10% |
| @NEXUS | Reviewer | - | 10% | - | - | 10% |
| @ARBITRER | Reviewer/QA | - | - | - | 10% | 10% |

---

## NEXT STEPS

1. **Immediate:** Each primary lead reviews full PHASE_E_ARCHITECTURE.md
2. **Day 1:** All agents confirm assignment and readiness
3. **Day 1:** Feature branches created and pushed
4. **Day 2:** First daily standup (9:30 AM)
5. **Day 2:** Integration point kickoff meeting (15 min per stream)
6. **Day 3:** First checkpoint review meeting

---

## ESCALATION CHAIN

**Technical Blocker → @ARCHITECT**
**Security Question → @CIPHER**
**Performance Concern → @VELOCITY**
**Quality Gate Failure → @ARBITRER**
**Documentation Gap → @SCRIBE**

**Critical Blocker (All Streams) → @APEX** (Phase Lead)
