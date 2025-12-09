# Automated Test-Fix Pattern with Claude Code

## Overview

This document describes a generalized pattern for continuous, automated testing and fixing using Claude Code. The system creates a feedback loop: **Test → Fail → Issue → Fix → Test** that runs until all tests pass.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Test Runner (run-tests.sh)                │
│  - Runs test suite with timeout                              │
│  - Captures output and environment context                   │
│  - On failure: Creates GitHub issue automatically            │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
            ┌─────────────────┐
            │  GitHub Issues   │ ◄──── Visible to humans
            │  (with context)  │       and Claude Code
            └────────┬─────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│              Issue Fixer (fix-issue.sh)                      │
│  - Fetches issue from GitHub                                 │
│  - Invokes Claude Code with detailed prompt                  │
│  - Claude investigates, fixes, tests, closes issue           │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│            Auto-Fix Loop (auto-fix-loop.sh)                  │
│  - Orchestrates the cycle                                    │
│  - Runs tests → Finds issues → Fixes → Repeat               │
│  - Stops when tests pass or max iterations reached           │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Test Runner (`run-tests.sh`)

**Purpose:** Execute tests and create GitHub issues on failure

**Key Features:**
- Automatic timeout detection
- Environment context capture (commit, OS, date)
- Diagnostic information (service status, connectivity)
- ANSI color code stripping for clean GitHub issues
- Duplicate issue prevention (optional)

**Template Structure:**
```bash
#!/bin/bash
# 1. Capture environment info
COMMIT=$(git rev-parse HEAD)
DATE=$(date -u)
OS=$(uname -a)

# 2. Run tests with timeout
timeout ${TIMEOUT} ${TEST_COMMAND} > output.txt || EXIT_CODE=$?

# 3. On failure: Create detailed GitHub issue
if [ $EXIT_CODE -ne 0 ]; then
    gh issue create \
        --title "Test Failure: ${TEST_NAME}" \
        --body "$(generate_issue_body)"
fi
```

**What to Include in Issues:**
- **Test command** that failed
- **Exit code** (with timeout annotation)
- **Full test output** (cleaned)
- **Environment**: commit hash, branch, OS, date
- **Diagnostic info**: service status, network checks
- **Files involved**: test code, implementation code
- **Expected vs actual behavior**

### 2. Issue Fixer (`fix-issue.sh`)

**Purpose:** Fetch GitHub issues and invoke Claude Code to fix them

**Key Features:**
- GitHub CLI integration
- Model selection (haiku/sonnet/opus)
- Permission modes (interactive, auto-edit, bypass)
- Structured prompt with clear instructions

**Template Structure:**
```bash
#!/bin/bash
# 1. Fetch issue from GitHub
ISSUE_BODY=$(gh issue view $ISSUE_NUM --json body --jq '.body')

# 2. Create structured prompt for Claude
PROMPT="
GitHub Issue #$ISSUE_NUM: $ISSUE_TITLE

$ISSUE_BODY

IMPORTANT: [Project-specific guidance]
- What code to fix vs what code is correct
- Where to look for the bug
- How to test the fix

Steps:
1. Read test output
2. Examine source code
3. Identify root cause
4. Implement fix
5. Test with: ./run-tests.sh
6. Close issue if fixed
"

# 3. Invoke Claude Code
claude --model $MODEL \
       --permission-mode $MODE \
       "$PROMPT"
```

**Permission Modes:**
- **Default (interactive)**: User approves each action
- **`acceptEdits`**: Auto-accept file edits, prompt for commands
- **`dangerouslySkip`**: Bypass all permissions (sandboxed only!)

### 3. Auto-Fix Loop (`auto-fix-loop.sh`)

**Purpose:** Orchestrate continuous test-fix cycles

**Template Structure:**
```bash
#!/bin/bash
MAX_ITERATIONS=10
iteration=0

while [ $iteration -lt $MAX_ITERATIONS ]; do
    # Run tests
    if ./run-tests.sh; then
        echo "SUCCESS! All tests passed!"
        exit 0
    fi

    # Find open issues
    ISSUE=$(gh issue list --state open --search "Test Failure" | head -1)

    if [ -z "$ISSUE" ]; then
        sleep 2
        continue
    fi

    # Fix the issue
    ./fix-issue.sh --no-prompts $ISSUE

    iteration=$((iteration + 1))
done

echo "Max iterations reached"
exit 1
```

## Sandboxing Strategy

### Why Sandbox?

Using `--dangerously-skip-permissions` requires a sandboxed environment because:
- Claude Code can execute arbitrary commands without approval
- File edits happen automatically
- Network operations proceed unchecked

### Sandboxing Options

#### Option 1: Local QEMU VM (Small Projects)
```bash
# Start VM with port forwarding
qemu-system-x86_64 \
    -m 2048 \
    -hda debian-vm.qcow2 \
    -netdev user,id=net0,hostfwd=tcp::2224-:22 \
    -enable-kvm -cpu host \
    -display none -daemonize

# Copy project to VM
rsync -avz ./ user@localhost:~/project/

# Run auto-fix loop in VM
ssh -p 2224 user@localhost 'cd ~/project && ./auto-fix-loop.sh'
```

**Pros:**
- Full isolation
- Easy to reset (snapshot/restore)
- Works offline

**Cons:**
- Requires local resources
- VM disk space limits

#### Option 2: Remote Server (Large Projects)
```bash
# Copy to remote with lots of space
rsync -avz ./ user@remote:/path/to/testing/

# Start VM on remote
ssh user@remote 'qemu-system-x86_64 ... -daemonize'

# Forward VM SSH port to localhost
ssh -L 2230:localhost:3260 user@remote

# Access remote VM as if local
ssh -p 2230 user@localhost 'cd ~/project && ./auto-fix-loop.sh'
```

**Pros:**
- Unlimited disk space
- Doesn't consume local resources
- Can run overnight

**Cons:**
- Requires network access
- More complex setup

#### Option 3: Docker Container (Lightweight)
```dockerfile
FROM rust:latest
RUN apt-get update && apt-get install -y gh libiscsi-dev
COPY . /project
WORKDIR /project
CMD ["./auto-fix-loop.sh"]
```

```bash
docker build -t auto-tester .
docker run --rm auto-tester
```

**Pros:**
- Lightweight
- Fast startup
- Easy to version control

**Cons:**
- Less isolation than VM
- May need privileged mode for some operations

## Adapting to Other Projects

### For Different Languages

**Python Project:**
```bash
# run-tests.sh
pytest tests/ --junit-xml=results.xml || EXIT_CODE=$?
```

**Node.js Project:**
```bash
# run-tests.sh
npm test 2>&1 | tee test-output.txt || EXIT_CODE=$?
```

**Go Project:**
```bash
# run-tests.sh
go test ./... -v -timeout 30s || EXIT_CODE=$?
```

### Key Customization Points

1. **Test Command** (`run-tests.sh`)
   - Language-specific test runner
   - Timeout appropriate for your tests
   - Output format parsing

2. **Diagnostic Checks** (`run-tests.sh`)
   - Service availability (databases, APIs, etc.)
   - Network connectivity
   - Configuration validation

3. **Fix Guidance** (`fix-issue.sh` prompt)
   - What code to modify vs what code is correct
   - Common pitfalls in your project
   - Testing methodology

4. **Dependencies** (VM setup)
   - Language runtime (Rust, Python, Node, etc.)
   - System libraries
   - Testing frameworks

## Best Practices

### 1. Issue Quality

**Good Issue:**
```markdown
## Test Failure: API Integration Test

**Exit Code:** 1 (Test failure)
**Date:** 2025-12-08 23:00:00 UTC

### Test Output
```
FAIL: test_user_authentication (0.123s)
  Expected status 200, got 401
  Response: {"error": "Invalid token"}
```

### Diagnostic Information
- API Server: Running on port 8000
- Database: Connected (postgres://localhost:5432)
- Auth Service: ✓ Reachable

### Files Involved
- Test: tests/test_auth.py
- Implementation: src/auth/token_validator.py
```

**Bad Issue:**
```markdown
Tests failed

Output: Error
```

### 2. Test Timeouts

Choose timeouts based on test suite characteristics:
- **Unit tests**: 10-30 seconds
- **Integration tests**: 1-5 minutes
- **E2E tests**: 5-15 minutes

Too short = false positives. Too long = wasted time on hangs.

### 3. Model Selection

- **Haiku**: Simple bugs, syntax errors, quick fixes
- **Sonnet**: Most issues, good balance of speed/capability
- **Opus**: Complex architectural problems, multi-file refactors

Start with haiku, escalate to sonnet if needed.

### 4. Iteration Limits

Set `MAX_ITERATIONS` based on:
- Test suite size (more tests = more potential issues = more iterations)
- Time budget (overnight = 50+ iterations, quick check = 5-10)
- Cost concerns (each iteration = API calls)

Typical values: 10-20 iterations

### 5. Commit Strategy

**Option A: One commit per fix**
```bash
# In fix-issue.sh, after tests pass:
git add -A
git commit -m "Fix issue #${ISSUE_NUM}"
```

**Option B: Batch commits**
```bash
# After auto-fix-loop completes successfully:
git add -A
git commit -m "Automated fixes for issues #1, #2, #3"
```

**Option C: No auto-commits** (recommended for first runs)
- Review all changes manually
- Commit selectively
- Learn what Claude is doing

## Workflow Examples

### Initial Development

```bash
# 1. Write tests for new feature (they fail)
./run-tests.sh full
# → Creates issues #10, #11, #12

# 2. Run auto-fix loop
./auto-fix-loop.sh 20 sonnet full
# → Fixes issues iteratively

# 3. Review changes
git diff

# 4. Commit if satisfied
git commit -am "Implement feature X with automated fixes"
```

### Regression Testing

```bash
# After making changes:
./run-tests.sh full

# If issues created:
./fix-issue.sh --auto-edit 15

# Verify fix:
./run-tests.sh full
```

### Overnight Fuzzing

```bash
# In VM, run long cycle:
nohup ./auto-fix-loop.sh 100 haiku full > overnight.log 2>&1 &

# Next morning:
tail overnight.log
gh issue list --state closed
git log
```

## Metrics and Monitoring

### Track Success Rate

```bash
# Count issues created
CREATED=$(gh issue list --search "Test Failure" --state all --json number --jq '. | length')

# Count issues fixed
FIXED=$(gh issue list --search "Test Failure" --state closed --json number --jq '. | length')

# Success rate
echo "Fixed: $FIXED / $CREATED = $((FIXED * 100 / CREATED))%"
```

### Cost Estimation

Approximate API costs per iteration:
- **Haiku**: $0.01 - $0.05 per issue fix
- **Sonnet**: $0.10 - $0.50 per issue fix
- **Opus**: $1.00 - $5.00 per issue fix

Example: 10 iterations with sonnet ≈ $1-$5

### Time Tracking

```bash
# Add to auto-fix-loop.sh
START=$(date +%s)
# ... run tests and fixes ...
END=$(date +%s)
DURATION=$((END - START))
echo "Total time: $((DURATION / 60)) minutes"
```

## Troubleshooting

### Issue: Claude Fixes Tests Instead of Code

**Problem:** Claude modifies test files to make tests pass instead of fixing bugs

**Solution:**
```bash
# In fix-issue.sh prompt, add:
IMPORTANT: The tests in tests/ are CORRECT and must NOT be modified.
Fix the implementation code in src/, not the test code.
```

### Issue: Infinite Loop (Same Issue Reopens)

**Problem:** Fix doesn't actually solve the problem, issue keeps coming back

**Solution:**
- Check if the fix is being tested correctly
- Add more context to the issue (logs, error messages)
- Escalate to stronger model (haiku → sonnet → opus)
- Add project-specific debugging hints

### Issue: Tests Pass Locally, Fail in VM

**Problem:** Different environment in VM

**Solution:**
- Ensure VM has all dependencies
- Check environment variables
- Verify file permissions
- Compare tool versions (compiler, runtime, etc.)

### Issue: VM Runs Out of Disk Space

**Problem:** Rust/Node/etc. fills up disk during compilation

**Solution:**
- Use larger disk image (extend qcow2)
- Use remote server with more space
- Clean build artifacts between iterations
- Use shallow git clones

## Security Considerations

### Safe to Auto-Fix

- ✅ Unit tests
- ✅ Integration tests (internal services)
- ✅ Code linting/formatting issues
- ✅ Type errors
- ✅ Documentation tests

### Review Before Accepting

- ⚠️ Security-sensitive code (auth, crypto, permissions)
- ⚠️ Database migrations
- ⚠️ API contract changes
- ⚠️ Dependency version updates
- ⚠️ Configuration file changes

### Never Auto-Fix

- ❌ Production deployments
- ❌ Credential/secret management
- ❌ Access control policies
- ❌ Billing/payment code

## Future Enhancements

### Parallel Testing

Run multiple test suites in parallel, create issues for each:

```bash
./run-tests.sh unit &
./run-tests.sh integration &
./run-tests.sh e2e &
wait

# Then fix all issues in parallel
for issue in $(gh issue list --json number); do
    ./fix-issue.sh --no-prompts $issue &
done
wait
```

### CI/CD Integration

```yaml
# .github/workflows/auto-fix.yml
name: Auto-Fix Tests
on: [push]
jobs:
  test-and-fix:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - run: ./run-tests.sh
      - if: failure()
        run: ./fix-issue.sh --auto-edit $(gh issue list --search "Test Failure" --json number --jq '.[0].number')
```

### Learning from History

Track which types of bugs Claude fixes best:

```bash
# Tag issues by category
gh issue create --label "type:timeout" ...
gh issue create --label "type:logic-error" ...

# Later, analyze success rates
gh issue list --label "type:timeout" --state closed
```

## Conclusion

This pattern enables:
- **Faster development**: Automated bug fixing
- **Better testing**: Issues are documented, not forgotten
- **Continuous improvement**: Loop runs until green
- **Knowledge capture**: All fixes tracked in git + GitHub

**Key to success:** High-quality issues with context. The better the issue description, the better Claude can fix it.

---

**Last Updated:** 2025-12-08
**Project:** iSCSI Rust Target (reference implementation)
**Pattern Version:** 1.0
