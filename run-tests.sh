#!/bin/bash
set -euo pipefail

# Test runner that posts GitHub issues on failures
# Usage: ./run-tests.sh [simple|full]
#   simple - Run simple_test (default)
#   full   - Run full iscsi-test-suite

REPO="lawless-m/iscsi-crate"

# Support both simple_test and full test suite
TEST_MODE="${1:-simple}"

if [ "$TEST_MODE" = "full" ]; then
    TEST_CMD="./iscsi-test-suite/iscsi-test-suite"
    TEST_ARGS="./test-config.toml"
    TEST_NAME="iscsi-test-suite"
else
    TEST_CMD="./simple_test"
    TEST_ARGS="iscsi://127.0.0.1:3261/iqn.2025-12.local:storage.memory-disk/0"
    TEST_NAME="simple_test"
fi

ISSUE_LABEL="test-failure"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================="
echo "iSCSI Test Runner"
echo "========================================="
echo

# Capture environment info
COMMIT_HASH=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
COMMIT_MSG=$(git log -1 --oneline 2>/dev/null || echo "unknown")
BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
OS_INFO=$(uname -a)
DATE=$(date -u +"%Y-%m-%d %H:%M:%S UTC")

echo "Environment:"
echo "  Commit: $COMMIT_HASH ($COMMIT_MSG)"
echo "  Branch: $BRANCH"
echo "  OS: $OS_INFO"
echo "  Date: $DATE"
echo

# Build test binary if needed
if [ "$TEST_MODE" = "full" ]; then
    echo "Building iscsi-test-suite..."
    if [ ! -d "./iscsi-test-suite" ]; then
        echo -e "${RED}ERROR: iscsi-test-suite directory not found${NC}"
        exit 2
    fi
    (cd iscsi-test-suite && make clean && make) || {
        echo -e "${RED}ERROR: Failed to build iscsi-test-suite${NC}"
        exit 2
    }
    echo "Build successful"
    echo
else
    # Build simple_test if needed
    if [ ! -f "$TEST_CMD" ]; then
        echo "Building simple_test..."
        gcc -o simple_test simple_test.c -liscsi || {
            echo -e "${RED}ERROR: Failed to build simple_test${NC}"
            exit 2
        }
        echo "Build successful"
        echo
    fi
fi

# Check if test binary exists
if [ ! -f "$TEST_CMD" ]; then
    echo -e "${RED}ERROR: Test binary not found: $TEST_CMD${NC}"
    exit 2
fi

# Start the target in background if not already running
if ! nc -z 127.0.0.1 3261 2>/dev/null; then
    echo "Starting iSCSI target..."
    cargo run --example simple_target > /tmp/simple_target.log 2>&1 &
    TARGET_PID=$!

    # Wait for target to be ready (max 30 seconds)
    for i in {1..300}; do
        if nc -z 127.0.0.1 3261 2>/dev/null; then
            echo "Target is ready"
            break
        fi
        sleep 0.1
    done

    # Check if target is now listening
    if ! nc -z 127.0.0.1 3261 2>/dev/null; then
        echo -e "${YELLOW}WARNING: iSCSI target failed to start or is not listening on port 3261${NC}"
        echo "Target log:"
        tail -50 /tmp/simple_target.log
        kill $TARGET_PID 2>/dev/null || true
        exit 1
    fi
else
    echo "Target is already running on port 3261"
    TARGET_PID=""
fi
echo

# Run the test and capture output
echo "Running: $TEST_CMD $TEST_ARGS"
echo "========================================="
echo

OUTPUT_FILE=$(mktemp)
EXIT_CODE=0
TIMEOUT_SECONDS=10

# Run test with timeout and capture output
timeout $TIMEOUT_SECONDS $TEST_CMD $TEST_ARGS 2>&1 | tee "$OUTPUT_FILE" || EXIT_CODE=$?

# Check if timed out
if [ $EXIT_CODE -eq 124 ]; then
    echo -e "\n${RED}TEST TIMED OUT after ${TIMEOUT_SECONDS}s${NC}" | tee -a "$OUTPUT_FILE"
fi

echo
echo "========================================="

# Check result
if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}✓ Tests PASSED${NC}"
    rm "$OUTPUT_FILE"
    exit 0
else
    echo -e "${RED}✗ Tests FAILED (exit code: $EXIT_CODE)${NC}"
    echo

    # Check if gh CLI is available
    if ! command -v gh &> /dev/null; then
        echo -e "${YELLOW}WARNING: gh CLI not found. Cannot post issue automatically.${NC}"
        echo "Install with: sudo apt-get install gh"
        echo
        echo "Test output saved to: $OUTPUT_FILE"
        exit $EXIT_CODE
    fi

    # Create issue title
    if [ $EXIT_CODE -eq 124 ]; then
        ISSUE_TITLE="Test Failure: $TEST_NAME - TIMEOUT (${TIMEOUT_SECONDS}s)"
    else
        ISSUE_TITLE="Test Failure: $TEST_NAME - Exit Code $EXIT_CODE"
    fi

    # Strip ANSI color codes from output for GitHub
    CLEAN_OUTPUT=$(cat "$OUTPUT_FILE" | sed 's/\x1b\[[0-9;]*m//g')

    # Create issue body
    if [ $EXIT_CODE -eq 124 ]; then
        EXIT_CODE_INFO="$EXIT_CODE (TIMEOUT - test exceeded ${TIMEOUT_SECONDS}s limit)"
    else
        EXIT_CODE_INFO="$EXIT_CODE"
    fi

    ISSUE_BODY=$(cat <<EOF
## Test Failure Report

**Test Command:** \`$TEST_CMD $TEST_ARGS\`
**Exit Code:** $EXIT_CODE_INFO
**Date:** $DATE

### Environment
- **Commit:** \`$COMMIT_HASH\`
- **Branch:** \`$BRANCH\`
- **Commit Message:** $COMMIT_MSG
- **OS:** $OS_INFO

### Test Output

\`\`\`
$CLEAN_OUTPUT
\`\`\`

### Files Involved
- Test Program: \`simple_test.c\`
- Test Binary: \`simple_test\`
- Target Example: \`examples/simple_target.rs\`
- Configuration: Target at 127.0.0.1:3261

### Diagnostic Information
- **Target Status:** $(ss -tlnp 2>/dev/null | grep :3261 > /dev/null && echo "Listening on port 3261" || echo "NOT listening on port 3261")
- **Network Test:** $(timeout 2 bash -c 'echo "" | nc -v 127.0.0.1 3261' 2>&1 | head -1 || echo "Cannot connect")

### Expected Behavior
All 5 tests should pass:
1. Create iSCSI context
2. Connect to target
3. INQUIRY command
4. READ CAPACITY command
5. READ/WRITE operations with data integrity check

### Actual Behavior
Test failed with exit code $EXIT_CODE. See output above.

---
🤖 Automatically generated by run-tests.sh
EOF
)

    # Check if an issue with this title already exists
    echo "Checking for existing issues..."
    EXISTING_ISSUE=$(gh issue list --repo lawless-m/iscsi-crate --state open --search "$ISSUE_TITLE" --json number --jq '.[0].number' 2>/dev/null || true)

    if [ -n "$EXISTING_ISSUE" ]; then
        echo -e "${YELLOW}Issue already exists: #$EXISTING_ISSUE${NC}"
        echo "Skipping duplicate issue creation"
        ISSUE_URL="https://github.com/lawless-m/iscsi-crate/issues/$EXISTING_ISSUE"
    else
        # Create the GitHub issue
        echo "Creating new GitHub issue..."
        ISSUE_URL=$(gh issue create \
            --repo lawless-m/iscsi-crate \
            --title "$ISSUE_TITLE" \
            --body "$ISSUE_BODY" \
            2>&1)
    fi

    if [[ "$ISSUE_URL" =~ ^https:// ]]; then
        echo -e "${GREEN}Issue created: $ISSUE_URL${NC}"
    else
        echo -e "${RED}Failed to create issue: $ISSUE_URL${NC}"
        echo "Issue body saved to: ${OUTPUT_FILE}.issue"
        echo "$ISSUE_BODY" > "${OUTPUT_FILE}.issue"
    fi

    echo
    echo "Test output saved to: $OUTPUT_FILE"

    # Clean up target if we started it
    if [ -n "$TARGET_PID" ]; then
        kill $TARGET_PID 2>/dev/null || true
    fi

    exit $EXIT_CODE
fi

# Clean up target if we started it
if [ -n "$TARGET_PID" ]; then
    kill $TARGET_PID 2>/dev/null || true
fi
