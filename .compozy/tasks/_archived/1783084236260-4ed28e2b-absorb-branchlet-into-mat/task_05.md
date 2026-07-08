---
status: completed
title: CLI restructuring with clap subcommands
type: backend
complexity: medium
dependencies:
  - task_04
---

# Task 05: CLI restructuring with clap subcommands

## Overview
Restructure the CLI from flat positional args to clap subcommands as defined in the TechSpec CLI Surface table. Add new flags (`--no-worktree`, `--no-merge`, `--use-tmux`). Maintain backward compatibility with `--close` / `-c` flag as a deprecated alias for `mat close`. Wire the CLI into the config system so CLI flags override config values.

<critical>
- ALWAYS READ the PRD and TechSpec before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — every task MUST include tests in deliverables
</critical>

<requirements>
- MUST define `Command` enum with variants: `Create`, `Close`, `ConfigList`, `ConfigGet`, `ConfigSet` as specified in TechSpec "Core Interfaces"
- MUST restructure clap definitions from flat `Cli` struct to subcommand model: `mat <type> <name>` (default), `mat close`, `mat config list/get/set`
- `mat <type> <name>` MUST continue to accept `task_type` and `task_name` as positional args (same UX as current)
- Create mode MUST accept `--source`, `--no-worktree`, `--use-tmux` flags
- Close mode MUST accept `--no-merge` flag
- `mat config` MUST have subcommands: `list`, `get <key>`, `set <key> <value>` with `--global` flag
- `mat --close` / `mat -c` MUST still work as deprecated alias, printing "Warning: --close is deprecated, use 'mat close' instead"
- MUST update `main.rs` to parse CLI into `Command` enum and dispatch to the appropriate handler
- MUST define `cli::parse` function that returns `Result<Command, MatError>` with validation errors for missing required args

## Subtasks
- [x] 05.1 Define `Command` enum with all variants in `src/cli.rs`
- [x] 05.2 Define clap subcommand structs for `Create`, `Close`, `ConfigList`, `ConfigGet`, `ConfigSet`
- [x] 05.3 Implement `cli::parse()` that converts clap-parsed args into `Command` enum
- [x] 05.4 Wire `--close`/`-c` as deprecated alias with warning message
- [x] 05.5 Update `main.rs` to dispatch `Command` variants to handler functions
- [x] 05.6 Write unit tests for CLI parsing of all subcommands and flag combinations

## Implementation Details

See TechSpec "CLI Surface" table for the complete subcommand + flags matrix. See TechSpec "Core Interfaces" for the `Command` enum definition. Backward compatibility with `mat --close` is required per TechSpec line 214.

### Relevant Files
- `src/cli.rs` — current `Cli` struct (from task_01), restructured into subcommands and `Command` enum
- `src/main.rs` — dispatch logic updated to match on `Command` variants
- `src/commands/create.rs` — handler for `Command::Create` (stub from task_01, filled by task_06)
- `src/commands/close.rs` — handler for `Command::Close` (stub from task_01, filled by task_07)
- `src/commands/config.rs` — handler for `ConfigList`, `ConfigGet`, `ConfigSet` (from task_04)
- `src/config.rs` — `Config` struct used as context for CLI flags (from task_04)

### Dependent Files
- `src/commands/create.rs` — receives parsed `Command::Create` with all flags (task_06)
- `src/commands/close.rs` — receives parsed `Command::Close` with `--no-merge` flag (task_07)

### Related ADRs
- [ADR-004: CLI Subcommand Structure and Config Management](../adrs/adr-004.md) — Decision to use clap subcommands and support backward-compatible `--close`

## Deliverables
- Updated `src/cli.rs` with `Command` enum and subcommand definitions
- Updated `src/main.rs` with `Command` dispatch logic
- Unit tests for CLI parsing (REQUIRED)

## Tests
- Unit tests:
  - [x] `mat feat login` parses to `Command::Create { task_type: "feat", task_name: "login", source: None, no_worktree: false, use_tmux: false }`
  - [x] `mat fix bug --no-worktree` parses to `Command::Create { task_type: "fix", task_name: "bug", no_worktree: true }`
  - [x] `mat feat login --source develop` parses with `source: Some("develop")`
  - [x] `mat feat login --use-tmux` parses with `use_tmux: true`
  - [x] `mat close` parses to `Command::Close { no_merge: false }`
  - [x] `mat close --no-merge` parses to `Command::Close { no_merge: true }`
  - [x] `mat --close` parses to `Command::Close { no_merge: false }` with deprecation warning
  - [x] `mat -c` parses to `Command::Close { no_merge: false }` with deprecation warning
  - [x] `mat config list` parses to `Command::ConfigList`
  - [x] `mat config get default_branch` parses to `Command::ConfigGet { key: "default_branch" }`
  - [x] `mat config set merge_strategy fast-forward` parses with `global: false`
  - [x] `mat config set --global default_branch develop` parses with `global: true`
  - [x] Missing task_type in create mode returns `MatError::Validation`
- Test coverage target: >=80%
- All tests must pass

## Success Criteria
- All tests passing
- Test coverage >=80%
- All subcommand variants parse correctly
- `mat --close` prints deprecation warning and functions identically to `mat close`
- `--no-worktree`, `--no-merge`, `--use-tmux` flags parse correctly
- Validation errors return `MatError::Validation` with descriptive messages
