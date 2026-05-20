# Task Memory: task_03.md

## Objective Snapshot

- Implement TmuxClient<R> in src/tmux.rs with all 10 tmux methods + close_current_window
- Implement naming module in src/naming.rs with Names struct, get_app_name, generate_names
- 80%+ test coverage for naming module

## Decisions

- TmuxClient uses internal `run_tmux` helper that converts MatError::Git -> MatError::Tmux, since RealRunner always returns MatError::Git regardless of program
- `is_running` calls runner.run("tmux", ...) directly (not through run_tmux) to catch all errors as Ok(false)
- `new_window` appends `-P -F "#{window_index}"` to parse the new window index from output
- `close_current_window` finds the first non-current window for switching (instead of hardcoded "0"/"1") to be more robust
- Naming module defines minimal Config struct in config.rs with just worktree_root field (task_04 will expand it)
- `generate_names` uses template substitution with {app}, {type}, {name} for custom worktree_root, or falls back to {repo_dir}.worktree/{worktree_name}/

## Learnings

- MockRunner responses can be reused by both GitClient and TmuxClient tests without modification
- The `Err(_)` catch-all in `run_tmux`'s match arm correctly passes through MatError::Io errors

## Files / Surfaces

- src/config.rs: Added minimal Config struct with worktree_root field
- src/tmux.rs: Full TmuxClient<R> implementation with 23 unit tests
- src/naming.rs: Names struct, get_app_name, generate_names with 13 unit tests

## Errors / Corrections

- None encountered; all 87 tests pass (33 git + 7 error + 13 naming + 23 tmux + 6 display + 5 cli)
- No warnings from cargo check

## Ready for Next Run

- All deliverables complete: src/tmux.rs, src/naming.rs, unit tests for both
- 13 naming tests cover all required test cases from task spec
- Config struct stubbed minimally — task_04 will expand it
