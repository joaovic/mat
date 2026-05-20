use std::env;
use std::path::Path;
use std::process::Command;

use crate::config::{Config, TmuxMode};
use crate::display::{print_info, print_success, print_warning};
use crate::error::MatError;
use crate::git::{CommandRunner, GitClient};
use crate::naming;
use crate::tmux::TmuxClient;

fn should_use_tmux(config: &Config, use_tmux_flag: bool) -> bool {
    match config.tmux.enabled {
        TmuxMode::Always => true,
        TmuxMode::Never => false,
        TmuxMode::Auto => {
            if use_tmux_flag {
                true
            } else {
                env::var("TMUX").is_ok()
            }
        }
    }
}

pub fn handle_create<R: CommandRunner>(
    task_type: &str,
    task_name: &str,
    source: Option<&str>,
    no_worktree: bool,
    use_tmux_flag: bool,
    config: &Config,
    git: &GitClient<R>,
    tmux: &TmuxClient<R>,
    app_name: &str,
    repo_dir: &Path,
) -> Result<(), MatError> {
    let source_branch = match source {
        Some(s) => s.to_string(),
        None => git
            .default_branch()
            .unwrap_or_else(|_| config.default_branch.clone()),
    };

    let names = naming::generate_names(app_name, task_type, task_name, config, repo_dir);

    if no_worktree {
        handle_no_worktree(git, &names, &source_branch)
    } else if should_use_tmux(config, use_tmux_flag) {
        handle_worktree_tmux(git, tmux, &names, &source_branch)
    } else {
        handle_worktree_shell(git, &names, &source_branch)
    }
}

fn handle_no_worktree<R: CommandRunner>(
    git: &GitClient<R>,
    names: &naming::Names,
    source_branch: &str,
) -> Result<(), MatError> {
    if git.has_uncommitted_changes()? {
        git.stash_push(&names.branch_name, false)?;
        print_success(&format!(
            "Changes stashed as 'mat:auto:{}'",
            names.branch_name
        ));
    }

    print_info(&format!(
        "Creating branch: {} from {}",
        names.branch_name, source_branch
    ));
    git.checkout_b(&names.branch_name, source_branch)?;
    print_success(&format!("Switched to branch {}", names.branch_name));

    println!();
    print_warning("No-worktree mode: changes are isolated to this branch, not a separate directory.");
    println!("  Stashed changes can be restored with: git stash pop");
    println!();

    print_success(&format!("Ready to work on {}", names.branch_name));

    Ok(())
}

fn handle_worktree_tmux<R: CommandRunner>(
    git: &GitClient<R>,
    tmux: &TmuxClient<R>,
    names: &naming::Names,
    source_branch: &str,
) -> Result<(), MatError> {
    print_info("Running prerequisite checks...");
    print_success("Git repository detected");
    print_success(&format!("Worktree name: {}", names.worktree_name));

    print_info(&format!(
        "Creating worktree: branch {} from {}",
        names.branch_name, source_branch
    ));

    let path_str = names.worktree_path.to_string_lossy().to_string();
    git.worktree_add(&path_str, &names.branch_name, source_branch)?;
    print_success(&format!("Worktree created at {}", path_str));

    let window_index = tmux.new_window(&path_str)?;
    tmux.rename_window(&names.window_name)?;

    // Ensure the new window's first pane is in the worktree.
    // Splits from this pane will inherit its cwd, so all new panels stay in the worktree.
    let _ = tmux.send_keys(&format!("{}.0", window_index), &format!("cd {}", path_str));

    println!();
    print_success(&format!(
        "Ready! Window '{}' is now open at:",
        names.window_name
    ));
    println!("  {}", path_str);

    Ok(())
}

fn try_open_new_terminal_tab(path: &std::path::Path) -> bool {
    if std::env::var("MAT_SKIP_TERMINAL").is_ok() {
        return false;
    }

    let path_str = path.to_string_lossy();

    #[cfg(target_os = "macos")]
    {
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();

        if term_program == "Apple_Terminal" {
            let script = format!(
                r#"tell application "Terminal"
                    activate
                    if not (exists window 1) then
                        do script "cd {path}"
                    else
                        tell front window
                            set newTab to make new tab
                            set selected to newTab
                            do script "cd {path}" in newTab
                        end tell
                    end if
                end tell"#
            );
            if Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return true;
            }
        } else if term_program == "iTerm.app" {
            let script = format!(
                r#"tell application "iTerm"
                    tell current window
                        create tab with default profile
                        tell current session
                            write text "cd {path}"
                        end tell
                    end tell
                end tell"#
            );
            if Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return true;
            }
        }

        // Fallback: open new Terminal window
        if Command::new("open")
            .args(["-a", "Terminal", &path_str])
            .spawn()
            .is_ok()
        {
            return true;
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Only attempt GUI terminals when a display server appears available
        let has_display = std::env::var("DISPLAY").is_ok()
            || std::env::var("WAYLAND_DISPLAY").is_ok();
        if !has_display {
            return false;
        }

        // Try opening a tab in the current terminal first
        let tab_candidates: [(&str, Vec<&str>); 3] = [
            ("gnome-terminal", vec!["--tab", "--working-directory", &path_str]),
            ("konsole", vec!["--new-tab", "--workdir", &path_str]),
            ("xfce4-terminal", vec!["--tab", "--working-directory", &path_str]),
        ];

        for (cmd, args) in &tab_candidates {
            if Command::new(cmd).args(args).spawn().is_ok() {
                return true;
            }
        }

        // WezTerm tab (works when inside a WezTerm instance)
        if Command::new("wezterm")
            .args(["cli", "spawn", "--cwd", &path_str])
            .spawn()
            .is_ok()
        {
            return true;
        }

        // Kitty tab (requires remote control)
        if Command::new("kitty")
            .args(["@", "launch", "--type=tab", "--cwd", &path_str])
            .spawn()
            .is_ok()
        {
            return true;
        }

        // Window fallbacks
        let window_candidates: [(&str, Vec<&str>); 5] = [
            ("gnome-terminal", vec!["--working-directory", &path_str]),
            ("konsole", vec!["--workdir", &path_str]),
            ("xfce4-terminal", vec!["--working-directory", &path_str]),
            ("alacritty", vec!["--working-directory", &path_str]),
            ("kitty", vec!["--directory", &path_str]),
        ];

        for (cmd, args) in &window_candidates {
            if Command::new(cmd).args(args).spawn().is_ok() {
                return true;
            }
        }

        // WezTerm window
        if Command::new("wezterm")
            .args(["start", "--cwd", &path_str])
            .spawn()
            .is_ok()
        {
            return true;
        }
    }

    false
}

fn handle_worktree_shell<R: CommandRunner>(
    git: &GitClient<R>,
    names: &naming::Names,
    source_branch: &str,
) -> Result<(), MatError> {
    print_success("Git repository detected");

    print_info(&format!(
        "Creating worktree: branch {} from {}",
        names.branch_name, source_branch
    ));

    let path_str = names.worktree_path.to_string_lossy().to_string();
    git.worktree_add(&path_str, &names.branch_name, source_branch)?;
    print_success(&format!("Worktree created at {}", path_str));

    if try_open_new_terminal_tab(&names.worktree_path) {
        print_success("Opened new terminal tab in worktree directory");
        println!("  {}", path_str);
    } else {
        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut child = Command::new(&shell)
            .current_dir(&names.worktree_path)
            .spawn()
            .map_err(|e| MatError::Io(std::sync::Arc::new(e)))?;

        print_success("Opening new shell in worktree directory...");
        println!("  (Type 'exit' to return to your original directory)");

        child.wait().map_err(|e| MatError::Io(std::sync::Arc::new(e)))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TmuxConfig;
    use crate::git::{CommandOutput, MockRunner};
    use std::path::Path;

    const APP_NAME: &str = "app";
    const REPO_DIR: &str = "/repo";

    fn repo() -> &'static Path {
        Path::new(REPO_DIR)
    }

    fn wt(task_type: &str, task_name: &str) -> String {
        format!("/repo.worktree/{}-{}/{}", APP_NAME, task_type, task_name)
    }

    fn run_shell_path(config: Config) -> Result<(), MatError> {
        let tmp = std::env::temp_dir().join("mat_test_create");
        let repo_dir = tmp.join("repo");
        // generate_names creates: {repo_dir}.worktree/{app}-{type}/{name}
        let tree_path = format!(
            "{}.worktree/{}-fix/typo",
            repo_dir.to_string_lossy(),
            APP_NAME
        );
        std::fs::create_dir_all(&tree_path).unwrap();

        let mut mock = MockRunner::new();
        mock.add_response("git", &["symbolic-ref", "refs/remotes/origin/HEAD"], ok_output("refs/remotes/origin/main\n"));
        mock.add_response("git", &["worktree", "add", "-b", "fix/typo", &tree_path, "main"], ok_output(""));

        let git = GitClient::new(mock);
        let tmux = TmuxClient::new(MockRunner::new());
        let prev_shell = env::var("SHELL").ok();
        let prev_skip_terminal = env::var("MAT_SKIP_TERMINAL").ok();
        env::set_var("SHELL", "true");
        env::set_var("MAT_SKIP_TERMINAL", "1");

        let app_name = APP_NAME;
        let result = handle_create("fix", "typo", None, false, false, &config, &git, &tmux, app_name, &repo_dir);

        match prev_shell {
            Some(s) => env::set_var("SHELL", s),
            None => env::remove_var("SHELL"),
        }
        match prev_skip_terminal {
            Some(s) => env::set_var("MAT_SKIP_TERMINAL", s),
            None => env::remove_var("MAT_SKIP_TERMINAL"),
        }
        let _ = std::fs::remove_dir_all(&tmp);

        result
    }

    fn ok_output(stdout: &str) -> CommandOutput {
        CommandOutput {
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    fn base_config() -> Config {
        Config::default()
    }

    fn config_tmux(enabled: TmuxMode) -> Config {
        Config {
            tmux: TmuxConfig { enabled },
            ..Config::default()
        }
    }

    // ── Worktree + TMUX path ──────────────────────────────────────

    #[test]
    fn test_worktree_tmux_path_full_flow() {
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_response("git", &["symbolic-ref", "refs/remotes/origin/HEAD"], ok_output("refs/remotes/origin/main\n"));
        mock.add_response("git", &["worktree", "add", "-b", "feat/login", &tree_path, "main"], ok_output(""));
        mock.add_response("tmux", &["new-window", "-c", &tree_path, "-P", "-F", "#{window_index}"], ok_output("2\n"));
        mock.add_response("tmux", &["rename-window", "app-feat/login"], ok_output(""));
        mock.add_response("tmux", &["send-keys", "-t", "2.0", &format!("cd {}", tree_path), "Enter"], ok_output(""));

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = config_tmux(TmuxMode::Always);

        let result = handle_create("feat", "login", None, false, false, &config, &git, &tmux, APP_NAME, repo());
        assert!(result.is_ok());
    }

    #[test]
    fn test_worktree_tmux_uses_source_flag() {
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_response("git", &["worktree", "add", "-b", "feat/login", &tree_path, "develop"], ok_output(""));
        mock.add_response("tmux", &["new-window", "-c", &tree_path, "-P", "-F", "#{window_index}"], ok_output("2\n"));
        mock.add_response("tmux", &["rename-window", "app-feat/login"], ok_output(""));
        mock.add_response("tmux", &["send-keys", "-t", "2.0", &format!("cd {}", tree_path), "Enter"], ok_output(""));

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = config_tmux(TmuxMode::Always);

        let result = handle_create("feat", "login", Some("develop"), false, false, &config, &git, &tmux, APP_NAME, repo());
        assert!(result.is_ok());
    }

    #[test]
    fn test_worktree_tmux_worktree_add_failure() {
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_response("git", &["symbolic-ref", "refs/remotes/origin/HEAD"], ok_output("refs/remotes/origin/main\n"));
        mock.add_error("git", &["worktree", "add", "-b", "feat/login", &tree_path, "main"], MatError::Git {
            command: "git worktree add".into(),
            stderr: "fatal: already exists".into(),
        });

        let git = GitClient::new(mock);
        let tmux = TmuxClient::new(MockRunner::new());
        let config = config_tmux(TmuxMode::Always);

        let result = handle_create("feat", "login", None, false, false, &config, &git, &tmux, APP_NAME, repo());
        assert!(result.is_err());
        match result.unwrap_err() {
            MatError::Git { ref stderr, .. } => assert!(stderr.contains("already exists")),
            _ => panic!("expected MatError::Git"),
        }
    }

    #[test]
    fn test_worktree_tmux_uses_default_branch_from_config_when_auto_detect_fails() {
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_error("git", &["symbolic-ref", "refs/remotes/origin/HEAD"], MatError::Git {
            command: "git symbolic-ref".into(),
            stderr: "no upstream".into(),
        });
        mock.add_response("git", &["worktree", "add", "-b", "feat/login", &tree_path, "develop"], ok_output(""));
        mock.add_response("tmux", &["new-window", "-c", &tree_path, "-P", "-F", "#{window_index}"], ok_output("2\n"));
        mock.add_response("tmux", &["rename-window", "app-feat/login"], ok_output(""));
        mock.add_response("tmux", &["send-keys", "-t", "2.0", &format!("cd {}", tree_path), "Enter"], ok_output(""));

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = Config {
            default_branch: "develop".into(),
            tmux: TmuxConfig { enabled: TmuxMode::Always },
            ..Config::default()
        };

        let result = handle_create("feat", "login", None, false, false, &config, &git, &tmux, APP_NAME, repo());
        assert!(result.is_ok());
    }

    // ── No-worktree path ──────────────────────────────────────────

    #[test]
    fn test_no_worktree_with_stash() {
        let mut mock = MockRunner::new();
        mock.add_response("git", &["status", "--porcelain"], ok_output(" M src/file.rs\n"));
        mock.add_response("git", &["stash", "push", "-m", "mat:auto:feat/login"], ok_output(""));
        mock.add_response("git", &["checkout", "-b", "feat/login", "main"], ok_output(""));

        let git = GitClient::new(mock);
        let tmux = TmuxClient::new(MockRunner::new());
        let config = base_config();

        let result = handle_create("feat", "login", Some("main"), true, false, &config, &git, &tmux, APP_NAME, repo());
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_worktree_no_uncommitted_changes() {
        let mut mock = MockRunner::new();
        mock.add_response("git", &["status", "--porcelain"], ok_output(""));
        mock.add_response("git", &["checkout", "-b", "feat/login", "main"], ok_output(""));

        let git = GitClient::new(mock);
        let tmux = TmuxClient::new(MockRunner::new());
        let config = base_config();

        let result = handle_create("feat", "login", Some("main"), true, false, &config, &git, &tmux, APP_NAME, repo());
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_worktree_worktree_add_not_called() {
        let mut mock = MockRunner::new();
        mock.add_response("git", &["status", "--porcelain"], ok_output(""));
        mock.add_response("git", &["checkout", "-b", "feat/login", "main"], ok_output(""));

        let git = GitClient::new(mock);
        let tmux = TmuxClient::new(MockRunner::new());
        let config = base_config();

        let result = handle_create("feat", "login", Some("main"), true, false, &config, &git, &tmux, APP_NAME, repo());
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_worktree_stash_not_called_when_clean() {
        let mut mock = MockRunner::new();
        mock.add_response("git", &["status", "--porcelain"], ok_output(""));
        mock.add_response("git", &["checkout", "-b", "feat/login", "main"], ok_output(""));

        let git = GitClient::new(mock);
        let tmux = TmuxClient::new(MockRunner::new());
        let config = base_config();

        let result = handle_create("feat", "login", Some("main"), true, false, &config, &git, &tmux, APP_NAME, repo());
        assert!(result.is_ok());
    }

    // ── Worktree shell (no-tmux) path ─────────────────────────────

    #[test]
    fn test_worktree_shell_path_worktree_add_called_tmux_not_called() {
        let result = run_shell_path(config_tmux(TmuxMode::Never));
        assert!(result.is_ok());
    }

    // ── Tmux mode config ──────────────────────────────────────────

    #[test]
    fn test_tmux_enabled_never_forces_no_tmux() {
        let result = run_shell_path(config_tmux(TmuxMode::Never));
        assert!(result.is_ok());
    }

    #[test]
    fn test_tmux_enabled_never_overrides_use_tmux_flag() {
        let config = config_tmux(TmuxMode::Never);
        let tmp = std::env::temp_dir().join("mat_test_create_override");
        let repo_dir = tmp.join("repo");
        let tree_path = format!(
            "{}.worktree/{}-feat/login",
            repo_dir.to_string_lossy(),
            APP_NAME
        );
        std::fs::create_dir_all(&tree_path).unwrap();

        let mut mock = MockRunner::new();
        mock.add_response("git", &["symbolic-ref", "refs/remotes/origin/HEAD"], ok_output("refs/remotes/origin/main\n"));
        mock.add_response("git", &["worktree", "add", "-b", "feat/login", &tree_path, "main"], ok_output(""));

        let git = GitClient::new(mock);
        let tmux = TmuxClient::new(MockRunner::new());
        let prev_shell = env::var("SHELL").ok();
        let prev_skip_terminal = env::var("MAT_SKIP_TERMINAL").ok();
        env::set_var("SHELL", "true");
        env::set_var("MAT_SKIP_TERMINAL", "1");

        let result = handle_create("feat", "login", None, false, true, &config, &git, &tmux, APP_NAME, &repo_dir);

        match prev_shell {
            Some(s) => env::set_var("SHELL", s),
            None => env::remove_var("SHELL"),
        }
        match prev_skip_terminal {
            Some(s) => env::set_var("MAT_SKIP_TERMINAL", s),
            None => env::remove_var("MAT_SKIP_TERMINAL"),
        }
        let _ = std::fs::remove_dir_all(&tmp);

        assert!(result.is_ok());
    }

    #[test]
    fn test_tmux_enabled_always_forces_tmux_even_without_env() {
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_response("git", &["symbolic-ref", "refs/remotes/origin/HEAD"], ok_output("refs/remotes/origin/main\n"));
        mock.add_response("git", &["worktree", "add", "-b", "feat/login", &tree_path, "main"], ok_output(""));
        mock.add_response("tmux", &["new-window", "-c", &tree_path, "-P", "-F", "#{window_index}"], ok_output("2\n"));
        mock.add_response("tmux", &["rename-window", "app-feat/login"], ok_output(""));
        mock.add_response("tmux", &["send-keys", "-t", "2.0", &format!("cd {}", tree_path), "Enter"], ok_output(""));

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = config_tmux(TmuxMode::Always);

        let result = handle_create("feat", "login", None, false, false, &config, &git, &tmux, APP_NAME, repo());
        assert!(result.is_ok());
    }

    #[test]
    fn test_tmux_enabled_always_fails_when_tmux_not_running() {
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_response("git", &["symbolic-ref", "refs/remotes/origin/HEAD"], ok_output("refs/remotes/origin/main\n"));
        mock.add_response("git", &["worktree", "add", "-b", "feat/login", &tree_path, "main"], ok_output(""));
        mock.add_error("tmux", &["new-window", "-c", &tree_path, "-P", "-F", "#{window_index}"], MatError::Tmux {
            command: "tmux new-window".into(),
            stderr: "no server running".into(),
        });

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = config_tmux(TmuxMode::Always);

        let result = handle_create("feat", "login", None, false, false, &config, &git, &tmux, APP_NAME, repo());
        assert!(result.is_err());
    }

    // ── Default branch resolution ─────────────────────────────────

    #[test]
    fn test_default_branch_from_config_when_source_not_given() {
        let mut mock = MockRunner::new();
        let tree_path = wt("chore", "update");
        mock.add_error("git", &["symbolic-ref", "refs/remotes/origin/HEAD"], MatError::Git {
            command: "git symbolic-ref".into(),
            stderr: "no upstream".into(),
        });
        mock.add_response("git", &["worktree", "add", "-b", "chore/update", &tree_path, "develop"], ok_output(""));
        mock.add_response("tmux", &["new-window", "-c", &tree_path, "-P", "-F", "#{window_index}"], ok_output("2\n"));
        mock.add_response("tmux", &["rename-window", "app-chore/update"], ok_output(""));
        mock.add_response("tmux", &["send-keys", "-t", "2.0", &format!("cd {}", tree_path), "Enter"], ok_output(""));

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = Config {
            default_branch: "develop".into(),
            tmux: TmuxConfig { enabled: TmuxMode::Always },
            ..Config::default()
        };

        let result = handle_create("chore", "update", None, false, false, &config, &git, &tmux, APP_NAME, repo());
        assert!(result.is_ok());
    }

    // ── should_use_tmux ───────────────────────────────────────────

    #[test]
    fn test_should_use_tmux_never() {
        let config = config_tmux(TmuxMode::Never);
        assert!(!should_use_tmux(&config, false));
        assert!(!should_use_tmux(&config, true));
    }

    #[test]
    fn test_should_use_tmux_always() {
        let config = config_tmux(TmuxMode::Always);
        assert!(should_use_tmux(&config, false));
        assert!(should_use_tmux(&config, true));
    }

    #[test]
    fn test_should_use_tmux_auto_with_flag() {
        let config = config_tmux(TmuxMode::Auto);
        assert!(should_use_tmux(&config, true));
    }

    #[test]
    fn test_should_use_tmux_auto_without_flag_and_no_tmux_env() {
        let config = config_tmux(TmuxMode::Auto);
        let old_tmux = env::var("TMUX").ok();
        env::remove_var("TMUX");
        assert!(!should_use_tmux(&config, false));
        if let Some(val) = old_tmux {
            env::set_var("TMUX", val);
        }
    }
}
