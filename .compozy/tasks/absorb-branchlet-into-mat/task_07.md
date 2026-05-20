---
status: completed
title: Close command rewrite
type: backend
complexity: high
dependencies:
  - task_02
  - task_03
  - task_04
  - task_05
---

# Task 07: Close command rewrite

## Overview
Rewrite `handle_close_mode` to use `GitClient`, `TmuxClient`, and `Config`. Implement auto-merge on close: check uncommitted changes → merge feature branch into base branch → on success, delete worktree and branch → close tmux window. Support `--no-merge` flag to skip merge but still clean up. Support merge conflict handling that aborts and leaves branches intact. Support no-worktree close flow (pop stash, merge, delete branch).

<critical>
- ALWAYS READ the PRD and TechSpec before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — every task MUST include tests in deliverables
</critical>

<requirements>
- MUST implement `handle_close(cmd: Command::Close) -> Result<(), MatError>` in `src/commands/close.rs`
- MUST load `Config` to get `merge_strategy` and `delete_branch` settings
- MUST follow the close flow sequence as specified in TechSpec "Data Flow: Close Mode":
  1. Check uncommitted changes via `GitClient::has_uncommitted_changes` — if dirty, print error and stop
  2. Get worktree info via `GitClient::worktree_list` — identify current worktree by matching CWD against worktree paths
  3. Extract branch name, source branch, worktree path from `WorktreeInfo`
  4. If `--no-merge` is NOT set: switch to source branch via `GitClient::checkout(source)`, merge feature branch via `GitClient::merge(branch, strategy)`
  5. On merge conflict: detect via non-zero exit from merge, print PRD-specified conflict error with file list, abort merge, do NOT delete worktree or branch
  6. On merge success OR `--no-merge`: delete worktree via `GitClient::worktree_remove`, delete branch via `GitClient::branch_delete` (if `config.delete_branch` is true and merge succeeded)
  7. Close tmux window via `TmuxClient::close_current_window`
  8. Print success message with merge result
- MUST support no-worktree close flow (when current directory is not a worktree but has a `mat:auto:` stash):
  - Attempt `GitClient::stash_pop("mat:auto:{branch}")` — if fails due to conflicts, print error and stop
  - Follow same merge/branch-delete flow as worktree path
- MUST print clear error messages for each failure scenario matching PRD "Primary Flow: Close" examples
- `--no-merge` flag MUST skip merge step entirely: delete worktree only, optionally delete branch, copy merge command to buffer, close tmux window

## Subtasks
- [x] 07.1 Implement worktree identification via `GitClient::worktree_list` with CWD matching
- [x] 07.2 Implement auto-merge flow: checkout source, merge branch with config strategy, handle conflicts
- [x] 07.3 Implement cleanup flow: delete worktree, delete branch (conditionally), close tmux window
- [x] 07.4 Implement `--no-merge` path: skip merge, delete worktree, copy merge command to buffer
- [x] 07.5 Implement no-worktree close flow: stash pop, merge, branch delete
- [x] 07.6 Write unit tests for all close paths using MockRunner

## Implementation Details

See TechSpec "Data Flow: Close Mode" diagram for the complete sequence. See PRD "Core Features" F5 for auto-merge feature specification. See PRD "Primary Flow: Close (Auto-Merge Success)" and "Close (Merge Conflict)" for exact output message formatting. The current close logic in `src/main.rs` lines 285-370 serves as a reference.

### Relevant Files
- `src/commands/close.rs` — main implementation file, contains `handle_close` function
- `src/git.rs` — `GitClient` for `has_uncommitted_changes`, `worktree_list`, `checkout`, `merge`, `worktree_remove`, `branch_delete`, `stash_pop` (task_02)
- `src/tmux.rs` — `TmuxClient` for `close_current_window`, `set_buffer`, `display_message`, `get_prefix` (task_03)
- `src/config.rs` — `Config` for `merge_strategy`, `delete_branch` (task_04)
- `src/cli.rs` — `Command::Close` variant with `no_merge` flag (task_05)
- `src/display.rs` — `print_success`, `print_info`, `print_tip`, `print_error` (task_01)

### Dependent Files
- None — this is a leaf task

### Related ADRs
- [ADR-002: Lifecycle Manager Approach for Task Closing](../adrs/adr-002.md) — Decision to auto-merge on close with configurable strategy, delete branch only on success
- [ADR-001: Absorb Branchlet Core Worktree Operations into Mat](../adrs/adr-001.md) — Branch deletion configuration

## Deliverables
- `src/commands/close.rs` with `handle_close` function supporting all close paths
- Unit tests with 80%+ coverage using MockRunner (REQUIRED)

## Tests
- Unit tests (using MockRunner):
  - [ ] Uncommitted changes: `has_uncommitted_changes=true` → prints error, returns `Err`, does NOT merge or delete
  - [ ] Auto-merge success: `has_uncommitted_changes=false`, merge exit 0 → `worktree_remove` called, `branch_delete` called if `delete_branch=true`
  - [ ] Auto-merge success with `delete_branch=false`: branch_delete is NOT called
  - [ ] Merge strategy `MergeCommit`: `merge` called with `--no-ff` flag
  - [ ] Merge strategy `FastForward`: `merge` called with `--ff-only` flag
  - [ ] Merge conflict: merge exit non-zero → prints conflict error message with branch name, does NOT call `worktree_remove` or `branch_delete`
  - [ ] `--no-merge` path: `merge` is NOT called; `worktree_remove` IS called; `tmux set-buffer` copies merge command to buffer
  - [ ] `--no-merge` with `delete_branch=true`: `branch_delete` IS called after worktree removal
  - [ ] No-worktree close: `worktree_list` finds no match → attempts `stash_pop("mat:auto:{branch}")`
  - [ ] No-worktree close stash pop failure: prints error with `git stash list` guidance, does NOT proceed
  - [ ] No-worktree close success: stash pop succeeds, merge succeeds, branch deleted
  - [ ] `tmux close_current_window` called after successful cleanup
  - [ ] All output messages match PRD "Primary Flow: Close" examples
- Test coverage target: >=80%
- All tests must pass

## Success Criteria
- All tests passing
- Test coverage >=80%
- `mat close` auto-merges clean branch into base with configured strategy
- `mat close` detects merge conflicts and preserves both branches
- `mat close --no-merge` deletes worktree without merging
- `mat close` in no-worktree mode restores stash and merges
- Output messages match PRD examples for success and conflict scenarios
