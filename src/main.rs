use clap::{Parser, ValueHint};
use console::Style;
use std::env;
use std::path::PathBuf;
use std::process::Command;

const BRANCHLET_SETTINGS: &str = ".branchlet/settings.json";

#[derive(Parser)]
#[command(name = "mat")]
#[command(version = "0.1.1")]
#[command(about = "Multi-Agent Task - Create TMUX window + Git worktree for new features", long_about = None)]
struct Cli {
    #[arg(help = "Task type (e.g., feat, fix, chore, refactor)", value_hint = ValueHint::Other)]
    task_type: Option<String>,

    #[arg(help = "Task name (e.g., increase-counter)", value_hint = ValueHint::Other)]
    task_name: Option<String>,

    #[arg(short, long, help = "Base branch to create worktree from")]
    source: Option<String>,

    #[arg(short, long, help = "Close the current task worktree")]
    close: bool,
}

fn print_error(msg: &str) {
    let red = Style::new().red().bold();
    eprintln!("{} {}", red.apply_to("ERROR:"), msg);
}

fn print_success(msg: &str) {
    let green = Style::new().green().bold();
    println!("{} {}", green.apply_to("✓"), msg);
}

fn print_info(msg: &str) {
    let blue = Style::new().cyan().bold();
    println!("{} {}", blue.apply_to("ℹ"), msg);
}

fn print_tip(msg: &str) {
    let yellow = Style::new().yellow().bold();
    println!("{} {}", yellow.apply_to("💡"), msg);
}

fn check_command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_tmux_running() -> bool {
    Command::new("tmux")
        .args(["list-sessions"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn get_current_branch() -> Result<String, String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if !output.status.success() {
        return Err("Not on a git branch".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_branchlet_settings_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(BRANCHLET_SETTINGS))
        .unwrap_or_else(|| PathBuf::from(BRANCHLET_SETTINGS))
}

fn check_branchlet_config() -> bool {
    get_branchlet_settings_path().exists()
}

fn get_tmux_prefix() -> String {
    let output = Command::new("tmux")
        .args(["show-options", "-g", "prefix"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let output = String::from_utf8_lossy(&o.stdout);
            output
                .trim()
                .strip_prefix("prefix ")
                .map(|s| s.replace("C-", "Ctrl-"))
                .unwrap_or_else(|| "Ctrl-b".to_string())
        }
        _ => "Ctrl-b".to_string(),
    }
}

fn check_uncommitted_changes() -> Result<bool, String> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    let status_output = String::from_utf8_lossy(&output.stdout);
    Ok(!status_output.trim().is_empty())
}

fn get_worktree_info() -> Result<(String, String, String), String> {
    let output = Command::new("branchlet")
        .args(["list", "--json"])
        .output()
        .map_err(|e| format!("Failed to execute branchlet: {}", e))?;

    if !output.status.success() {
        return Err("Failed to list branchlet worktrees".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let worktrees: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse branchlet output: {}", e))?;

    let current_path =
        env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    for worktree in worktrees {
        let worktree_path = worktree["path"]
            .as_str()
            .ok_or("Missing path in worktree")?;
        let path_buf = PathBuf::from(worktree_path);
        let name = path_buf
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or("Missing name in worktree")?;

        if current_path.starts_with(worktree_path)
            || worktree_path == current_path.to_string_lossy()
        {
            let branch = worktree["branch"]
                .as_str()
                .ok_or("Missing branch in worktree")?;
            let source = worktree["source"].as_str().unwrap_or("main");
            return Ok((branch.to_string(), source.to_string(), name));
        }
    }

    Err("Not in a Branchlet worktree. Run this command from within a worktree.".to_string())
}

fn delete_worktree(worktree_name: &str) -> Result<(), String> {
    let output = Command::new("branchlet")
        .args(["delete", "-n", worktree_name])
        .output()
        .map_err(|e| format!("Failed to execute branchlet: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to delete worktree: {}", stderr));
    }

    Ok(())
}

fn close_current_tmux_window() -> Result<String, String> {
    let output = Command::new("tmux")
        .args(["list-windows", "-F", "#{window_index}"])
        .output()
        .map_err(|e| format!("Failed to list TMUX windows: {}", e))?;

    let windows = String::from_utf8_lossy(&output.stdout);
    let window_count = windows.lines().count();

    let current_window_output = Command::new("tmux")
        .args(["display-message", "-p", "#{window_index}"])
        .output()
        .map_err(|e| format!("Failed to get current window: {}", e))?;

    let current_window = String::from_utf8_lossy(&current_window_output.stdout)
        .trim()
        .to_string();

    let switch_target = if window_count > 1 { "0" } else { "1" };

    let switch_output = Command::new("tmux")
        .args(["select-window", "-t", switch_target])
        .output()
        .map_err(|e| format!("Failed to switch TMUX window: {}", e))?;

    if !switch_output.status.success() {
        return Err("Failed to switch to target window".to_string());
    }

    let kill_output = Command::new("tmux")
        .args(["kill-window", "-t", &current_window])
        .output();

    match kill_output {
        Ok(o) if o.status.success() => Ok(switch_target.to_string()),
        Ok(_) => Ok(switch_target.to_string()),
        Err(_) => Ok(switch_target.to_string()),
    }
}

fn send_message_to_tmux_window(window: &str, message: &str) -> Result<(), String> {
    let output = Command::new("tmux")
        .args(["display-message", "-t", window, message])
        .output()
        .map_err(|e| format!("Failed to send message to TMUX window: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to send message: {}", stderr));
    }

    Ok(())
}

fn prepare_merge_command(branch_name: &str) -> String {
    format!(
        "git merge {} --no-ff -m \"Merge {}\"",
        branch_name, branch_name
    )
}

fn run_prerequisite_checks() -> Result<(), String> {
    print_info("Running prerequisite checks...");

    if !check_tmux_running() {
        return Err("TMUX is not running. Please start a TMUX session first.".to_string());
    }
    print_success("TMUX is running");

    if !check_command_exists("branchlet") {
        return Err("Branchlet is not installed or not in PATH.".to_string());
    }
    print_success("Branchlet is installed");

    if !check_git_repo() {
        return Err("Current directory is not a git repository.".to_string());
    }
    print_success("Current directory is a git repository");

    if !check_branchlet_config() {
        let path = get_branchlet_settings_path();
        return Err(format!(
            "Branchlet config not found at: {}. Please run 'branchlet settings' first.",
            path.display()
        ));
    }
    print_success("Branchlet config exists");

    Ok(())
}

fn get_app_name() -> String {
    env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "app".to_string())
}

fn main() {
    let cli = Cli::parse();

    if cli.close {
        handle_close_mode();
    } else {
        handle_create_mode(&cli);
    }
}

fn handle_close_mode() {
    if let Err(e) = run_prerequisite_checks() {
        print_error(&e);
        std::process::exit(1);
    }

    print_info("Checking for uncommitted changes...");

    match check_uncommitted_changes() {
        Ok(true) => {
            print_error(
                "You have uncommitted changes. Please commit or discard them before closing.",
            );
            print_info("Run 'git status' to see your changes.");
            std::process::exit(1);
        }
        Ok(false) => {
            print_success("No uncommitted changes");
        }
        Err(e) => {
            print_error(&format!("Failed to check git status: {}", e));
            std::process::exit(1);
        }
    }

    print_info("Getting worktree info...");

    let (branch_name, source_branch, worktree_name) = match get_worktree_info() {
        Ok(info) => info,
        Err(e) => {
            print_error(&e);
            std::process::exit(1);
        }
    };

    print_success(&format!("Current branch: {}", branch_name));
    print_success(&format!("Source branch: {}", source_branch));

    print_info(&format!("Deleting worktree: {}", worktree_name));

    if let Err(e) = delete_worktree(&worktree_name) {
        print_error(&e);
        std::process::exit(1);
    }

    print_success("Worktree deleted");

    let merge_command = prepare_merge_command(&branch_name);

    let tmux_set_buffer = Command::new("tmux")
        .args(["set-buffer", &merge_command])
        .output();

    match tmux_set_buffer {
        Ok(o) if o.status.success() => {
            print_success("Merge command copied to TMUX buffer");
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            print_error(&format!("Failed to copy to TMUX buffer: {}", stderr));
        }
        Err(e) => {
            print_error(&format!("Failed to execute tmux set-buffer: {}", e));
        }
    }

    let prefix = get_tmux_prefix();
    let tip_message = format!(
        "You are now ready to merge your feature! Press {} then ] to paste the merge command. Merge command: {}",
        prefix, merge_command
    );

    let _ = send_message_to_tmux_window("0", &tip_message);

    print_info("Closing TMUX window...");

    let _target_window = match close_current_tmux_window() {
        Ok(w) => w,
        Err(e) => {
            print_error(&e);
            std::process::exit(1);
        }
    };

    print_success("TMUX window closed");
}

fn handle_create_mode(cli: &Cli) {
    let task_type = match &cli.task_type {
        Some(t) => t,
        None => {
            print_error("Task type is required when not using --close");
            std::process::exit(1);
        }
    };

    let task_name = match &cli.task_name {
        Some(n) => n,
        None => {
            print_error("Task name is required when not using --close");
            std::process::exit(1);
        }
    };

    if let Err(e) = run_prerequisite_checks() {
        print_error(&e);
        std::process::exit(1);
    }

    let app_name = get_app_name();
    let base_branch = cli
        .source
        .clone()
        .unwrap_or_else(|| get_current_branch().unwrap_or_else(|_| "main".to_string()));

    let window_name = format!("{}-{}/{}", app_name, task_type, task_name);
    let branch_name = format!("{}/{}", task_type, task_name);
    let worktree_name = format!("{}-{}", app_name, task_name);

    print_info(&format!(
        "Creating worktree: name={}, source={}, branch={}",
        worktree_name, base_branch, branch_name
    ));

    let branchlet_output = Command::new("branchlet")
        .args([
            "create",
            "-n",
            &worktree_name,
            "-s",
            &base_branch,
            "-b",
            &branch_name,
        ])
        .output()
        .expect("Failed to execute branchlet");

    if !branchlet_output.status.success() {
        let stderr = String::from_utf8_lossy(&branchlet_output.stderr);
        print_error(&format!("Branchlet failed: {}", stderr));
        std::process::exit(1);
    }

    let stdout = String::from_utf8_lossy(&branchlet_output.stdout);
    let worktree_path = stdout.trim().to_string();

    if worktree_path.is_empty() {
        print_error("Branchlet did not return a worktree path");
        std::process::exit(1);
    }

    print_success(&format!("Worktree created at: {}", worktree_path));

    let tmux_new_window = Command::new("tmux")
        .args(["new-window", "-c", &worktree_path])
        .output();

    match tmux_new_window {
        Ok(o) if o.status.success() => {
            print_success("TMUX window created");
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            print_error(&format!("Failed to create TMUX window: {}", stderr));
            std::process::exit(1);
        }
        Err(e) => {
            print_error(&format!("Failed to execute tmux: {}", e));
            std::process::exit(1);
        }
    }

    let tmux_rename = Command::new("tmux")
        .args(["rename-window", &window_name])
        .output();

    match tmux_rename {
        Ok(o) if o.status.success() => {
            print_success(&format!("Window renamed to: {}", window_name));
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            print_error(&format!("Failed to rename TMUX window: {}", stderr));
            std::process::exit(1);
        }
        Err(e) => {
            print_error(&format!("Failed to execute tmux rename: {}", e));
            std::process::exit(1);
        }
    }

    let cd_command = format!("cd {}", worktree_path);

    let tmux_set_buffer = Command::new("tmux")
        .args(["set-buffer", &cd_command])
        .output();

    match tmux_set_buffer {
        Ok(o) if o.status.success() => {
            print_success("CD command copied to TMUX buffer");
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            print_error(&format!("Failed to copy to TMUX buffer: {}", stderr));
        }
        Err(e) => {
            print_error(&format!("Failed to execute tmux set-buffer: {}", e));
        }
    }

    println!();
    print_success(&format!(
        "Ready! Window '{}' is now open at: {}",
        window_name, worktree_path
    ));

    println!();
    print_tip("To cd into the new worktree from other TMUX panels:");
    let prefix = get_tmux_prefix();
    println!("  Press {} then ] to paste the cd command", prefix);
}
