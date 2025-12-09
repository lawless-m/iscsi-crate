#!/bin/bash
set -euo pipefail

# Test runner that posts GitHub issues on failures
# Usage: ./run-tests.sh [simple|full|discovery|commands|io]
#   simple    - Run simple_test (default)
#   full      - Run full iscsi-test-suite (all tests)
#   discovery - Run discovery test category only
#   commands  - Run SCSI command test category only
#   io        - Run I/O test category only

REPO="lawless-m/iscsi-crate"

# Support both simple_test and full test suite
TEST_MODE="${1:-simple}"

case "$TEST_MODE" in
    full)
        TEST_CMD="./iscsi-test-suite/iscsi-test-suite"
        TEST_ARGS="./test-config.toml"
        TEST_NAME="iscsi-test-suite (all)"
        ;;
    discovery)
        TEST_CMD="./iscsi-test-suite/iscsi-test-suite"
        TEST_ARGS="-c discovery ./test-config.toml"
        TEST_NAME="iscsi-test-suite (discovery)"
        ;;
    commands)
        TEST_CMD="./iscsi-test-suite/iscsi-test-suite"
        TEST_ARGS="-c commands ./test-config.toml"
        TEST_NAME="iscsi-test-suite (commands)"
        ;;
    io)
        TEST_CMD="./iscsi-test-suite/iscsi-test-suite"
        TEST_ARGS="-c io ./test-config.toml"
        TEST_NAME="iscsi-test-suite (io)"
        ;;
    simple)
        TEST_CMD="./simple_test"
        TEST_ARGS="iscsi://127.0.0.1:3261/iqn.2025-12.local:storage.memory-disk/0"
        TEST_NAME="simple_test"
        ;;
    *)
        echo "Error: Unknown test mode '$TEST_MODE'"
        echo "Usage: $0 [simple|full|discovery|commands|io]"
        exit 2
        ;;
esac

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
if [ "$TEST_MODE" = "simple" ]; then
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
else
    # Build iscsi-test-suite for all other modes (full, discovery, commands, io)
    echo "Building iscsi-test-suite..."
    if [ ! -d "./iscsi-test-suite" ]; then
        echo -e "${RED}ERROR: iscsi-test-suite directory not found${NC}"
        exit 2
    fi
    (cd iscsi-test-suite && chmod -R u+w obj 2>/dev/null || true && make clean && make) || {
        echo -e "${RED}ERROR: Failed to build iscsi-test-suite${NC}"
        exit 2
    }
    echo "Build successful"
    echo
fi

# Check if test binary exists
if [ ! -f "$TEST_CMD" ]; then
    echo -e "${RED}ERROR: Test binary not found: $TEST_CMD${NC}"
    exit 2
fi

# Kill any existing target to ensure fresh start with latest code
echo "Stopping any existing iSCSI target..."
pkill -f simple_target 2>/dev/null || true
# Give it a moment to release the port
sleep 0.5

# Start the target in background
echo "Starting iSCSI target with latest code..."
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
echo

# Run the test and capture output
echo "Running: $TEST_CMD $TEST_ARGS"
echo "========================================="
echo

OUTPUT_FILE=$(mktemp)
EXIT_CODE=0
TIMEOUT_SECONDS=30

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

    # Parse failed test names from output
    # Look for lines like "  TI-007: Large Transfer Read [FAIL]"
    FAILED_TESTS=$(grep -E '\[.*FAIL.*\]' "$OUTPUT_FILE" | sed 's/\x1b\[[0-9;]*m//g' | sed -E 's/^[[:space:]]+([A-Z]+-[0-9]+):.*$/\1/' | tr '\n' ', ' | sed 's/,$//' || echo "")

    # Create issue title
    if [ $EXIT_CODE -eq 124 ]; then
        if [ -n "$FAILED_TESTS" ]; then
            ISSUE_TITLE="Test Failure: $FAILED_TESTS - TIMEOUT (${TIMEOUT_SECONDS}s)"
        else
            ISSUE_TITLE="Test Failure: $TEST_NAME - TIMEOUT (${TIMEOUT_SECONDS}s)"
        fi
    else
        if [ -n "$FAILED_TESTS" ]; then
            ISSUE_TITLE="Test Failure: $FAILED_TESTS"
        else
            ISSUE_TITLE="Test Failure: $TEST_NAME - Exit Code $EXIT_CODE"
        fi
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
$(if [ "$TEST_MODE" = "simple" ]; then
    echo "- Test Program: \`simple_test.c\`"
    echo "- Test Binary: \`simple_test\`"
    echo "- Target Example: \`examples/simple_target.rs\`"
else
    echo "- Test Suite: \`iscsi-test-suite/\`"
    echo "- Config: \`test-config.toml\`"
    echo "- Test Mode: $TEST_NAME"
    echo "- Target Implementation: \`src/target.rs\`, \`src/pdu.rs\`, \`src/scsi.rs\`"
fi)
- Configuration: Target at 127.0.0.1:3261

### Diagnostic Information
- **Target Connectivity:** $(timeout 2 bash -c 'echo "" | nc -v 127.0.0.1 3261' 2>&1 | head -1 || echo "Cannot connect to 127.0.0.1:3261")

### Expected Behavior
$(if [ "$TEST_MODE" = "simple" ]; then
    echo "All 5 basic tests should pass:"
    echo "1. Create iSCSI context"
    echo "2. Connect to target"
    echo "3. INQUIRY command"
    echo "4. READ CAPACITY command"
    echo "5. READ/WRITE operations with data integrity check"
elif [ "$TEST_MODE" = "full" ]; then
    echo "All 33 tests from the comprehensive iSCSI test suite should pass."
    echo "Current failures indicate protocol-level bugs in the Rust iSCSI target implementation."
else
    echo "All tests from the $TEST_NAME category should pass."
    echo "Current failures indicate protocol-level bugs in the Rust iSCSI target implementation."
fi)

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
