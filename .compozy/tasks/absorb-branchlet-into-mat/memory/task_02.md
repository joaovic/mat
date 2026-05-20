# Task Memory: task_02.md

Keep only task-local execution context here. Do not duplicate facts that are obvious from the repository, task file, PRD documents, or git history.

## Objective Snapshot

Completed implementation of CommandRunner trait, RealRunner, MockRunner, GitClient<R>, WorktreeInfo, MergeStrategy in src/git.rs with 33 unit tests.

## Important Decisions

- MockRunner stores `Result<CommandOutput, MatError>` in HashMap, with `add_response()` for Ok and `add_error()` for Err — enables testing both success and error paths
- `MatError::Io` wraps `Arc<std::io::Error>` instead of `std::io::Error` to allow Clone derive (needed by MockRunner)
- `worktree_list()` determines `is_main` by running `git rev-parse --show-toplevel` and comparing paths
- `stash_push()` prepends `mat:auto:` to the message argument before passing `-m` flag

## Learnings

- `MatError` needs `Debug` derive for `Result::unwrap()` in tests
- `std::io::Error` does not implement `Clone`, so `Arc` wrapping is needed when `MatError` derives Clone
- `Arc<io::Error>::as_ref()` returns `&io::Error` for Display formatting
- MockRunner's `add_response` vs `add_error` distinction is important: RealRunner converts non-zero exit to Err, MockRunner returned whatever was stored originally

## Files / Surfaces

- `src/git.rs` — new 790-line file with all implementations
- `src/error.rs` — added Debug and Clone derives, wrapped Io variant in Arc

## Errors / Corrections

- First compile failed: `MatError` missing `Debug` derive (needed for `unwrap()` in tests)
- Second compile failed: `MatError` derives Clone but `io::Error` is not Clone — fixed with `Arc<io::Error>`
- `test_is_repo_returns_false_when_not_a_repo`: MockRunner returned `Ok(CommandOutput{status:1})` instead of `Err(MatError::Git)` — fixed by switching from `add_response` to `add_error`
- `test_is_repo_propagates_io_error`: needed non-Git error injected via `add_error` with `MatError::Io(Arc::new(...))`

## Ready for Next Run

All 33 git tests pass. Zero clippy warnings. Ready for handoff to task_03.
