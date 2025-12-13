#!/bin/bash
# Run integration tests against a live target server

set -e

echo "========================================="
echo "iSCSI Integration Tests"
echo "========================================="
echo ""

# Start simple target on port 13260
echo "Starting iSCSI target on 127.0.0.1:13260..."
cargo run --release --example simple_target -- 127.0.0.1:13260 > /tmp/iscsi-target-integration.log 2>&1 &
TARGET_PID=$!

# Ensure target is killed on exit
trap "echo 'Stopping target...'; kill $TARGET_PID 2>/dev/null || true; wait $TARGET_PID 2>/dev/null || true" EXIT

# Wait for target to start
echo "Waiting for target to initialize..."
sleep 2

echo ""
echo "========================================="
echo "Running Status Code Integration Tests"
echo "========================================="
echo ""

# Run the SERVICE_UNAVAILABLE test
echo "Test: Graceful Shutdown (SERVICE_UNAVAILABLE 0x0301)"
if cargo test --release --test status_code_tests test_server_returns_service_unavailable_on_shutdown -- --ignored --nocapture --test-threads=1; then
    echo "✓ PASS: Graceful shutdown test"
else
    echo "✗ FAIL: Graceful shutdown test"
    exit 1
fi

echo ""
echo "========================================="
echo "All integration tests passed!"
echo "========================================="
