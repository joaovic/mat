# Mat — Replace TMUX with Herdr

## Objective

Replace all tmux integration in the `mat` Rust project with herdr CLI commands.
The tool should no longer depend on tmux for terminal multiplexing — use herdr instead.

## Changes Required

### 1. Rename `src/tmux.rs` → `src/herdr.rs`

Replace the entire `TmuxClient<R>` struct with `HerdrClient<R>`. 
Map tmux commands to herdr equivalents:

| Old tmux command | New herdr command |
|---|---|
| `tmux new-window -c <path> -P -F "#{window_index}"` | `herdr workspace create --cwd <path> --label <name>` |
| `tmux rename-window <name>` | Remove — done by workspace create label |
| `tmux list-windows -F "#{window_index}"` | `herdr workspace list` (parse id) |
| `tmux select-window -t <target>` | `herdr workspace focus <id>` |
| `tmux kill-window -t <index>` | `herdr workspace close <id>` |
| `tmux set-buffer <text>` | Remove — not needed for herdr |
| `tmux send-keys -t <target> <keys> Enter` | Remove — not needed |
| `tmux display-message -p "#{window_index}"` | Remove |
| `tmux kill-session` | Remove |
| `tmux show-options -g prefix` | Remove (test-only) |

The `HerdrClient` needs these methods:
- `create_workspace(path: &str, label: &str) -> Result<String, MatError>` — returns workspace ID
- `find_workspace_by_path(path: &str) -> Result<Option<String>, MatError>` — searches workspace list by path
- `close_workspace(id: &str) -> Result<(), MatError>`
- `list_workspaces() -> Result<Vec<String>, MatError>` — returns IDs

The `run_herdr` method should call `herdr` binary instead of `tmux`.

When herdr server is not running, `herdr workspace list` will fail — treat that gracefully.

### 2. Update `src/main.rs`

- Change `mod tmux` to `mod herdr`
- Replace `TmuxClient` with `HerdrClient` in both Create and Close branches
- The constructor stays the same: `HerdrClient::new(RealRunner)`

### 3. Update `src/error.rs`

- Rename `MatError::Tmux { command, stderr }` to `MatError::Herdr { command, stderr }`
- Update the Display impl
- Update tests

### 4. Update `src/config.rs`

- Rename `TmuxMode` enum to `HerdrMode` (variants: Auto, Always, Never)
- Rename `TmuxConfig` struct to `HerdrConfig` (field: `enabled: HerdrMode`)
- Update all references in config resolution functions
- Update key from `tmux.enabled` to `herdr.enabled` in `effective_values()`, `value_for_key()`, config keys list

### 5. Update `src/cli.rs`

- Remove `--use-tmux` flag from the Create subcommand
- Remove the `use_tmux` field from `Command::Create`
- Update tests that reference `use_tmux`
- Update the about message (no longer mentions TMUX)

### 6. Update `src/commands/create.rs`

- Replace `TmuxClient` with `HerdrClient`
- Rename `should_use_tmux()` to `should_use_herdr()`
- Replace `handle_worktree_tmux()` with `handle_worktree_herdr()`:
  - Creates git worktree (same)
  - Calls `herdr_client.create_workspace(path, label)` instead of tmux new-window
  - Prints success message about herdr workspace
- Remove `tmux.send_keys()` and `tmux.set_buffer()` calls

### 7. Update `src/commands/close.rs`

- Replace `TmuxClient` with `HerdrClient`
- Replace `should_use_tmux()` with `should_use_herdr()`
- Replace tmux window close logic:
  ```rust
  if use_herdr {
      if let Ok(Some(ws_id)) = tmux.find_workspace_by_path(&path_str) {
          tmux.close_workspace(&ws_id)?;
          print_success("Herdr workspace closed");
      }
  }
  ```
- Remove set_buffer logic

### 8. Update `src/naming.rs`

- Remove `window_name` field from the `Names` struct (herdr doesn't need it)
- Update `generate_names()` to not populate `window_name`
- Update tests

### 9. Update all tests

- Replace `tmux_close_mocks()` with `herdr_close_mocks()`
- Replace `config_tmux_never()` / `config_tmux_always()` with `config_herdr_never()` / `config_herdr_always()`
- Replace `tmux::` references with `herdr::`
- Replace all mock responses from `tmux` commands to `herdr` commands
- Remove tests for removed functionality (set_buffer, send_keys, display_message, etc.)

### 10. Update `Cargo.toml` version

- Bump version to `0.4.0` (breaking change — dropped tmux, replaced with herdr)

## Build & Test

After all changes:
```bash
cargo build 2>&1
cargo test 2>&1
```

Make sure all tests pass. Fix any compilation errors.

## Verification

- `cargo build` compiles without errors
- `cargo test` passes all tests
- `mat create feat test` creates a git worktree (and optionally a herdr workspace if herdr is running)
- `mat close` merges and cleans up without trying to call tmux