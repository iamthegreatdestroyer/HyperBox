# Phase E - Quick Start Guide for Agents

**Phase E Duration:** Weeks 2-5 (Feb 24 - Mar 21, 2026)
**This Guide:** Quick reference for all agents

---

## YOUR ROLE AT A GLANCE

### If You're @APEX
**Role:** Primary Lead - PSI Memory Monitoring (Stream A)
- **Allocation:** 40% to Phase E
- **Your Focus:** Implement PSI metrics reader, unit tests, daemon integration
- **Support from:** @VELOCITY (performance), @PULSE (metrics)
- **Reviewers:** @ARCHITECT, @CIPHER
- **Success:** 5-15% memory pressure reduction + all tests passing
- **Daily Check-in:** Report completed tasks, blockers, progress

### If You're @VELOCITY
**Role:** Primary Lead - EROFS + Fscache (Stream B)
- **Allocation:** 40% to Phase E
- **Your Focus:** EROFS implementation, Fscache integration, benchmarking
- **Support from:** @APEX (core), @CIPHER (security)
- **Reviewers:** @ARCHITECT, @NEXUS
- **Success:** 30-50% faster image pulls + all tests passing
- **Daily Check-in:** Report completed tasks, blockers, progress

### If You're @QUANTUM
**Role:** Primary Lead - OpenTelemetry eBPF (Stream C)
- **Allocation:** 40% to Phase E
- **Your Focus:** eBPF programs, OpenTelemetry integration, tracing
- **Support from:** @NEURAL (analytics), @PULSE (monitoring)
- **Reviewers:** @ARCHITECT, @SCRIBE
- **Success:** <2% CPU overhead + 95%+ span coverage
- **Daily Check-in:** Report completed tasks, blockers, progress

### If You're @CIPHER
**Role:** Primary Lead - Seccomp Auto-generation (Stream D)
- **Allocation:** 40% to Phase E
- **Your Focus:** Profile generation algorithm, learning mode, tests
- **Support from:** @FORTRESS (hardening), @APEX (core)
- **Reviewers:** @ARCHITECT, @ARBITRER
- **Success:** 50-80% smaller profiles + zero false negatives
- **Daily Check-in:** Report completed tasks, blockers, progress

### If You're a Support Agent
**Your Role:** Help your primary lead
- Allocation: 20-30% of your time
- **Stream A Support:** @VELOCITY, @PULSE
- **Stream B Support:** @APEX, @CIPHER
- **Stream C Support:** @NEURAL, @PULSE
- **Stream D Support:** @FORTRESS, @APEX

### If You're a Reviewer
**Your Role:** Code review & architecture validation
- Allocation: 10% per stream (varies by reviewer)
- **Key:** All PRs need 1+ approval before merge
- **SLA:** 1-2 hour turnaround on reviews
- **Focus:** Architecture, performance, tests, security, docs

---

## YOUR FIRST WEEK (Feb 24-28)

### Monday (Feb 24) - Kickoff
**9:30 AM:** Attend your stream's first standup
**10:00 AM - 5:00 PM:** Start implementation
**Activities:**
- [ ] Pull latest `develop` branch
- [ ] Create feature branch (feat/psi, feat/erofs, etc.)
- [ ] Set up local dev environment
- [ ] Begin core implementation

**Daily Standup (9:30 AM):**
```
✅ Completed Yesterday: (First day, n/a)
📅 Today's Plan:
  - Task 1: [Your task]
  - Task 2: [Your task]
  - Target: 50-75 LOC + 2-3 tests
🚨 Blockers: None (yet)
```

### Tuesday (Feb 25) - Development
**Activities:**
- [ ] Write core implementation (100-150 LOC)
- [ ] Add 5-8 unit tests
- [ ] Run `cargo test` locally (verify passing)
- [ ] Commit progress

**Daily Standup (9:30 AM):**
```
✅ Completed Yesterday:
  - Wrote PSI reader (150 LOC)
  - Added 5 unit tests
📅 Today's Plan:
  - Integrate with daemon
  - Add 3 more tests
  - Target: 80 LOC + 3 tests
🚨 Blockers: None
```

### Wednesday (Feb 26) - Midweek Checkpoint
**3:00 PM:** Checkpoint meeting (per stream)
**Activities:**
- [ ] Have 40-50% code written
- [ ] Have 50%+ tests passing
- [ ] Update your progress report
- [ ] Identify any blockers (resolve or report)

**Checkpoint Report:**
```
Stream: [Your stream]
Code: 50% complete ([X]/[Y] LOC)
Tests: 50% passing ([A]/[B] tests)
Blockers: [None / describe]
Status: [On track / At risk]
Confidence: [X]%
```

### Thursday (Feb 27) - Push to Merge
**Activities:**
- [ ] Finish remaining code (~40% left)
- [ ] Complete all tests
- [ ] Fix clippy warnings, format code
- [ ] Prepare for code review

**Daily Quick Sync (9:45 AM):**
```
Any blockers? Any merge conflicts anticipated?
Status: On track for Friday completion?
```

### Friday (Feb 28) - Sprint Complete
**1:00 PM:** Checkpoint meeting (final decision)
**Activities:**
- [ ] Have >95% code written
- [ ] Have 100% tests passing
- [ ] All quality gates met
- [ ] Ready for code review

**Checkpoint Report (Final):**
```
Stream: [Your stream]
Code: 95%+ complete ([X]/400 LOC for A, etc.)
Tests: 100% passing ([A]/[B] tests)
Performance: [Metric achieved / on track]
Blockers: None
Status: READY TO MERGE
Confidence: 95%+
```

**Post-Checkpoint:** Create PR with all code, wait for code review

---

## DAILY STANDUP (15 min, 9:30 AM)

### How to Standup
1. **Copy template** from DAILY_STANDUP_TEMPLATE.md
2. **Fill out sections** (5 min to prepare)
3. **Post to #development** as Slack thread
4. **Read others' standups** (2 min)
5. **Respond to questions** (immediately)

### What to Include
```
✅ COMPLETED YESTERDAY
├─ Task 1: [Description + result]
├─ Task 2: [Description + result]
└─ Tests: [X passing, 0 failing]

📊 PROGRESS
├─ Overall: X% complete
├─ Tests: Y/Z passing
├─ Code: A LOC written, B LOC remaining
└─ Status: [On track / At risk]

📅 TODAY'S PLAN
├─ Task 1: [Description + target]
├─ Task 2: [Description + target]
└─ Daily goal: 60+ LOC or 2+ tests

🚨 BLOCKERS
├─ [ ] None (clear to proceed)
└─ [If yes] [Description + severity]
```

### What Happens If You Mention a Blocker
1. **Post in standup** with details
2. **Tag blocking agent** (if someone else owns it)
3. **Tag @APEX** if critical/blocking all work
4. **Expect resolution** within 1 hour (or workaround)
5. **Follow up** in next day's standup

---

## CODE REVIEW PROCESS

### When Your Code is Ready
1. **Verify locally:**
   ```bash
   cargo build -p <crate>
   cargo test -p <crate>
   cargo clippy -p <crate> -- -D warnings
   cargo fmt --all -- --check
   ```

2. **Push branch:**
   ```bash
   git push origin feat/<your-stream>
   ```

3. **Create GitHub PR:**
   - **Title:** `[PHASE-E] Feature Name - Stream X`
   - **Description:** Use template from PHASE_E_MERGE_PROTOCOL.md
   - **Assign reviewers:** Your designated reviewers
   - **Link issue:** If applicable

4. **Wait for checks:** CI/CD runs automatically (5-10 min)
   - ✅ All green? Proceed
   - ❌ Any red? Fix locally, push again

5. **Review feedback:**
   - Reviewer may request changes or approve
   - Fix issues, commit, push
   - Tag reviewer for re-review if significant changes

6. **Approval:**
   - When reviewer approves: Ready to merge
   - Contact review lead if stuck >2 hours

### Merge Process (You or Review Lead)
```bash
git switch develop
git pull origin develop
git merge --squash feat/<your-stream>
git commit -m "[PHASE-E] Feature Name - Stream X"
git push origin develop
git push origin --delete feat/<your-stream>
```

---

## YOUR WEEKLY GOALS

### Week 2 (Development)
- [ ] Stream code: >95% complete (350+, 580+, 400+, 250+ LOC)
- [ ] All tests: 100% passing
- [ ] Quality gates: Zero warnings, format correct
- [ ] Benchmarks: On target for performance
- [ ] Documentation: 90%+ complete
- [ ] Status: Ready to merge by Fri EOD

### Week 3 (Integration)
- [ ] Code merged to develop
- [ ] Cross-stream integration tests passing
- [ ] No regressions detected
- [ ] Performance validated
- [ ] Ready for staging environment

### Week 4 (Validation)
- [ ] Staging tests complete
- [ ] Performance benchmarks confirmed
- [ ] Edge cases handled
- [ ] Documentation finalized

### Week 5 (Release)
- [ ] Production-ready
- [ ] All validation complete
- [ ] Phase E complete ✓

---

## FEATURE BRANCH WORKFLOW

### Create Your Branch
```bash
git fetch origin
git switch develop
git pull origin develop
git switch -c feat/<your-stream> origin/develop
```

### Work on Your Branch
```bash
# Make changes
git add <files>
git commit -m "Description of changes"

# Test locally
cargo test -p <crate>

# Push to remote
git push origin feat/<your-stream>
```

### Keep Branch Updated
```bash
# If develop gets new commits:
git fetch origin
git rebase origin/develop
# OR
git merge origin/develop

# Resolve conflicts if any
# Then push
git push origin feat/<your-stream>
```

### Squash & Merge (Final)
```bash
git switch develop
git pull origin develop
git merge --squash feat/<your-stream>
git commit -m "[PHASE-E] Your Feature - Stream X"
git push origin develop
git branch -d feat/<your-stream>
git push origin --delete feat/<your-stream>
```

---

## COMMON ISSUES & SOLUTIONS

### "My code won't compile"
1. Check error message
2. Fix the issue
3. `cargo build -p <crate>` to verify
4. Commit and push
5. Try again

### "Tests are failing"
1. Run tests locally: `cargo test -p <crate>`
2. Review test failures
3. Fix code (not tests, unless test is wrong)
4. `cargo test -p <crate>` to verify all pass
5. Commit and push

### "Clippy has warnings"
1. Run clippy: `cargo clippy -p <crate> -- -D warnings`
2. Review warnings
3. Fix warnings (don't suppress unless justified)
4. Run clippy again
5. Commit and push

### "I'm blocked and can't proceed"
1. **Post in standup:** Describe blocker, severity, workaround attempt
2. **Tag @APEX** if critical
3. **Wait for response:** Usually within 1 hour
4. **Implement workaround** or **pivot to different task**
5. **Keep making progress** where possible

### "Code review is taking too long"
1. **Wait 2 hours** (normal for thoughtful review)
2. **If >2 hours:** Ping reviewer in PR: "@Reviewer - still waiting"
3. **If >4 hours:** Escalate to @APEX
4. **Critical path?** Talk to @APEX immediately

### "I'm ahead of schedule"
1. **Celebrate!** You're doing great
2. **Options:**
   - Start on next feature (if any)
   - Help other streams (pair programming)
   - Add more test coverage
   - Performance optimization
   - Documentation improvements

---

## QUICK REFERENCE: YOUR DOCUMENTS

**Read These (in order):**
1. **PHASE_E_ARCHITECTURE.md** - What are we building?
2. **PHASE_E_ASSIGNMENTS.md** - Who is doing what?
3. **DAILY_STANDUP_TEMPLATE.md** - How do we sync daily?
4. **PHASE_E_MERGE_PROTOCOL.md** - How do we merge code?
5. **PHASE_E_CHECKPOINTS.md** - When do we review progress?
6. **BENCHMARK_PLAN.md** - How do we measure success?

**Quick Ref:**
- Git workflow → PHASE_E_MERGE_PROTOCOL.md
- Standup format → DAILY_STANDUP_TEMPLATE.md
- Your team → PHASE_E_ASSIGNMENTS.md
- Performance targets → BENCHMARK_PLAN.md
- Schedule → PHASE_E_CHECKPOINTS.md

---

## CONTACT & ESCALATION

### Your Team Lead
- Stream A: @APEX
- Stream B: @VELOCITY
- Stream C: @QUANTUM
- Stream D: @CIPHER

### Reviewers
- Architecture: @ARCHITECT
- Security: @CIPHER
- Performance: @VELOCITY
- Quality: @ARBITRER
- Documentation: @SCRIBE

### Phase Lead (Critical Issues)
- **@APEX** for all critical blockers

### Slack Channels
- **#development** - Daily standups
- **#phase-e-sync** - Weekly sync
- **#phase-e-critical** - Critical blockers only

---

## THE 4-WEEK JOURNEY

```
Week 2: BUILD IT
├─ 9:30 AM standups (daily)
├─ 50-100 LOC per day
├─ Wed checkpoint (midweek check)
└─ Fri checkpoint (merge decision)

Week 3: INTEGRATE IT
├─ Merge to develop
├─ Cross-stream testing
└─ Performance validation

Week 4: OPTIMIZE IT
├─ Staging environment
├─ Final benchmarks
└─ Edge case handling

Week 5: SHIP IT
├─ Production validation
└─ Phase E complete! 🎉
```

---

## SUCCESS LOOKS LIKE

**By Friday Week 2:**
- ✓ Your code >95% written
- ✓ Your tests 100% passing
- ✓ All quality gates passed
- ✓ Ready to merge
- ✓ Performance target on track

**By Wednesday Week 3:**
- ✓ Code merged to develop
- ✓ Integration working
- ✓ No regressions
- ✓ Ready for staging

**By Friday Week 5:**
- ✓ All 4 streams complete
- ✓ All targets achieved
- ✓ Production-ready
- ✓ Phase E SUCCESS! 🚀

---

## FINAL REMINDERS

1. **Show up to standups** - Daily 9:30 AM is critical
2. **Keep code quality high** - No shortcuts, zero warnings
3. **Tests are first-class** - Same importance as code
4. **Ask for help early** - Better to escalate than struggle
5. **Documentation matters** - Future you will thank you
6. **Performance targets are real** - Measure, optimize, validate
7. **Celebrate milestones** - Fri Week 2 checkpoint is a big deal

---

## YOU'VE GOT THIS!

Phase E is a sprint, not a marathon. Clear processes, good communication, and daily sync means you'll stay unblocked and on track.

**Let's make Phase E successful together! 🚀**

Questions? Ask in #development or DM your team lead.

Good luck!
