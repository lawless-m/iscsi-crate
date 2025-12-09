#!/bin/bash
set -euo pipefail

# Helper script to run implement-issue.sh in Docker on vsprod
# Usage: ./run-implement-vsprod.sh [model] <issue-number>

WORK_DIR="/nonreplicated/testing/iscsi-auto-fix"
CONTAINER_NAME="iscsi-implement"
IMAGE_NAME="iscsi-auto-test"

MODEL="${1:-haiku}"
ISSUE_NUM="${2:-}"

if [ -z "$ISSUE_NUM" ]; then
    echo "Usage: $0 [model] <issue-number>"
    echo
    echo "Example: $0 haiku 54"
    echo "         $0 sonnet 55"
    echo
    echo "Available open issues:"
    gh issue list --repo lawless-m/iscsi-crate --state open --json number,title --jq '.[] | "  #\(.number): \(.title)"'
    exit 1
fi

# Check if model is actually a number (user might have swapped parameters)
if [[ "$MODEL" =~ ^[0-9]+$ ]]; then
    # First param is a number - either:
    # 1. User provided only issue number (no second param)
    # 2. User swapped parameters: issue-number then model
    if [ -n "$ISSUE_NUM" ] && [[ ! "$ISSUE_NUM" =~ ^[0-9]+$ ]]; then
        # Second param is not a number, so it's the model
        # User ran: ./script 5 sonnet (swapped order)
        TEMP="$MODEL"
        MODEL="$ISSUE_NUM"
        ISSUE_NUM="$TEMP"
    else
        # User provided only issue number, no model specified
        ISSUE_NUM="$MODEL"
        MODEL="haiku"
    fi
fi

echo "========================================="
echo "Implementing GitHub Issue"
echo "========================================="
echo "Issue: #$ISSUE_NUM"
echo "Model: $MODEL"
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
    -v ~/.ssh:/home/claude/.ssh:ro \
    -v ~/.config/gh:/home/claude/.config/gh:ro \
    -v ~/.claude/.credentials.json:/home/claude/.claude/.credentials.json:ro \
    $IMAGE_NAME \
    /bin/bash -c "
        # Install debugging tools with smart error handling
        chmod +x /repo/install-debug-tools.sh && /repo/install-debug-tools.sh

        # Pull latest changes
        cd /repo
        git fetch origin 2>&1 | grep -v 'credential-!' || true
        git reset --hard origin/master

        # Run implement-issue.sh
        source ~/.cargo/env && ./implement-issue.sh --model $MODEL $ISSUE_NUM
    "

EXIT_CODE=$?

echo ""
echo "========================================="
echo "Implementation completed with exit code: $EXIT_CODE"
echo "========================================="
echo ""

if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ Implementation and tests passed!"
else
    echo "⚠️ Implementation or tests failed (exit $EXIT_CODE)"
fi

echo ""
echo "To view logs:"
echo "  docker logs $CONTAINER_NAME"
echo ""
echo "To check what was implemented:"
echo "  cd $WORK_DIR/repo && git log --oneline -5"
echo "  cd $WORK_DIR/repo && git diff HEAD~1"
echo ""
echo "To check issue status:"
echo "  gh issue view --repo lawless-m/iscsi-crate $ISSUE_NUM"
echo ""

exit $EXIT_CODE
