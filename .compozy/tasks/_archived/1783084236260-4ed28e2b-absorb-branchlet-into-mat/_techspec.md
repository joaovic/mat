# TechSpec: Absorb Branchlet into Mat

## Executive Summary

This specification covers the implementation of absorbing branchlet's git worktree operations into mat as native `git worktree` commands, replacing all `branchlet` CLI invocations. The primary technical trade-off is between **refactoring velocity and testability**: we must split the 505-line `main.rs` monolith into modules with a `MatError` enum and `CommandRunner` trait before adding features — otherwise the new code paths (no-worktree, auto-merge, config) will be untestable and unmaintainable. The implementation uses clap subcommands, serde+toml for two-tier config, and a `CommandRunner` trait enabling `MockRunner` for unit tests. Git and tmux operations remain shell-out via `std::process::Command` (no git2 dependency).

## System Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────┐
│                    main.rs (entry)                  │
│  Parse CLI → dispatch to command handlers → exit    │
└───────────┬──────────┬──────────┬───────────────────┘
            │          │          │
    ┌───────▼──┐  ┌────▼─────┐  ┌▼──────────────┐
    │commands/ │  │commands/  │  │commands/       │
    │create.rs │  │close.rs   │  │config.rs       │
    └──────┬───┘  └────┬─────┘  └───────┬────────┘
           │           │                │
    ┌──────▼───────────▼────────────────▼──────────┐
    │                Core Modules                   │
    │  ┌──────────┐ ┌──────────┐ ┌────────────────┐ │
    │  │ git.rs    │ │ tmux.rs  │ │ naming.rs     │ │
    │  │GitClient  │ │TmuxClient│ │name generation│ │
    │  │  ↓        │ │  ↓       │ │               │ │
    │  │Command    │ │Command   │ │               │ │
    │  │Runner     │ │Runner    │ │               │ │
    │  └───────────┘ └──────────┘ └───────────────┘ │
    │                                               │
    │  ┌───────────┐ ┌──────────┐ ┌───────────────┐ │
    │  │ config.rs │ │display.rs│ │  error.rs     │ │
    │  │Config     │ │styled    │ │  MatError     │ │
    │  │struct+    │ │output    │ │  enum         │ │
    │  │load/merge │ │          │ │               │ │
    │  └───────────┘ └──────────┘ └───────────────┘ │
    └───────────────────────────────────────────────┘
            │              │
    ┌───────▼──────────────▼─────────────────────────┐
    │  External Commands                              │
    │  git worktree add/list/remove                   │
    │  git checkout/merge/stash/branch                │
    │  tmux new-window/rename-window/set-buffer/...   │
    └─────────────────────────────────────────────────┘
```

| Module | Responsibility | Dependencies |
|--------|---------------|-------------|
| `cli` | CLI argument parsing via clap subcommands | clap |
| `config` | Load/merge/write TOML config; `mat config` subcommand handlers | serde, toml |
| `display` | Styled terminal output (error, success, info, tip) | console |
| `git` | All git command execution via `GitClient` + `CommandRunner` | std::process |
| `tmux` | All tmux command execution via `TmuxClient` + `CommandRunner` | std::process |
| `naming` | Name generation for worktree, branch, window names | None (pure) |
| `commands/create` | Create workflow orchestration | git, tmux, naming, config, display |
| `commands/close` | Close workflow orchestration (with auto-merge) | git, tmux, naming, config, display |
| `commands/config` | Config subcommand handlers | config, display |
| `error` | `MatError` enum with `Git`, `Tmux`, `Config`, `Validation`, `Io` variants | None |

### Data Flow: Create Mode

```
User: mat feat login
  │
  ├─ cli::parse() → Create { task_type: "feat", task_name: "login", source: None, no_worktree: false }
  │
  ├─ config::load() → Config { default_branch: "main", delete_branch: true, ... }
  │
  ├─ naming::generate(app_name, task_type, task_name) → Names { branch, worktree, window, path }
  │
  ├─ git::worktree_add(names)? OR git::checkout_b(names) // --no-worktree
  │
  ├─ tmux::new_window(path)? OR shell::new_shell(path) // no tmux
  │
  └─ display::success("Ready! ...")
```

### Data Flow: Close Mode

```
User: mat close
  │
  ├─ config::load() → Config
  │
  ├─ git::status_porcelain()? → check uncommitted changes
  │
  ├─ git::worktree_list()? → find current worktree info (branch, source, path)
  │
  ├─ git::checkout(source)? → switch to base branch
  │
  ├─ git::merge(branch, strategy)? → auto-merge (config.strategy)
  │
  ├─ git::worktree_remove(path)? → delete worktree
  │
  ├─ git::branch_delete(branch)? → delete branch (config.delete_branch)
  │
  ├─ tmux::close_window()? → close tmux window
  │
  └─ display::success("Feature merged successfully!")
```

## Implementation Design

### Core Interfaces

```rust
// error.rs
pub enum MatError {
    Git { command: String, stderr: String },
    Tmux { command: String, stderr: String },
    Config { key: String, reason: String },
    Validation { message: String },
    Io(std::io::Error),
}

// cli.rs
pub enum Command {
    Create { task_type: String, task_name: String, source: Option<String>,
             no_worktree: bool, use_tmux: bool },
    Close { no_merge: bool },
    ConfigList,
    ConfigGet { key: String },
    ConfigSet { key: String, value: String, global: bool },
}

// config.rs
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub default_branch: Option<String>,
    pub delete_branch: bool,
    pub merge_strategy: MergeStrategy,
    pub worktree_root: Option<String>,
    pub tmux: TmuxConfig,
}

pub enum MergeStrategy { MergeCommit, FastForward }

#[derive(Debug, Deserialize, Default)]
pub struct TmuxConfig { pub enabled: TmuxMode }
pub enum TmuxMode { Auto, Always, Never }

impl Config {
    pub fn load() -> Result<Config, MatError>;   // merge global + local
    pub fn effective_value(key: &str) -> String;   // with source annotation
    pub fn set(key: &str, value: &str, global: bool) -> Result<(), MatError>;
}

// CommandRunner trait for testability
pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, MatError>;
}
pub struct RealRunner;
pub struct MockRunner { /* canned responses */ }

// git.rs
pub struct GitClient<R: CommandRunner> { runner: R }
impl<R: CommandRunner> GitClient<R> {
    pub fn is_repo(&self) -> Result<bool, MatError>;
    pub fn current_branch(&self) -> Result<String, MatError>;
    pub fn default_branch(&self) -> Result<String, MatError>;
    pub fn has_uncommitted_changes(&self) -> Result<bool, MatError>;
    pub fn worktree_add(&self, path: &str, branch: &str, source: &str) -> Result<String, MatError>;
    pub fn worktree_list(&self) -> Result<Vec<WorktreeInfo>, MatError>;
    pub fn worktree_remove(&self, path: &str) -> Result<(), MatError>;
    pub fn checkout(&self, branch: &str) -> Result<(), MatError>;
    pub fn checkout_b(&self, branch: &str, source: &str) -> Result<(), MatError>;
    pub fn merge(&self, branch: &str, strategy: MergeStrategy) -> Result<(), MatError>;
    pub fn branch_delete(&self, branch: &str) -> Result<(), MatError>;
    pub fn stash_push(&self, message: &str, include_untracked: bool) -> Result<(), MatError>;
    pub fn stash_pop(&self, stash_ref: &str) -> Result<(), MatError>;
}

// naming.rs
pub struct Names {
    pub branch_name: String,    // feat/login
    pub worktree_name: String,   // dashboard-feat/login
    pub window_name: String,    // dashboard-feat/login (same for tmux)
    pub worktree_path: PathBuf, // /path/to/repo.worktree/dashboard-feat/login
}

pub fn generate_names(app_name: &str, task_type: &str, task_name: &str,
                      config: &Config, repo_dir: &Path) -> Names;
```

### Data Models

```rust
// Worktree info parsed from `git worktree list --porcelain`
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,        // "feat/login" or "detached"
    pub commit: String,        // HEAD hash
    pub is_main: bool,
}

// Stash info for no-worktree mode
pub struct StashEntry {
    pub message: String,       // "mat:auto:feat/login"
    pub index: usize,          // stash@{0}
}
```

### CLI Surface

| Subcommand | Flags | Description |
|------------|-------|-------------|
| `mat <type> <name>` | `--source`, `--no-worktree`, `--use-tmux` | Create task environment |
| `mat close` | `--no-merge` | Close and merge task |
| `mat config list` | — | Show effective config with sources |
| `mat config get <key>` | — | Show single config value |
| `mat config set <key> <value>` | `--global` | Set config value |

**Backward compatibility**: `mat --close` / `mat -c` is deprecated but still works as an alias for `mat close`. A deprecation warning is printed when used.

### Config File Format

**Global**: `~/.config/mat/config.toml`

```toml
default_branch = "main"
delete_branch = true
merge_strategy = "merge-commit"
# worktree_root not set = default behavior
tmux.enabled = "auto"
```

**Project**: `.mat.toml` (in repo root)

```toml
default_branch = "develop"
merge_strategy = "fast-forward"
```

Merge priority: CLI flag > project config > global config > hardcoded default.

### Worktree Path Resolution

Default pattern: `{repo_dir}.worktree/{worktree_name}/`

If `worktree_root` is set in config, it supports template variables:
- `{app}` — repo directory basename
- `{type}` — task type (e.g., "feat")
- `{name}` — task name (e.g., "login")

Example: `worktree_root = "/tmp/worktrees/{app}/{type}"` produces `/tmp/worktrees/dashboard/feat/login/`

## Integration Points

| External Service | Purpose | Command | Error Handling |
|-----------------|---------|---------|----------------|
| **git** (required) | All git operations | `git worktree add/list/remove`, `git checkout/merge/stash/branch/status` | `MatError::Git` with command name and stderr |
| **tmux** (optional) | Window management | `tmux new-window/rename-window/set-buffer/display-message/list-windows/select-window/kill-window` | `MatError::Tmux` with command; auto-detect `$TMUX` for graceful fallback |
| **shell** (fallback) | Open new process when no tmux | `$SHELL` in worktree directory | Falls back to printing cd command |

## Impact Analysis

| Component | Impact Type | Description and Risk | Required Action |
|-----------|-------------|---------------------|-----------------|
| `src/main.rs` | **Replaced** | Current 505-line monolith split into 10 modules | Refactor into module structure |
| `Cargo.toml` | **Modified** | Add `serde`, `toml`, remove `dirs` and `serde_json`; `clap` stays with `derive` | Update dependencies |
| `branchlet` dependency | **Removed** | All 3 branchlet CLI calls replaced with native git commands | Remove from prerequisite checks |
| CLI interface | **Modified** | Add subcommands (`close`, `config`), add flags (`--no-worktree`, `--no-merge`, `--use-tmux`) | Restructure clap definitions |
| `BRANCHLET_SETTINGS` constant | **Removed** | No longer needed | Delete |
| Error handling | **Replaced** | `Result<T, String>` + `process::exit(1)` → `Result<T, MatError>` with single exit point | Refactor all error paths |

## Testing Approach

### Unit Tests

- **CommandRunner trait**: `MockRunner` returns canned output for all git/tmux commands. Tests inject `MockRunner` into `GitClient` and `TmuxClient` to test business logic without real git/tmux.
- **naming module**: Pure function tests for all name generation (`generate_names`). Test collision prevention with different task_type values.
- **config module**: Test config loading, merging (local overrides global), defaults, and `mat config set` write operations. Use temp dirs for filesystem isolation.
- **Merge strategy logic**: Test `merge` function with `MergeCommit` and `FastForward` strategies, verifying correct git flags.
- **Error variants**: Test that `MatError` correctly categorizes git failures, tmux failures, config errors, and validation errors.

### Integration Tests

- **Create workflow end-to-end**: Create a temporary git repo, run `mat feat test-task`, verify worktree exists, branch exists, and naming is correct.
- **Close workflow end-to-end**: Create a worktree, make a commit, run `mat close`, verify merge happened and worktree is deleted.
- **No-worktree mode**: Run `mat --no-worktree fix bug`, verify branch created, stash guard worked, and close flow restores stash.
- **Config commands**: `mat config list` shows merged values; `mat config set` writes to correct file.
- **Error paths**: Uncommitted changes on close, merge conflicts, locked worktrees, missing git repo.

## Development Sequencing

### Build Order

1. **Module split + MatError** — Split `main.rs` into module files, define `MatError` enum, convert all `Result<T, String>` to `Result<T, MatError>`, remove `process::exit()` calls. Single exit point in `main()`. *(No new features yet — just restructuring.)*
2. **CommandRunner trait + GitClient** — Define `CommandRunner` trait, implement `RealRunner` and `MockRunner`. Create `GitClient<R>` with all git operations (worktree_add, worktree_list, worktree_remove, checkout, merge, branch_delete, stash_push, stash_pop, status_porcelain, current_branch, default_branch). Depends on step 1.
3. **TmuxClient + naming** — Create `TmuxClient<R>` with all tmux operations. Create `naming` module with `generate_names()`. Depends on step 1.
4. **Config system** — Implement `Config` struct with `serde::Deserialize`, two-tier loading (global + local), merging, `mat config` subcommands. Depends on step 1.
5. **CLI restructuring** — Convert clap definitions to subcommand model (`mat <type> <name>`, `mat close`, `mat config`). Add `--no-worktree`, `--no-merge`, `--use-tmux` flags. Depends on step 4.
6. **Create command rewrite** — Rewrite `handle_create_mode` using `GitClient`, `TmuxClient`, `Config`, and `naming`. Support no-worktree mode and tmux detection. Depends on steps 2, 3, 4, 5.
7. **Close command rewrite** — Rewrite `handle_close_mode` using `GitClient`, `TmuxClient`, `Config`. Add auto-merge logic, merge conflict handling, `--no-merge` flag. Depends on steps 2, 3, 4, 5.
8. **Remove branchlet dependency** — Delete all branchlet-related code, remove `dirs` and `serde_json` from `Cargo.toml`, remove prerequisite checks for branchlet. Depends on steps 6 and 7.
9. **Integration tests** — Write end-to-end tests for create, close, no-worktree, and config workflows. Depends on steps 6, 7, 8.

### Technical Dependencies

- **New crates needed**: `serde` (with `derive` feature), `toml` (for config parsing)
- **Crates removed**: `dirs` (no longer needed — config uses `dirs` crate still for home dir), `serde_json` (replaced by `git worktree list --porcelain` parsing — actually keep `serde_json` since we don't use it for config)
- **Crates kept**: `clap` (with `derive`), `console`, `serde_json` (for parsing git worktree list porcelain output is text, not JSON — actually we can remove `serde_json` since we parse porcelain text, not JSON)
- **Note**: Re-evaluate crate needs. `git worktree list --porcelain` outputs text, not JSON. `serde_json` was only used for parsing `branchlet list --json`. With branchlet removed, `serde_json` can be removed. However, `toml` crate needs `serde` with derive macros.

## Monitoring and Observability

- **Exit codes**: `0` for success, `1` for general errors, `2` for config errors, `3` for git errors
- **Status messages**: Every operation prints progress via `display` module (`ℹ`, `✓`, `ERROR:`, `💡`)
- **Verbose mode** (future): `--verbose` flag to print every git/tmux command executed
- **Error context**: Every `MatError` includes the command that failed and its stderr output

## Technical Considerations

### Key Decisions

| Decision | Choice | Rationale | Trade-off | Alternatives Rejected |
|----------|--------|-----------|-----------|----------------------|
| Error handling | `MatError` enum | Typed pattern matching, single exit point, testable | More boilerplate than anyhow | `anyhow` (loses typed errors), `Result<T, String>` (untestable) |
| Module structure | 10 modules | Clear boundaries, each module owns its domain | More files to navigate | Lib+binary split (premature), monolith (unmaintainable at new feature count) |
| Testing approach | `CommandRunner` trait | Enables unit testing of all business logic | MockRunner maintenance burden | Black-box testing only (can't test internals), mockall (adds dev dep) |
| Config crate | serde + toml (manual) | Minimal deps, sufficient for 5 keys, transparent merging | Hand-written merge and set operations | `config` crate (15+ deps, overkill), `confy` (single-file only) |
| CLI structure | Clap subcommands | Extensible, idiomatic, supports `mat config` | Slight breaking change for `--close` users | Flat flags (doesn't scale), hybrid (inconsistent) |
| Worktree path | `{repo}.worktree/{name}/` with config template | Matches branchlet convention, configurable | Additional config key | Let git decide (loses naming control), global dir (isolates from repo) |

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Refactoring introduces regressions | Medium | High — existing create/close flows break | Write integration tests against current behavior BEFORE refactoring; use `branchlet` as oracle |
| `toml` crate's `to_string()` reformats files | Medium | Low — `mat config set` may strip comments | Use `toml_edit` crate for comment-preserving edits, or accept reformatting and document it |
| `git worktree add` path conflicts with existing directories | Low | Medium — error from git | Check path existence before calling git; provide clear error message |
| `sys::process::Command` output handling for large repos | Low | Low — worktree list could be large | Paginate or limit output; stream large outputs |
| Merge conflict detection requires parsing git output | Medium | Medium — git's conflict message format varies across versions | Use exit code (non-zero = conflict) rather than parsing stderr |

## Architecture Decision Records

- [ADR-001: Absorb Branchlet Core Worktree Operations into Mat](adrs/adr-001.md) — Decision to replace branchlet dependency with direct git worktree commands, add no-worktree mode, TOML config, and TMUX auto-detection
- [ADR-002: Lifecycle Manager Approach for Task Closing](adrs/adr-002.md) — Decision to auto-merge on close with configurable strategy, delete branch only on success, and support `--no-merge` flag
- [ADR-003: Module Architecture and Error Handling Strategy](adrs/adr-003.md) — Decision to split main.rs into 10 modules, define MatError enum, implement CommandRunner trait for testability
- [ADR-004: CLI Subcommand Structure and Config Management](adrs/adr-004.md) — Decision to use clap subcommands, serde+toml for manual config management, and branchlet-style worktree path templates
