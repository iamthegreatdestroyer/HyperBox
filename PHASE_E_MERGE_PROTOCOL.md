# Phase E - Code Review & Merge Protocol

## Quick Reference

```
Development Branch: develop
Feature Branches: feat/psi, feat/erofs, feat/otel-ebpf, feat/seccomp-gen
PR Naming: [PHASE-E] Feature Name - Stream X
Merge Strategy: Squash merge to develop
Review Turnaround: 1-2 hours max
```

---

## BRANCH STRATEGY

### Repository Branch Structure

```
main (production)
  ↑ (merge after validation)
develop (Phase E integration)
  ↑ (merge after full test suite)
├─ feat/psi (Stream A)
├─ feat/erofs (Stream B)
├─ feat/otel-ebpf (Stream C)
└─ feat/seccomp-gen (Stream D)
```

### Creating Feature Branches

**Before creating branch:**
```bash
# Ensure develop is latest
git fetch origin
git switch develop
git pull origin develop

# Verify no uncommitted changes
git status
```

**Create feature branch:**
```bash
git switch -c feat/psi origin/develop
git switch -c feat/erofs origin/develop
git switch -c feat/otel-ebpf origin/develop
git switch -c feat/seccomp-gen origin/develop
```

**Push to remote:**
```bash
git push -u origin feat/psi
git push -u origin feat/erofs
git push -u origin feat/otel-ebpf
git push -u origin feat/seccomp-gen
```

---

## AUTOMATED QUALITY GATES

### Gate 1: Must Pass (Blocking)

All of these MUST pass before code review can begin:

#### Compilation
```bash
# Stream A
cargo build -p hyperbox-core --all-features 2>&1 | grep -E "error|warning"

# Stream B
cargo build -p hyperbox-optimize --all-features 2>&1 | grep -E "error|warning"

# Stream C
cargo build -p hyperbox-daemon --all-features 2>&1 | grep -E "error|warning"

# Stream D
cargo build -p hyperbox-core --all-features 2>&1 | grep -E "error|warning"
```

**Acceptance Criteria:**
- ✅ Zero compilation errors
- ✅ Zero compilation warnings
- ✅ All dependencies resolve

#### Tests - Unit + Integration
```bash
# Run all tests for your crate
cargo test -p <crate-name> --all-features 2>&1 | tail -20

# Check for failures
cargo test -p <crate-name> --all-features -- --nocapture 2>&1 | grep -E "FAILED|test result"
```

**Acceptance Criteria:**
- ✅ 100% of unit tests passing
- ✅ 100% of integration tests passing
- ✅ No flaky tests (repeatable failures)
- ✅ No timeout issues

#### Clippy - Lint Checks
```bash
# Zero clippy warnings
cargo clippy -p <crate-name> --all-features -- -D warnings 2>&1
```

**Acceptance Criteria:**
- ✅ Zero clippy warnings
- ✅ All suggestions addressed
- ✅ No `#![allow(...)]` overrides without justification

#### Formatting
```bash
# Verify formatting (doesn't modify)
cargo fmt --all -- --check 2>&1

# If failed, auto-fix:
cargo fmt --all
```

**Acceptance Criteria:**
- ✅ All code follows rustfmt style
- ✅ No trailing whitespace
- ✅ Proper indentation (4 spaces)

#### Doc Tests
```bash
# Run documentation tests
cargo test --doc -p <crate-name> --all-features 2>&1 | tail -20
```

**Acceptance Criteria:**
- ✅ All doc examples compile
- ✅ All doc examples pass tests
- ✅ No broken documentation

### Gate 2: CI/CD Pipeline (Automated)

```
GitHub Actions checks (must all pass):
├─ cargo build --all-features
├─ cargo test --all-features
├─ cargo clippy --all-features -- -D warnings
├─ cargo fmt --all -- --check
├─ cargo test --doc --all-features
└─ cargo check --workspace --all-features
```

**PR cannot be merged without:**
- ✅ All status checks green
- ✅ No conflicts with develop branch
- ✅ At least 1 approval from designated reviewer

---

## CODE REVIEW PROCESS

### Step 1: Create Pull Request

**Title Format:**
```
[PHASE-E] Feature Name - Stream X
```

**Examples:**
```
[PHASE-E] PSI Memory Monitoring Integration - Stream A
[PHASE-E] EROFS + Fscache Implementation - Stream B
[PHASE-E] OpenTelemetry eBPF Tracing - Stream C
[PHASE-E] Seccomp Auto-generation Engine - Stream D
```

**PR Description Template:**

```markdown
## Description
Brief description of what this PR accomplishes.

## Related Issue
Closes #XXX (if applicable)

## Type of Change
- [ ] New feature
- [ ] Bug fix
- [ ] Performance improvement
- [ ] Documentation update
- [ ] Refactoring

## Acceptance Criteria
- [ ] Criterion 1 from PHASE_E_ARCHITECTURE.md
- [ ] Criterion 2
- [ ] Criterion 3

## Testing
- [ ] All unit tests pass (X/X)
- [ ] All integration tests pass (Y/Y)
- [ ] New tests added for this feature (Z tests)
- [ ] Manual testing completed

## Performance Impact
- [ ] No performance degradation
- [ ] Performance improvement: [metric] [improvement%]
- [ ] Benchmarks: [link to results]

## Security Implications
- [ ] No security implications
- [ ] Security review completed by @CIPHER
- [ ] Security fixes included: [list]

## Documentation
- [ ] Public API documented
- [ ] Integration guide provided
- [ ] Examples added
- [ ] README updated (if needed)

## Checklist
- [ ] Code follows style guidelines
- [ ] No clippy warnings
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] No breaking changes
- [ ] Backward compatible
```

### Step 2: Assign Reviewers

**Assign based on PHASE_E_ASSIGNMENTS.md:**

| Stream | Primary Reviewer | Secondary Reviewer |
|--------|------------------|-------------------|
| A (PSI) | @ARCHITECT | @CIPHER |
| B (EROFS) | @ARCHITECT | @NEXUS |
| C (eBPF) | @ARCHITECT | @SCRIBE |
| D (Seccomp) | @ARCHITECT | @ARBITRER |

**Assign in PR:**
```
Reviewers: @ARCHITECT, @[SECONDARY]
```

### Step 3: Automated Checks

**CI/CD Pipeline runs automatically:**
- Wait for all checks to show as passing (green ✓)
- Cannot proceed to review if any check fails (red ✗)
- Automated checks: 5-10 minutes typically

**If a check fails:**
1. Click "Details" on failing check
2. Review the failure details
3. Fix the issue locally
4. Commit and push
5. CI/CD re-runs automatically
6. Repeat until all checks pass

### Step 4: Code Review

**Reviewer responsibilities:**

#### Architecture Soundness
- [ ] Design aligns with PHASE_E_ARCHITECTURE.md
- [ ] Integration points correctly implemented
- [ ] No unplanned dependencies introduced
- [ ] Error handling appropriate
- [ ] Logging and debugging capabilities adequate

#### Performance
- [ ] No obvious performance regressions
- [ ] Appropriate data structures used
- [ ] No unnecessary allocations
- [ ] Algorithms efficient for use case
- [ ] Benchmarks show expected improvement

#### Test Coverage
- [ ] New code has tests (target: >80% coverage)
- [ ] Edge cases covered
- [ ] Error cases tested
- [ ] Integration tests comprehensive
- [ ] No flaky tests

#### Security Review
- [ ] No security vulnerabilities introduced
- [ ] Input validation appropriate
- [ ] Unsafe code justified and documented
- [ ] No privilege escalation vectors
- [ ] Safe error messages (no info disclosure)

#### Documentation
- [ ] Public APIs documented
- [ ] Complex logic explained
- [ ] Examples provided where helpful
- [ ] Integration guide complete
- [ ] Troubleshooting guide (if needed)

#### Code Quality
- [ ] Follows codebase patterns
- [ ] Variable names clear and meaningful
- [ ] Comments explain "why", not "what"
- [ ] No dead code
- [ ] No debug statements left in

### Step 5: Review Feedback

**Reviewer provides feedback:**
- Use PR comments for general feedback
- Use line-by-line review for specific code issues
- Request changes or approve

**Options:**
- **Comment:** Feedback, but doesn't block merge
- **Request Changes:** Issues that must be fixed
- **Approve:** Ready to merge

**Author responds to feedback:**
- Address all requested changes
- Commit and push fixes
- Reply to review comments with resolution
- Request re-review if significant changes made

### Step 6: Approval & Merge

**Merge Requirements:**
- ✅ All automated checks passing
- ✅ At least 1 approved review from designated reviewer
- ✅ No conflicting changes with develop
- ✅ All conversations resolved
- ✅ All feedback addressed

**Merge Process:**

1. **Verify status:** All green checks, 1+ approval
2. **Check conflicts:** "Merge" button shows one of:
   - "Merge pull request" (no conflicts)
   - "Resolve conflicts first" (has conflicts)
3. **Squash merge to develop:**
   ```bash
   git switch develop
   git pull origin develop
   git merge --squash feat/psi
   git commit -m "[PHASE-E] PSI Memory Monitoring - Stream A"
   git push origin develop
   ```
4. **Delete feature branch:**
   ```bash
   git push origin --delete feat/psi
   git branch -D feat/psi
   ```
5. **Verify merge:** Check develop branch, verify commit history

**Merge Commit Message Format:**
```
[PHASE-E] Feature Name - Stream X

- Brief description of changes
- Key implementation details
- Link to PR #XXX
```

---

## QUALITY GATE CHECKLIST

Before PR merge, verify ALL of these:

### Code Quality
- [ ] Zero compilation warnings: `cargo build -p <crate> -q 2>&1`
- [ ] Zero clippy warnings: `cargo clippy -p <crate> -- -D warnings`
- [ ] Formatting correct: `cargo fmt --all -- --check`
- [ ] All tests pass: `cargo test -p <crate> --all-features`
- [ ] Doc tests pass: `cargo test --doc -p <crate>`

### Testing
- [ ] Unit test count: >= expected (15, 20, 18, 12 by stream)
- [ ] Integration test count: >= expected
- [ ] Test coverage: >= 80%
- [ ] All tests deterministic (no flakes)
- [ ] New tests for new code

### Architecture
- [ ] Design aligns with PHASE_E_ARCHITECTURE.md
- [ ] Integration points correct
- [ ] No breaking changes to public APIs
- [ ] Backward compatible
- [ ] Dependencies reviewed

### Performance
- [ ] Benchmarks show target improvement
  - Stream A: 5-15% memory pressure reduction
  - Stream B: 30-50% faster images
  - Stream C: <2% CPU overhead
  - Stream D: 50-80% smaller profiles
- [ ] No performance regressions
- [ ] Resource usage acceptable

### Security
- [ ] Security implications reviewed
- [ ] No new vulnerabilities
- [ ] Input validation adequate
- [ ] Error messages safe (no info disclosure)
- [ ] Privilege boundaries respected

### Documentation
- [ ] All public APIs documented
- [ ] Examples provided
- [ ] Integration guide complete
- [ ] Troubleshooting guide (if needed)
- [ ] README updated (if needed)

---

## MERGE CONFLICT RESOLUTION

If your feature branch conflicts with develop:

1. **Update develop in branch:**
   ```bash
   git fetch origin
   git rebase origin/develop
   # OR
   git merge origin/develop
   ```

2. **Resolve conflicts:**
   - Open conflicted files
   - Resolve merge markers (`<<<<<<`, `======`, `>>>>>>`)
   - Keep both versions if needed
   - Test after resolving

3. **Verify resolution:**
   ```bash
   cargo build -p <crate>
   cargo test -p <crate>
   ```

4. **Commit merge:**
   ```bash
   git add .
   git commit -m "Resolve conflicts with develop"
   git push origin feat/psi
   ```

5. **Re-run tests:** CI/CD will re-run all checks

---

## MERGE TIMING

### During Week 2 (Development Phase)
- **Mon-Thu:** Feature branches active, no merges
- **Fri (EOD):** Code review begins, first merges possible
- **Best practice:** Wait for checkpoint approval before merge

### Checkpoint Merges (Coordinated)
- **Friday EOD Week 2:** After checkpoint approval
- **Merge order:** No particular order, all can merge simultaneously
- **All features expected:** To merge by Fri EOD Week 2

### After Merge
- **Full test suite:** Runs on develop after each merge
- **Verify:** `cargo check --workspace --all-features`
- **Monitor:** Watch for integration issues

---

## ESCALATION PATH FOR STUCK REVIEWS

If review takes longer than 2 hours:

1. **Ping reviewer in PR:** "@ARCHITECT - waiting for review"
2. **Escalate to @APEX** if critical path blocker
3. **Consider parallel reviewer** if original unavailable
4. **Document delay** in daily standup

---

## METRICS TRACKED

Each merged PR contributes to:
- **Velocity:** PRs merged per day/week
- **Quality:** Bug-fix PRs after merge (should be <5%)
- **Review efficiency:** Average review time (target: 1-2 hours)
- **Cycle time:** Time from branch creation to merge (target: 3-5 days)

---

## EXAMPLE WORKFLOW

### Create and develop feature:
```bash
git switch -c feat/psi origin/develop
# ... make changes ...
cargo test -p hyperbox-core
cargo clippy -p hyperbox-core -- -D warnings
cargo fmt --all
git commit -m "Add PSI memory monitoring"
git push -u origin feat/psi
```

### Create PR on GitHub:
```
Title: [PHASE-E] PSI Memory Monitoring Integration - Stream A
Description: [Use template above]
Reviewers: @ARCHITECT, @CIPHER
```

### Wait for checks:
```
CI checks running...
✓ Compilation
✓ Tests
✓ Clippy
✓ Formatting
✓ Doc tests
```

### Respond to review:
```
@ARCHITECT: "Question: should PSI reading fail-safe?"
Response: "Yes, will handle gracefully with fallback"
Commit: Fix based on feedback
Push: `git push origin feat/psi`
Re-review by @ARCHITECT
```

### Merge:
```bash
git switch develop
git pull origin develop
git merge --squash feat/psi
git commit -m "[PHASE-E] PSI Memory Monitoring - Stream A

Implements PSI metrics reading with graceful fallback."
git push origin develop
git push origin --delete feat/psi
```

### Verify merge:
```bash
cargo check --workspace --all-features
cargo test --workspace --all-features
```

Complete! Feature merged and integrated.
