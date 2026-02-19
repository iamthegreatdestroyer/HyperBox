# AGENT BRIEFING: PHASE E EXECUTION
**Date:** February 19, 2026
**Effective:** Monday, February 24, 2026 @ 9:30 AM
**Duration:** 4 weeks + 5 days (Feb 24 - Mar 21)

---

## 📢 PHASE E LAUNCH ANNOUNCEMENT

**All 40+ HyperBox Agents:**

We are launching **PHASE E EXECUTION** - the most significant development sprint in HyperBox history. Over the next 4 weeks, all hands are on deck to deliver 4 major features across 4 parallel work streams.

### What You're Delivering
1. **PSI Memory Monitoring** - Real-time memory pressure monitoring for better container efficiency
2. **EROFS + Fscache** - Lightning-fast container image delivery (30-50% faster)
3. **OpenTelemetry eBPF** - Deep observability with kernel-level tracing
4. **Seccomp Auto-generation** - AI-powered security profile generation

### The Scale
- **4 Parallel Streams** running simultaneously
- **40+ Agents** working in coordinated teams
- **1,800+ Lines of Code** to be written
- **65+ Unit Tests** to be created and passed
- **4 Quality Gates** enforced for every merge
- **Daily Synchronization** at 9:30 AM sharp

### The Timeline
- **Feb 24 (Mon):** Phase E Launches - Standup #1
- **Feb 26 (Wed):** Midweek Checkpoint - 40-50% complete target
- **Feb 28 (Fri):** Sprint Completion - Go/No-Go merge decision
- **Mar 3-7:** Integration Phase - Cross-stream validation
- **Mar 10-14:** Optimization Phase - Performance tuning
- **Mar 17-21:** Release Phase - Production deployment

---

## 👥 YOUR TEAM ASSIGNMENT

### If You're a Primary Stream Lead
**You are:** @APEX, @VELOCITY, @QUANTUM, or @CIPHER

**Your Responsibilities:**
1. Lead your stream to successful completion
2. Coordinate your 3-4 supporting agents
3. Daily standup at assigned time (9:30 AM + 30 min offsets)
4. Review all PRs from your stream within 1-2 hours
5. Escalate blockers immediately
6. Hit your performance targets
7. Daily reporting on progress, blockers, and metrics

**Your Daily Schedule:**
- **9:30 AM:** Lead your stream standup (15 min)
- **Throughout day:** Code review, unblock agents, mentor
- **End of day:** Update progress tracking dashboard
- **Wed 4 PM:** Extended checkpoint review
- **Fri 4 PM:** Final sprint review & merge decision

---

### If You're a Support Agent
**You are:** One of the 12 agents supporting the primary leads

**Your Responsibilities:**
1. Support your primary lead with specific deliverables
2. Attend daily 15-minute standup at assigned time
3. Submit PRs for review (expect 1-2 hour turnaround)
4. Write unit tests (target: 5-8 per day)
5. Help unblock other team members
6. Report blockers and risks daily
7. Hit your individual contribution targets

**Your Daily Schedule:**
- **9:30 AM slot assigned to your stream:** Attend standup (15 min)
- **Throughout day:** Write code, submit PRs, review peers
- **Daily target:** 60-150 LOC or 2-8 unit tests
- **Blocker:** Escalate within 1 hour of discovery

---

### If You're an Architect/Reviewer
**You are:** @ARCHITECT, @ARBITRER, @SCRIBE, or @NEXUS

**Your Responsibilities:**
1. Ensure design quality across all streams
2. Review all PRs for architectural soundness
3. Validate quality gates are met (0 warnings, 100% tests, etc.)
4. Document and clarify integration points
5. Escalate design issues to @APEX immediately
6. Provide mentorship and guidance

**Your Daily Schedule:**
- **9:30 AM:** Optional attendance at standups
- **Throughout day:** Code review (1-2 hour SLA)
- **Wed/Fri:** Participate in checkpoint reviews
- **As needed:** Architecture consultations

---

## 🎯 WHAT SUCCESS LOOKS LIKE

### For Your Individual Contribution
✅ You complete your assigned deliverables on time
✅ Your code compiles with zero warnings
✅ Your tests pass 100%
✅ Your PRs get approved within 2 hours
✅ You escalate blockers immediately
✅ You report progress honestly at daily standups
✅ You help unblock other team members

### For Your Stream
✅ 90%+ code complete by Fri Feb 28
✅ 90%+ tests passing by Fri Feb 28
✅ Performance targets on track
✅ Zero critical bugs
✅ Merged and validated by Wed Mar 5
✅ Ready for production by Mar 21

### For PHASE E
✅ All 4 streams shipped on time
✅ All performance targets achieved
✅ Zero security regressions
✅ 100% test coverage maintained
✅ Full documentation complete
✅ Production-ready release

---

## 📋 THE DAILY STANDUP (9:30 AM START)

### Timing (Staggered by Stream)
```
Stream A (PSI):       9:30 AM  - @APEX leading
Stream B (EROFS):     10:00 AM - @VELOCITY leading
Stream C (eBPF):      10:30 AM - @QUANTUM leading
Stream D (Seccomp):   11:00 AM - @CIPHER leading
```

### What You Report (Each Person)
1. **What did you complete yesterday?**
   - Tasks finished
   - LOC written (estimate)
   - Tests added

2. **What's your current progress?**
   - % of your deliverable complete
   - Code status (compiled? tests passing?)
   - Any quality issues?

3. **What's your plan for today?**
   - Specific tasks you'll complete
   - Target LOC/tests for the day
   - Integration points with other streams

4. **Do you have any blockers?**
   - What's preventing progress?
   - How severe (High/Medium/Low)?
   - What's your workaround?

5. **Does anyone else need to know?**
   - Other stream dependencies?
   - @ARCHITECT review needed?
   - Security check required?

6. **What metrics matter?**
   - Code quality issues?
   - Performance impact?
   - Test coverage status?

### Format
- **Location:** Slack thread in #development
- **Duration:** 15 minutes total (all team members speaking)
- **Format:** Each person 2-3 minutes, stream lead summarizes
- **Record:** Screenshot/summary posted after
- **Required:** All team members must attend or async report

### Daily Targets
- **Minimum:** 60 LOC written OR 2 unit tests created
- **Expected:** 100-150 LOC written OR 5-8 unit tests created

---

## 🔄 THE CODE REVIEW PROCESS

### When You Submit a PR
1. **Branch Name:** Use assigned stream branch (feat/psi, feat/erofs, feat/otel-ebpf, feat/seccomp-gen)
2. **PR Title:** `[PHASE-E] Feature Name - Stream X`
3. **Checklist:** Include completed task checklist in PR description
4. **Assign Reviewers:**
   - Primary: Your stream lead
   - Secondary: @ARCHITECT (always)
   - Quality: @ARBITRER (if applicable)

### What Gets Reviewed
✓ Code compiles with zero warnings
✓ 100% of tests pass
✓ Clippy linting passes (zero warnings)
✓ Code is formatted (cargo fmt)
✓ Doc examples compile and pass
✓ Design is sound (no breaking changes)
✓ Performance impact acceptable
✓ Security implications reviewed
✓ Error handling is proper
✓ Tests are comprehensive

### SLA for Reviews
- **Expected:** 1-2 hours turnaround
- **If >2 hours:** Ping reviewer in Slack
- **If >3 hours:** Escalate to @ARCHITECT
- **If >4 hours:** Escalate to @APEX (Phase Lead)

### After Approval
- Squash merge to your feature branch
- Delete feature branch after merge
- Update progress tracking
- Move to next task

---

## 🚨 BLOCKER PROTOCOL

### If You Get Stuck
1. **Acknowledge the blocker** - Don't wait
2. **Post in #phase-e-critical** on Slack
3. **Tag the right person:**
   - Technical issue → @ARCHITECT
   - Security issue → @CIPHER
   - Performance issue → @VELOCITY
   - Quality issue → @ARBITRER
   - Compilation error → @APEX
4. **Include context:**
   - What's the issue?
   - Why is it blocking you?
   - What have you tried?
   - What's the timeline impact?
5. **Expect response:** Within 15-30 minutes
6. **If not resolved in 1 hour:** Escalate to @APEX immediately

### Escalation Chain
```
You → Stream Lead (15 min SLA)
      ↓
Stream Lead → @ARCHITECT or domain expert (15 min SLA)
      ↓
Domain Expert → @APEX (Phase Lead) (10 min SLA)
```

### Examples of Critical Blockers
- Compilation error blocking your branch
- Test infrastructure issue
- Dependency conflict
- Security concern discovered
- Performance regression unexpected
- Design question blocking implementation

---

## ✅ QUALITY GATES (YOU MUST PASS THESE)

### Gate 1: Compilation
Before you submit a PR:
```bash
cargo build --release
```
✅ **Must:** Compile with zero errors
✅ **Must:** Compile with zero warnings

### Gate 2: Testing
Before you submit a PR:
```bash
cargo test --all
```
✅ **Must:** All tests pass (100%)
✅ **Must:** No flaky tests
✅ **Must:** Test coverage >80%

### Gate 3: Linting
Before you submit a PR:
```bash
cargo clippy --all -- -D warnings
```
✅ **Must:** Zero clippy warnings
✅ **Must:** Address all suggestions

### Gate 4: Formatting
Before you submit a PR:
```bash
cargo fmt --all
```
✅ **Must:** All code properly formatted
✅ **Must:** No trailing whitespace

### Gate 5: Documentation
Before you submit a PR:
```bash
cargo test --doc
```
✅ **Must:** All doc examples compile
✅ **Must:** All doc examples pass

### If Any Gate Fails
- Fix the issue locally
- Re-run the gate
- Push the fix
- Notify your reviewer
- Do NOT ask for exception to quality gates

---

## 📊 DAILY PROGRESS TRACKING

### Update Your Status Daily
Tracked metrics (Stream Leads report):
- **Code completion %** (target: +10-15% per day Week 2)
- **Tests passing %** (target: 90%+ by Fri)
- **Blockers count** (target: 0)
- **PR reviews pending** (target: 0 >2 hrs old)
- **Performance vs target** (trend tracking)

### Weekly Checkpoint Meetings
- **Wed 4 PM:** Midweek status (40-50% target)
- **Fri 4 PM:** Sprint completion (90%+ target) + merge decision

### Public Dashboard
Updated daily in repo:
```
PHASE_E_EXECUTION_DASHBOARD.md
```
Check it to see where all streams stand.

---

## 🎯 YOUR SPECIFIC DELIVERABLES

### STREAM A - PSI Memory Monitoring (@APEX)
```
Goal: 5-15% memory pressure reduction
Target: 400 LOC, 15+ tests

@APEX (Primary):
  - crates/hyperbox-core/src/memory/psi.rs (PSI reader)
  - Unit tests with edge cases
  - Integration with swap tuning

@VELOCITY (Support):
  - Performance benchmarks
  - CPU/memory overhead analysis
  - Pressure threshold optimization

@PULSE (Support):
  - /metrics/memory/psi endpoint
  - Dashboard templates
  - Real-time monitoring integration
```

### STREAM B - EROFS + Fscache (@VELOCITY)
```
Goal: 30-50% faster image pulls
Target: 600 LOC, 20+ tests

@VELOCITY (Primary):
  - crates/hyperbox-optimize/src/storage/erofs.rs
  - Fscache integration
  - Integration tests with real images

@APEX (Support):
  - Runtime integration
  - Kernel version detection
  - Fallback logic

@CIPHER (Support):
  - Security validation
  - Access control review
  - EROFS security analysis
```

### STREAM C - OpenTelemetry eBPF (@QUANTUM)
```
Goal: <2% CPU overhead, 95%+ syscall coverage
Target: 500 LOC, 18+ tests

@QUANTUM (Primary):
  - crates/hyperbox-daemon/src/observability/ebpf.rs
  - eBPF program development
  - OpenTelemetry integration

@NEURAL (Support):
  - Trace data analytics
  - Performance telemetry
  - Statistical analysis

@PULSE (Support):
  - /traces/* endpoint
  - Real-time streaming
  - Dashboard templates
```

### STREAM D - Seccomp Auto-generation (@CIPHER)
```
Goal: 50-80% smaller profiles
Target: 300 LOC, 12+ tests

@CIPHER (Primary):
  - crates/hyperbox-core/src/isolation/seccomp_gen.rs
  - Profile generation algorithm
  - Learning mode (--learn-seccomp)

@FORTRESS (Support):
  - Security hardening validation
  - False positive/negative testing
  - Profile quality metrics

@APEX (Support):
  - Runtime integration
  - Container lifecycle integration
  - Backward compatibility
```

---

## 🏆 RECOGNITION & REWARDS

### For Individual Contributors
✅ Deliver on time + quality = Public recognition
✅ Go above and beyond = Bonus tasks/learning
✅ Unblock others = Leadership credit

### For Stream Teams
✅ Hit performance targets = Team celebration
✅ Ship on time = Phase F priority assignments
✅ Zero critical bugs = Early Phase F access

### For Phase E Completion
✅ All streams shipped = Company celebration
✅ No major blockers = Vacation day
✅ Performance exceeded = Special recognition

---

## 📚 REFERENCE MATERIALS

All documents in `/s/HyperBox/`:

1. **PHASE_E_ARCHITECTURE.md** - Technical design for all 4 streams
2. **PHASE_E_ASSIGNMENTS.md** - Detailed role assignments
3. **PHASE_E_MERGE_PROTOCOL.md** - Code review & quality gates
4. **PHASE_E_CHECKPOINTS.md** - Sync schedule and go/no-go criteria
5. **DAILY_STANDUP_TEMPLATE.md** - Standup format and examples
6. **BENCHMARK_PLAN.md** - Performance measurement plan
7. **PHASE_E_STATUS_REPORT.md** - Overall readiness status
8. **PHASE_E_EXECUTION_DASHBOARD.md** - Real-time progress tracking (NEW)

---

## 🚀 LAUNCH DAY CHECKLIST (MONDAY FEB 24)

### Before 9:30 AM Monday
- [ ] Read PHASE_E_ARCHITECTURE.md for your stream
- [ ] Read your role description in PHASE_E_ASSIGNMENTS.md
- [ ] Pull latest code: `git pull origin main`
- [ ] Checkout your feature branch: `git checkout feat/[your-stream]`
- [ ] Verify local build: `cargo build && cargo test`
- [ ] Have Slack open and ready
- [ ] Have your IDE and terminal ready

### At 9:30 AM Monday (Stream A Starts)
- Standup begins in #development thread
- Stream leads, post your team's kickoff message
- Agents, stand by for first standup

### 10:00 AM Monday (Stream B Starts)
- Stream B standup begins
- Code implementation underway

### 10:30 AM Monday (Stream C Starts)
- Stream C standup begins
- All teams actively coding

### 11:00 AM Monday (Stream D Starts)
- Stream D standup begins
- All 4 streams in full execution

---

## 💪 FINAL MESSAGE FROM LEADERSHIP

Team, this is the most ambitious sprint in HyperBox history. We're asking you to deliver 4 major features across 4 parallel streams in just 4 weeks. We believe in your ability to do this.

**What we expect:**
- Daily communication and transparency
- Immediate escalation of blockers (don't wait)
- High code quality (zero warnings, 100% tests)
- Honest progress reporting
- Mutual support across streams

**What you can expect from us:**
- Clear guidance and architecture
- Fast code review turnaround (1-2 hours)
- Immediate blocker resolution (15-30 min SLA)
- Recognition for great work
- Celebration when we ship

**Our target:** Ship all 4 features on time with zero critical bugs and all performance targets achieved.

**Your commitment:** You will attend daily standups, submit high-quality code, write comprehensive tests, and escalate blockers immediately.

**Let's build something great together.** 🚀

---

## 📞 EMERGENCY CONTACTS

If you need immediate help:
- **Phase Lead (@APEX):** #phase-e-critical
- **Architects (@ARCHITECT):** #phase-e-critical
- **Security (@CIPHER):** #phase-e-critical
- **Performance (@VELOCITY):** #phase-e-critical
- **Quality (@ARBITRER):** #phase-e-critical

---

**PHASE E Execution Begins: Monday, February 24, 2026 @ 9:30 AM**

**Your stream is counting on you. Let's go! 🚀**

---

Document prepared: February 19, 2026
For: All 40+ HyperBox Agents
Status: Ready for Distribution
