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
        -h|--help)
            echo "Usage: $0 [options] <issue-number>"
            echo
            echo "Options:"
            echo "  --model <model>      Model to use (haiku, sonnet, opus) - default: haiku"
            echo "  --auto-edit          Auto-accept file edits (still prompts for bash)"
            echo "  --no-prompts         Bypass all permissions (sandboxed environments only)"
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

# Create a formatted prompt for Claude Code
PROMPT=$(cat <<EOF
GitHub Issue #$ISSUE_NUM: $ISSUE_TITLE
URL: $ISSUE_URL

$ISSUE_BODY

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

    # Invoke claude with the prompt
    claude "${CLAUDE_OPTS[@]}" "$PROMPT"
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
