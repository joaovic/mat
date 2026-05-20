# Task Memory: task_05.md

## Objective Snapshot

Restructure CLI from flat positional args to clap subcommands. `mat <type> <name>` (default create mode), `mat close`, `mat config list/get/set`. Backward compat via `--close`/`-c` deprecated flag. 151 tests green.

## Important Decisions

- Used hybrid approach: top-level `Cli` struct has both positional args (`task_type`, `task_name`) and `Option<MatCommand>` subcommand. Clap matches subcommands first; if no match, falls through to positional args.
- `Cli::try_parse()` + `cli_to_command()` split: `try_parse` handles clap errors, `cli_to_command` does our validation and conversion. Tests use `try_parse_from` + `cli_to_command`.
- `--no-worktree`, `--use-tmux`, `--no-merge` are top-level flags (on `Cli` struct), not scoped to subcommands. Keeps it simple since `mat <type> <name>` isn't a subcommand.
- `Command` enum aliased as `CliCmd` in `main.rs` to avoid conflict with `std::process::Command`.
- `handle_create_mode` signature changed from `(&Cli)` to `(&str, &str, Option<&str>)` — validation moved to `cli_to_command`.

## Learnings

- clap 4.x supports positional args + subcommands on the same struct; subcommands take precedence over positional args when names match.
- `close` as both subcommand name and potential positional arg works because clap tries subcommand match first.

## Files / Surfaces

- `src/cli.rs` — full rewrite: Command enum, subcommand enums, parse(), cli_to_command(), 19 tests
- `src/main.rs` — dispatch on Command variants, updated handler signatures

## Errors / Corrections

None.

## Ready for Next Run

Task complete. Task 06 and 07 will consume the `Command` variants from `cli::parse()`.
