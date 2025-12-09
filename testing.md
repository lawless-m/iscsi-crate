# Test Automation with GitHub Issues

## Goal
Create an automation system that runs tests, and when they fail, posts an issue to GitHub. Then Claude Code can pick up those issues and attempt to fix them.

## Current Project Context
- **Testing Setup**: C tests running against a Rust backend
- Both the tests and the backend were written by us - we control everything
- Tests are already written and working

## What We Need

### 1. Shell Script Test Runner
A shell script that:
- Runs the C test suite
- Captures test output
- On failure: posts an issue to GitHub with the test failure details
- Uses `gh` CLI or GitHub API for posting issues

### 2. Issue Format
The issue should include:
- Which test(s) failed
- Test output / error messages
- Any relevant context (commit hash, environment details, etc.)
- Enough information for Claude Code to understand what broke

### 3. GitHub Integration
- Use `gh` CLI (already available)
- Post to the project's repository
- Consider: should we check if issue already exists for this failure?

## Approach
Start simple and specific to this project. Don't try to generalize yet - we'll learn what works first, then extract patterns for other projects later.

## Status: ✅ IMPLEMENTED

The test automation system has been implemented and tested.

## Usage

### Running Tests Manually

```bash
# Run simple smoke test (default)
./run-tests.sh

# Run full test suite
./run-tests.sh full
```

This will:
1. Auto-start the iSCSI target if not running
2. Run the test suite (`simple_test` or full `iscsi-test-suite`)
3. Capture all output with a 10-second timeout
4. On failure: automatically post a GitHub issue (if `gh` CLI is configured)
5. Save the issue body locally if GitHub posting fails
6. Clean up the target if we started it

### What the Issue Contains

Each GitHub issue includes:
- **Test command** that was run
- **Exit code** (with timeout annotation if applicable)
- **Full test output** (ANSI color codes stripped)
- **Environment details**: commit hash, branch, OS info, date
- **Diagnostic info**: target status, network connectivity
- **Files involved**: test program, target code
- **Expected vs actual behavior**

### GitHub CLI Setup

To enable automatic issue posting:

```bash
# Install gh CLI (if not already installed)
sudo apt-get install gh

# Authenticate
gh auth login

# Test it
gh issue list
```

### Automated Issue Fixing

The `fix-issue.sh` script automatically invokes Claude Code to fix issues:

```bash
# Interactive mode - you approve each step (recommended)
./fix-issue.sh 3

# Auto-accept file edits, but prompt for bash commands
./fix-issue.sh --auto-edit 3

# Use a more powerful model for complex issues
./fix-issue.sh --model sonnet 3

# Combine: auto-accept edits with sonnet model
./fix-issue.sh --model sonnet --auto-edit 3

# Full bypass (sandboxed environments only)
./fix-issue.sh --no-prompts 3
```

**How it works:**
1. Fetches the full issue from GitHub
2. Formats it as a detailed prompt
3. Invokes Claude Code with appropriate model and options
4. Claude investigates, fixes, tests, and closes the issue

**Model selection:**
- `haiku` (default): Fast and cheap for simple fixes
- `sonnet`: Balanced for most issues
- `opus`: For complex architectural problems

**Permission modes:**
- Default: Interactive - you approve each action
- `--auto-edit`: Auto-accepts file edits, prompts for commands (good balance)
- `--no-prompts`: Bypasses all permissions (only for sandboxed/trusted environments)

### Automated Test-Fix Loop

The ultimate automation: continuously test and fix until all tests pass:

#### Option 1: Run Locally (Interactive or Semi-Automated)

```bash
# Run with defaults (10 iterations max, haiku model)
./auto-fix-loop.sh

# Custom iterations and model
./auto-fix-loop.sh 20 sonnet

# Full test suite
./auto-fix-loop.sh 10 haiku full
```

**Note:** Requires user approval for bash commands unless using `--no-prompts` in fix-issue.sh

#### Option 2: Sandboxed VM (Fully Automated, No Prompts) ⭐

For **truly** hands-off automation with `--dangerously-skip-permissions`, run in a sandboxed VM:

```bash
# One-time setup: Start VM and copy project
./vm-setup.sh

# SSH into VM and run the loop
ssh -p 2224 debian@localhost
cd ~/iscsi-crate
./auto-fix-loop.sh 10 haiku full

# Or run directly from host
ssh -p 2224 debian@localhost 'cd ~/iscsi-crate && ./auto-fix-loop.sh 10 haiku full'
```

**Why use the VM?**
- ✅ Safe to use `--dangerously-skip-permissions` (no risk to host)
- ✅ Completely isolated environment
- ✅ Zero babysitting - walk away and come back to results
- ✅ Perfect for overnight runs
- ✅ Easy to reset if something goes wrong

**What it does:**
1. Run tests
2. If tests fail → Find open test failure issue
3. Auto-fix the issue with `--no-prompts` mode
4. Go back to step 1
5. Repeat until tests pass or max iterations reached

**This is how you win:** The loop keeps fixing issues until your test suite is green, learning from each failure. Perfect for:
- Initial bring-up of a new feature
- Regression testing after major changes
- CI/CD integration (run overnight, wake up to green tests)
- Fuzzing-driven development (find + fix bugs automatically)

### Next Steps (Not Yet Done)
1. ✅ Have Claude Code pick up and attempt to fix issues
2. ✅ Add support for the full test suite
3. ✅ Create automated test-fix loop
4. Add CI/CD integration (GitHub Actions)

## Notes
- This is the first implementation - expect to learn and refine
- Focus on making it work for this specific C/Rust project first
- Future: will expand to multiple languages and projects
- The script includes duplicate detection to avoid spamming issues
