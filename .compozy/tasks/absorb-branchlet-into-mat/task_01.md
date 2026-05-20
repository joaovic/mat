---
status: pending
title: Module split and MatError enum
type: refactor
complexity: critical
dependencies: []
---

# Task 01: Module split and MatError enum

## Overview
Split the 505-line monolithic `src/main.rs` into the 10-module structure defined in the TechSpec System Architecture. Convert all `Result<T, String>` return types to `Result<T, MatError>`, remove all 15 `std::process::exit(1)` calls, and establish a single exit point in `main()`. No behavior changes — this is pure restructuring.

<critical>
- ALWAYS READ the PRD and TechSpec before starting
- REFERENCE TECHSPEC for implementation details — do not duplicate here
- FOCUS ON "WHAT" — describe what needs to be accomplished, not how
- MINIMIZE CODE — show code only to illustrate current structure or problem areas
- TESTS REQUIRED — every task MUST include tests in deliverables
</critical>

<requirements>
- MUST create `src/error.rs` with `MatError` enum containing variants `Git`, `Tmux`, `Config`, `Validation`, `Io` as defined in TechSpec "Core Interfaces"
- MUST create `src/display.rs` with `print_error`, `print_success`, `print_info`, `print_tip` functions (extracted from lines 27-45 of current main.rs)
- MUST create `src/cli.rs` with the `Cli` struct and clap derive macros (extracted from lines 9-25)
- MUST create placeholder module files for `src/config.rs`, `src/git.rs`, `src/tmux.rs`, `src/naming.rs` with empty struct stubs to allow compilation
- MUST create placeholder module files for `src/commands/create.rs`, `src/commands/close.rs`, `src/commands/config.rs`
- MUST remove all 15 `std::process::exit(1)` calls (lines 288, 299, 306, 316, 327, 365, 377, 385, 391, 425, 433, 449, 453, 468, 472)
- MUST keep `src/main.rs` as the entry point that parses CLI, dispatches to handlers, and exits on error with appropriate exit codes
- SHOULD preserve all existing CLI argument parsing and all existing output messages verbatim
- MUST NOT add new features — all existing create/close behavior must compile and run identically
</requirements>

## Subtasks
- [ ] 01.1 Create `src/error.rs` with `MatError` enum and `impl Display` and `impl From<std::io::Error>` for it
- [ ] 01.2 Create `src/display.rs` and move the four print functions (error, success, info, tip) into it
- [ ] 01.3 Create `src/cli.rs` and move the `Cli` struct with clap derive into it
- [ ] 01.4 Create placeholder modules (`config.rs`, `git.rs`, `tmux.rs`, `naming.rs`) with stub types (empty structs) so the code compiles
- [ ] 01.5 Create placeholder command modules (`commands/create.rs`, `commands/close.rs`, `commands/config.rs`) with stub handler functions
- [ ] 01.6 Rewrite `handle_create_mode` and `handle_close_mode` to return `Result<(), MatError>` instead of calling `process::exit(1)`
- [ ] 01.7 Update `main()` to have a single exit point: match on `Result` and call `process::exit` with the appropriate code only once

## Implementation Details

See TechSpec "System Architecture" section for the complete module diagram and "Core Interfaces" for the `MatError` enum definition.

### Relevant Files
- `src/main.rs` — current 505-line monolith, to be refactored into modules
- `src/error.rs` — new file for `MatError` enum (see TechSpec lines 108-115)
- `src/display.rs` — new file for styled output, extracts `print_error`, `print_success`, `print_info`, `print_tip` (lines 27-45)
- `src/cli.rs` — new file for CLI args, extracts `Cli` struct (lines 9-25)
- `Cargo.toml` — may need `[lib]` section if creating lib.rs; verify compilation after split

### Dependent Files
- `src/git.rs` — placeholder, will be filled by task_02
- `src/tmux.rs` — placeholder, will be filled by task_03
- `src/naming.rs` — placeholder, will be filled by task_03
- `src/config.rs` — placeholder, will be filled by task_04
- `src/commands/create.rs` — placeholder, will be filled by task_06
- `src/commands/close.rs` — placeholder, will be filled by task_07
- `src/commands/config.rs` — placeholder, will be filled by task_04

### Related ADRs
- [ADR-003: Module Architecture and Error Handling Strategy](../adrs/adr-003.md) — Decision to split main.rs into 10 modules and define MatError enum

## Deliverables
- `src/error.rs` with `MatError` enum and `Display` impl
- `src/display.rs` with four print functions
- `src/cli.rs` with `Cli` struct
- Placeholder module files: `config.rs`, `git.rs`, `tmux.rs`, `naming.rs`
- Placeholder command modules: `commands/create.rs`, `commands/close.rs`, `commands/config.rs`
- Refactored `src/main.rs` with single exit point
- Unit tests for `MatError` display formatting (REQUIRED)
- Test coverage >=80% for error.rs and display.rs modules

## Tests
- Unit tests:
  - [ ] `MatError::Git` displays command name and stderr in its message
  - [ ] `MatError::Validation` displays the validation message
  - [ ] `MatError::Io` correctly wraps `std::io::Error` via `From` impl
  - [ ] `print_error` writes to stderr with red "ERROR:" prefix
  - [ ] `print_success` writes to stdout with green checkmark prefix
  - [ ] CLI parsing: `mat feat login` parses correctly into task_type="feat", task_name="login"
  - [ ] CLI parsing: `mat --close` sets close=true and ignores positional args

## Success Criteria
- All tests passing
- Test coverage >=80% for new modules (error.rs, display.rs)
- `cargo build` succeeds after refactoring
- `cargo run -- feat test-task` produces identical output to pre-refactoring binary
- `cargo run -- --close` produces identical output to pre-refactoring binary
- Zero `std::process::exit()` calls outside of `main()` function
