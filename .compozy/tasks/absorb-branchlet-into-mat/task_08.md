---
status: pending
title: Remove branchlet dependency
type: chore
complexity: low
dependencies:
  - task_06
  - task_07
---

# Task 08: Remove branchlet dependency

## Overview
Remove all remaining branchlet-related code, constants, and dependencies from the project. Delete `BRANCHLET_SETTINGS` constant, remove `dirs` and `serde_json` crates from `Cargo.toml` (no longer needed after replacing branchlet JSON output with git porcelain text parsing), and remove `check_branchlet_config` and `check_command_exists("branchlet")` from prerequisite checks. Verify the binary compiles and runs without `branchlet` installed.

<critical>
- ALWAYS READ the PRD and TechSpec before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — every task MUST include tests in deliverables
</critical>

<requirements>
- MUST remove `BRANCHLET_SETTINGS` constant from codebase
- MUST remove `check_branchlet_config` function
- MUST remove `check_command_exists("branchlet")` from prerequisite checks
- MUST remove `get_branchlet_settings_path` function
- MUST remove `dirs` crate from `Cargo.toml` dependencies (was only used for `~/.branchlet/settings.json` path — config now uses `dirs::config_dir()` from `dirs` crate still needed — verify before removing)
- MUST remove `serde_json` crate from `Cargo.toml` dependencies (was only used for `branchlet list --json` parsing — replaced by `git worktree list --porcelain` text parsing)
- MUST update all prerequisite check functions to remove branchlet references
- MUST update `run_prerequisite_checks` to only check: tmux running (if applicable), git repo
- MUST NOT introduce any new behavior — only remove dead code and dependencies
- MUST verify `cargo build` succeeds after removal
- MUST verify `cargo run -- feat test-task` works without `branchlet` installed in PATH

## Subtasks
- [ ] 08.1 Remove `BRANCHLET_SETTINGS` constant and all references
- [ ] 08.2 Remove `check_branchlet_config` and `get_branchlet_settings_path` functions
- [ ] 08.3 Remove `check_command_exists("branchlet")` from prerequisite checks
- [ ] 08.4 Remove `dirs` crate from `Cargo.toml` (verify it's no longer needed — config may still use `dirs::config_dir()`)
- [ ] 08.5 Remove `serde_json` crate from `Cargo.toml`
- [ ] 08.6 Run `cargo build` and verify compilation
- [ ] 08.7 Run existing tests to confirm no regressions

## Implementation Details

The branchlet dependency was used exclusively in three places: `get_worktree_info` (lines 122-161), `delete_worktree` (lines 163-175), and `handle_create_mode` line 409. All three have been replaced by task_06 and task_07 using `GitClient`. This task only removes the leftover constants, imports, and prerequisite checks.

### Relevant Files
- `src/main.rs` — remove branchlet-related prerequisite calls, update `run_prerequisite_checks`
- `src/git.rs` — no changes needed (already implements worktree ops from task_02)
- `Cargo.toml` — remove `dirs` and `serde_json` from `[dependencies]`

### Dependent Files
- None — this is the cleanup task after all rewrites

### Related ADRs
- [ADR-001: Absorb Branchlet Core Worktree Operations into Mat](../adrs/adr-001.md) — Decision to replace branchlet with direct git commands

## Deliverables
- Updated `src/main.rs` without branchlet prerequisite checks
- Updated `Cargo.toml` without `dirs` and `serde_json` dependencies
- Successful `cargo build`

## Tests
- Unit tests:
  - [ ] `run_prerequisite_checks` no longer references branchlet
  - [ ] `run_prerequisite_checks` still verifies git repo is present
- Integration tests:
  - [ ] `cargo build` succeeds without `serde_json` in dependencies
  - [ ] `cargo build` succeeds without `dirs` in dependencies
  - [ ] Binary runs `mat feat test-task` without branchlet installed
- Test coverage target: >=80%
- All tests must pass

## Success Criteria
- All tests passing
- `cargo build` succeeds
- `cargo run -- feat test-task` works with no `branchlet` in PATH
- Zero references to `branchlet` in any source file
- `Cargo.toml` has no `dirs` or `serde_json` dependencies
