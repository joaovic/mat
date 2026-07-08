# Task Memory: task_08.md

Keep only task-local execution context here. Do not duplicate facts that are obvious from the repository, task file, PRD documents, or git history.

## Objective Snapshot

Remove all remaining branchlet-related dependencies from the project.

## Important Decisions

- `dirs` crate must remain — still used by `src/config.rs` (lines 315, 390) for `dirs::config_dir()`.
- `serde_json` was not used in any Rust source code — safe to remove.
- All branchlet-related functions (BRANCHLET_SETTINGS, check_branchlet_config, get_branchlet_settings_path, run_prerequisite_checks, check_command_exists) were already removed by tasks 06/07.

## Learnings

- Test `test_tmux_enabled_never_forces_no_tmux` is flaky under parallel execution due to shared hardcoded temp dir `mat_test_create` between two tests. It passes when run in isolation or with `--test-threads=1`.
- `rtk` is a cargo wrapper that transforms output; use raw `/home/joao/.cargo/bin/cargo` for direct output.

## Files / Surfaces

- `Cargo.toml` — removed `serde_json = "1.0"` from `[dependencies]`
- `src/main.rs` — no changes needed, already clean
- `README.md` — still references branchlet in requirements (out of scope for this task)

## Errors / Corrections

- None.

## Ready for Next Run

All changes complete and verified.
