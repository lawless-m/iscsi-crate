#!/bin/bash
set -euo pipefail

# Simple script to run tests against Rust iSCSI target
# Usage: ./test-rust.sh [mode]
#   mode: quick, full, or specific test name (default: full)

WORK_DIR="/nonreplicated/testing/iscsi-auto-fix"
IMAGE_NAME="iscsi-auto-test"

MODE="${1:-full}"

echo "========================================="
echo "Running Tests Against Rust Target"
echo "========================================="
echo "Mode: $MODE"
echo ""

# Check if setup has been run
if [ ! -d "$WORK_DIR/repo" ]; then
    echo "Error: Work directory not found. Please run ./docker-setup-vsprod.sh first"
    exit 1
fi

# Run tests against Rust target in Docker
docker run --rm \
    -v $WORK_DIR/repo:/repo \
    $IMAGE_NAME \
    /bin/bash -c "source ~/.cargo/env && cd /repo && ./run-tests.sh $MODE"

EXIT_CODE=$?

echo ""
echo "========================================="
if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ Rust target tests PASSED"
elif [ $EXIT_CODE -eq 124 ]; then
    echo "❌ Rust target tests TIMED OUT"
else
    echo "❌ Rust target tests FAILED (exit code: $EXIT_CODE)"
fi
echo "========================================="
echo ""

exit $EXIT_CODE
