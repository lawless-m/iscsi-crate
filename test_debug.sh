#!/bin/bash

pkill -9 -f simple_target 2>/dev/null || true
sleep 1

echo "Building..."
cargo build --example simple_target 2>&1 | tail -3

echo "Starting target..."
RUST_LOG=debug ./target/debug/examples/simple_target > /tmp/target.log 2>&1 &
TARGET_PID=$!
sleep 3

if ! nc -z 127.0.0.1 3261 2>/dev/null; then
    echo "Target failed to start"
    tail -50 /tmp/target.log
    kill $TARGET_PID 2>/dev/null || true
    exit 1
fi

echo "Running TC-008..."
timeout 30 ./iscsi-test-suite/iscsi-test-suite ./test-config.toml > /tmp/test.log 2>&1 || true

sleep 2
kill $TARGET_PID 2>/dev/null || true

echo ""
echo "=== TEST OUTPUT ==="
grep -A 3 "TC-008" /tmp/test.log

echo ""
echo "=== TARGET LOGS (looking for invalid command) ==="
tail -150 /tmp/target.log | grep -A 3 -B 3 "0xFF\|CDB\[0\]=0xff\|sense_key=0x05\|CHECK_CONDITION"
