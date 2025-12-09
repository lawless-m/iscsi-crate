#!/bin/bash
set -e

# Build
cargo build --release 2>&1 | grep -E "(error|Finished)"

# Start target with debug logging
RUST_LOG=debug ./target/release/examples/simple_target 2>&1 | grep -E "(sense|Invalid|TC-008|Sending)" &
TARGET_PID=$!

# Wait for target to start
sleep 3

# Run just TC-008
./iscsi-test-suite/iscsi-test-suite ./test-config.toml 2>&1 | grep -A 3 "TC-008"

# Stop target
kill $TARGET_PID 2>/dev/null || true
wait $TARGET_PID 2>/dev/null || true
