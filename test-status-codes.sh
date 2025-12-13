#!/bin/bash
# Test script to verify all implemented RFC 3720 status codes against a live server

set -e

echo "========================================="
echo "RFC 3720 Status Code Integration Tests"
echo "========================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Function to run a test
run_test() {
    local test_name="$1"
    local expected_result="$2"  # "pass" or "fail"
    local command="$3"

    TESTS_RUN=$((TESTS_RUN + 1))
    echo -n "Test $TESTS_RUN: $test_name ... "

    if [ "$expected_result" = "pass" ]; then
        # Expect command to succeed
        if eval "$command" > /dev/null 2>&1; then
            echo -e "${GREEN}PASS${NC}"
            TESTS_PASSED=$((TESTS_PASSED + 1))
        else
            echo -e "${RED}FAIL${NC} (expected success)"
            TESTS_FAILED=$((TESTS_FAILED + 1))
        fi
    else
        # Expect command to fail
        if eval "$command" > /dev/null 2>&1; then
            echo -e "${RED}FAIL${NC} (expected failure)"
            TESTS_FAILED=$((TESTS_FAILED + 1))
        else
            echo -e "${GREEN}PASS${NC}"
            TESTS_PASSED=$((TESTS_PASSED + 1))
        fi
    fi
}

# Function to check if a string contains another string
check_error_message() {
    local test_name="$1"
    local command="$2"
    local expected_text="$3"

    TESTS_RUN=$((TESTS_RUN + 1))
    echo -n "Test $TESTS_RUN: $test_name ... "

    local output=$(eval "$command" 2>&1 || true)

    if echo "$output" | grep -qi "$expected_text"; then
        echo -e "${GREEN}PASS${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}"
        echo "  Expected: '$expected_text'"
        echo "  Got: $output"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

echo "Building project..."
cargo build --release --example chap_target --example simple_client 2>&1 | grep -E "(Compiling|Finished)" || true
echo ""

# Start CHAP-enabled target in background
echo "Starting iSCSI target with CHAP authentication..."
cargo run --release --example chap_target -- 127.0.0.1:13260 > /tmp/iscsi-target-test.log 2>&1 &
TARGET_PID=$!

# Ensure target is killed on exit
trap "kill $TARGET_PID 2>/dev/null || true" EXIT

# Wait for target to start
sleep 2

echo ""
echo "========================================="
echo "Testing Implemented Status Codes"
echo "========================================="
echo ""

# Test 1: SUCCESS (0x0000) - Valid credentials
echo "--- Testing 0x0000 (SUCCESS) ---"
run_test "Login with correct CHAP credentials" "pass" \
    "cargo run --release --example simple_client -- 127.0.0.1:13260 iqn.2025-12.local:initiator iqn.2025-12.local:storage.chap testuser testpass"

echo ""

# Test 2: AUTH_FAILURE (0x0201) - Wrong password
echo "--- Testing 0x0201 (AUTH_FAILURE) ---"
check_error_message "Wrong CHAP password returns AUTH_FAILURE" \
    "cargo run --release --example simple_client -- 127.0.0.1:13260 iqn.2025-12.local:initiator iqn.2025-12.local:storage.chap testuser wrongpass" \
    "Authentication failed"

check_error_message "Wrong CHAP username returns AUTH_FAILURE" \
    "cargo run --release --example simple_client -- 127.0.0.1:13260 iqn.2025-12.local:initiator iqn.2025-12.local:storage.chap wronguser testpass" \
    "Authentication failed"

echo ""

# Test 3: TARGET_NOT_FOUND (0x0203)
echo "--- Testing 0x0203 (TARGET_NOT_FOUND) ---"
check_error_message "Wrong target IQN returns TARGET_NOT_FOUND" \
    "cargo run --release --example simple_client -- 127.0.0.1:13260 iqn.2025-12.local:initiator iqn.wrong.target.name testuser testpass" \
    "Target not found"

echo ""

# Test 4: Verify error messages are helpful
echo "--- Testing Error Message Quality ---"
check_error_message "AUTH_FAILURE mentions credentials" \
    "cargo run --release --example simple_client -- 127.0.0.1:13260 iqn.2025-12.local:initiator iqn.2025-12.local:storage.chap testuser wrongpass" \
    "password"

check_error_message "TARGET_NOT_FOUND suggests discovery" \
    "cargo run --release --example simple_client -- 127.0.0.1:13260 iqn.2025-12.local:initiator iqn.wrong.target testuser testpass" \
    "discovery"

echo ""
echo "========================================="
echo "Test Summary"
echo "========================================="
echo "Tests run:    $TESTS_RUN"
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
if [ $TESTS_FAILED -gt 0 ]; then
    echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"
else
    echo -e "Tests failed: $TESTS_FAILED"
fi
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
