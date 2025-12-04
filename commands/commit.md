# Git Commit Command

Create a well-crafted git commit for the current changes.

## Instructions

1. **Gather information** (run these in parallel):
   - `git status` - see all untracked and modified files
   - `git diff --staged` - see what's already staged
   - `git diff` - see unstaged changes
   - `git log --oneline -10` - see recent commit message style

2. **Analyze the changes**:
   - Determine the nature of changes (feature, fix, refactor, docs, test, chore, etc.)
   - Identify which files should be committed together
   - Check for files that should NOT be committed (secrets, .env, credentials, temp files, etc.)
   - If there are unrelated changes, ask the user if they want separate commits

3. **Stage the appropriate files**:
   - Use `git add <files>` for specific files, or `git add .` if all changes are related
   - Never stage files containing secrets or credentials without explicit user approval

4. **Write a good commit message**:
   - Use conventional commit format when appropriate: `type: description`
   - Types: feat, fix, docs, style, refactor, test, chore, perf, ci, build
   - First line: imperative mood, max 50 chars, no period (e.g., "Add user authentication")
   - Focus on WHY, not just WHAT
   - If more detail needed, add blank line then body with context

5. **Create the commit** using a HEREDOC for proper formatting:
   ```bash
   git commit -m "$(cat <<'EOF'
   type: Short description here

   Optional longer description explaining the why.

   🤖 Generated with [Claude Code](https://claude.com/claude-code)

   Co-Authored-By: Claude <noreply@anthropic.com>
   EOF
   )"
   ```

6. **Verify the commit**:
   - Run `git status` to confirm clean working tree (or remaining unstaged changes)
   - If pre-commit hooks modify files, amend the commit (after checking authorship)

## Important Rules

- NEVER commit files containing secrets, API keys, or credentials
- NEVER use `--no-verify` to skip hooks unless explicitly requested
- NEVER amend commits that have been pushed or authored by others
- NEVER force push to main/master
- If unsure about what to include, ASK the user first
- Keep commits atomic - one logical change per commit
