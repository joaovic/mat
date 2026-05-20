---
status: completed
title: Integration tests
type: test
complexity: medium
dependencies:
    - task_06
    - task_07
    - task_08
---

# Task 09: Integration tests

## Overview
Write end-to-end integration tests that create temporary git repositories with real `git` commands and exercise the full `mat` workflow: create a task (worktree + tmux path), close it (auto-merge + cleanup), no-worktree mode (stash guard + branch creation), config commands, and error scenarios. These tests validate that all modules work together correctly with real git operations.

<critical>
- ALWAYS READ the PRD and TechSpec before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — every task MUST include tests in deliverables
</critical>

<requirements>
- MUST create `tests/integration_test.rs` with integration tests using real git repositories in temporary directories
- MUST use `tempfile` crate (add to `[dev-dependencies]`) for creating temp directories and git repos
- MUST NOT mock git or tmux in integration tests — use real `git` binary for git operations
- MUST set up test fixtures: create git repo, make initial commits, create a branch, run mat create, verify worktree exists
- MUST test the full create→work→close lifecycle: create worktree, commit a change in it, close with auto-merge, verify merge occurred
- MUST test no-worktree mode: create branch with stash guard, verify branch exists, verify stash was pushed
- MUST test config commands: `mat config set`, `mat config get`, `mat config list` output
- MUST test error scenarios: close with uncommitted changes, close with merge conflicts, create outside git repo
- MUST test tmux detection: verify behavior when `$TMUX` is set vs unset (use `std::env::remove_var("TMUX")` in tests)
- SHOULD test the updated naming convention: verify `mat feat login` and `mat fix login` produce different worktree names

## Subtasks
- [ ] 09.1 Add `tempfile` to `[dev-dependencies]` in `Cargo.toml`
- [ ] 09.2 Create `tests/integration_test.rs` with helper functions: `create_temp_git_repo()`, `make_commit(repo, msg)`, `run_mat(args)`
- [ ] 09.3 Write create flow integration tests: worktree + tmux, no-worktree, worktree without tmux
- [ ] 09.4 Write close flow integration tests: auto-merge success, merge conflict, `--no-merge`, branch deletion
- [ ] 09.5 Write config command integration tests: set, get, list
- [ ] 09.6 Write error scenario tests: uncommitted changes on close, merge conflicts, missing git repo
- [ ] 09.7 Write naming convention tests: collision prevention with different task types

## Implementation Details

See TechSpec "Testing Approach - Integration Tests" for the test scenarios. See PRD "User Experience" for expected output messages. Integration tests use `std::process::Command` to run the compiled `mat` binary against real git repos in temp directories.

### Relevant Files
- `tests/integration_test.rs` — new file, all integration tests
- `Cargo.toml` — add `tempfile` to `[dev-dependencies]`
- `src/main.rs` — the binary under test

### Dependent Files
- None — this is the final task

### Related ADRs
- [ADR-001: Absorb Branchlet Core Worktree Operations into Mat](../adrs/adr-001.md) — Core feature scope to test
- [ADR-002: Lifecycle Manager Approach for Task Closing](../adrs/adr-002.md) — Auto-merge behavior to test
- [ADR-003: Module Architecture and Error Handling Strategy](../adrs/adr-003.md) — CommandRunner trait enables unit tests; integration tests cover full shell-out

## Deliverables
- `tests/integration_test.rs` with integration tests covering all major workflows
- Updated `Cargo.toml` with `tempfile` dev-dependency
- All integration tests passing

## Tests
- Integration tests:
  - [ ] Create worktree: `mat feat test-feature` creates worktree directory, branch `feat/test-feature` exists
  - [ ] Create worktree naming: `mat feat login` and `mat fix login` produce different worktree directory names
  - [ ] Create no-worktree: `mat --no-worktree fix test-bug` creates branch `fix/test-bug`, does NOT create worktree
  - [ ] Create no-worktree with uncommitted changes: stash pushed before branch switch
  - [ ] Close auto-merge: create worktree, commit change, `mat close` merges into base branch
  - [ ] Close auto-merge deletes branch: `mat close` with `delete_branch=true` removes feature branch
  - [ ] Close with merge conflict: create conflicting changes in worktree and base, `mat close` exits with conflict error
  - [ ] Close --no-merge: `mat close --no-merge` deletes worktree without merging
  - [ ] Close outside worktree: `mat close` in non-worktree directory fails with descriptive error
  - [ ] Close with uncommitted changes: `mat close` with dirty worktree exits with error
  - [ ] Config set and get: `mat config set merge_strategy fast-forward` followed by `mat config get merge_strategy` returns `fast-forward`
  - [ ] Config list: `mat config list` shows effective values with source annotations
  - [ ] TMUX detection: when `$TMUX` unset, mat opens new shell process not tmux window
  - [ ] Missing git repo: `mat feat login` outside git repo returns error
- All tests must pass

## Success Criteria
- All integration tests passing
- `cargo test` runs all unit + integration tests successfully
- Full create→work→close lifecycle works end-to-end with real git repos
- Merge conflict detection works correctly
- No-worktree stash guard preserves and restores uncommitted changes
- Config commands read and write TOML correctly
