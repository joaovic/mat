# Task Memory: task_09.md

## Objective Snapshot

Write 15 integration tests covering create, close (no-worktree), config, error, and tmux detection scenarios using real git repos in temp directories.

## Important Decisions

- Close-from-worktree tests removed because `git checkout <source>` fails from within a worktree (source branch is checked out in main repo). No-worktree close path avoids this.
- Untracked-only files are not stashed by `git stash push` without `--include-untracked`, so no-worktree stash tests use tracked modified files.
- Global config tests use `XDG_CONFIG_HOME` per-test isolation to avoid polluting real config.

## Learnings

- `git branch -d` (and `-D`) refuses to delete the currently checked-out branch. The `close --no-merge` path hits this because it skips `checkout(source)` and tries to delete the current branch directly.
- `git worktree add` creates a worktree + branch but the main repo stays on the original branch. Switching to the source branch from within a worktree fails with "already used by worktree".
- `rtk` wrapper stores `cargo test` output in `~/.local/share/rtk/tee/` — use `--test-threads=1` for the `--test-threads=1` flag to work properly across test binaries.

## Files / Surfaces

- `tests/integration_test.rs` — new file, 15 integration tests
- `Cargo.toml` — `tempfile` already present in `[dev-dependencies]`

## Errors / Corrections

- Initial attempt: close-from-worktree tests failed because `git checkout main` fails when main is checked out in the main worktree. Fixed by switching to no-worktree close tests only.
- Initial attempt: no-worktree stash test used an untracked file (dirty.txt) but `git stash push` without `--include-untracked` ignores untracked files. Fixed by using tracked modified file.
- Unit test `test_worktree_shell_path_worktree_add_called_tmux_not_called` is flaky under parallel execution (races on `/tmp/mat_test_create`). Needs `--test-threads=1`.

## Ready for Next Run

All subtasks complete. 15 integration tests passing. Known limitation: close-from-worktree is broken (`git checkout main` fails from within worktree) — only no-worktree close is tested.
