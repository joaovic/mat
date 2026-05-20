# Task Memory: task_04.md

## Objective Snapshot

Implement two-tier TOML config system with serde+toml: Config struct, MergeStrategy/TmuxMode/TmuxConfig types, load/merge/set logic, `mat config list/get/set` CLI subcommands. All 137 tests pass (was 87 before, 50 new config tests added).

## Important Decisions

- MergeStrategy moved from git.rs to config.rs with serde Deserialize impl (custom string-based). git.rs imports from config.rs.
- Two-type approach: RawConfig (all Option fields for parsing) + Config (resolved fields with source tracking HashMap).
- Custom Deserialize impls for MergeStrategy and TmuxMode (string-based from TOML).
- Config::load runs `git rev-parse --show-toplevel` directly via Command, not through GitClient.
- All Config fields pub to allow struct update syntax in naming.rs tests.
- sourcetracking via HashMap<String, Source> stored directly in Config struct.

## Learnings

- `#[serde(rename_all = "kebab-case")]` doesn't work for string-valued TOML enums; need custom Deserialize impl.
- toml crate's Value type supports `Table::entry().or_insert_with(|| toml::Value::Table(...))` for nested key creation.
- Moving MergeStrategy from git.rs to config.rs required no changes to git.rs merge method — import-only change.

## Files / Surfaces

- `Cargo.toml` — added `serde` (with derive) and `toml = "0.8"`
- `src/config.rs` — full rewrite: Config, RawConfig, MergeStrategy, TmuxConfig, TmuxMode, Source, ConfigEntry, load/merge/set functions
- `src/git.rs` — removed MergeStrategy enum definition, added `use crate::config::MergeStrategy`
- `src/commands/config.rs` — handle_config_list, handle_config_get, handle_config_set implementations with validation
- `src/naming.rs` — updated 2 struct literal constructors to use `..Config::default()` syntax
- `src/main.rs` — unchanged

## Errors / Corrections

- Initial config.rs had `use tempfile` which isn't a dependency — removed those tests.
- Unused imports in commands/config.rs (`ConfigEntry`, `Source`, `print_error`) removed on first compile cycle.

## Ready for Next Run
