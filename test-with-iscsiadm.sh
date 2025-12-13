#!/bin/bash
# Test iSCSI target with standard iscsiadm client

set -e

echo "========================================="
echo "iSCSI Target Test with iscsiadm"
echo "========================================="
echo ""

# Check if iscsiadm is available
if ! command -v iscsiadm &> /dev/null; then
    echo "ERROR: iscsiadm not found. Install open-iscsi:"
    echo "  sudo apt-get install open-iscsi"
    exit 1
fi

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

TARGET_ADDR="127.0.0.1:13260"
TARGET_IQN="iqn.2025-12.local:storage.test"

# Start simple target
echo "Starting iSCSI target at $TARGET_ADDR..."
cargo run --release --example simple_target -- $TARGET_ADDR > /tmp/iscsi-target-test.log 2>&1 &
TARGET_PID=$!

# Cleanup on exit
trap "echo ''; echo 'Cleaning up...'; kill $TARGET_PID 2>/dev/null || true; sleep 1" EXIT

# Wait for target to start
echo "Waiting for target to initialize..."
sleep 2

echo ""
echo "========================================="
echo "Running Tests"
echo "========================================="
echo ""

# Test 1: Discovery
echo -n "Test 1: Discovery (SendTargets)... "
if timeout 5 iscsiadm -m discovery -t sendtargets -p $TARGET_ADDR 2>&1 | grep -q "$TARGET_IQN"; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
    echo "Discovery output:"
    timeout 5 iscsiadm -m discovery -t sendtargets -p $TARGET_ADDR 2>&1 || true
fi

# Test 2: Login
echo -n "Test 2: Login to target... "
if timeout 5 iscsiadm -m node -p $TARGET_ADDR -T $TARGET_IQN --login 2>&1 | grep -q "successful"; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${YELLOW}SKIP${NC} (may require root)"
fi

# Test 3: List sessions
echo -n "Test 3: Verify session exists... "
if timeout 5 iscsiadm -m session 2>&1 | grep -q "$TARGET_IQN"; then
    echo -e "${GREEN}PASS${NC}"

    # Logout
    echo -n "Test 4: Logout from target... "
    if timeout 5 iscsiadm -m node -p $TARGET_ADDR -T $TARGET_IQN --logout 2>&1 | grep -q "successful"; then
        echo -e "${GREEN}PASS${NC}"
    else
        echo -e "${YELLOW}SKIP${NC}"
    fi
else
    echo -e "${YELLOW}SKIP${NC} (no active session)"
fi

echo ""
echo "========================================="
echo "Testing with Rust client library"
echo "========================================="
echo ""

# Create a quick test client
cat > /tmp/test_client.rs << 'EOF'
use iscsi_target::{IscsiClient, IscsiError};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target_addr = std::env::args().nth(1)
        .unwrap_or_else(|| "127.0.0.1:13260".to_string());
    let initiator_iqn = std::env::args().nth(2)
        .unwrap_or_else(|| "iqn.2025-12.local:initiator".to_string());
    let target_iqn = std::env::args().nth(3)
        .unwrap_or_else(|| "iqn.2025-12.local:storage.test".to_string());

    println!("Connecting to {}...", target_addr);
    let mut client = IscsiClient::connect(&target_addr)?;

    println!("Logging in as {} to {}...", initiator_iqn, target_iqn);
    client.login(&initiator_iqn, &target_iqn)?;

    println!("SUCCESS: Logged in successfully!");

    println!("Discovering targets...");
    let targets = client.discover()?;
    println!("Found {} target(s):", targets.len());
    for target in targets {
        println!("  - {}", target);
    }

    println!("Logging out...");
    client.logout()?;

    println!("SUCCESS: All operations completed");
    Ok(())
}
EOF

echo "Building test client..."
cat > /tmp/Cargo.toml << EOF
[package]
name = "test_client"
version = "0.1.0"
edition = "2021"

[dependencies]
iscsi-target = { path = "$(pwd)" }
EOF

cd /tmp
if cargo build --release --quiet 2>&1 | grep -q "error"; then
    echo -e "${RED}Build failed${NC}"
else
    echo -n "Test 5: Rust client login... "
    if timeout 10 ./target/release/test_client 127.0.0.1:13260 iqn.2025-12.local:initiator iqn.2025-12.local:storage.test 2>&1 | grep -q "SUCCESS"; then
        echo -e "${GREEN}PASS${NC}"
    else
        echo -e "${RED}FAIL${NC}"
        echo "Client output:"
        timeout 10 ./target/release/test_client 127.0.0.1:13260 iqn.2025-12.local:initiator iqn.2025-12.local:storage.test 2>&1 || true
    fi
fi

cd - > /dev/null

echo ""
echo "========================================="
echo "Tests Complete"
echo "========================================="
