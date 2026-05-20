---
status: pending
title: Config system with serde+toml
type: backend
complexity: medium
dependencies:
  - task_01
---

# Task 04: Config system with serde+toml

## Overview
Implement two-tier TOML configuration: global `~/.config/mat/config.toml` and project-local `.mat.toml`. Build `Config` struct with `serde::Deserialize`, manual merge logic (project overrides global), and `mat config list/get/set` CLI subcommands. Add `serde` (with `derive` feature) and `toml` crates to `Cargo.toml`.

<critical>
- ALWAYS READ the PRD and TechSpec before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — every task MUST include tests in deliverables
</critical>

<requirements>
- MUST add `serde = { version = "1", features = ["derive"] }` and `toml = "0.8"` to `Cargo.toml` dependencies
- MUST define `Config` struct with `#[derive(Debug, Deserialize, Default)]` containing fields: `default_branch: Option<String>`, `delete_branch: bool`, `merge_strategy: MergeStrategy`, `worktree_root: Option<String>`, `tmux: TmuxConfig` as specified in TechSpec "Core Interfaces"
- MUST define `MergeStrategy` enum with `MergeCommit` and `FastForward` variants, deserializing from `"merge-commit"` and `"fast-forward"` strings
- MUST define `TmuxConfig` struct with `enabled: TmuxMode` and `TmuxMode` enum with `Auto`, `Always`, `Never` variants, deserializing from `"auto"`, `"always"`, `"never"`
- `Config::load()` MUST load global config from `~/.config/mat/config.toml` using `dirs::config_dir()` for XDG resolution
- `Config::load()` MUST load project config from `.mat.toml` in the git repo root (detected via `git rev-parse --show-toplevel`)
- Merge MUST follow: project config overrides global config; unset fields retain global defaults; CLI flags override both (handled in task_05)
- `Config::default_branch` MUST default to hardcoded `"main"` when neither config sets it
- `Config::delete_branch` MUST default to `true`
- `Config::merge_strategy` MUST default to `MergeStrategy::MergeCommit`
- `Config::tmux.enabled` MUST default to `TmuxMode::Auto`
- MUST implement `Config::set(key, value, global)` that writes to the appropriate TOML file
- MUST implement `mat config list` showing all effective values with source annotations (e.g., `"(project: .mat.toml)"` or `"(default)"`)
- MUST implement `mat config get <key>` showing a single value with its source
- `Config::set` MUST support `--global` flag to write to global config instead of project

## Subtasks
- [ ] 04.1 Add `serde` and `toml` crates to `Cargo.toml` dependencies
- [ ] 04.2 Define `Config`, `MergeStrategy`, `TmuxConfig`, `TmuxMode` types with serde derives in `src/config.rs`
- [ ] 04.3 Implement `Config::load()` with two-tier file loading and merge logic
- [ ] 04.4 Implement `Config::set()` for writing single key changes to TOML files
- [ ] 04.5 Implement `effective_value()` that returns value with source annotation
- [ ] 04.6 Implement `commands/config.rs` handlers for `list`, `get`, `set` subcommands
- [ ] 04.7 Write unit tests for config loading, merging, defaults, and set operations

## Implementation Details

See TechSpec "Core Interfaces" for `Config` struct, `MergeStrategy` enum, and `Config::load/set` signatures. See TechSpec "Config File Format" for the TOML structure. See PRD F3 for config key descriptions and CLI command behavior.

### Relevant Files
- `src/config.rs` — new file, contains `Config` struct, `MergeStrategy`, `TmuxConfig`, `TmuxMode`, loading/merging/setting logic
- `src/commands/config.rs` — new file, contains `handle_config_list`, `handle_config_get`, `handle_config_set`
- `src/error.rs` — `MatError::Config` variant for config file errors (created in task_01)
- `Cargo.toml` — add `serde` and `toml` dependencies

### Dependent Files
- `src/cli.rs` — will reference config types for CLI restructuring (task_05)
- `src/commands/create.rs` — will use `Config::load()` to get `default_branch`, `worktree_root`, `tmux.enabled` (task_06)
- `src/commands/close.rs` — will use `Config::load()` to get `delete_branch`, `merge_strategy` (task_07)

### Related ADRs
- [ADR-001: Absorb Branchlet Core Worktree Operations into Mat](../adrs/adr-001.md) — Two-tier TOML config decision
- [ADR-004: CLI Subcommand Structure and Config Management](../adrs/adr-004.md) — Decision to use serde+toml manual config management

## Deliverables
- `src/config.rs` with `Config` struct and load/merge/set logic
- `src/commands/config.rs` with `list`, `get`, `set` command handlers
- Updated `Cargo.toml` with `serde` and `toml` dependencies
- Unit tests with 80%+ coverage for config loading and merging (REQUIRED)
- Unit tests for config set operations (REQUIRED)

## Tests
- Unit tests:
  - [ ] `Config::load` with only global config returns global values
  - [ ] `Config::load` with project config overriding `default_branch` returns project value
  - [ ] `Config::load` with unset project fields returns global defaults for those fields
  - [ ] `Config::load` with neither config returns hardcoded defaults (`delete_branch=true`, `merge_strategy=MergeCommit`)
  - [ ] `Config::load` with missing files returns default config without error
  - [ ] `Config::load` with malformed TOML returns `MatError::Config`
  - [ ] `MergeStrategy` deserializes `"merge-commit"` to `MergeCommit`
  - [ ] `MergeStrategy` deserializes `"fast-forward"` to `FastForward`
  - [ ] `TmuxMode` deserializes `"auto"`, `"always"`, `"never"` correctly
  - [ ] `Config::set("default_branch", "develop", false)` writes to `.mat.toml`
  - [ ] `Config::set("default_branch", "develop", true)` writes to `~/.config/mat/config.toml`
  - [ ] `effective_value("default_branch")` shows source annotation: `"main (default)"` or `"develop (project: .mat.toml)"`
- Test coverage target: >=80%
- All tests must pass

## Success Criteria
- All tests passing
- Test coverage >=80%
- `Config::load()` correctly merges two-tier config with project overriding global
- `mat config list` shows effective values with source annotations
- `mat config set` writes to correct file (project by default, global with `--global`)
- Config defaults match PRD F3 specification
