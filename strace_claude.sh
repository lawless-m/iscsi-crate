#!/bin/bash
# Attach strace to Claude Code process in the Docker container
# Usage: ./strace_claude.sh

set -euo pipefail

CONTAINER_NAME="iscsi-auto-fix"

echo "========================================="
echo "Tracing Claude Code file operations"
echo "========================================="
echo "Container: $CONTAINER_NAME"
echo "Filtering: /repo/ operations"
echo "Press Ctrl+C to stop"
echo "========================================="
echo

# Check if container is running
if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "Error: Container '$CONTAINER_NAME' is not running"
    exit 1
fi

# Check if strace is installed
if ! docker exec "$CONTAINER_NAME" which strace >/dev/null 2>&1; then
    echo "Installing strace in container..."
    docker exec -u root "$CONTAINER_NAME" sh -c 'apt-get update -qq && apt-get install -y -qq strace' >/dev/null
    echo "strace installed"
    echo
fi

# Find Claude process PID
CLAUDE_PID=$(docker exec "$CONTAINER_NAME" sh -c "pgrep -f '^claude' | head -1" || true)

if [ -z "$CLAUDE_PID" ]; then
    echo "Error: Claude Code process not found in container"
    echo
    echo "Running processes:"
    docker exec "$CONTAINER_NAME" sh -c "ps aux | grep -E '(claude|node)' | grep -v grep"
    exit 1
fi

echo "Found Claude process: PID $CLAUDE_PID"
echo

# Attach strace and filter for repo file operations
docker exec -u root "$CONTAINER_NAME" sh -c "
    strace -f -e trace=openat,read,write,stat,lstat -p $CLAUDE_PID 2>&1
" | while IFS= read -r line; do
    # Only show repo-related operations
    if echo "$line" | grep -q -E '/repo/(src|examples|Cargo)'; then
        echo "[$(date +%H:%M:%S)] $line"
    fi
done
