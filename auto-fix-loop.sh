#!/bin/bash
set -euo pipefail

# Continuous test-fix-test loop
# Runs tests, fixes failures, repeats until all tests pass or max iterations reached
# Usage: ./auto-fix-loop.sh [iterations] [model] [test-mode]

MAX_ITERATIONS=${1:-10}  # Default: 10 iterations max
MODEL=${2:-haiku}        # Default model: haiku for cost efficiency
TEST_MODE=${3:-simple}   # Default: simple test (use "full" for full suite)

echo "========================================="
echo "Automated Test-Fix Loop"
echo "========================================="
echo "Max iterations: $MAX_ITERATIONS"
echo "Model: $MODEL"
echo "Test mode: $TEST_MODE"
echo "========================================="
echo

# Pull latest changes from GitHub
echo "Pulling latest changes from GitHub..."
git pull origin master || {
    echo "Warning: Failed to pull latest changes, continuing with local version"
}
echo

iteration=0
while [ $iteration -lt $MAX_ITERATIONS ]; do
    iteration=$((iteration + 1))
    echo
    echo "========================================="
    echo "ITERATION $iteration / $MAX_ITERATIONS"
    echo "========================================="
    echo

    # Run tests
    echo "Running tests ($TEST_MODE mode)..."
    if ./run-tests.sh "$TEST_MODE"; then
        echo
        echo "========================================="
        echo "SUCCESS! All tests passed!"
        echo "========================================="
        echo "Total iterations: $iteration"
        exit 0
    fi

    # Tests failed - check for new issues
    echo
    echo "Tests failed. Checking for open issues..."

    OPEN_ISSUES=$(gh issue list --repo lawless-m/iscsi-crate --state open --label test-failure --json number --jq '.[].number' 2>/dev/null || true)
    if [ -z "$OPEN_ISSUES" ]; then
        # Try without label filter
        OPEN_ISSUES=$(gh issue list --repo lawless-m/iscsi-crate --state open --search "Test Failure" --json number --jq '.[].number' 2>/dev/null | head -1 || true)
    fi

    if [ -z "$OPEN_ISSUES" ]; then
        echo "No open test failure issues found. This might be a transient failure."
        echo "Retrying in iteration $((iteration + 1))..."
        sleep 2
        continue
    fi

    # Fix the first open issue
    ISSUE_NUM=$(echo "$OPEN_ISSUES" | head -1)
    echo "Found open issue: #$ISSUE_NUM"
    echo

    # Before attempting fix, check if there are uncommitted changes from previous attempt
    if [ $iteration -gt 1 ]; then
        if [ -n "$(git status --short | grep -v '^??')" ]; then
            echo "Committing previous failed attempt #$((iteration - 1)) to WIP branch..."

            # Create/switch to WIP branch for failed attempts
            WIP_BRANCH="auto-fix-wip/issue-${ISSUE_NUM}"

            # Stash changes temporarily
            git stash push -m "WIP iteration $((iteration - 1))" || true

            # Create branch from master if it doesn't exist, or checkout if it does
            if git show-ref --verify --quiet "refs/heads/$WIP_BRANCH"; then
                git checkout "$WIP_BRANCH"
                git merge master --no-edit || true
            else
                git checkout -b "$WIP_BRANCH" master
            fi

            # Apply the stashed changes
            git stash pop || true

            # Commit the failed attempt to WIP branch
            git add -A
            git commit -m "WIP: Attempted fix iteration $((iteration - 1)) - tests still failing

Previous attempt did not resolve the issue. Tests still show:
- TI-007: Large Transfer Read - data mismatch
- TI-008: Large Transfer Write - data mismatch

This commit preserves the attempted changes for context." || true

            # Push WIP branch (not master!)
            git push -u origin "$WIP_BRANCH" || true

            # Return to master for next attempt
            git checkout master

            # Clean up any remaining changes
            git reset --hard HEAD
        fi
    fi

    echo "Attempting automated fix (iteration $iteration)..."

    # Run fix with iteration context for full automation
    if ./fix-issue.sh --model "$MODEL" --no-prompts --iteration "$iteration" "$ISSUE_NUM"; then
        echo "Fix attempt completed"
    else
        echo "Fix attempt failed - will retry on next iteration"
    fi

    # Brief pause before next iteration
    sleep 2
done

echo
echo "========================================="
echo "Max iterations ($MAX_ITERATIONS) reached"
echo "========================================="
echo

# Show remaining issues
REMAINING=$(gh issue list --repo lawless-m/iscsi-crate --state open --search "Test Failure" --json number,title --jq '.[] | "#\(.number): \(.title)"' 2>/dev/null || true)
if [ -n "$REMAINING" ]; then
    echo "Remaining open issues:"
    echo "$REMAINING"
    exit 1
else
    echo "No remaining issues, but tests still failing."
    echo "May need manual investigation."
    exit 1
fi
