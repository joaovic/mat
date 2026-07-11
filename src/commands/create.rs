use std::cell::Cell;
use std::env;
use std::path::Path;
use std::process::Command;

use crate::config::{Config, HerdrMode, Settings};
use crate::display::{print_info, print_success, print_warning};
use crate::error::MatError;
use crate::git::{CommandRunner, GitClient};
use crate::herdr::HerdrClient;
use crate::naming;

thread_local! {
    static TEST_HERDR_INSIDE: Cell<Option<bool>> = Cell::new(None);
}

fn is_inside_herdr() -> bool {
    TEST_HERDR_INSIDE.with(|cell| {
        cell.get().unwrap_or_else(|| std::env::var("HERDR_ENV").map_or(false, |v| v == "1"))
    })
}

#[cfg(test)]
fn set_test_herdr_inside(value: bool) {
    TEST_HERDR_INSIDE.with(|cell| cell.set(Some(value)));
}

#[cfg(test)]
fn clear_test_herdr_inside() {
    TEST_HERDR_INSIDE.with(|cell| cell.set(None));
}

fn should_use_herdr(config: &Config) -> bool {
    match config.herdr.enabled {
        HerdrMode::Always => true,
        HerdrMode::Never => false,
        HerdrMode::Auto => is_inside_herdr(),
    }
}

fn store_source_branch<R: CommandRunner>(git: &GitClient<R>, branch: &str, source: &str) {
    let key = format!("branch.{}.mat-source", branch);
    let _ = git.config_set(&key, source);
}

fn store_cwd<R: CommandRunner>(git: &GitClient<R>, branch: &str) {
    if let Ok(cwd) = env::current_dir() {
        let key = format!("branch.{}.mat-cwd", branch);
        let _ = git.config_set(&key, &naming::normalize_path(&cwd));
    }
}

fn store_herdr_workspace<R: CommandRunner>(git: &GitClient<R>, branch: &str, workspace_id: &str) {
    let key = format!("branch.{}.mat-herdr-workspace", branch);
    let _ = git.config_set(&key, workspace_id);
}

pub fn handle_create<R: CommandRunner>(
    task_type: &str,
    task_name: &str,
    source: Option<&str>,
    no_worktree: bool,
    config: &Config,
    settings: &Settings,
    git: &GitClient<R>,
    herdr: &HerdrClient<R>,
    app_name: &str,
    repo_dir: &Path,
) -> Result<(), MatError> {
    let source_branch = match source {
        Some(s) => s.to_string(),
        None => {
            let current = git.current_branch().ok().filter(|b| !b.is_empty());
            match current {
                Some(branch) => branch,
                None => git
                    .default_branch()
                    .unwrap_or_else(|_| config.default_branch.clone()),
            }
        }
    };

    let mut names = naming::generate_names(app_name, task_type, task_name, config, repo_dir);

    // Override worktree path when a custom path_template is set in settings
    if settings.worktree.path_template != "$BASE_PATH.wtree" {
        names.worktree_path = crate::config::process_path_template(
            &settings.worktree.path_template,
            repo_dir,
            app_name,
            task_type,
            task_name,
        );
    }

    if no_worktree {
        handle_no_worktree(git, &names, &source_branch)
    } else if should_use_herdr(config) && is_inside_herdr() {
        handle_worktree_herdr_tab(git, herdr, &names, &source_branch, settings, repo_dir)
    } else if should_use_herdr(config) {
        handle_worktree_herdr_workspace(git, herdr, &names, &source_branch, settings, repo_dir)
    } else {
        handle_worktree_shell(git, &names, &source_branch, settings, repo_dir)
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
    store_source_branch(git, &names.branch_name, source_branch);
    print_success(&format!("Switched to branch {}", names.branch_name));

    println!();
    print_warning(
        "No-worktree mode: changes are isolated to this branch, not a separate directory.",
    );
    println!("  Stashed changes can be restored with: git stash pop");
    println!();

    print_success(&format!("Ready to work on {}", names.branch_name));

    Ok(())
}

fn handle_worktree_herdr_workspace<R: CommandRunner>(
    git: &GitClient<R>,
    herdr: &HerdrClient<R>,
    names: &naming::Names,
    source_branch: &str,
    _settings: &Settings,
    repo_dir: &Path,
) -> Result<(), MatError> {
    print_info("Running prerequisite checks...");
    print_success("Git repository detected");
    print_success(&format!("Worktree name: {}", names.worktree_name));

    print_info(&format!(
        "Creating worktree via herdr: branch {} from {}",
        names.branch_name, source_branch
    ));

    let cwd_str = naming::normalize_path(repo_dir);
    let result = herdr.create_worktree(
        &cwd_str,
        &names.branch_name,
        source_branch,
        &names.worktree_name,
        None,
    )?;

    store_source_branch(git, &names.branch_name, source_branch);
    store_cwd(git, &names.branch_name);
    store_herdr_workspace(git, &names.branch_name, &result.workspace_id);

    print_success(&format!("Worktree created at {}", result.path));

    println!();
    print_success(&format!(
        "Ready! Herdr workspace '{}' is now open at:",
        names.worktree_name
    ));
    println!("  {}", result.path);

    Ok(())
}

fn handle_worktree_herdr_tab<R: CommandRunner>(
    git: &GitClient<R>,
    herdr: &HerdrClient<R>,
    names: &naming::Names,
    source_branch: &str,
    settings: &Settings,
    repo_dir: &Path,
) -> Result<(), MatError> {
    print_success("Git repository detected");

    print_info(&format!(
        "Creating worktree: branch {} from {}",
        names.branch_name, source_branch
    ));

    let path_str = naming::normalize_path(&names.worktree_path);
    git.worktree_add(&path_str, &names.branch_name, source_branch)?;
    store_source_branch(git, &names.branch_name, source_branch);
    store_cwd(git, &names.branch_name);
    print_success(&format!("Worktree created at {}", path_str));

    if !settings.worktree.copy_patterns.is_empty() {
        let _ = crate::config::copy_worktree_files(repo_dir, &names.worktree_path, settings);
    }

    if !settings.worktree.post_create_cmd.is_empty() {
        let _ = crate::config::run_post_create_commands(
            &names.worktree_path,
            &settings.worktree.post_create_cmd,
        );
    }

    print_info("Setting up herdr tab...");
    let workspace_id = herdr.current_workspace()?;

    let (tab_id, root_pane_id) =
        herdr.tab_create(&workspace_id, Some(&names.worktree_name))?;

    let right_pane_id =
        herdr.pane_split(&root_pane_id, "right", true)?;

    herdr.pane_run(
        &right_pane_id,
        &format!("cd {} && opencode .", path_str),
    )?;

    let bottom_pane_id = herdr.pane_split(&root_pane_id, "down", true)?;

    herdr.pane_run(&root_pane_id, &format!("cd {} && ll", path_str))?;

    herdr.pane_run(&bottom_pane_id, &format!("cd {}", path_str))?;

    herdr.tab_focus(&tab_id)?;

    println!();
    print_success(&format!(
        "Ready! Herdr tab '{}' with panel layout set up at:",
        names.worktree_name
    ));
    println!("  {}", path_str);

    Ok(())
}

fn try_open_new_terminal_tab(path: &std::path::Path) -> bool {
    if std::env::var("MAT_SKIP_TERMINAL").is_ok() {
        return false;
    }

    let path_str = naming::normalize_path(path);

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

    #[cfg(target_os = "windows")]
    {
        // Try Windows Terminal (wt.exe ships with Windows 10/11)
        if Command::new("wt")
            .args(["-d", &path_str, "new-tab"])
            .spawn()
            .is_ok()
        {
            return true;
        }

        // Fallback: PowerShell
        if Command::new("powershell.exe")
            .args([
                "-NoExit",
                "-Command",
                &format!("cd '{}'", path_str.replace('\'', "''")),
            ])
            .spawn()
            .is_ok()
        {
            return true;
        }

        // Fallback: cmd.exe
        if Command::new("cmd.exe")
            .args(["/K", &format!("cd /d \"{}\"", path_str)])
            .spawn()
            .is_ok()
        {
            return true;
        }
        // All failed — fall through to final `false`
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // Only attempt GUI terminals when a display server appears available
        let has_display =
            std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok();
        if !has_display {
            return false;
        }

        // Try opening a tab in the current terminal first
        let tab_candidates: [(&str, Vec<&str>); 3] = [
            (
                "gnome-terminal",
                vec!["--tab", "--working-directory", &path_str],
            ),
            ("konsole", vec!["--new-tab", "--workdir", &path_str]),
            (
                "xfce4-terminal",
                vec!["--tab", "--working-directory", &path_str],
            ),
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
    settings: &Settings,
    source_dir: &Path,
) -> Result<(), MatError> {
    print_success("Git repository detected");

    print_info(&format!(
        "Creating worktree: branch {} from {}",
        names.branch_name, source_branch
    ));

    let path_str = naming::normalize_path(&names.worktree_path);
    git.worktree_add(&path_str, &names.branch_name, source_branch)?;
    store_source_branch(git, &names.branch_name, source_branch);
    store_cwd(git, &names.branch_name);
    print_success(&format!("Worktree created at {}", path_str));

    // Copy files from source directory to worktree
    if !settings.worktree.copy_patterns.is_empty() {
        let _ = crate::config::copy_worktree_files(source_dir, &names.worktree_path, settings);
    }

    // Run post-create commands
    if !settings.worktree.post_create_cmd.is_empty() {
        let _ = crate::config::run_post_create_commands(
            &names.worktree_path,
            &settings.worktree.post_create_cmd,
        );
    }

    if try_open_new_terminal_tab(&names.worktree_path) {
        print_success("Opened new terminal tab in worktree directory");
        println!("  {}", path_str);
    } else {
        let shell = if cfg!(target_os = "windows") {
            env::var("SHELL")
                .or_else(|_| env::var("ComSpec"))
                .unwrap_or_else(|_| "powershell.exe".to_string())
        } else {
            env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
        };
        let mut child = Command::new(&shell)
            .current_dir(&names.worktree_path)
            .spawn()
            .map_err(|e| MatError::Io(std::sync::Arc::new(e)))?;

        print_success("Opening new shell in worktree directory...");
        println!("  (Type 'exit' to return to your original directory)");

        child
            .wait()
            .map_err(|e| MatError::Io(std::sync::Arc::new(e)))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HerdrConfig;
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
        let tree_path = format!(
            "{}.worktree/{}-fix/typo",
            repo_dir.to_string_lossy(),
            APP_NAME
        );
        std::fs::create_dir_all(&tree_path).unwrap();

        let mut mock = MockRunner::new();
        mock.add_response("git", &["branch", "--show-current"], ok_output("main\n"));
        mock.add_response(
            "git",
            &["worktree", "add", "-b", "fix/typo", &tree_path, "main"],
            ok_output(""),
        );

        let git = GitClient::new(mock);
        let herdr = HerdrClient::new(MockRunner::new());
        let prev_shell = env::var("SHELL").ok();
        let prev_skip_terminal = env::var("MAT_SKIP_TERMINAL").ok();
        env::set_var("SHELL", "true");
        env::set_var("MAT_SKIP_TERMINAL", "1");

        let settings = crate::config::Settings {
            worktree: crate::config::WorktreeSettings {
                post_create_cmd: vec![],
                ..crate::config::WorktreeSettings::default()
            },
        };
        let app_name = APP_NAME;
        let result = handle_create(
            "fix", "typo", None, false, &config, &settings, &git, &herdr, app_name, &repo_dir,
        );

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

    fn base_settings() -> Settings {
        Settings::default()
    }

    fn clear_test_herdr_env() -> Option<String> {
        let prev = std::env::var("HERDR_ENV").ok();
        std::env::remove_var("HERDR_ENV");
        prev
    }

    fn restore_test_herdr_env(prev: Option<String>) {
        match prev {
            Some(v) => std::env::set_var("HERDR_ENV", v),
            None => std::env::remove_var("HERDR_ENV"),
        }
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

    fn config_herdr(enabled: HerdrMode) -> Config {
        Config {
            herdr: HerdrConfig { enabled },
            ..Config::default()
        }
    }

    fn herdr_create_json(ws_id: &str, branch: &str, path: &str) -> String {
        format!(
            r#"{{"result":{{"workspace":{{"workspace_id":"{}"}},"worktree":{{"branch":"{}","path":"{}"}}}}}}"#,
            ws_id, branch, path
        )
    }

    // ── is_inside_herdr ───────────────────────────────────────────

    #[test]
    fn test_is_inside_herdr_returns_false_when_var_unset() {
        clear_test_herdr_inside();
        let prev_env = clear_test_herdr_env();
        assert!(!is_inside_herdr());
        restore_test_herdr_env(prev_env);
    }

    #[test]
    fn test_is_inside_herdr_returns_true_when_var_is_1() {
        clear_test_herdr_inside();
        let prev_env = clear_test_herdr_env();
        std::env::set_var("HERDR_ENV", "1");
        assert!(is_inside_herdr());
        restore_test_herdr_env(prev_env);
    }

    #[test]
    fn test_is_inside_herdr_returns_false_when_var_is_0() {
        clear_test_herdr_inside();
        let prev_env = clear_test_herdr_env();
        std::env::set_var("HERDR_ENV", "0");
        assert!(!is_inside_herdr());
        restore_test_herdr_env(prev_env);
    }

    // ── Worktree + Herdr workspace path ───────────────────────────

    #[test]
    fn test_worktree_herdr_path_full_flow() {
        set_test_herdr_inside(false);
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_response("git", &["branch", "--show-current"], ok_output("main\n"));
        mock.add_response(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                REPO_DIR,
                "--branch",
                "feat/login",
                "--base",
                "main",
                "--label",
                "app-feat/login",
                "--no-focus",
                "--json",
            ],
            ok_output(&herdr_create_json("w1", "feat/login", &tree_path)),
        );

        let settings = base_settings();
        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr(HerdrMode::Always);

        let result = handle_create(
            "feat",
            "login",
            None,
            false,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_worktree_herdr_uses_source_flag() {
        set_test_herdr_inside(false);
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_response(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                REPO_DIR,
                "--branch",
                "feat/login",
                "--base",
                "develop",
                "--label",
                "app-feat/login",
                "--no-focus",
                "--json",
            ],
            ok_output(&herdr_create_json("w2", "feat/login", &tree_path)),
        );

        let settings = base_settings();
        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr(HerdrMode::Always);

        let result = handle_create(
            "feat",
            "login",
            Some("develop"),
            false,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_worktree_herdr_create_failure() {
        set_test_herdr_inside(false);
        let mut mock = MockRunner::new();
        mock.add_response("git", &["branch", "--show-current"], ok_output("main\n"));
        mock.add_error(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                REPO_DIR,
                "--branch",
                "feat/login",
                "--base",
                "main",
                "--label",
                "app-feat/login",
                "--no-focus",
                "--json",
            ],
            MatError::Herdr {
                command: "herdr worktree create".into(),
                stderr: "server not running".into(),
            },
        );

        let settings = base_settings();
        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr(HerdrMode::Always);

        let result = handle_create(
            "feat",
            "login",
            None,
            false,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            MatError::Herdr { ref stderr, .. } => assert!(stderr.contains("server not running")),
            _ => panic!("expected MatError::Herdr"),
        }
    }

    #[test]
    fn test_worktree_herdr_uses_default_branch_from_config_when_auto_detect_fails() {
        set_test_herdr_inside(false);
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_response("git", &["branch", "--show-current"], ok_output("\n"));
        mock.add_error(
            "git",
            &["symbolic-ref", "refs/remotes/origin/HEAD"],
            MatError::Git {
                command: "git symbolic-ref".into(),
                stderr: "no upstream".into(),
            },
        );
        mock.add_response(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                REPO_DIR,
                "--branch",
                "feat/login",
                "--base",
                "develop",
                "--label",
                "app-feat/login",
                "--no-focus",
                "--json",
            ],
            ok_output(&herdr_create_json("w3", "feat/login", &tree_path)),
        );

        let settings = base_settings();
        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = Config {
            default_branch: "develop".into(),
            herdr: HerdrConfig {
                enabled: HerdrMode::Always,
            },
            ..Config::default()
        };

        let result = handle_create(
            "feat",
            "login",
            None,
            false,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        assert!(result.is_ok());
    }

    // ── No-worktree path ──────────────────────────────────────────

    #[test]
    fn test_no_worktree_with_stash() {
        let mut mock = MockRunner::new();
        mock.add_response(
            "git",
            &["status", "--porcelain"],
            ok_output(" M src/file.rs\n"),
        );
        mock.add_response(
            "git",
            &["stash", "push", "-m", "mat:auto:feat/login"],
            ok_output(""),
        );
        mock.add_response(
            "git",
            &["checkout", "-b", "feat/login", "main"],
            ok_output(""),
        );

        let git = GitClient::new(mock);
        let herdr = HerdrClient::new(MockRunner::new());
        let config = base_config();
        let settings = base_settings();

        let result = handle_create(
            "feat",
            "login",
            Some("main"),
            true,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_worktree_no_uncommitted_changes() {
        let mut mock = MockRunner::new();
        mock.add_response("git", &["status", "--porcelain"], ok_output(""));
        mock.add_response(
            "git",
            &["checkout", "-b", "feat/login", "main"],
            ok_output(""),
        );

        let git = GitClient::new(mock);
        let herdr = HerdrClient::new(MockRunner::new());
        let config = base_config();
        let settings = base_settings();

        let result = handle_create(
            "feat",
            "login",
            Some("main"),
            true,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_worktree_worktree_add_not_called() {
        let mut mock = MockRunner::new();
        mock.add_response("git", &["status", "--porcelain"], ok_output(""));
        mock.add_response(
            "git",
            &["checkout", "-b", "feat/login", "main"],
            ok_output(""),
        );

        let git = GitClient::new(mock);
        let herdr = HerdrClient::new(MockRunner::new());
        let config = base_config();
        let settings = base_settings();

        let result = handle_create(
            "feat",
            "login",
            Some("main"),
            true,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_worktree_stash_not_called_when_clean() {
        let mut mock = MockRunner::new();
        mock.add_response("git", &["status", "--porcelain"], ok_output(""));
        mock.add_response(
            "git",
            &["checkout", "-b", "feat/login", "main"],
            ok_output(""),
        );

        let git = GitClient::new(mock);
        let herdr = HerdrClient::new(MockRunner::new());
        let config = base_config();
        let settings = base_settings();

        let result = handle_create(
            "feat",
            "login",
            Some("main"),
            true,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        assert!(result.is_ok());
    }

    // ── Worktree shell (no-herdr) path ─────────────────────────

    #[test]
    fn test_worktree_shell_path_worktree_add_called_herdr_not_called() {
        let result = run_shell_path(config_herdr(HerdrMode::Never));
        assert!(result.is_ok());
    }

    // ── Herdr mode config ──────────────────────────────────────────

    #[test]
    fn test_herdr_enabled_never_forces_no_herdr() {
        let result = run_shell_path(config_herdr(HerdrMode::Never));
        assert!(result.is_ok());
    }

    #[test]
    fn test_herdr_enabled_always_forces_herdr_even_without_env() {
        set_test_herdr_inside(false);
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_response("git", &["branch", "--show-current"], ok_output("main\n"));
        mock.add_response(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                REPO_DIR,
                "--branch",
                "feat/login",
                "--base",
                "main",
                "--label",
                "app-feat/login",
                "--no-focus",
                "--json",
            ],
            ok_output(&herdr_create_json("w1", "feat/login", &tree_path)),
        );

        let settings = base_settings();
        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr(HerdrMode::Always);

        let result = handle_create(
            "feat",
            "login",
            None,
            false,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_herdr_enabled_always_fails_when_herdr_not_running() {
        set_test_herdr_inside(false);
        let mut mock = MockRunner::new();
        mock.add_response("git", &["branch", "--show-current"], ok_output("main\n"));
        mock.add_error(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                REPO_DIR,
                "--branch",
                "feat/login",
                "--base",
                "main",
                "--label",
                "app-feat/login",
                "--no-focus",
                "--json",
            ],
            MatError::Herdr {
                command: "herdr worktree create".into(),
                stderr: "server not running".into(),
            },
        );

        let settings = base_settings();
        let git = GitClient::new(mock);
        let herdr = HerdrClient::new(MockRunner::new());
        let config = config_herdr(HerdrMode::Always);

        let result = handle_create(
            "feat",
            "login",
            None,
            false,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        assert!(result.is_err());
    }

    // ── Worktree + Herdr tab path (inside herdr) ───────────────

    #[test]
    fn test_worktree_herdr_tab_full_flow() {
        set_test_herdr_inside(true);
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_response("git", &["branch", "--show-current"], ok_output("main\n"));
        mock.add_response(
            "git",
            &["worktree", "add", "-b", "feat/login", &tree_path, "main"],
            ok_output(""),
        );
        mock.add_response(
            "herdr",
            &["pane", "list"],
            ok_output(r#"{"result":{"panes":[{"pane_id":"1-1","workspace_id":"1","focused":true}]}}"#),
        );
        mock.add_response(
            "herdr",
            &["tab", "create", "--workspace", "1", "--label", "app-feat/login"],
            ok_output(r#"{"result":{"tab":{"tab_id":"1:2"},"root_pane":{"pane_id":"1-3"}}}"#),
        );
        mock.add_response(
            "herdr",
            &["pane", "split", "1-3", "--direction", "right", "--no-focus"],
            ok_output(r#"{"result":{"pane":{"pane_id":"1-4"}}}"#),
        );
        mock.add_response(
            "herdr",
            &["pane", "run", "1-4", &format!("cd {} && opencode .", tree_path)],
            ok_output(""),
        );
        mock.add_response(
            "herdr",
            &["pane", "split", "1-3", "--direction", "down", "--no-focus"],
            ok_output(r#"{"result":{"pane":{"pane_id":"1-5"}}}"#),
        );
        mock.add_response(
            "herdr",
            &["pane", "run", "1-3", &format!("cd {} && ll", tree_path)],
            ok_output(""),
        );
        mock.add_response(
            "herdr",
            &["pane", "run", "1-5", &format!("cd {}", tree_path)],
            ok_output(""),
        );
        mock.add_response(
            "herdr",
            &["tab", "focus", "1:2"],
            ok_output(""),
        );

        let settings = base_settings();
        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr(HerdrMode::Always);

        let result = handle_create(
            "feat",
            "login",
            None,
            false,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_worktree_herdr_tab_inside_herdr_only() {
        // When not inside herdr, should NOT call pane/tab methods
        set_test_herdr_inside(false);
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_response("git", &["branch", "--show-current"], ok_output("main\n"));
        mock.add_response(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                REPO_DIR,
                "--branch",
                "feat/login",
                "--base",
                "main",
                "--label",
                "app-feat/login",
                "--no-focus",
                "--json",
            ],
            ok_output(&herdr_create_json("w1", "feat/login", &tree_path)),
        );

        let settings = base_settings();
        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr(HerdrMode::Always);

        let result = handle_create(
            "feat",
            "login",
            None,
            false,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        // Without HERDR_ENV, falls through to handle_worktree_herdr_workspace
        // which needs herdr worktree create mock (set up above)
        assert!(result.is_ok());
    }

    // ── Default branch resolution ─────────────────────────────────

    #[test]
    fn test_default_branch_from_config_when_source_not_given() {
        set_test_herdr_inside(false);
        let mut mock = MockRunner::new();
        let tree_path = wt("chore", "update");
        mock.add_response("git", &["branch", "--show-current"], ok_output("\n"));
        mock.add_error(
            "git",
            &["symbolic-ref", "refs/remotes/origin/HEAD"],
            MatError::Git {
                command: "git symbolic-ref".into(),
                stderr: "no upstream".into(),
            },
        );
        mock.add_response(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                REPO_DIR,
                "--branch",
                "chore/update",
                "--base",
                "develop",
                "--label",
                "app-chore/update",
                "--no-focus",
                "--json",
            ],
            ok_output(&herdr_create_json("w4", "chore/update", &tree_path)),
        );

        let settings = base_settings();
        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = Config {
            default_branch: "develop".into(),
            herdr: HerdrConfig {
                enabled: HerdrMode::Always,
            },
            ..Config::default()
        };

        let result = handle_create(
            "chore",
            "update",
            None,
            false,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        assert!(result.is_ok());
    }

    // ── should_use_herdr ───────────────────────────────────────────

    #[test]
    fn test_should_use_herdr_never() {
        let config = config_herdr(HerdrMode::Never);
        assert!(!should_use_herdr(&config));
    }

    #[test]
    fn test_should_use_herdr_always() {
        let config = config_herdr(HerdrMode::Always);
        assert!(should_use_herdr(&config));
    }

    #[test]
    fn test_should_use_herdr_auto_without_env() {
        set_test_herdr_inside(false);
        let config = config_herdr(HerdrMode::Auto);
        assert!(!should_use_herdr(&config));
    }

    #[test]
    fn test_should_use_herdr_auto_with_env() {
        set_test_herdr_inside(true);
        let config = config_herdr(HerdrMode::Auto);
        assert!(should_use_herdr(&config));
    }

    #[test]
    fn test_uses_current_branch_as_source_when_not_provided() {
        set_test_herdr_inside(false);
        let mut mock = MockRunner::new();
        let tree_path = wt("feat", "login");
        mock.add_response("git", &["branch", "--show-current"], ok_output("develop\n"));
        mock.add_response(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                REPO_DIR,
                "--branch",
                "feat/login",
                "--base",
                "develop",
                "--label",
                "app-feat/login",
                "--no-focus",
                "--json",
            ],
            ok_output(&herdr_create_json("w5", "feat/login", &tree_path)),
        );

        let settings = base_settings();
        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr(HerdrMode::Always);

        let result = handle_create(
            "feat",
            "login",
            None,
            false,
            &config,
            &settings,
            &git,
            &herdr,
            APP_NAME,
            repo(),
        );
        assert!(result.is_ok());
    }
}
