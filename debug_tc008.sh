#!/bin/bash
set -e

# Start target
echo "Starting target..."
RUST_LOG=info ./target/release/examples/simple_target > /tmp/target.log 2>&1 &
TARGET_PID=$!

# Wait for target to start
sleep 2

echo "Running TC-008 test..."
# Run just TC-008
./iscsi-test-suite/iscsi-test-suite ./test-config.toml 2>&1 | grep -A 5 "TC-008"

# Show target log
echo ""
echo "=== Target log ==="
grep -E "(sense|Invalid|TC-008|Sending|Unsupported)" /tmp/target.log | tail -20

# Stop target
kill $TARGET_PID 2>/dev/null || true
wait $TARGET_PID 2>/dev/null || true

echo "Done"
