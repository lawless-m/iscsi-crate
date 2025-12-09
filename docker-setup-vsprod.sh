#!/bin/bash
set -euo pipefail

# Comprehensive Docker setup for auto-fix loop on vsprod
# This script sets up a proper git clone with push access

WORK_DIR="/nonreplicated/testing/iscsi-auto-fix"
REPO_URL="git@github.com:lawless-m/iscsi-crate.git"
CONTAINER_NAME="iscsi-auto-fix"
IMAGE_NAME="iscsi-auto-test"

echo "========================================="
echo "Setting up iSCSI Auto-Fix Environment"
echo "========================================="

# 1. Clean up any existing setup
echo "Cleaning up existing containers..."
docker rm -f $CONTAINER_NAME 2>/dev/null || true

# 2. Ensure work directory exists
echo "Setting up work directory: $WORK_DIR"
mkdir -p "$WORK_DIR"
cd "$WORK_DIR"

# 3. Setup repository (update if exists, clone if not)
if [ -d "repo/.git" ]; then
    echo "Repository exists, updating..."
    cd repo
    git fetch origin
    git reset --hard origin/master
else
    echo "Cloning repository..."
    rm -rf repo
    git clone "$REPO_URL" repo
    cd repo
fi

# 4. Configure git for commits
echo "Configuring git..."
git config user.name "Claude Code Auto-Fixer"
git config user.email "noreply@anthropic.com"

# 5. Build the Docker image
echo "Building Docker image..."
cd "$WORK_DIR/repo"
docker build -t $IMAGE_NAME -f - . <<'DOCKERFILE'
FROM debian:12

# Install dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    libiscsi-dev \
    curl \
    git \
    netcat-openbsd \
    nodejs \
    npm \
    procps \
    tcpdump \
    tshark \
    strace \
    gdb \
    valgrind \
    inotify-tools \
    bsdmainutils \
    sudo \
    tgt \
    && rm -rf /var/lib/apt/lists/*

# Install GitHub CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
    && apt-get update \
    && apt-get install -y gh \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -s /bin/bash claude && \
    echo 'claude ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers

# Install Rust as claude user
USER claude
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/home/claude/.cargo/bin:${PATH}"

# Switch back to root for final setup
USER root

# Create Claude Code wrapper script
RUN echo '#!/bin/bash\nexec npx -y @anthropic-ai/claude-code "$@"' > /usr/local/bin/claude && \
    chmod +x /usr/local/bin/claude

# Ensure claude owns necessary directories
RUN mkdir -p /home/claude/.claude /home/claude/.config/gh && \
    chown -R claude:claude /home/claude

# Set working directory
WORKDIR /repo
RUN chown -R claude:claude /repo

# Switch to non-root user
USER claude

CMD ["/bin/bash"]
DOCKERFILE

# 6. Create symlinks for easy access to scripts
echo "Creating symlinks..."
cd "$WORK_DIR"
ln -sf repo/run-auto-fix-vsprod.sh .
ln -sf repo/run-implement-vsprod.sh .
ln -sf repo/docker-setup-vsprod.sh .
ln -sf repo/test-tgtd.sh .
ln -sf repo/test-rust.sh .

# 7. Display instructions
echo ""
echo "========================================="
echo "Setup Complete!"
echo "========================================="
echo ""
echo "Work directory: $WORK_DIR/repo"
echo "Docker image: $IMAGE_NAME"
echo ""
echo "To run the auto-fix loop:"
echo ""
echo "  docker run --name $CONTAINER_NAME \\"
echo "    -v $WORK_DIR/repo:/repo \\"
echo "    -v ~/.ssh:/home/claude/.ssh:ro \\"
echo "    -v ~/.config/gh:/home/claude/.config/gh:ro \\"
echo "    -v ~/.claude/.credentials.json:/home/claude/.claude/.credentials.json:ro \\"
echo "    $IMAGE_NAME \\"
echo "    /bin/bash -c 'source ~/.cargo/env && cd /repo && ./auto-fix-loop.sh 10 haiku full'"
echo ""
echo "Or use the helper script: ./run-auto-fix-vsprod.sh"
echo ""
