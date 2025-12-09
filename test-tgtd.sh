#!/bin/bash
set -euo pipefail

# Simple script to run tests against TGTD (reference implementation)
# Usage: ./test-tgtd.sh [category]
#   category: discovery, commands, io, or full (default: full)

WORK_DIR="/nonreplicated/testing/iscsi-auto-fix"
IMAGE_NAME="iscsi-auto-test"
MODE="${1:-full}"

echo "========================================="
echo "Running Tests Against TGTD"
echo "========================================="
echo "Mode: $MODE"
echo ""

# Check if setup has been run
if [ ! -d "$WORK_DIR/repo" ]; then
    echo "Error: Work directory not found. Please run ./docker-setup-vsprod.sh first"
    exit 1
fi

# Run tests against TGTD in Docker
docker run --rm \
    -v $WORK_DIR/repo:/repo \
    $IMAGE_NAME \
    /bin/bash -c "cd /repo && sudo timeout 60 ./validate-against-tgtd.sh $MODE"

EXIT_CODE=$?

echo ""
echo "========================================="
if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ TGTD tests PASSED"
elif [ $EXIT_CODE -eq 124 ]; then
    echo "❌ TGTD tests TIMED OUT"
else
    echo "❌ TGTD tests FAILED (exit code: $EXIT_CODE)"
fi
echo "========================================="
echo ""

exit $EXIT_CODE
