---
status: pending
title: CommandRunner trait and GitClient
type: backend
complexity: high
dependencies:
  - task_01
---

# Task 02: CommandRunner trait and GitClient

## Overview
Implement the `CommandRunner` trait with `RealRunner` and `MockRunner` implementations to enable testability of all external command invocations. Build `GitClient<R>` with all git operations: worktree CRUD (`git worktree add/list/remove`), branch management (`checkout`, `checkout -b`, `merge`, `branch -d`), stash operations (`push`, `pop`), and repository queries (`is_repo`, `current_branch`, `default_branch`, `has_uncommitted_changes`).

<critical>
- ALWAYS READ the PRD and TechSpec before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — every task MUST include tests in deliverables
</critical>

<requirements>
- MUST define `CommandRunner` trait with a single `run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, MatError>` method as specified in TechSpec "Core Interfaces"
- MUST implement `CommandOutput` struct with `stdout: String`, `stderr: String`, `status: i32` fields
- MUST implement `RealRunner` that shells out via `std::process::Command`, capturing stdout/stderr, returning `MatError::Git` on non-zero exit
- MUST implement `MockRunner` with `HashMap<String, CommandOutput>` for canned responses, enabling unit tests to inject mock git/tmux output
- MUST implement `GitClient<R: CommandRunner>` with all methods: `worktree_add`, `worktree_list`, `worktree_remove`, `checkout`, `checkout_b`, `merge`, `branch_delete`, `stash_push`, `stash_pop`, `is_repo`, `current_branch`, `default_branch`, `has_uncommitted_changes`
- `worktree_add` MUST invoke `git worktree add -b <branch> <path> <source>` and return the worktree path on success
- `worktree_list` MUST invoke `git worktree list --porcelain` and parse output into `Vec<WorktreeInfo>` (see TechSpec "Data Models")
- `merge` MUST accept `MergeStrategy` enum and pass `--no-ff` for MergeCommit or `--ff-only` for FastForward
- `stash_push` MUST use named stashes with `mat:auto:` prefix message
- `branch_delete` MUST use `git branch -d` (safe delete) for normal case
- MUST NOT use `git2` crate — all operations shell out to `git` binary

## Subtasks
- [ ] 02.1 Define `CommandRunner` trait, `CommandOutput` struct, `RealRunner`, and `MockRunner` in `src/git.rs`
- [ ] 02.2 Implement `GitClient<R>` struct with `CommandRunner` as a generic parameter
- [ ] 02.3 Implement repository query methods: `is_repo`, `current_branch`, `default_branch`, `has_uncommitted_changes`
- [ ] 02.4 Implement worktree CRUD: `worktree_add`, `worktree_list`, `worktree_remove`
- [ ] 02.5 Implement branch management: `checkout`, `checkout_b`, `merge`, `branch_delete`
- [ ] 02.6 Implement stash operations: `stash_push` (with `mat:auto:` prefix), `stash_pop`
- [ ] 02.7 Write unit tests for all GitClient methods using MockRunner

## Implementation Details

See TechSpec "Core Interfaces" section for the `GitClient` method signatures and `CommandRunner` trait definition. See TechSpec "Data Models" for `WorktreeInfo` struct.

### Relevant Files
- `src/git.rs` — new file, contains `CommandRunner` trait, `RealRunner`, `MockRunner`, `GitClient`, and `WorktreeInfo`
- `src/error.rs` — `MatError::Git` variant used for all git errors (created in task_01)
- `Cargo.toml` — no new dependencies needed; git ops use `std::process::Command`

### Dependent Files
- `src/commands/create.rs` — will use `GitClient::worktree_add`, `checkout_b`, `stash_push` (task_06)
- `src/commands/close.rs` — will use `GitClient::worktree_list`, `worktree_remove`, `checkout`, `merge`, `branch_delete` (task_07)

### Related ADRs
- [ADR-001: Absorb Branchlet Core Worktree Operations into Mat](../adrs/adr-001.md) — Decision to replace branchlet with direct git worktree commands
- [ADR-003: Module Architecture and Error Handling Strategy](../adrs/adr-003.md) — Decision to use CommandRunner trait for testability

## Deliverables
- `src/git.rs` with `CommandRunner` trait, `RealRunner`, `MockRunner`, `GitClient<R>`, `WorktreeInfo`
- Unit tests with 80%+ coverage for all GitClient methods (REQUIRED)
- MockRunner tests verifying correct git command construction and argument passing

## Tests
- Unit tests:
  - [ ] `GitClient::is_repo` returns true when `git rev-parse --git-dir` succeeds
  - [ ] `GitClient::current_branch` parses branch name from `git branch --show-current` stdout
  - [ ] `GitClient::has_uncommitted_changes` returns true when `git status --porcelain` has output
  - [ ] `GitClient::worktree_add` constructs correct args: `-b <branch> <path> <source>`
  - [ ] `GitClient::worktree_list` parses porcelain output into `Vec<WorktreeInfo>` correctly
  - [ ] `GitClient::worktree_remove` invokes `git worktree remove <path>`
  - [ ] `GitClient::merge` with `MergeStrategy::MergeCommit` passes `--no-ff` flag
  - [ ] `GitClient::merge` with `MergeStrategy::FastForward` passes `--ff-only` flag
  - [ ] `GitClient::stash_push` uses `-m "mat:auto:<branch>"` message format
  - [ ] `GitClient::branch_delete` uses `-d` flag for safe delete
  - [ ] `MockRunner` returns canned stdout/stderr/status for configured program+args combinations
  - [ ] All git methods return `MatError::Git` on non-zero exit status
- Test coverage target: >=80%
- All tests must pass

## Success Criteria
- All tests passing
- Test coverage >=80%
- `GitClient` compiles and all methods are callable
- MockRunner correctly simulates git command output for testing
- No `branchlet` commands invoked — all operations use `git` binary
