---
status: pending
title: Create command rewrite
type: backend
complexity: high
dependencies:
  - task_02
  - task_03
  - task_04
  - task_05
---

# Task 06: Create command rewrite

## Overview
Rewrite `handle_create_mode` to use the new `GitClient`, `TmuxClient`, `Config`, and `naming` modules. Support three execution paths: (1) worktree + tmux (default), (2) no-worktree mode with stash guard (`--no-worktree` flag), and (3) worktree without tmux (auto-detect `$TMUX` and fallback to new shell process). Replace the `branchlet create` call with `GitClient::worktree_add` and implement the PRD F2 stash guard for no-worktree mode.

<critical>
- ALWAYS READ the PRD and TechSpec before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — every task MUST include tests in deliverables
</critical>

<requirements>
- MUST implement `handle_create(cmd: Command::Create) -> Result<(), MatError>` in `src/commands/create.rs`
- MUST load `Config` via `Config::load()`, resolve effective `default_branch` (config > auto-detect > "main")
- MUST call `naming::generate_names()` with app_name, task_type, task_name, config, and repo_dir
- MUST support three execution paths based on flags and environment:
  1. **Worktree + TMUX** (default when `$TMUX` set and `--no-worktree` not set): create worktree via `GitClient::worktree_add`, open tmux window via `TmuxClient::new_window`, rename window, copy cd command to buffer
  2. **No-worktree mode** (`--no-worktree` flag): check for uncommitted changes; if found, `GitClient::stash_push("mat:auto:{branch}")`; create branch via `GitClient::checkout_b`; print PRD F2 disclaimer message
  3. **Worktree without TMUX** (no `$TMUX`, no `--use-tmux`): create worktree, open new shell process via `std::process::Command::new($SHELL)` with working dir set to worktree path
- MUST print the PRD-specified output messages at each step (see PRD "Primary Flow: Create" examples)
- MUST print clear disclaimer for no-worktree mode: "No-worktree mode: changes are isolated to this branch, not a separate directory. Stashed changes can be restored with: git stash pop"
- MUST handle errors at each step: if worktree_add fails, print error and return `Err`; if tmux fails but `--use-tmux` was specified, print error and stop
- MUST use `tmux.enabled` config to override auto-detection: `always` forces tmux path, `never` forces non-tmux path

## Subtasks
- [ ] 06.1 Implement worktree + TMUX path using GitClient and TmuxClient
- [ ] 06.2 Implement no-worktree path with stash guard and branch creation
- [ ] 06.3 Implement worktree without TMUX path (new shell process fallback)
- [ ] 06.4 Wire tmux detection (`$TMUX` env var) and config override (`tmux.enabled`)
- [ ] 06.5 Format all output messages to match PRD User Experience examples
- [ ] 06.6 Write unit tests for all three execution paths using MockRunner

## Implementation Details

See TechSpec "Data Flow: Create Mode" diagram for the complete sequence. See PRD "Core Features" F1 and F2 for feature specifications. See PRD "Primary Flow: Create" for exact output message formatting. The current create logic in `src/main.rs` lines 372-505 serves as a reference for the existing behavior that must be preserved (plus the new features).

### Relevant Files
- `src/commands/create.rs` — main implementation file, contains `handle_create` function
- `src/git.rs` — `GitClient` for `worktree_add`, `checkout_b`, `stash_push`, `has_uncommitted_changes` (task_02)
- `src/tmux.rs` — `TmuxClient` for `new_window`, `rename_window`, `set_buffer` (task_03)
- `src/naming.rs` — `generate_names`, `get_app_name` (task_03)
- `src/config.rs` — `Config` for `default_branch`, `worktree_root`, `tmux.enabled` (task_04)
- `src/cli.rs` — `Command::Create` variant with all flags (task_05)
- `src/display.rs` — `print_success`, `print_info`, `print_tip`, `print_error` (task_01)

### Dependent Files
- None — this is a leaf task (other tasks depend on it, but it depends on lower-level modules)

### Related ADRs
- [ADR-001: Absorb Branchlet Core Worktree Operations into Mat](../adrs/adr-001.md) — No-worktree mode and TMUX auto-detection
- [ADR-002: Lifecycle Manager Approach for Task Closing](../adrs/adr-002.md) — Context for the full lifecycle (create → close)

## Deliverables
- `src/commands/create.rs` with `handle_create` function supporting all three execution paths
- Unit tests with 80%+ coverage using MockRunner (REQUIRED)
- Integration test coverage for worktree creation flow

## Tests
- Unit tests (using MockRunner):
  - [ ] Worktree+TMUX path: `worktree_add` called with correct branch, path, source; `tmux new-window` called with worktree path
  - [ ] Worktree+TMUX path: `tmux rename-window` called with window name from naming module
  - [ ] Worktree+TMUX path: `tmux set-buffer` called with `cd <path>` command
  - [ ] No-worktree path: `stash_push` called with message `"mat:auto:feat/login"` when uncommitted changes exist
  - [ ] No-worktree path: `checkout_b` called with branch name and source branch
  - [ ] No-worktree path: disclaimer message printed to stdout
  - [ ] No-worktree path: `worktree_add` is NOT called
  - [ ] No-worktree path without uncommitted changes: `stash_push` is NOT called
  - [ ] No-TMUX path: `worktree_add` called, tmux methods NOT called
  - [ ] No-TMUX path: new shell process spawned with correct working directory
  - [ ] `tmux.enabled = "never"` config forces no-TMUX path regardless of `$TMUX`
  - [ ] `tmux.enabled = "always"` config forces TMUX path, fails if tmux not running
  - [ ] `default_branch` from config used when `--source` not provided
  - [ ] `worktree_add` failure returns `MatError::Git` and prints error message
- Test coverage target: >=80%
- All tests must pass

## Success Criteria
- All tests passing
- Test coverage >=80%
- `mat feat login` creates worktree, opens tmux window, renames it, copies cd command (identical to current behavior minus branchlet)
- `mat --no-worktree feat login` creates branch with stash guard, prints disclaimer
- `mat feat login` outside tmux opens new shell process in worktree
- All output messages match PRD "Primary Flow: Create" examples
