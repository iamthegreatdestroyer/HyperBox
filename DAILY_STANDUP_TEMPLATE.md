# Phase E - Daily Standup Template

## Schedule & Format

**When:** 9:30 AM Daily (Monday - Friday)
**Duration:** 15 minutes maximum
**Location:** Slack #development channel (thread format)
**Duration Format:** 15-minute standup + 5-minute overflow/blockers (if needed)

---

## STANDUP THREAD STRUCTURE

Post daily standup as a **Slack thread** in #development:

```
📋 PHASE E Daily Standup - [STREAM NAME] - [DATE]
```

Each agent reports in this format:

---

## STANDUP TEMPLATE (Copy/Paste Daily)

```
╔══════════════════════════════════════════════════════════════════════════════╗
║                  PHASE E STANDUP - [DATE: Day, Date Time]                   ║
║                    Stream: [STREAM NAME] | Date: YYYY-MM-DD                 ║
╚══════════════════════════════════════════════════════════════════════════════╝

📍 AGENT: @[AGENT_NAME]
├─ Role: [Primary/Support/Reviewer]
└─ Allocation: [X]% of time to Phase E

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ COMPLETED YESTERDAY:
├─ [Task 1 with result]
│  └─ Details: [e.g., "Implemented PSI reader, 150 LOC"]
├─ [Task 2 with result]
│  └─ Details: [e.g., "Added unit tests, 23/23 passing"]
├─ [Task 3 with result]
│  └─ Details: [e.g., "Resolved kernel version detection"]
└─ Tests: [N passing, 0 failing]

📊 PROGRESS SNAPSHOT:
├─ Overall: [X]% complete toward acceptance criteria
├─ Code: [Y LOC written, Z LOC remaining]
├─ Tests: [A/B unit tests passing]
├─ Performance: [Current metric vs target]
└─ Status: [On track / At risk / Blocked]

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📅 TODAY'S PLAN:
├─ Task 1: [Description]
│  └─ Target: [e.g., "50 LOC, 5 tests"]
├─ Task 2: [Description]
│  └─ Target: [e.g., "3 unit tests, pass all"]
├─ Task 3: [Description]
│  └─ Target: [e.g., "Performance benchmarking"]
└─ Daily Target: [X LOC or Y tests or Z hours]

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🚨 BLOCKERS / RISKS:
├─ [ ] Blocker 1: [Description]
│  ├─ Severity: [Critical / High / Medium / Low]
│  ├─ Impact: [What is blocked?]
│  ├─ Workaround: [Temporary solution or N/A]
│  └─ Owner: [@AGENT or "needs assignment"]
│
└─ [ ] No blockers - clear to proceed

⚠️  RISKS (May become blockers):
├─ [Risk description]
│  └─ Mitigation: [What will prevent this?]
└─ [None identified]

📞 ESCALATION NEEDED:
├─ Technical: [Yes/No] → Escalate to @ARCHITECT
├─ Security: [Yes/No] → Escalate to @CIPHER
├─ Performance: [Yes/No] → Escalate to @VELOCITY
├─ Quality: [Yes/No] → Escalate to @ARBITRER
└─ Documentation: [Yes/No] → Escalate to @SCRIBE

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📈 METRICS:
├─ Code Quality:
│  ├─ Clippy warnings: 0
│  ├─ Test coverage: [X]%
│  └─ Doc coverage: [X]%
│
├─ Performance:
│  ├─ Benchmark metric: [Current value]
│  ├─ Target: [Target value]
│  └─ Status: [On track / Behind / Ahead]
│
├─ Dependencies:
│  ├─ Waiting on: [None / list items]
│  └─ Blocking: [None / list items]
│
└─ Confidence:
   ├─ Week completion: [0-100]%
   └─ Phase completion: [0-100]%

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

💡 NOTES / CONTEXT:
├─ Branch: [feat/psi, feat/erofs, feat/otel-ebpf, feat/seccomp-gen]
├─ Last commit: [timestamp, brief description]
├─ PRs in review: [#XXX, #YYY, or "none"]
└─ Upcoming risks: [List anything coming up]

═══════════════════════════════════════════════════════════════════════════════
```

---

## STANDUP EXAMPLES BY STREAM

### Example: @APEX (PSI Memory - Day 3)

```
╔══════════════════════════════════════════════════════════════════════════════╗
║                  PHASE E STANDUP - Monday, Feb 24, 2026                      ║
║               Stream A: PSI Memory Monitoring | Branch: feat/psi             ║
╚══════════════════════════════════════════════════════════════════════════════╝

📍 AGENT: @APEX
├─ Role: Primary Lead (Core Runtime Expert)
└─ Allocation: 40% of time to Phase E

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ COMPLETED YESTERDAY:
├─ Implemented PSI metrics reader (150 LOC)
│  └─ Details: Reads from /proc/pressure/memory, parses correctly
├─ Added 8 unit tests for PSI parsing
│  └─ Details: 8/8 passing, edge cases covered
├─ Resolved kernel version detection
│  └─ Details: Graceful fallback on non-Linux
└─ Tests: 8/8 passing

📊 PROGRESS SNAPSHOT:
├─ Overall: 35% complete
├─ Code: 150/400 LOC written
├─ Tests: 8/15 unit tests complete
├─ Performance: Baseline measurement pending
└─ Status: On track

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📅 TODAY'S PLAN:
├─ Task 1: Integrate with daemon state.rs
│  └─ Target: 80 LOC, establish data structures
├─ Task 2: Add swap tuning integration
│  └─ Target: 70 LOC, hook into existing logic
├─ Task 3: Add 4 more unit tests
│  └─ Target: 4 tests, all passing
└─ Daily Target: 150 LOC + 4 tests

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🚨 BLOCKERS / RISKS:
├─ [ ] None identified
└─ [ ] All systems go

⚠️  RISKS (May become blockers):
├─ Linux-only feature (mitigation: non-Linux gracefully skip)
└─ None other

📞 ESCALATION NEEDED:
├─ Technical: No
├─ Security: No
├─ Performance: No
├─ Quality: No
└─ Documentation: No

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📈 METRICS:
├─ Code Quality:
│  ├─ Clippy warnings: 0
│  ├─ Test coverage: 85%
│  └─ Doc coverage: 90%
│
├─ Performance:
│  ├─ Monitoring overhead: TBD (pending @VELOCITY benchmarks)
│  ├─ Target: <1% CPU
│  └─ Status: On track (will measure today)
│
├─ Dependencies:
│  ├─ Waiting on: None
│  └─ Blocking: None
│
└─ Confidence:
   ├─ Week completion: 85%
   └─ Phase completion: 60%

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

💡 NOTES / CONTEXT:
├─ Branch: feat/psi (up-to-date with develop)
├─ Last commit: 2h ago, "Add PSI parsing with edge cases"
├─ PRs in review: None yet (feature still in progress)
└─ Upcoming: Tomorrow planning /metrics endpoint with @PULSE

═══════════════════════════════════════════════════════════════════════════════
```

---

### Example: @VELOCITY (EROFS - Day 5)

```
╔══════════════════════════════════════════════════════════════════════════════╗
║                  PHASE E STANDUP - Friday, Feb 28, 2026                      ║
║             Stream B: EROFS + Fscache | Branch: feat/erofs                  ║
╚══════════════════════════════════════════════════════════════════════════════╝

📍 AGENT: @VELOCITY
├─ Role: Primary Lead (Performance Expert)
└─ Allocation: 40% of time to Phase E

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ COMPLETED YESTERDAY:
├─ Integrated EROFS with composefs fallback (120 LOC)
│  └─ Details: Auto-detection of kernel version, seamless fallback
├─ Added integration test suite (8 tests)
│  └─ Details: Tests for both EROFS and fallback paths
├─ Performance benchmarks: 35% improvement measured
│  └─ Details: 100MB image: 2.8s (EROFS) vs 4.3s (composefs)
└─ Tests: 12/20 passing

📊 PROGRESS SNAPSHOT:
├─ Overall: 60% complete
├─ Code: 280/600 LOC written
├─ Tests: 12/20 integration tests complete
├─ Performance: 35% measured (target: 30-50%)
└─ Status: On track

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📅 TODAY'S PLAN:
├─ Task 1: Add Fscache persistence layer (100 LOC)
│  └─ Target: Complete implementation
├─ Task 2: Add 5 more integration tests for Fscache
│  └─ Target: 5 tests, all passing
├─ Task 3: Performance tune lazy-loading pipeline
│  └─ Target: Achieve 40%+ improvement
└─ Daily Target: 100 LOC + 5 tests + performance tuning

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🚨 BLOCKERS / RISKS:
├─ [ ] Fscache dependency availability
│  ├─ Severity: Medium
│  ├─ Impact: Feature requires Fscache library
│  ├─ Workaround: Vendored dependency fallback prepared
│  └─ Owner: Resolving today
└─ [ ] No other blockers

⚠️  RISKS (May become blockers):
├─ Older kernel testing not complete (mitigation: @APEX handling fallback)
└─ Large file stress tests still needed

📞 ESCALATION NEEDED:
├─ Technical: No
├─ Security: No (covered by @CIPHER review)
├─ Performance: No (on track)
├─ Quality: No
└─ Documentation: No

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📈 METRICS:
├─ Code Quality:
│  ├─ Clippy warnings: 0
│  ├─ Test coverage: 82%
│  └─ Doc coverage: 88%
│
├─ Performance:
│  ├─ Current improvement: 35% (100MB image)
│  ├─ Target: 30-50%
│  └─ Status: Ahead of schedule, pushing for 45%+
│
├─ Dependencies:
│  ├─ Waiting on: None
│  └─ Blocking: None
│
└─ Confidence:
   ├─ Week completion: 90%
   └─ Phase completion: 70%

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

💡 NOTES / CONTEXT:
├─ Branch: feat/erofs (ahead of develop, ready for cherry-picks)
├─ Last commit: 4h ago, "Add integration tests for EROFS/composefs"
├─ PRs in review: None yet (checkpoint Friday, merge Monday)
└─ Upcoming: Merge Friday after checkpoint + @ARCHITECT review

═══════════════════════════════════════════════════════════════════════════════
```

---

## DAILY STANDUP CHECKLIST

### Before Standup (Prepare in 5 min)

- [ ] Review git log since yesterday (`git log --oneline -n 10`)
- [ ] Check test results (`cargo test -p <crate> 2>&1 | tail -30`)
- [ ] Count LOC written today (`git diff HEAD~1 --stat`)
- [ ] Note any blockers or risks that emerged
- [ ] Verify clippy/formatting (`cargo clippy -p <crate>`, `cargo fmt --check`)

### Standup (15 minutes)

1. **Post template** (1 min) - Copy/paste and fill out
2. **Wait for reactions** (2 min) - Emoji reactions from team
3. **Respond to questions** (5 min) - Answer team questions
4. **Escalate if needed** (2 min) - Tag appropriate reviewers
5. **Next day prep** (5 min) - Plan tomorrow's work

### After Standup (Sync with team)

- [ ] Review other agents' standups (2 min)
- [ ] Identify any cross-team dependencies
- [ ] Respond to blocking questions immediately
- [ ] Update blockers/risks if new ones emerged

---

## STANDUP FREQUENCY BY STREAM

| Stream | Mon | Tue | Wed | Thu | Fri | Weekend |
|--------|-----|-----|-----|-----|-----|---------|
| A - PSI | 9:30 | 9:30 | 9:30 | 9:30 | 9:30 | Off |
| B - EROFS | 9:30 | 9:30 | 9:30 | 9:30 | 9:30 | Off |
| C - eBPF | 9:30 | 9:30 | 9:30 | 9:30 | 9:30 | Off |
| D - Seccomp | 9:30 | 9:30 | 9:30 | 9:30 | 9:30 | Off |

**Special Days:**
- **Wed EOD:** Checkpoint standup (extended, all streams)
- **Fri EOD:** Weekly review standup (extended, all streams)

---

## ESCALATION FROM STANDUP

If standup reveals a blocker:

1. **Post blocker in standup** with severity
2. **Tag appropriate owner** (see PHASE_E_ASSIGNMENTS.md)
3. **Set resolution deadline** (usually same day)
4. **Daily follow-up** in next standup until resolved

**Critical Blocker (blocks all work):**
- Escalate to @APEX immediately
- Separate #phase-e-critical channel if needed

---

## STANDUP METRICS TRACKED

Each standup provides real-time visibility into:

- **Velocity:** LOC/tests per day per agent
- **Quality:** Test coverage, bugs found/fixed
- **Blockers:** What's slowing us down
- **Confidence:** Are we on track?
- **Dependencies:** Who is waiting on whom?

This data feeds into:
- **Daily dashboards** (real-time progress)
- **Weekly reviews** (velocity trends)
- **Phase completion forecast** (are we on schedule?)
