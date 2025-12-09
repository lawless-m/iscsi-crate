#!/bin/bash
set -euo pipefail

# Comprehensive test runner for all iSCSI target configurations
# Tests: simple (no auth), CHAP, and Mutual CHAP

REPO="lawless-m/iscsi-crate"
RESULTS_DIR="test-results"
mkdir -p "$RESULTS_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "========================================="
echo "Comprehensive iSCSI Target Test Suite"
echo "========================================="
echo

# Track overall results
TOTAL_PASSED=0
TOTAL_FAILED=0
TOTAL_SKIPPED=0

# Test configurations
declare -A CONFIGS=(
    ["simple"]="test_config.ini:simple_target:3261"
    ["chap"]="chap_config.ini:chap_target:3262"
    ["mutual_chap"]="mutual_chap_config.ini:mutual_chap_target:3263"
)

for config_name in "${!CONFIGS[@]}"; do
    IFS=':' read -r config_file target_name port <<< "${CONFIGS[$config_name]}"

    echo -e "${BLUE}=========================================${NC}"
    echo -e "${BLUE}Testing: $config_name ($target_name)${NC}"
    echo -e "${BLUE}=========================================${NC}"
    echo

    # Kill any existing target on this port
    lsof -ti:$port | xargs kill -9 2>/dev/null || true
    sleep 1

    # Start the target
    echo "Starting $target_name on port $port..."
    cargo run --example $target_name > "$RESULTS_DIR/${config_name}_target.log" 2>&1 &
    TARGET_PID=$!

    # Wait for target to be ready
    for i in {1..30}; do
        if nc -z 127.0.0.1 $port 2>/dev/null; then
            echo -e "${GREEN}Target ready${NC}"
            break
        fi
        sleep 0.1
    done

    if ! nc -z 127.0.0.1 $port 2>/dev/null; then
        echo -e "${RED}ERROR: Target failed to start${NC}"
        kill $TARGET_PID 2>/dev/null || true
        continue
    fi

    # Run the test suite
    echo "Running test suite with $config_file..."
    if ./iscsi-test-suite/iscsi-test-suite "./iscsi-test-suite/config/$config_file" > "$RESULTS_DIR/${config_name}_results.txt" 2>&1; then
        echo -e "${GREEN}✓ All tests passed for $config_name${NC}"
        TEST_EXIT=0
    else
        echo -e "${RED}✗ Some tests failed for $config_name${NC}"
        TEST_EXIT=$?
    fi

    # Parse results
    PASSED=$(grep -E "Results: [0-9]+ passed" "$RESULTS_DIR/${config_name}_results.txt" | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+' || echo 0)
    FAILED=$(grep -E "[0-9]+ failed" "$RESULTS_DIR/${config_name}_results.txt" | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+' || echo 0)
    SKIPPED=$(grep -E "[0-9]+ skipped" "$RESULTS_DIR/${config_name}_results.txt" | grep -oE '[0-9]+ skipped' | grep -oE '[0-9]+' || echo 0)

    echo "  Passed: $PASSED"
    echo "  Failed: $FAILED"
    echo "  Skipped: $SKIPPED"
    echo

    TOTAL_PASSED=$((TOTAL_PASSED + PASSED))
    TOTAL_FAILED=$((TOTAL_FAILED + FAILED))
    TOTAL_SKIPPED=$((TOTAL_SKIPPED + SKIPPED))

    # Kill the target
    kill $TARGET_PID 2>/dev/null || true
    sleep 1
done

echo "========================================="
echo "Overall Results"
echo "========================================="
echo -e "${GREEN}Total Passed: $TOTAL_PASSED${NC}"
if [ $TOTAL_FAILED -gt 0 ]; then
    echo -e "${RED}Total Failed: $TOTAL_FAILED${NC}"
else
    echo -e "${GREEN}Total Failed: $TOTAL_FAILED${NC}"
fi
echo -e "${YELLOW}Total Skipped: $TOTAL_SKIPPED${NC}"
echo

if [ $TOTAL_FAILED -eq 0 ]; then
    echo -e "${GREEN}=========================================${NC}"
    echo -e "${GREEN}SUCCESS! All tests passed!${NC}"
    echo -e "${GREEN}=========================================${NC}"
    exit 0
else
    echo -e "${RED}=========================================${NC}"
    echo -e "${RED}FAILURE! Some tests failed${NC}"
    echo -e "${RED}=========================================${NC}"
    echo
    echo "Check detailed results in $RESULTS_DIR/"
    exit 1
fi
