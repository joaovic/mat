# Task Memory: task_07.md

## Objective Snapshot
Rewrite `handle_close` to use GitClient, TmuxClient, Config with auto-merge, conflict handling, --no-merge, and no-worktree support.

## Important Decisions
- `do_merge` is a separate helper fn that handles checkout + merge + conflict detection/abort in one step
- Source branch for merge target is determined by `git.default_branch()` → `config.default_branch`
- No-worktree close uses `stash_pop("mat:auto:{branch}")` directly; stash-not-found errors are non-fatal (continues), but conflict errors are fatal
- `MergeStrategy` is imported from `crate::config::MergeStrategy` (not from git.rs re-export)
- `abort_merge()` added to `GitClient` to support `git merge --abort` after conflict

## Learnings
- `MergeStrategy` doesn't implement `Copy`, so `.clone()` is needed when passing to `git.merge()`
- Worktree matching via `current_dir.starts_with(&wt.path)` reliably identifies the current worktree
- Conflict file parsing: stderr lines containing "CONFLICT" have format `CONFLICT (type): Merge conflict in <file>`

## Files / Surfaces
- `src/commands/close.rs` — complete rewrite with `handle_close` and `do_merge` + 17 unit tests
- `src/git.rs` — added `abort_merge()` method to `GitClient`
- `src/main.rs` — Close dispatch now uses commands::close::handle_close with proper dependency injection; removed handle_close_mode + all old helper functions (run_prerequisite_checks, get_worktree_info, delete_worktree, close_current_tmux_window, etc.)

## Errors / Corrections
- Compile error: `MergeStrategy` was imported from `crate::git` but it's a re-export; fixed to import from `crate::config`
- Compile error: `*strategy` move out of reference; fixed with `.clone()`
- Warnings: unused imports `TmuxConfig`, `WorktreeInfo`, unused var `cwd` — all cleaned up

## Ready for Next Run
- Task 07 complete: 186 total unit tests pass (17 new close tests)
- Dead code removed from main.rs (~250 lines of old handle_close_mode + helpers)
