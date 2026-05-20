# Workflow Memory

## Current State

- Module structure established: cli, commands (create/close/config), config, display, error, git, tmux, naming
- `MatError` enum live in `src/error.rs` with `Git`, `Tmux`, `Config`, `Validation`, `Io` variants
- All `Result<T, String>` converted to `Result<T, MatError>` chain
- Single `process::exit(1)` in `main()` only
- 186 unit tests + 15 integration tests pass (201 total)
- Integration tests in `tests/integration_test.rs` cover create (worktree/no-worktree), close (no-worktree only), config, error, tmux detection
- `print_warning` added to display.rs for no-worktree disclaimer
- `MatError` derives `Debug` and `Clone`; `Io` variant wraps `Arc<std::io::Error>` instead of bare `std::io::Error`
- `GitClient<R>` fully implemented with all 12 methods, `CommandRunner` trait, `RealRunner`, `MockRunner`, `MergeStrategy`, `WorktreeInfo`
- `TmuxClient<R>` fully implemented with all 10 tmux methods + close_current_window in src/tmux.rs
- `Names` struct, `get_app_name`, `generate_names` implemented in src/naming.rs
- `Config` fully implemented with two-tier TOML loading, merge logic, source tracking, `mat config list/get/set` command handlers
- `src/commands/config.rs` fully implemented
- `serde_json` removed from Cargo.toml (no longer needed); `dirs` kept (still needed for `config.rs`)

## Shared Decisions

- MockRunner stores `Result<CommandOutput, MatError>` to support both success and error responses via `add_response`/`add_error` methods
- `MatError::Io` wraps `Arc<std::io::Error>` to allow `Clone` derive (required by MockRunner)
- TmuxClient uses internal `run_tmux` helper to convert `MatError::Git` → `MatError::Tmux`, because RealRunner always returns `MatError::Git` regardless of program
- Naming module convention: `branch_name = {type}/{name}`, `worktree_name = {app}-{type}/{name}`, `window_name = {app}-{type}/{name}` (includes task_type per PRD F1)
- `MergeStrategy` moved from `src/git.rs` to `src/config.rs` with serde Deserialize — `git.rs` imports from `config.rs`. Tasks 05-09 should import from `crate::config::MergeStrategy`, not from git.
- Command handlers should accept environment-dependent values (app_name, repo_dir, config) as explicit parameters for testability, rather than calling `naming::get_app_name()` / `env::current_dir()` / `Config::load()` internally. Callers in `main.rs` resolve these and pass them in. Use an IIFE closure in `fn main()` match arms to allow `?` operator.
- `MockRunner` has `#[derive(Clone)]` — clone it when both `GitClient` and `TmuxClient` need the same mock responses in a single test.

## Shared Learnings

- `clap::Parser::try_parse_from(["mat", "feat", "login"])` enables CLI parsing unit tests without actual args
- `env::current_dir()?` works with `MatError` via `impl From<std::io::Error> for MatError` — no manual mapping needed for io errors
- Empty (0-byte) `.rs` files are valid Rust modules — sufficient for placeholders
- Config uses two-type approach (RawConfig for parsing, Config for resolved values) with `HashMap<String, Source>` for source tracking. Config struct has `pub sources`, `pub global_path`, `pub project_path` fields.
- Test `test_tmux_enabled_never_forces_no_tmux` is flaky under parallel execution (race on shared `mat_test_create` temp dir). Use `--test-threads=1` for reliable results.
- `handle_close` calls `git checkout <source>` from the worktree, which fails because source is checked out in main repo ("already used by worktree"). Close-from-worktree is broken; only no-worktree close works.
- `close --no-merge` in no-worktree mode tries `git branch -d` on the currently checked-out branch without switching away first — git refuses. Only auto-merge (which calls `checkout(source)` first) can delete the branch.
- `git stash push` without `--include-untracked` only stashes tracked modified files. Untracked files are silently ignored by the create flow's stash guard.

## Open Risks

- README.md still references branchlet in requirements — should be updated when docs are revised
- test_tmux_enabled_never_forces_no_tmux and test_worktree_shell_path_worktree_add_called_tmux_not_called share a hardcoded temp dir path; only one can run at a time
- Close-from-worktree is broken: `handle_close` calls `git checkout main` which fails because main is checked out in the main worktree. Fix requires running merge from the main repo dir, not the worktree.
- No-worktree `close --no-merge` can't delete the feature branch because it's still checked out — `handle_close` should switch to the source branch before `branch_delete`.

## Handoffs

- Task 02 complete: `CommandRunner`, `RealRunner`, `MockRunner`, `GitClient<R>`, `MergeStrategy`, `WorktreeInfo` all in `src/git.rs` with 33 tests
- Task 03 complete: `TmuxClient<R>` in `src/tmux.rs` (23 tests), `Names`+`generate_names` in `src/naming.rs` (13 tests)
- Task 04 complete: `Config` struct + load/merge/set in `src/config.rs`, handlers in `src/commands/config.rs`, 50 config tests.
- Task 05 complete: CLI restructured to clap subcommands with `Command` enum, 19 CLI unit tests. `--close`/`-c` kept as deprecated alias.
- Task 06 complete: `handle_create` in `src/commands/create.rs` with 3 execution paths, 18 unit tests. `print_warning` added to display.rs. MockRunner now derives Clone. Old `handle_create_mode` removed from main.rs.
- Task 07 complete: `handle_close` in `src/commands/close.rs` with auto-merge, --no-merge, stash restore for no-worktree mode, 17 unit tests. Old helpers purged from main.rs.
- Task 08 complete: `serde_json` removed from Cargo.toml. All branchlet references in Rust source code eliminated.
- Task 09 complete: `tests/integration_test.rs` with 15 integration tests covering create, close (no-worktree), config, error, and tmux detection. Tempfile already in dev-deps. Run with `--test-threads=1` for reliability.
