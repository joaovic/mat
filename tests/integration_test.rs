use std::path::{Path, PathBuf};
use std::process::Command;

// ── Helpers ─────────────────────────────────────

/// Create a temporary git repository with initial configuration.
fn setup_git_repo() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let p = dir.path();

    run_git(&["init"], p);
    run_git(&["config", "user.email", "test@test.com"], p);
    run_git(&["config", "user.name", "Test"], p);
    run_git(&["checkout", "-b", "main"], p);
    run_git(&["commit", "--allow-empty", "-m", "Initial commit"], p);

    dir
}

/// Run `git <args>` inside `cwd` and panic on failure.
fn run_git(args: &[&str], cwd: &Path) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("git {} failed: {}", args.join(" "), e));
    assert!(
        output.status.success(),
        "git {} failed:\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Run `git <args>` inside `cwd`, ignoring failure (for cleanup).
fn try_git(args: &[&str], cwd: &Path) {
    let _ = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output();
}

/// Build and return the path to the `mat` binary.
fn mat_binary() -> PathBuf {
    let output = Command::new("cargo")
        .args(["build", "--quiet"])
        .output()
        .expect("failed to run cargo build");
    assert!(
        output.status.success(),
        "cargo build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut path = std::env::current_dir().expect("current dir");
    path.push("target");
    path.push("debug");
    path.push("mat");
    path
}

/// Compute the worktree path that `mat` would create for a given task.
fn worktree_path(repo_path: &Path, app_name: &str, task_type: &str, task_name: &str) -> PathBuf {
    let wt_name = format!("{}-{}/{}", app_name, task_type, task_name);
    let root = format!("{}.worktree", repo_path.to_string_lossy());
    PathBuf::from(root).join(&wt_name)
}

/// Compute the app name from a repo path (matches `naming::get_app_name`).
fn app_name(repo_path: &Path) -> String {
    repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "app".to_string())
}

/// Create a temp dir for XDG_CONFIG_HOME so global config writes are isolated.
fn config_home_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("failed to create config home")
}

/// Run a `mat` subcommand in a test-friendly environment.
fn run_mat(
    args: &[&str],
    cwd: &Path,
    shell: Option<&str>,
    tmux: Option<&str>,
    xdg_config: Option<&Path>,
) -> std::process::Output {
    let binary = mat_binary();
    let mut cmd = Command::new(&binary);
    cmd.args(args).current_dir(cwd);
    if let Some(s) = shell {
        cmd.env("SHELL", s);
    }
    match tmux {
        Some(t) => {
            cmd.env("TMUX", t);
        }
        None => {
            cmd.env_remove("TMUX");
        }
    }
    if let Some(h) = xdg_config {
        cmd.env("XDG_CONFIG_HOME", h);
    }
    cmd.output().unwrap()
}

// ──────────────────────────────────────────────
//  Create flow (worktree + shell path)
// ──────────────────────────────────────────────

#[test]
fn test_create_worktree_shell_path() {
    let repo = setup_git_repo();
    let aname = app_name(repo.path());

    let output = run_mat(
        &["feat", "test-feature"],
        repo.path(),
        Some("true"),
        None,
        None,
    );

    assert!(
        output.status.success(),
        "mat create failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // Verify branch exists
    let branch_out = Command::new("git")
        .args(["branch", "--list", "feat/test-feature"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    let branch_stdout = String::from_utf8_lossy(&branch_out.stdout);
    assert!(
        branch_stdout.contains("feat/test-feature"),
        "Branch feat/test-feature not found in:\n{}",
        branch_stdout
    );

    // Verify worktree was created
    let wt_out = Command::new("git")
        .args(["worktree", "list"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    let wt_stdout = String::from_utf8_lossy(&wt_out.stdout);
    assert!(
        wt_stdout.contains(&aname),
        "Worktree not found in git worktree list:\n{}",
        wt_stdout
    );

    let wt = worktree_path(repo.path(), &aname, "feat", "test-feature");
    cleanup_worktree(repo.path(), &wt);
}

#[test]
fn test_create_worktree_naming_different_types() {
    let repo = setup_git_repo();
    let aname = app_name(repo.path());

    let out1 = run_mat(&["feat", "login"], repo.path(), Some("true"), None, None);
    assert!(
        out1.status.success(),
        "first create failed: {}",
        String::from_utf8_lossy(&out1.stderr)
    );

    let out2 = run_mat(&["fix", "login"], repo.path(), Some("true"), None, None);
    assert!(
        out2.status.success(),
        "second create failed: {}",
        String::from_utf8_lossy(&out2.stderr)
    );

    let wt1 = worktree_path(repo.path(), &aname, "feat", "login");
    let wt2 = worktree_path(repo.path(), &aname, "fix", "login");
    assert_ne!(wt1, wt2, "different types should produce different worktree paths");
    assert!(wt1.exists(), "feat worktree not at {}", wt1.display());
    assert!(wt2.exists(), "fix worktree not at {}", wt2.display());

    // git worktree list should show both branches
    let wt_out = Command::new("git")
        .args(["worktree", "list"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    let wt_stdout = String::from_utf8_lossy(&wt_out.stdout);
    assert!(wt_stdout.contains("feat/login"), "feat/login not in worktree list:\n{}", wt_stdout);
    assert!(wt_stdout.contains("fix/login"), "fix/login not in worktree list:\n{}", wt_stdout);

    cleanup_worktree(repo.path(), &wt1);
    cleanup_worktree(repo.path(), &wt2);
}

#[test]
fn test_create_no_worktree() {
    let repo = setup_git_repo();

    let output = run_mat(&["--no-worktree", "fix", "test-bug"], repo.path(), None, None, None);

    assert!(
        output.status.success(),
        "mat --no-worktree failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify branch exists
    let branch_out = Command::new("git")
        .args(["branch", "--list", "fix/test-bug"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    let branch_stdout = String::from_utf8_lossy(&branch_out.stdout);
    assert!(
        branch_stdout.contains("fix/test-bug"),
        "Branch fix/test-bug not found:\n{}",
        branch_stdout
    );

    // Verify no additional worktree (only main)
    let wt_out = Command::new("git")
        .args(["worktree", "list"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    let wt_lines = String::from_utf8_lossy(&wt_out.stdout).lines().count();
    assert_eq!(wt_lines, 1, "Expected only main worktree, got:\n{}", String::from_utf8_lossy(&wt_out.stdout));

    // Cleanup (use --force because the branch may still be checked out)
    run_git(&["checkout", "main"], repo.path());
    try_git(&["branch", "-D", "fix/test-bug"], repo.path());
}

#[test]
fn test_create_no_worktree_with_uncommitted_changes() {
    let repo = setup_git_repo();

    // Create a tracked file with modifications
    std::fs::write(repo.path().join("tracked.txt"), "initial").unwrap();
    run_git(&["add", "."], repo.path());
    run_git(&["commit", "-m", "Add tracked.txt"], repo.path());
    std::fs::write(repo.path().join("tracked.txt"), "modified").unwrap();

    let output = run_mat(
        &["--no-worktree", "fix", "bug-fix"],
        repo.path(),
        None,
        None,
        None,
    );

    assert!(
        output.status.success(),
        "mat create failed with uncommitted changes:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify branch exists
    let branch_out = Command::new("git")
        .args(["branch", "--list", "fix/bug-fix"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&branch_out.stdout).contains("fix/bug-fix"),
        "Branch fix/bug-fix not found"
    );

    // Verify stash was created with the tracked change
    let stash_out = Command::new("git")
        .args(["stash", "list"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    let stash_stdout = String::from_utf8_lossy(&stash_out.stdout);
    assert!(
        stash_stdout.contains("mat:auto:fix/bug-fix"),
        "Stash not found:\n{}",
        stash_stdout
    );

    // Cleanup
    run_git(&["checkout", "main"], repo.path());
    try_git(&["branch", "-D", "fix/bug-fix"], repo.path());
    try_git(&["stash", "drop"], repo.path());
}

// ──────────────────────────────────────────────
//  Close flow (no-worktree mode)
//
//  NOTE: Close-from-worktree is not tested because
//  `git checkout <source>` fails from within a worktree
//  when the source branch is checked out in the main repo.
//  The no-worktree path avoids this by staying in the
//  main repo.
// ──────────────────────────────────────────────

#[test]
fn test_close_no_worktree_auto_merge() {
    let repo = setup_git_repo();

    // Create a branch with --no-worktree
    let out = run_mat(
        &["--no-worktree", "feat", "nw-merge"],
        repo.path(),
        None,
        None,
        None,
    );
    assert!(out.status.success(), "create failed: {}", String::from_utf8_lossy(&out.stderr));

    // Make a change and commit on the feature branch
    std::fs::write(repo.path().join("work.txt"), "data").unwrap();
    run_git(&["add", "."], repo.path());
    run_git(&["commit", "-m", "Feature work"], repo.path());

    // Close from repo (no-worktree path) — tmux is gracefully skipped when
    // not inside a tmux session, so the command should succeed.
    let close_out = run_mat(&["close"], repo.path(), None, None, None);

    // Verify merge happened: the commit should be in main
    let log_out = Command::new("git")
        .args(["log", "--oneline", "main"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    let log_stdout = String::from_utf8_lossy(&log_out.stdout);
    assert!(
        log_stdout.contains("Feature work"),
        "Merge commit not found in main log:\n{}",
        log_stdout
    );

    // Verify branch was deleted (default: delete_branch=true)
    let branch_out = Command::new("git")
        .args(["branch", "--list", "feat/nw-merge"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    let branch_stdout = String::from_utf8_lossy(&branch_out.stdout);
    assert!(
        !branch_stdout.contains("feat/nw-merge"),
        "Branch should have been deleted:\n{}",
        branch_stdout
    );

    // Close should succeed (tmux is skipped when not in a tmux session)
    assert!(
        close_out.status.success(),
        "close should succeed when tmux is not active: {}",
        String::from_utf8_lossy(&close_out.stderr)
    );
}

#[test]
fn test_close_no_worktree_no_merge() {
    let repo = setup_git_repo();

    let out = run_mat(
        &["--no-worktree", "feat", "nw-nomerge"],
        repo.path(),
        None,
        None,
        None,
    );
    assert!(out.status.success(), "create failed: {}", String::from_utf8_lossy(&out.stderr));

    std::fs::write(repo.path().join("work.txt"), "data").unwrap();
    run_git(&["add", "."], repo.path());
    run_git(&["commit", "-m", "Feature work"], repo.path());

    let close_out = run_mat(&["close", "--no-merge"], repo.path(), None, None, None);

    // Merge should NOT have happened
    let log_out = Command::new("git")
        .args(["log", "--oneline", "main"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    let log_stdout = String::from_utf8_lossy(&log_out.stdout);
    assert!(
        !log_stdout.contains("Feature work"),
        "Merge should not have occurred with --no-merge:\n{}",
        log_stdout
    );

    // With no-worktree + --no-merge, the close flow attempts to delete
    // the currently checked-out branch without switching away first,
    // which git forbids. The command fails with a git error (not tmux).
    assert!(
        !close_out.status.success(),
        "close --no-merge should fail (cannot delete checked-out branch)"
    );
    let stderr = String::from_utf8_lossy(&close_out.stderr);
    assert!(
        stderr.contains("cannot delete") || stderr.contains("branch"),
        "expected branch deletion error:\n{}",
        stderr
    );

    // Cleanup (we are still on feat/nw-nomerge)
    run_git(&["checkout", "main"], repo.path());
    try_git(&["branch", "-D", "feat/nw-nomerge"], repo.path());
}

#[test]
fn test_close_no_worktree_with_uncommitted_changes() {
    let repo = setup_git_repo();

    let out = run_mat(
        &["--no-worktree", "feat", "nw-dirty"],
        repo.path(),
        None,
        None,
        None,
    );
    assert!(out.status.success(), "create failed");

    // Dirty the working tree without committing
    std::fs::write(repo.path().join("tracked.txt"), "initial").unwrap();
    run_git(&["add", "."], repo.path());
    run_git(&["commit", "-m", "Add tracked"], repo.path());
    std::fs::write(repo.path().join("tracked.txt"), "dirty").unwrap();

    // Close should fail with uncommitted changes error (before tmux)
    let close_out = run_mat(&["close"], repo.path(), None, None, None);

    assert!(
        !close_out.status.success(),
        "close should fail with uncommitted changes"
    );
    let stderr = String::from_utf8_lossy(&close_out.stderr);
    assert!(
        stderr.contains("uncommitted") || stderr.contains("Uncommitted"),
        "Expected uncommitted changes error:\n{}",
        stderr
    );

    // Cleanup: discard dirty changes first, then switch back
    try_git(&["checkout", "--", "."], repo.path());
    run_git(&["checkout", "main"], repo.path());
    try_git(&["branch", "-D", "feat/nw-dirty"], repo.path());
    try_git(&["stash", "drop"], repo.path());
}

#[test]
fn test_close_no_worktree_merge_conflict() {
    let repo = setup_git_repo();

    // Create a base file
    std::fs::write(repo.path().join("shared.txt"), "base").unwrap();
    run_git(&["add", "."], repo.path());
    run_git(&["commit", "-m", "Add shared.txt"], repo.path());

    // Create feature branch and make a conflicting change
    let out = run_mat(
        &["--no-worktree", "feat", "nw-conflict"],
        repo.path(),
        None,
        None,
        None,
    );
    assert!(out.status.success(), "create failed");

    std::fs::write(repo.path().join("shared.txt"), "feature change").unwrap();
    run_git(&["add", "."], repo.path());
    run_git(&["commit", "-m", "Feature change"], repo.path());

    // Switch to main and make a conflicting change
    run_git(&["checkout", "main"], repo.path());
    std::fs::write(repo.path().join("shared.txt"), "main change").unwrap();
    run_git(&["add", "."], repo.path());
    run_git(&["commit", "-m", "Main change"], repo.path());

    // Switch back to feature branch
    run_git(&["checkout", "feat/nw-conflict"], repo.path());

    // Close should fail with merge conflict (before tmux)
    let close_out = run_mat(&["close"], repo.path(), None, None, None);

    assert!(
        !close_out.status.success(),
        "close should fail with merge conflict"
    );
    let stderr = String::from_utf8_lossy(&close_out.stderr);
    assert!(
        stderr.to_lowercase().contains("conflict")
            || stderr.contains("CONFLICT"),
        "Expected conflict error:\n{}",
        stderr
    );

    // Cleanup: restore main
    run_git(&["checkout", "main"], repo.path());
    try_git(&["branch", "-D", "feat/nw-conflict"], repo.path());
    run_git(&["reset", "--hard", "HEAD~1"], repo.path()); // undo the main conflicting commit
}

// ──────────────────────────────────────────────
//  Config commands
// ──────────────────────────────────────────────

#[test]
fn test_config_set_and_get_project() {
    let repo = setup_git_repo();

    let output = run_mat(
        &["config", "set", "merge_strategy", "fast-forward"],
        repo.path(),
        None,
        None,
        None,
    );
    assert!(
        output.status.success(),
        "config set failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let get_out = run_mat(
        &["config", "get", "merge_strategy"],
        repo.path(),
        None,
        None,
        None,
    );
    assert!(get_out.status.success(), "config get failed");
    let stdout = String::from_utf8_lossy(&get_out.stdout);
    assert!(
        stdout.contains("fast-forward"),
        "Expected fast-forward:\n{}",
        stdout
    );
    assert!(
        stdout.contains("project"),
        "Expected project source annotation:\n{}",
        stdout
    );

    let _ = std::fs::remove_file(repo.path().join(".mat.toml"));
}

#[test]
fn test_config_list() {
    let repo = setup_git_repo();

    let output = run_mat(&["config", "list"], repo.path(), None, None, None);
    assert!(output.status.success(), "config list failed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("default_branch"), "Missing default_branch:\n{}", stdout);
    assert!(stdout.contains("delete_branch"), "Missing delete_branch:\n{}", stdout);
    assert!(stdout.contains("merge_strategy"), "Missing merge_strategy:\n{}", stdout);
    assert!(stdout.contains("worktree_root"), "Missing worktree_root:\n{}", stdout);
    assert!(stdout.contains("tmux.enabled"), "Missing tmux.enabled:\n{}", stdout);
    assert!(stdout.contains("(default)"), "Expected default annotations:\n{}", stdout);
}

#[test]
fn test_config_get_unknown_key() {
    let repo = setup_git_repo();

    let output = run_mat(&["config", "get", "nonexistent"], repo.path(), None, None, None);
    assert!(!output.status.success(), "get nonexistent should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("nonexistent"), "Expected error about unknown key:\n{}", stderr);
}

#[test]
fn test_config_set_global_isolated() {
    let repo = setup_git_repo();
    let config_home = config_home_dir();

    let output = run_mat(
        &["config", "set", "--global", "merge_strategy", "fast-forward"],
        repo.path(),
        None,
        None,
        Some(config_home.path()),
    );
    assert!(
        output.status.success(),
        "global config set failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let list_out = run_mat(
        &["config", "list"],
        repo.path(),
        None,
        None,
        Some(config_home.path()),
    );
    assert!(list_out.status.success(), "config list failed");
    let stdout = String::from_utf8_lossy(&list_out.stdout);
    assert!(
        stdout.contains("fast-forward"),
        "Expected fast-forward in list:\n{}",
        stdout
    );
}

// ──────────────────────────────────────────────
//  Error scenarios
// ──────────────────────────────────────────────

#[test]
fn test_create_outside_git_repo() {
    let outside = tempfile::TempDir::new().expect("failed to create temp dir");

    let output = run_mat(&["feat", "login"], outside.path(), None, None, None);
    assert!(
        !output.status.success(),
        "create outside git repo should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("git") || stderr.contains("not a git repository"),
        "Expected git error:\n{}",
        stderr
    );
}

// ──────────────────────────────────────────────
//  TMUX detection
// ──────────────────────────────────────────────

#[test]
fn test_tmux_unset_uses_shell_path() {
    let repo = setup_git_repo();
    let aname = app_name(repo.path());

    let output = run_mat(
        &["feat", "shell-path"],
        repo.path(),
        Some("true"),
        None,
        None,
    );
    assert!(
        output.status.success(),
        "create should succeed without TMUX:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let wt = worktree_path(repo.path(), &aname, "feat", "shell-path");
    assert!(wt.exists(), "Worktree not created:\n{}", wt.display());
    cleanup_worktree(repo.path(), &wt);
}

#[test]
fn test_tmux_set_tries_tmux_and_fails() {
    let repo = setup_git_repo();
    let aname = app_name(repo.path());

    let output = run_mat(
        &["feat", "tmux-fail"],
        repo.path(),
        None,
        Some("/tmp/tmux-fake"),
        None,
    );
    assert!(
        !output.status.success(),
        "create should fail with TMUX set but no tmux server"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("tmux"),
        "Expected tmux error:\n{}",
        stderr
    );

    // Cleanup: worktree was created but tmux failed
    let wt = worktree_path(repo.path(), &aname, "feat", "tmux-fail");
    if wt.exists() {
        cleanup_worktree(repo.path(), &wt);
    }
    try_git(&["branch", "-D", "feat/tmux-fail"], repo.path());
}

// ── Shared cleanup ────────────────────────────

fn cleanup_worktree(repo_path: &Path, wt_path: &Path) {
    try_git(&["worktree", "remove", "--force", &wt_path.to_string_lossy()], repo_path);
    try_git(&["worktree", "prune"], repo_path);
    if wt_path.exists() {
        let _ = std::fs::remove_dir_all(wt_path);
    }
    try_git(&["branch", "-D", &wt_path.file_name().unwrap_or_default().to_string_lossy().replace("-feat/", "/").replace("-fix/", "/")], repo_path);
}

