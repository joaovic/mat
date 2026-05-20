---
status: pending
title: TmuxClient and naming module
type: backend
complexity: medium
dependencies:
  - task_01
---

# Task 03: TmuxClient and naming module

## Overview
Implement `TmuxClient<R>` with all tmux operations (new-window, rename-window, list-windows, select-window, kill-window, set-buffer, display-message, get-prefix, is-running) and the `naming` module with `generate_names()` function that produces the four names needed for both create and close workflows. The naming module must use the updated convention from the PRD: worktree name includes task_type to prevent collisions.

<critical>
- ALWAYS READ the PRD and TechSpec before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — every task MUST include tests in deliverables
</critical>

<requirements>
- MUST implement `TmuxClient<R: CommandRunner>` with methods: `new_window`, `rename_window`, `list_windows`, `select_window`, `kill_window`, `set_buffer`, `display_message`, `get_prefix`, `is_running`, `current_window_index`
- All tmux methods MUST use `CommandRunner` (not direct `std::process::Command`) for testability
- `new_window` MUST invoke `tmux new-window -c <path>` and return the new window index
- `rename_window` MUST invoke `tmux rename-window <name>`
- `close_current_window` MUST list windows, get current index, switch to another window, then kill the current window (per current logic in lines 177-215)
- MUST implement `naming::generate_names(app_name, task_type, task_name, config, repo_dir) -> Names` returning `Names { branch_name, worktree_name, window_name, worktree_path }`
- Worktree name MUST follow updated convention: `{app}-{type}/{name}` (includes task_type, per PRD F1)
- Branch name MUST follow convention: `{type}/{name}`
- Window name MUST follow convention: `{app}-{type}/{name}`
- `worktree_path` MUST default to `{repo_dir}.worktree/{worktree_name}/` when `config.worktree_root` is not set
- `worktree_path` MUST support `{app}`, `{type}`, `{name}` template variables when `config.worktree_root` is configured
- `get_app_name` logic (basename of CWD) MUST be included in naming module

## Subtasks
- [ ] 03.1 Implement `TmuxClient<R>` struct with `CommandRunner` generic parameter in `src/tmux.rs`
- [ ] 03.2 Implement window management methods: `new_window`, `rename_window`, `list_windows`, `select_window`, `kill_window`, `current_window_index`
- [ ] 03.3 Implement utility methods: `set_buffer`, `display_message`, `get_prefix`, `is_running`
- [ ] 03.4 Implement `Names` struct and `get_app_name` helper in `src/naming.rs`
- [ ] 03.5 Implement `generate_names` with config-driven worktree path resolution and template variable substitution
- [ ] 03.6 Write unit tests for naming module and TmuxClient using MockRunner

## Implementation Details

See TechSpec "Core Interfaces" for `Names` struct and `generate_names` signature. See TechSpec "Worktree Path Resolution" for template variable format. The current tmux logic to port is in `src/main.rs` lines 55-61, 94-110, 177-229, 334-335, 438-455, 457-474.

### Relevant Files
- `src/tmux.rs` — new file, contains `TmuxClient<R>` and all tmux command wrappers
- `src/naming.rs` — new file, contains `Names` struct, `get_app_name`, `generate_names`
- `src/git.rs` — `CommandRunner` trait, `RealRunner`, `MockRunner` (from task_02)
- `src/config.rs` — `Config::worktree_root` field used for path resolution (from task_04)

### Dependent Files
- `src/commands/create.rs` — will use `TmuxClient::new_window`, `rename_window`, `set_buffer` and `naming::generate_names` (task_06)
- `src/commands/close.rs` — will use `TmuxClient::close_current_window`, `set_buffer`, `display_message`, `get_prefix` (task_07)

### Related ADRs
- [ADR-001: Absorb Branchlet Core Worktree Operations into Mat](../adrs/adr-001.md) — TMUX auto-detection and updated naming convention
- [ADR-004: CLI Subcommand Structure and Config Management](../adrs/adr-004.md) — Worktree path template support

## Deliverables
- `src/tmux.rs` with `TmuxClient<R>` and all tmux command wrappers
- `src/naming.rs` with `Names` struct, `get_app_name`, `generate_names`
- Unit tests with 80%+ coverage for naming module (REQUIRED)
- Unit tests for TmuxClient methods using MockRunner (REQUIRED)

## Tests
- Unit tests:
  - [ ] `generate_names` produces worktree_name `"dashboard-feat/login"` for app="dashboard", type="feat", name="login"
  - [ ] `generate_names` produces branch_name `"feat/login"` for type="feat", name="login"
  - [ ] `generate_names` produces window_name `"dashboard-feat/login"`
  - [ ] Different task_type produces different worktree_name (collision prevention)
  - [ ] `worktree_path` defaults to `{repo_dir}.worktree/{worktree_name}/` when no config set
  - [ ] `worktree_path` substitutes `{app}`, `{type}`, `{name}` template variables from config
  - [ ] `get_app_name` extracts directory basename from CWD
  - [ ] `TmuxClient::new_window` constructs correct args: `-c <path>`
  - [ ] `TmuxClient::rename_window` passes window name correctly
  - [ ] `TmuxClient::close_current_window` switches before killing on multi-window session
  - [ ] `TmuxClient::get_prefix` parses `C-b` from `tmux show-options -g prefix` output
  - [ ] All tmux methods return `MatError::Tmux` on non-zero exit status
- Test coverage target: >=80%
- All tests must pass

## Success Criteria
- All tests passing
- Test coverage >=80% for naming module
- Naming module prevents worktree name collisions (different types produce different names)
- TmuxClient correctly wraps all 8 tmux subcommands from the current codebase
- `generate_names` returns unique worktree names for `mat feat login` and `mat fix login`
