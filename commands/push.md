# Git Push Command

Commit any pending changes and push to the remote repository.

## Instructions

1. **Check for uncommitted changes**:
   ```bash
   git status
   ```

2. **If there are uncommitted changes**:
   - Run `/commit` first to create a proper commit
   - Wait for the commit to complete before proceeding

3. **Check remote status**:
   ```bash
   git fetch
   git status
   ```
   - If behind remote, warn the user and ask if they want to pull first
   - If diverged, warn and suggest `git pull --rebase` or manual resolution

4. **Push to remote**:
   ```bash
   git push
   ```

5. **Verify success**:
   - Confirm the push completed
   - Show the remote URL and branch pushed to

## Important Rules

- NEVER force push to main/master
- NEVER push if there are uncommitted changes without committing first
- If push is rejected, explain why and suggest solutions (pull, rebase, etc.)
