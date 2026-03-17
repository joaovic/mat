# Feature Plan: Close Task Capability

## Overview
Add the ability to close a previously created task worktree, with safety checks and merge preparation.

## Requirements

### CLI Arguments
- New flag: `--close` or `-c`
- When used, the app operates in "close mode" instead of "create mode"
- No additional arguments required (uses current worktree context)

### Behavior Flow

1. **Prerequisite Checks**
   - Verify TMUX is running
   - Verify current directory is inside a Branchlet worktree
   - Check for uncommitted changes (via `git status --porcelain`)
     - If uncommitted changes exist: STOP and alert user to commit or discard first
     - If clean: proceed

2. **Worktree Cleanup**
   - Get current worktree info (branch name, base branch) from Branchlet
   - Delete current worktree using Branchlet

3. **TMUX Window Management**
   - Get user's TMUX prefix from config (detect `Ctrl+a`, `Ctrl+b`, etc.)
   - Close current TMUX window
   - Switch to TMUX window index 0 (original window)

4. **Merge Preparation**
   - On base branch, run `git status` to ensure it's clean
   - Construct merge command: `git merge <feature-branch> --no-ff -m "Merge <feature-branch>"`
   - Save merge command to TMUX clipboard (using `tmux set-buffer`)
   - Print instructions to console:
     ```
     You are now ready to merge your feature!
     Press [{tmux-prefix}] then ] to paste the merge command
     
     Merge command: git merge <branch-name> --no-ff -m "Merge <branch-name>"
     ```

### Edge Cases
- User not in a worktree directory → error with helpful message
- Base branch doesn't exist locally → warn user, still prepare merge command
- TMUX prefix detection fallback to `Ctrl+b` if not detected

## Implementation Notes
- Use existing error handling patterns from `main.rs`
- Reuse TMUX prefix detection logic already in codebase
- Branchlet delete command: `branchlet delete <worktree-name>` or similar

## Acceptance Criteria
- [x] `--close` / `-c` flag works
- [x] Prevents closing with uncommitted changes
- [x] Successfully deletes worktree via Branchlet
- [x] Closes current TMUX window and returns to window 0
- [x] Merge command saved to TMUX clipboard
- [x] Console shows instructions with correct TMUX prefix
- [x] Works with user's custom TMUX prefix configuration
