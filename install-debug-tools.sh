#!/bin/bash
# Smart debugging tool installer
# Only reports if tools are genuinely missing, suppresses apt permission errors

TOOLS="tcpdump tshark strace gdb valgrind inotify-tools hexdump"
MISSING=""

# Check which tools are missing
for tool in $TOOLS; do
    if ! command -v $tool &>/dev/null; then
        MISSING="$MISSING $tool"
    fi
done

if [ -z "$MISSING" ]; then
    # All tools already available
    exit 0
fi

# Try to install missing tools
echo "Installing debugging tools:$MISSING"

# Attempt installation, capture only actual failures (not permission errors)
INSTALL_OUTPUT=$(DEBIAN_FRONTEND=noninteractive apt-get update -qq 2>&1 && \
                 DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
                 tcpdump tshark strace gdb valgrind inotify-tools \
                 libiscsi-dev binutils 2>&1 | \
                 grep -v "Permission denied" | \
                 grep -v "lock file" | \
                 grep -v "List directory" | \
                 grep -v "are you root" || true)

# Check again after installation attempt
STILL_MISSING=""
for tool in $MISSING; do
    if ! command -v $tool &>/dev/null; then
        STILL_MISSING="$STILL_MISSING $tool"
    fi
done

if [ -n "$STILL_MISSING" ]; then
    echo "⚠️  Some tools unavailable:$STILL_MISSING"
    echo "This may limit debugging capabilities but won't prevent testing."
else
    echo "✓ Debugging tools ready"
fi
