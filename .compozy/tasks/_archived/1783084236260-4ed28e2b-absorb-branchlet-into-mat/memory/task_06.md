# Task Memory: task_06.md

## Objective Snapshot
- Implement `handle_create` in `src/commands/create.rs` with 3 execution paths
- Wire into `main.rs` dispatch
- Comprehensive unit tests with MockRunner

## Important Decisions
- `handle_create` accepts `app_name: &str` and `repo_dir: &Path` as parameters for testability, rather than calling `naming::get_app_name()` and `env::current_dir()` internally
- Used `TmuxMode::Always` in tmux path tests to force tmux path regardless of `$TMUX` env (not set in test env)
- Shell path tests create temp directories and set `$SHELL=true` to avoid hanging on real shell spawn
- Used IIFE closure in main.rs for `Create` arm to allow `?` operator inside `fn main()`

## Learnings
- `impl CommandRunner` in function signatures allows different runner types (RealRunner vs MockRunner) for production vs test
- Added `#[derive(Clone)]` to MockRunner to allow sharing responses between GitClient and TmuxClient in tests
- Temp directories must match `generate_names` convention: `{repo_dir}.worktree/{app}-{type}/{name}`
- `print_warning` added to display.rs for the no-worktree disclaimer message

## Files / Surfaces
- `src/commands/create.rs` — new implementation (512 lines)
- `src/display.rs` — added `print_warning` function
- `src/git.rs` — added `#[derive(Clone)]` to `MockRunner`
- `src/main.rs` — updated dispatch to use `commands::create::handle_create`, removed old `handle_create_mode` and `get_app_name`

## Errors / Corrections
- Initial test failures: `handle_create` called `env::current_dir()` internally, making tests environment-dependent. Fixed by passing `app_name` and `repo_dir` as parameters.
- Shell path tests failed to create dirs at `/` filesystem root. Fixed by using temp directories.
- `?` operator not allowed in `fn main()`. Fixed by wrapping create block in IIFE closure.

## Ready for Next Run
- Task 07: Close command rewrite — will similarly rewrite `handle_close_mode` and clean up remaining old functions in main.rs
