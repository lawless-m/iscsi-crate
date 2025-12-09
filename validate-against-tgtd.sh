#!/bin/bash
set -euo pipefail

# Validates tests against TGTD reference implementation
# Usage: ./validate-against-tgtd.sh
#
# Returns:
#   0 - All tests pass against TGTD
#   1 - Some tests fail against TGTD (test is buggy!)
#   2 - TGTD not available / setup failed

echo "========================================="
echo "TGTD Validation"
echo "========================================="
echo "Validating tests against TGTD reference implementation..."
echo

# Check if TGTD is available
if ! command -v tgtd &> /dev/null; then
    echo "ERROR: tgtd not found. Install with:"
    echo "  sudo apt-get install tgt"
    exit 2
fi

# Create TGTD config
TGTD_CONFIG="/tmp/tgtd-$$.conf"
cat > "$TGTD_CONFIG" <<'EOF'
<target iqn.2025-12.local:storage.tgtd-validation>
    backing-store /tmp/tgtd-disk.img
    # Allow connections from localhost
    initiator-address 127.0.0.1
</target>
EOF

# Create backing store (256MB)
if [ ! -f /tmp/tgtd-disk.img ]; then
    echo "Creating TGTD backing store (256MB)..."
    dd if=/dev/zero of=/tmp/tgtd-disk.img bs=1M count=256 2>/dev/null
fi

# Kill any existing tgtd
sudo pkill tgtd 2>/dev/null || true
sleep 0.5

# Start tgtd
echo "Starting TGTD..."
sudo tgtd -f &
TGTD_PID=$!
sleep 1

# Check if tgtd started
if ! ps -p $TGTD_PID > /dev/null 2>&1; then
    echo "ERROR: Failed to start tgtd"
    exit 2
fi

# Configure target
echo "Configuring TGTD target..."
sudo tgtadm --lld iscsi --mode target --op new --tid 1 --targetname iqn.2025-12.local:storage.tgtd-validation
sudo tgtadm --lld iscsi --mode logicalunit --op new --tid 1 --lun 1 --backing-store /tmp/tgtd-disk.img
sudo tgtadm --lld iscsi --mode target --op bind --tid 1 --initiator-address ALL

# Wait for target to be ready
sleep 1

# Verify target is accessible
if ! nc -z 127.0.0.1 3260 2>/dev/null; then
    echo "ERROR: TGTD not listening on port 3260"
    sudo kill $TGTD_PID 2>/dev/null || true
    exit 2
fi

echo "TGTD ready"
echo

# Create test config for TGTD
TGTD_TEST_CONFIG="/tmp/tgtd-test-config.toml"
cat > "$TGTD_TEST_CONFIG" <<'EOF'
[target]
portal = 127.0.0.1:3260
iqn = iqn.2025-12.local:storage.tgtd-validation
lun = 1

[options]
timeout = 30
verbosity = 1
stop_on_fail = false
EOF

# Build test suite if needed
if [ ! -f ./iscsi-test-suite/iscsi-test-suite ]; then
    echo "Building test suite..."
    (cd iscsi-test-suite && make clean && make)
fi

# Run tests against TGTD
echo "Running tests against TGTD..."
echo "========================================="
./iscsi-test-suite/iscsi-test-suite "$TGTD_TEST_CONFIG" 2>&1 | tee /tmp/tgtd-validation.log
TEST_EXIT=$?

echo
echo "========================================="

# Cleanup
echo "Cleaning up TGTD..."
sudo tgtadm --lld iscsi --mode target --op delete --tid 1 2>/dev/null || true
sudo kill $TGTD_PID 2>/dev/null || true
rm -f "$TGTD_CONFIG" "$TGTD_TEST_CONFIG"

if [ $TEST_EXIT -eq 0 ]; then
    echo "✅ All tests PASSED against TGTD"
    echo "Tests are valid. If they fail against Rust target, the target has bugs."
    exit 0
else
    echo "❌ Tests FAILED against TGTD (exit code: $TEST_EXIT)"
    echo "The test implementation is likely buggy!"
    echo "Output saved to: /tmp/tgtd-validation.log"
    exit 1
fi
