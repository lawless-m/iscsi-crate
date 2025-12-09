#!/bin/bash
set -euo pipefail

# Setup QEMU VM on remote vsprod for automated testing
# Usage: ./remote-vm-setup.sh

REMOTE_HOST="matt@vsprod"
REMOTE_DIR="/nonreplicated/testing/iscsi-test-vm"
REMOTE_PORT=3260  # SSH port for the VM on remote side
LOCAL_PORT=2230   # Forward to this port on local machine

echo "========================================="
echo "Remote QEMU VM Setup for iSCSI Testing"
echo "========================================="
echo "Remote: $REMOTE_HOST:$REMOTE_DIR"
echo "VM SSH will be forwarded to localhost:$LOCAL_PORT"
echo "========================================="
echo

# Create remote directory structure
echo "Creating remote directory..."
ssh $REMOTE_HOST "mkdir -p $REMOTE_DIR/project $REMOTE_DIR/vm-data"

# Check if we have a Debian image locally to upload
if [ -f "/home/matt/Git/VoE/qemu-vms/debian-12-generic-amd64.qcow2" ]; then
    echo "Found local Debian image. Checking if remote needs it..."

    if ! ssh $REMOTE_HOST "[ -f $REMOTE_DIR/vm-data/debian-test.qcow2 ]"; then
        echo "Copying Debian image to remote (this may take a while)..."
        scp /home/matt/Git/VoE/qemu-vms/debian-12-generic-amd64.qcow2 \
            $REMOTE_HOST:$REMOTE_DIR/vm-data/debian-test.qcow2
    else
        echo "Remote VM image already exists"
    fi
else
    echo "WARNING: No local Debian image found"
    echo "You'll need to create one on the remote system"
fi

# Copy project files
echo "Syncing project files to remote..."
rsync -avz --progress \
    --exclude='target/' \
    --exclude='.git/' \
    --exclude='*.qcow2' \
    --exclude='tmp/' \
    ./ $REMOTE_HOST:$REMOTE_DIR/project/

# Create VM startup script on remote
echo "Creating VM control scripts on remote..."
ssh $REMOTE_HOST "cat > $REMOTE_DIR/start-vm.sh" <<'REMOTE_SCRIPT'
#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"

if [ -f vm.pid ] && kill -0 $(cat vm.pid) 2>/dev/null; then
    echo "VM already running (PID $(cat vm.pid))"
    exit 0
fi

echo "Starting QEMU VM for iSCSI testing..."

qemu-system-x86_64 \
    -name iscsi-test-vm \
    -m 4096 \
    -hda vm-data/debian-test.qcow2 \
    -netdev user,id=net0,hostfwd=tcp::3260-:22 \
    -device e1000,netdev=net0 \
    -enable-kvm -cpu host \
    -display none \
    -daemonize \
    -pidfile vm.pid

echo "VM started. Waiting for SSH..."
sleep 5

# Wait for SSH
for i in {1..30}; do
    if ssh -p 3260 -o StrictHostKeyChecking=no -o ConnectTimeout=2 \
        debian@localhost 'echo ready' 2>/dev/null; then
        echo "VM is ready on port 3260!"
        exit 0
    fi
    echo "  Waiting... ($i/30)"
    sleep 2
done

echo "ERROR: VM failed to become ready"
exit 1
REMOTE_SCRIPT

# Create stop script
ssh $REMOTE_HOST "cat > $REMOTE_DIR/stop-vm.sh" <<'REMOTE_SCRIPT'
#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"

if [ ! -f vm.pid ]; then
    echo "No VM PID file found"
    exit 0
fi

PID=$(cat vm.pid)
if kill -0 $PID 2>/dev/null; then
    echo "Stopping VM (PID $PID)..."
    kill $PID
    sleep 2

    # Force kill if still running
    if kill -0 $PID 2>/dev/null; then
        echo "Force killing..."
        kill -9 $PID
    fi

    rm -f vm.pid
    echo "VM stopped"
else
    echo "VM not running"
    rm -f vm.pid
fi
REMOTE_SCRIPT

# Make scripts executable
ssh $REMOTE_HOST "chmod +x $REMOTE_DIR/*.sh"

# Start the VM
echo
echo "Starting VM on remote system..."
ssh $REMOTE_HOST "$REMOTE_DIR/start-vm.sh"

# Set up SSH port forwarding
echo
echo "Setting up SSH port forwarding..."
echo "Forwarding localhost:$LOCAL_PORT -> $REMOTE_HOST:3260 -> VM:22"

# Kill any existing forwarding on this port
pkill -f "ssh.*$LOCAL_PORT:localhost:3260.*$REMOTE_HOST" 2>/dev/null || true

# Start port forwarding in background
ssh -f -N -L $LOCAL_PORT:localhost:3260 $REMOTE_HOST

echo
echo "========================================="
echo "Remote VM Setup Complete!"
echo "========================================="
echo
echo "The VM is running on $REMOTE_HOST"
echo "You can access it locally via: ssh -p $LOCAL_PORT debian@localhost"
echo
echo "To set up the environment in the VM:"
echo "  ssh -p $LOCAL_PORT debian@localhost"
echo "  cd /home/debian"
echo "  # Then copy project and install deps"
echo
echo "To stop the remote VM:"
echo "  ssh $REMOTE_HOST '$REMOTE_DIR/stop-vm.sh'"
echo
echo "To stop port forwarding:"
echo "  pkill -f 'ssh.*$LOCAL_PORT:localhost:3260'"
echo
