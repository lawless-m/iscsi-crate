#!/bin/bash
set -e

# Start target in background
cargo run --release -- --target iqn.2025-12.local:storage.memory-disk --lun 0 --size 100 > /tmp/target.log 2>&1 &
TARGET_PID=$!

# Wait for target to start
sleep 3

# Run test
cd iscsi-test-suite
./iscsi-test-suite iscsi://127.0.0.1/iqn.2025-12.local:storage.memory-disk/0

# Kill target
kill $TARGET_PID 2>/dev/null || true
