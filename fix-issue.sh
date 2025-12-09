#!/bin/bash
set -euo pipefail

# Helper script to fetch a GitHub issue and automatically invoke Claude Code to fix it
# Usage: ./fix-issue.sh [options] <issue-number>
#
# Options:
#   --model <model>      Model to use (haiku, sonnet, opus) - default: haiku for cost efficiency
#   --auto-edit          Auto-accept file edits (still prompts for bash commands)
#   --no-prompts         Bypass all permissions (use only in sandboxed/trusted environments)

# Parse options
MODEL="haiku"
PERMISSION_MODE=""
ITERATION=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --model)
            MODEL="$2"
            shift 2
            ;;
        --auto-edit)
            PERMISSION_MODE="acceptEdits"
            shift
            ;;
        --no-prompts)
            PERMISSION_MODE="dangerouslySkip"
            shift
            ;;
        --iteration)
            ITERATION="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [options] <issue-number>"
            echo
            echo "Options:"
            echo "  --model <model>      Model to use (haiku, sonnet, opus) - default: haiku"
            echo "  --auto-edit          Auto-accept file edits (still prompts for bash)"
            echo "  --no-prompts         Bypass all permissions (sandboxed environments only)"
            echo "  --iteration <num>    Iteration number (for tracking repeated attempts)"
            echo
            echo "Available open issues:"
            gh issue list --repo lawless-m/iscsi-crate --state open --json number,title --jq '.[] | "  #\(.number): \(.title)"'
            exit 0
            ;;
        *)
            ISSUE_NUM="$1"
            shift
            ;;
    esac
done

if [ -z "${ISSUE_NUM:-}" ]; then
    echo "Usage: $0 [options] <issue-number>"
    echo
    echo "Available open issues:"
    gh issue list --repo lawless-m/iscsi-crate --state open --json number,title --jq '.[] | "  #\(.number): \(.title)"'
    exit 1
fi

echo "Fetching issue #$ISSUE_NUM..."
echo

# Get issue details (let it fail naturally if issue doesn't exist)
ISSUE_TITLE=$(gh issue view --repo lawless-m/iscsi-crate "$ISSUE_NUM" --json title --jq '.title' 2>&1)
if [ $? -ne 0 ]; then
    echo "Error: Could not fetch issue #$ISSUE_NUM"
    echo "$ISSUE_TITLE"
    exit 1
fi

ISSUE_BODY=$(gh issue view --repo lawless-m/iscsi-crate "$ISSUE_NUM" --json body --jq '.body')
ISSUE_URL=$(gh issue view --repo lawless-m/iscsi-crate "$ISSUE_NUM" --json url --jq '.url')

# Gather context about previous attempts if this is not the first iteration
PREVIOUS_ATTEMPTS=""
if [ -n "$ITERATION" ] && [ "$ITERATION" -gt 1 ]; then
    echo "Gathering context from previous attempts..."

    # Check if WIP branch exists for this issue
    WIP_BRANCH="auto-fix-wip/issue-${ISSUE_NUM}"
    WIP_COMMITS=""

    if git show-ref --verify --quiet "refs/remotes/origin/$WIP_BRANCH"; then
        # Fetch latest from WIP branch
        git fetch origin "$WIP_BRANCH" 2>/dev/null || true

        # Look for WIP commits from previous iterations in the WIP branch
        WIP_COMMITS=$(git log --oneline "origin/$WIP_BRANCH" --grep="WIP: Attempted fix iteration" -5 2>/dev/null || echo "")
    fi

    if [ -n "$WIP_COMMITS" ]; then
        PREVIOUS_ATTEMPTS=$(cat <<ATTEMPTS

================================================================================
PREVIOUS ATTEMPTS (You have tried fixing this before):
================================================================================

This is attempt #$ITERATION to fix this issue. Previous attempts have failed.

Failed attempts are tracked in branch: $WIP_BRANCH

Recent failed attempts:
$WIP_COMMITS

IMPORTANT: Review what was tried before by examining these commits from the WIP branch:
  git fetch origin $WIP_BRANCH
  git show origin/$WIP_BRANCH
  git log -p origin/$WIP_BRANCH

DO NOT repeat the same approach. Try a different strategy:
- If previous attempts modified data encoding, try the decoding/parsing side
- If previous attempts changed algorithms, try different data structures
- If previous attempts fixed symptoms, look for root causes deeper in the call stack
- If previous attempts added edge case handling, reconsider the main logic path
- If previous attempts were complex, try a simpler approach
- Consider adding detailed logging/tracing to understand actual runtime behavior
- Review the test failure output carefully for clues previous attempts may have missed

ATTEMPTS
)
    fi
fi

# Create a formatted prompt for Claude Code
PROMPT=$(cat <<EOF
GitHub Issue #$ISSUE_NUM: $ISSUE_TITLE
URL: $ISSUE_URL

$ISSUE_BODY
$PREVIOUS_ATTEMPTS

================================================================================

Please investigate and fix the issue described above.

IMPORTANT: This is a Rust iSCSI target implementation being tested by C test programs.
- The tests (simple_test.c, iscsi-test-suite/) are CORRECT and should NOT be modified
- Fix the RUST TARGET CODE in examples/ or src/ directories
- The tests are validating that the Rust implementation follows the iSCSI RFC correctly

Steps:
1. Read the test output and diagnostic information
2. Examine the Rust target implementation (examples/simple_target.rs, src/)
3. Identify why the Rust target is failing the test
4. Fix the RUST CODE (not the test code)
5. Test the fix by running: ./run-tests.sh
6. If tests pass:
   a. Commit your changes with a descriptive commit message
   b. Push to GitHub with: git push origin master
   c. Close the issue with: gh issue close --repo lawless-m/iscsi-crate $ISSUE_NUM --comment "Fixed: [brief explanation]"
EOF
)

# Check if we should auto-invoke Claude Code
if command -v claude &> /dev/null; then
    echo "Invoking Claude Code to fix issue #$ISSUE_NUM..."
    echo "  Model: $MODEL"

    if [ -n "$PERMISSION_MODE" ]; then
        case "$PERMISSION_MODE" in
            acceptEdits)
                echo "  Mode: Auto-accept edits (prompts for bash commands)"
                ;;
            dangerouslySkip)
                echo "  Mode: FULLY AUTOMATED - Zero prompts"
                ;;
        esac
    else
        echo "  Mode: Interactive (normal prompts)"
    fi
    echo

    # Build claude command with options
    CLAUDE_OPTS=(
        --model "$MODEL"
        --verbose
        --append-system-prompt "You are fixing GitHub issue #$ISSUE_NUM in an automated workflow. Be concise and focused."
    )

    # Add permission mode if specified
    if [ -n "$PERMISSION_MODE" ]; then
        if [ "$PERMISSION_MODE" = "dangerouslySkip" ]; then
            CLAUDE_OPTS+=(--dangerously-skip-permissions)
        else
            CLAUDE_OPTS+=(--permission-mode "$PERMISSION_MODE")
        fi
    fi

    # Invoke claude with the prompt (direct output, no buffering)
    echo "========================================="
    echo "Claude Code is now working on the fix..."
    echo "Model: $MODEL"
    echo "Started at: $(date)"
    echo "========================================="
    echo

    # Run Claude directly without output manipulation
    # Let it output naturally in real-time
    set +e  # Don't exit on error
    claude "${CLAUDE_OPTS[@]}" "$PROMPT"
    CLAUDE_EXIT_CODE=$?
    set -e

    echo
    echo "========================================="
    echo "Claude Code finished with exit code: $CLAUDE_EXIT_CODE"
    echo "Finished at: $(date)"
    echo "========================================="

    exit $CLAUDE_EXIT_CODE
else
    # Fall back to just displaying the prompt
    echo "================================================================================
GitHub Issue #$ISSUE_NUM
================================================================================

Title: $ISSUE_TITLE
URL: $ISSUE_URL

$ISSUE_BODY

================================================================================
PROMPT FOR CLAUDE CODE:
================================================================================

$PROMPT

================================================================================
NOTE: Install 'claude' CLI to auto-invoke Claude Code with this prompt
================================================================================
"
fi
