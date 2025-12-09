#!/bin/bash
set -euo pipefail

# Setup and run automated test-fix loop in sandboxed Debian VM
# This allows using --dangerously-skip-permissions safely

VM_IMAGE="/home/matt/Git/VoE/qemu-vms/debian-12-generic-amd64.qcow2"
VM_SSH_PORT=2224
VM_USER="debian"
VM_PASS="debian"

echo "========================================="
echo "Sandboxed Auto-Fix Loop Setup"
echo "========================================="
echo

# Check if VM SSH is available
echo "Checking if VM is accessible on port $VM_SSH_PORT..."
if ! ssh -p $VM_SSH_PORT -o ConnectTimeout=2 -o StrictHostKeyChecking=no \
    ${VM_USER}@localhost 'echo ready' 2>/dev/null; then
    echo "ERROR: Cannot connect to VM on port $VM_SSH_PORT"
    echo "The VM should be started by systemd on boot"
    echo "Check: sudo systemctl status qemu-debian-vm"
    exit 1
fi
echo "VM is accessible!"

echo
echo "========================================="
echo "Copying project to VM..."
echo "========================================="

# Create project directory in VM
ssh -p $VM_SSH_PORT ${VM_USER}@localhost "mkdir -p ~/iscsi-crate"

# Sync project files (exclude large artifacts)
rsync -avz --progress -e "ssh -p $VM_SSH_PORT -o StrictHostKeyChecking=no" \
    --exclude='target/' \
    --exclude='.git/' \
    --exclude='*.qcow2' \
    --exclude='tmp/' \
    ./ ${VM_USER}@localhost:~/iscsi-crate/

echo
echo "========================================="
echo "Setting up environment in VM..."
echo "========================================="

ssh -p $VM_SSH_PORT ${VM_USER}@localhost <<'VMSETUP'
cd ~/iscsi-crate

# Install dependencies if not already installed
if ! command -v cargo &>/dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

if ! dpkg -l | grep -q libiscsi-dev; then
    echo "Installing libiscsi-dev..."
    sudo apt update
    sudo apt install -y libiscsi-dev build-essential
fi

if ! command -v gh &>/dev/null; then
    echo "Installing GitHub CLI..."
    curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null
    sudo apt update
    sudo apt install -y gh
fi

# Compile test binaries
echo "Compiling tests..."
gcc -o simple_test simple_test.c -liscsi

cd iscsi-test-suite
make clean && make
cd ..

echo "Environment setup complete!"
VMSETUP

echo
echo "========================================="
echo "VM Setup Complete!"
echo "========================================="
echo
echo "To run the auto-fix loop in the VM:"
echo "  ssh -p $VM_SSH_PORT ${VM_USER}@localhost"
echo "  cd ~/iscsi-crate"
echo "  ./auto-fix-loop.sh 10 haiku full"
echo
echo "Or run it directly from host:"
echo "  ssh -p $VM_SSH_PORT ${VM_USER}@localhost 'cd ~/iscsi-crate && ./auto-fix-loop.sh 10 haiku full'"
echo
echo "To stop the VM:"
echo "  sudo kill \$(cat /tmp/iscsi-test-vm.pid)"
echo
