#!/bin/bash
set -euo pipefail

# Helper script to run the auto-fix loop in Docker on vsprod
# Usage: ./run-auto-fix-vsprod.sh [iterations] [model] [mode]

WORK_DIR="/nonreplicated/testing/iscsi-auto-fix"
CONTAINER_NAME="iscsi-auto-fix"
IMAGE_NAME="iscsi-auto-test"

ITERATIONS="${1:-10}"
MODEL="${2:-haiku}"
MODE="${3:-full}"

echo "========================================="
echo "Starting iSCSI Auto-Fix Loop"
echo "========================================="
echo "Iterations: $ITERATIONS"
echo "Model: $MODEL"
echo "Mode: $MODE"
echo "========================================="
echo ""

# Check if setup has been run
if [ ! -d "$WORK_DIR/repo" ]; then
    echo "Error: Work directory not found. Please run ./docker-setup-vsprod.sh first"
    exit 1
fi

# Remove any existing container
docker rm -f $CONTAINER_NAME 2>/dev/null || true

# Run the container
docker run --name $CONTAINER_NAME \
    -v $WORK_DIR/repo:/repo \
    -v ~/.config/gh:/home/claude/.config/gh \
    -v ~/.claude/.credentials.json:/home/claude/.claude/.credentials.json:ro \
    $IMAGE_NAME \
    /bin/bash -c "source ~/.cargo/env && cd /repo && ./auto-fix-loop.sh $ITERATIONS $MODEL $MODE"

EXIT_CODE=$?

echo ""
echo "========================================="
echo "Auto-fix loop completed with exit code: $EXIT_CODE"
echo "========================================="
echo ""
echo "To view logs from inside container:"
echo "  docker logs $CONTAINER_NAME"
echo ""
echo "To check git status:"
echo "  cd $WORK_DIR/repo && git log --oneline -10"
echo ""

exit $EXIT_CODE
