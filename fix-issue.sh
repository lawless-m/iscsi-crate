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

# Fetch issue labels to determine what type of fix is needed
ISSUE_LABELS=$(gh issue view --repo lawless-m/iscsi-crate "$ISSUE_NUM" --json labels --jq '.labels[].name' | tr '\n' ',' || echo "")

# Determine issue type from labels
IS_TEST_BUG=false
IS_TARGET_BUG=false
if echo "$ISSUE_LABELS" | grep -q "test-bug"; then
    IS_TEST_BUG=true
elif echo "$ISSUE_LABELS" | grep -q "target-bug"; then
    IS_TARGET_BUG=true
fi

# Fetch issue comments for additional context
ISSUE_COMMENTS=$(gh issue view --repo lawless-m/iscsi-crate "$ISSUE_NUM" --json comments --jq '.comments[] | "## Comment by @\(.author.login) (\(.createdAt))\n\n\(.body)\n"' || echo "")
if [ -n "$ISSUE_COMMENTS" ]; then
    COMMENTS_SECTION=$(cat <<COMMENTS

================================================================================
ISSUE COMMENTS (Important context and updates):
================================================================================

$ISSUE_COMMENTS
COMMENTS
)
else
    COMMENTS_SECTION=""
fi

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

# Customize prompt based on issue type (from labels)
if [ "$IS_TEST_BUG" = true ]; then
    ISSUE_TYPE_GUIDANCE=$(cat <<GUIDANCE
CRITICAL - ISSUE TYPE: TEST BUG (based on label: test-bug)

This issue is labeled as "test-bug", which means TGTD validation showed the test itself is incorrect.

WHAT TO FIX:
- Fix the TEST CODE in iscsi-test-suite/ directory to match TGTD behavior
- DO NOT modify the Rust target code in src/ or examples/
- The Rust target is likely correct; the test has wrong expectations or incorrect implementation

WHY:
- The test also fails against TGTD (reference iSCSI implementation)
- This proves the test logic is flawed, not the Rust target
- TGTD defines the de facto standard behavior for iSCSI implementations

HOW TO FIX:
1. Run the test against TGTD to understand what TGTD actually does
2. Update test expectations to match TGTD's actual behavior
3. If TGTD accepts something, test should verify acceptance (not rejection)
4. If TGTD rejects something, test should verify rejection
5. The goal is a WORKING test that validates real-world behavior

ABSOLUTELY FORBIDDEN:
- DO NOT just return TEST_SKIP - that's avoiding the problem, not fixing it
- DO NOT skip tests unless they are genuinely impossible to implement
- Skipping a test is admitting defeat - we need working tests, not disabled tests
- If you can't figure out how to fix the test, document what you tried and leave it open

WHERE TO LOOK:
- Test implementation in iscsi-test-suite/src/
- Test expectations and assertions
- TGTD validation output in the issue description
- Compare against iSCSI RFC 3720 specification AND real-world TGTD behavior
GUIDANCE
)
elif [ "$IS_TARGET_BUG" = true ]; then
    ISSUE_TYPE_GUIDANCE=$(cat <<GUIDANCE
CRITICAL - ISSUE TYPE: TARGET BUG (based on label: target-bug)

This issue is labeled as "target-bug", which means TGTD validation showed the test is correct.

WHAT TO FIX:
- Fix the RUST TARGET CODE in src/ or examples/ directories
- DO NOT modify the test code in iscsi-test-suite/
- The tests are correct and validating proper iSCSI RFC compliance

WHY:
- The same test PASSES against TGTD (reference iSCSI implementation)
- This proves the Rust target implementation is incorrect

WHERE TO LOOK:
- Rust target implementation in examples/simple_target.rs
- Core protocol logic in src/target.rs, src/pdu.rs, src/scsi.rs
- Compare against iSCSI RFC 3720 specification
GUIDANCE
)
else
    ISSUE_TYPE_GUIDANCE=$(cat <<GUIDANCE
NOTE: Issue type not specified via labels. Investigate to determine if this is a test bug or target bug.
- Check if TGTD validation results are included in the issue description
- If unclear, you may need to run TGTD validation yourself
GUIDANCE
)
fi

# Create a formatted prompt for Claude Code
PROMPT=$(cat <<EOF
GitHub Issue #$ISSUE_NUM: $ISSUE_TITLE
URL: $ISSUE_URL

$ISSUE_BODY
$COMMENTS_SECTION
$PREVIOUS_ATTEMPTS

================================================================================

Please investigate and fix the issue described above.

$ISSUE_TYPE_GUIDANCE

TOOLS AND ENVIRONMENT:
- Install debugging tools if needed: ./install-debug-tools.sh (handles permissions gracefully)
- Or install manually: apt-get update && apt-get install -y <tool>
- Already available: strace, inotify-tools, gcc, make, cargo, git
- Additional tools available: tcpdump, tshark, gdb, valgrind, hexdump
- The environment is Debian-based headless Docker container with root access

Steps:
1. Read the test output and diagnostic information
2. Identify what needs to be fixed based on the issue label and TGTD validation
3. Fix the appropriate code (test code for test-bug, Rust target for target-bug)
4. Test the fix by running: ./run-tests.sh

5. Based on test results:

   **If ALL tests pass:**
   a. Commit your changes with a descriptive commit message
   b. Push to GitHub: git push origin master
   c. Close the issue: gh issue close --repo lawless-m/iscsi-crate $ISSUE_NUM --comment "Fixed: [brief explanation]"

   **If tests STILL FAIL (but you made progress):**
   a. Commit your changes with a descriptive commit message
   b. Push to GitHub: git push origin master
   c. LEAVE THE ISSUE OPEN - Add a comment documenting:
      - What you tried and why
      - What you learned from the attempt
      - Specific next steps to try in the next iteration
      Use: gh issue comment --repo lawless-m/iscsi-crate $ISSUE_NUM --body "Your analysis and next steps"

   **If you made no meaningful changes:**
   a. DO NOT commit anything
   b. LEAVE THE ISSUE OPEN
   c. Add a comment explaining what you investigated
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
