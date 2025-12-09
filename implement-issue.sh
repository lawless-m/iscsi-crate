#!/bin/bash
set -euo pipefail

# Script to implement new features/tests from GitHub issues
# Usage: ./implement-issue.sh [options] <issue-number>
#
# Unlike fix-issue.sh (which fixes failing tests), this implements new features.
# Useful for implementing skipped tests or adding new functionality.

# Parse options
MODEL="sonnet"
PERMISSION_MODE="dangerouslySkip"
ITERATION=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --model)
            MODEL="$2"
            shift 2
            ;;
        --iteration)
            ITERATION="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [options] <issue-number>"
            echo
            echo "Options:"
            echo "  --model <model>      Model to use (haiku, sonnet, opus) - default: sonnet"
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

# Get issue details
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

    WIP_BRANCH="auto-fix-wip/issue-${ISSUE_NUM}"
    WIP_COMMITS=""

    if git show-ref --verify --quiet "refs/remotes/origin/$WIP_BRANCH"; then
        git fetch origin "$WIP_BRANCH" 2>/dev/null || true
        WIP_COMMITS=$(git log --oneline "origin/$WIP_BRANCH" --grep="WIP: Attempted implementation iteration" -5 2>/dev/null || echo "")
    fi

    if [ -n "$WIP_COMMITS" ]; then
        PREVIOUS_ATTEMPTS=$(cat <<ATTEMPTS

================================================================================
PREVIOUS ATTEMPTS (You have tried implementing this before)
================================================================================

This is attempt #$ITERATION to implement this feature. Previous attempts were incomplete.

Failed attempts are tracked in branch: $WIP_BRANCH

Recent attempts:
$WIP_COMMITS

IMPORTANT: Review what was tried before:
  git fetch origin $WIP_BRANCH
  git show origin/$WIP_BRANCH
  git log -p origin/$WIP_BRANCH

Try a different or improved approach based on what was learned.

ATTEMPTS
)
    fi
fi

# Create prompt for Claude Code
PROMPT=$(cat <<EOF
GitHub Issue #$ISSUE_NUM: $ISSUE_TITLE
URL: $ISSUE_URL

$ISSUE_BODY
$PREVIOUS_ATTEMPTS

================================================================================

Please implement the feature/test described in this issue.

IMPORTANT: This is feature implementation, not bug fixing.
- Implement new functionality or new test cases
- Follow existing code patterns and conventions
- Add appropriate error handling and validation
- Update documentation if needed

TOOLS AND ENVIRONMENT:
- Install debugging tools if needed: ./install-debug-tools.sh (handles permissions gracefully)
- Or install manually: apt-get update && apt-get install -y <tool>
- Already available: strace, inotify-tools, gcc, make, cargo, git
- Additional tools available: tcpdump, tshark, gdb, valgrind, hexdump
- The environment is Debian-based headless Docker container with root access

Steps:
1. Read and understand the feature requirements
2. Examine existing code to understand patterns and conventions
3. Implement the feature following the existing style
4. Commit your changes with a descriptive commit message
5. Push to GitHub: git push origin master

6. Add a comment to the issue summarizing what you implemented:
   gh issue comment --repo lawless-m/iscsi-crate $ISSUE_NUM --body "Your implementation summary"

IMPORTANT: DO NOT run tests yourself. DO NOT close the issue.
The wrapper script will run tests and close the issue if they pass.
Your job is only to implement, commit, push, and document what you did.
EOF
)

# Check if we should auto-invoke Claude Code
if command -v claude &> /dev/null; then
    echo "Invoking Claude Code to implement issue #$ISSUE_NUM..."
    echo "  Model: $MODEL"
    echo "  Mode: FULLY AUTOMATED"
    echo

    # Build claude command
    CLAUDE_OPTS=(
        --model "$MODEL"
        --verbose
        --append-system-prompt "You are implementing GitHub issue #$ISSUE_NUM. Focus on implementing the feature completely and correctly."
        --dangerously-skip-permissions
    )

    echo "========================================="
    echo "Claude Code is now working on implementation..."
    echo "Model: $MODEL"
    echo "Started at: $(date)"
    echo "========================================="
    echo

    set +e
    claude "${CLAUDE_OPTS[@]}" "$PROMPT"
    CLAUDE_EXIT_CODE=$?
    set -e

    echo
    echo "========================================="
    echo "Claude Code finished with exit code: $CLAUDE_EXIT_CODE"
    echo "Finished at: $(date)"
    echo "========================================="

    # Test gating: Only close issue if tests pass
    if [ $CLAUDE_EXIT_CODE -eq 0 ]; then
        # Detect if this is a test implementation issue
        IS_TEST_IMPL=false
        if echo "$ISSUE_TITLE" | grep -iE "(implement|add).*test|test.*implement" > /dev/null; then
            IS_TEST_IMPL=true
        fi

        if [ "$IS_TEST_IMPL" = "true" ]; then
            echo ""
            echo "🔍 Detected test implementation - validating against TGTD first..."
            echo "========================================="
            echo ""

            # For test implementations, validate against TGTD first
            if [ -f "./validate-against-tgtd.sh" ]; then
                set +e
                sudo timeout 60 ./validate-against-tgtd.sh
                TGTD_EXIT_CODE=$?
                set -e

                echo ""
                echo "========================================="
                echo "TGTD validation finished with exit code: $TGTD_EXIT_CODE"
                echo "========================================="
                echo ""

                if [ $TGTD_EXIT_CODE -eq 0 ]; then
                    echo "✅ TGTD validation passed - test implementation is correct"
                    echo "   Now testing against our Rust target..."
                    echo ""
                elif [ $TGTD_EXIT_CODE -eq 124 ]; then
                    echo "❌ TGTD validation timed out - test implementation has bugs"
                    gh issue comment --repo lawless-m/iscsi-crate $ISSUE_NUM --body "⚠️ Test implementation timed out against TGTD (reference implementation). The test code itself has bugs. Exit code: 124"
                    exit 1
                else
                    echo "❌ TGTD validation failed - test implementation has bugs"
                    gh issue comment --repo lawless-m/iscsi-crate $ISSUE_NUM --body "⚠️ Test implementation failed against TGTD (reference implementation). The test code itself has bugs. Exit code: $TGTD_EXIT_CODE"
                    exit 1
                fi
            else
                echo "⚠️ validate-against-tgtd.sh not found, skipping TGTD validation"
            fi
        fi

        echo ""
        echo "Running tests against our target..."
        echo "========================================="

        set +e
        ./run-tests.sh full
        TEST_EXIT_CODE=$?
        set -e

        echo ""
        echo "========================================="
        echo "Tests finished with exit code: $TEST_EXIT_CODE"
        echo "========================================="
        echo ""

        if [ $TEST_EXIT_CODE -eq 0 ]; then
            echo "✅ Tests passed! Closing issue #$ISSUE_NUM"
            gh issue close --repo lawless-m/iscsi-crate $ISSUE_NUM --comment "Implementation complete and all tests pass. ✅"
        elif [ $TEST_EXIT_CODE -eq 124 ]; then
            if [ "$IS_TEST_IMPL" = "true" ]; then
                echo "❌ Our target timed out (but TGTD passed) - this is a target bug, not a test bug"
                gh issue comment --repo lawless-m/iscsi-crate $ISSUE_NUM --body "⚠️ Test passed against TGTD but our Rust target timed out (exit 124). This indicates our target has a bug handling these inputs. Leaving issue open for target fixes."
            else
                echo "❌ Tests timed out (exit $TEST_EXIT_CODE). Leaving issue open."
                gh issue comment --repo lawless-m/iscsi-crate $ISSUE_NUM --body "⚠️ Implementation introduced a timeout (exit code 124). Tests hung after 30 seconds. Leaving issue open for debugging."
            fi
        else
            echo "❌ Tests failed (exit $TEST_EXIT_CODE). Leaving issue open."
            gh issue comment --repo lawless-m/iscsi-crate $ISSUE_NUM --body "⚠️ Implementation complete but tests failed with exit code $TEST_EXIT_CODE. Leaving issue open for fixes."
        fi
    else
        echo "⚠️ Claude Code did not complete successfully (exit $CLAUDE_EXIT_CODE)"
        echo "Leaving issue #$ISSUE_NUM open."
    fi

    exit $CLAUDE_EXIT_CODE
else
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
