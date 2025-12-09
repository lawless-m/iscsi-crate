#!/bin/bash
# Watch Claude Code file activity in the Docker container
# Usage: ./watch_claude.sh

set -euo pipefail

CONTAINER_NAME="iscsi-auto-fix"

echo "========================================="
echo "Watching Claude Code file activity"
echo "========================================="
echo "Container: $CONTAINER_NAME"
echo "Monitoring: /repo/src and /repo/examples"
echo "Press Ctrl+C to stop"
echo "========================================="
echo

# Check if container is running
if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "Error: Container '$CONTAINER_NAME' is not running"
    exit 1
fi

# Check if inotifywait is installed
if ! docker exec "$CONTAINER_NAME" which inotifywait >/dev/null 2>&1; then
    echo "Installing inotify-tools in container..."
    docker exec -u root "$CONTAINER_NAME" sh -c 'apt-get update -qq && apt-get install -y -qq inotify-tools' >/dev/null
    echo "inotify-tools installed"
    echo
fi

# Watch for file changes in real-time
docker exec "$CONTAINER_NAME" sh -c '
    inotifywait -m -r \
        -e modify,create,delete,move \
        --format "%T %w%f %e" \
        --timefmt "%H:%M:%S" \
        /repo/src /repo/examples 2>/dev/null | \
    grep -v "\.git"
'
