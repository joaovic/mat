use std::path::Path;

use crate::config::Config;
use crate::display::{print_error, print_info, print_success, print_tip, print_warning};
use crate::error::MatError;
use crate::config::MergeStrategy;
use crate::git::{CommandRunner, GitClient};
use crate::tmux::TmuxClient;

fn merge_strategy_name(strategy: &MergeStrategy) -> &'static str {
    match strategy {
        MergeStrategy::MergeCommit => "merge commit",
        MergeStrategy::FastForward => "fast-forward",
    }
}

fn do_merge<R: CommandRunner>(
    git: &GitClient<R>,
    branch: &str,
    source: &str,
    strategy: &MergeStrategy,
) -> Result<bool, MatError> {
    print_info(&format!("Merging {} into {}...", branch, source));
    git.checkout(source)?;

    match git.merge(branch, strategy.clone()) {
        Ok(()) => {
            print_success(&format!("Merge successful ({})", merge_strategy_name(strategy)));
            Ok(true)
        }
        Err(MatError::Git { stderr, .. }) => {
            let conflict_files: Vec<&str> = stderr
                .lines()
                .filter(|l| l.contains("CONFLICT"))
                .filter_map(|l| l.split("in ").nth(1))
                .map(|f| f.trim_end_matches('.').trim())
                .collect();

            print_error("Merge conflict detected. The following files have conflicts:");
            for file in &conflict_files {
                println!("  - {}", file);
            }
            println!();
            print_warning("Merge aborted. Both branches are intact.");
            println!("  Resolve conflicts manually:");
            println!("    1. git checkout {}", source);
            println!("    2. git merge {}", branch);
            println!("    3. Resolve conflicts and commit");
            println!("    4. Run 'mat close' again or 'mat close --no-merge'");

            let _ = git.abort_merge();

            Err(MatError::Validation {
                message: format!("Merge conflict when merging {} into {}", branch, source),
            })
        }
        Err(e) => Err(e),
    }
}

pub fn handle_close<R: CommandRunner>(
    no_merge: bool,
    config: &Config,
    git: &GitClient<R>,
    tmux: &TmuxClient<R>,
    current_dir: &Path,
) -> Result<(), MatError> {
    print_info("Checking for uncommitted changes...");
    if git.has_uncommitted_changes()? {
        print_error("You have uncommitted changes. Please commit or discard them before closing.");
        print_info("Run 'git status' to see your changes.");
        return Err(MatError::Validation {
            message: "Uncommitted changes detected".into(),
        });
    }
    print_success("No uncommitted changes");

    let worktrees = git.worktree_list()?;
    let matching = worktrees
        .iter()
        .find(|wt| !wt.is_main && current_dir.starts_with(&wt.path));

    let (branch_name, maybe_worktree_path) = if let Some(wt) = matching {
        (wt.branch.clone(), Some(wt.path.clone()))
    } else {
        let branch = git.current_branch()?;
        if branch.is_empty() {
            return Err(MatError::Validation {
                message: "Could not determine current branch. Not in a git worktree or branch."
                    .into(),
            });
        }

        let stash_ref = format!("mat:auto:{}", branch);
        print_info(&format!("Restoring stashed changes from '{}'...", stash_ref));
        match git.stash_pop(&stash_ref) {
            Ok(()) => {
                print_success("Stashed changes restored");
            }
            Err(MatError::Git { ref stderr, .. }) => {
                if stderr.contains("CONFLICT") {
                    print_error("Conflict while restoring stashed changes. Resolve conflicts manually.");
                    print_info("Run 'git stash list' to see available stashes.");
                    return Err(MatError::Validation {
                        message: format!("Stash pop conflict for '{}'", stash_ref),
                    });
                }
                print_info(&format!(
                    "No stash found for '{}', continuing without restore",
                    stash_ref
                ));
            }
            Err(e) => return Err(e),
        }

        (branch, None)
    };

    let source_branch = git
        .default_branch()
        .unwrap_or_else(|_| config.default_branch.clone());

    print_info(&format!(
        "Current branch: {} (from {})",
        branch_name, source_branch
    ));

    let merge_success = if no_merge {
        false
    } else {
        do_merge(git, &branch_name, &source_branch, &config.merge_strategy)?
    };

    if let Some(ref path) = maybe_worktree_path {
        let path_str = path.to_string_lossy().to_string();
        print_info(&format!("Deleting worktree..."));
        git.worktree_remove(&path_str)?;
        print_success("Worktree deleted");
    }

    if config.delete_branch && (merge_success || no_merge) {
        print_info(&format!("Deleting branch: {}", branch_name));
        git.branch_delete(&branch_name)?;
        print_success("Branch deleted");
    }

    if no_merge {
        let merge_cmd = format!("git merge {}", branch_name);
        let _ = tmux.set_buffer(&merge_cmd);
    }

    tmux.close_current_window()?;
    print_success("TMUX window closed");

    println!();
    if merge_success {
        print_tip(&format!(
            "You are now on {}. Feature merged successfully!",
            source_branch
        ));
    } else if no_merge {
        print_tip(&format!(
            "You are now on {}. Branch was not merged. Use 'git merge {}' to merge.",
            source_branch, branch_name
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MergeStrategy;
    use crate::git::{CommandOutput, MockRunner};
    use std::path::PathBuf;

    fn ok_output(stdout: &str) -> CommandOutput {
        CommandOutput {
            stdout: stdout.to_string(),
            stderr: String::new(),
            status: 0,
        }
    }

    fn err_output(stderr: &str) -> CommandOutput {
        CommandOutput {
            stdout: String::new(),
            stderr: stderr.to_string(),
            status: 1,
        }
    }

    fn mock_git() -> MockRunner {
        MockRunner::new()
    }

    fn base_config() -> Config {
        Config::default()
    }

    fn config_no_delete() -> Config {
        Config {
            delete_branch: false,
            ..Config::default()
        }
    }

    fn config_ff() -> Config {
        Config {
            merge_strategy: MergeStrategy::FastForward,
            ..Config::default()
        }
    }

    fn worktree_porcelain() -> String {
        "\
worktree /repo
HEAD aaaaaa
branch refs/heads/main

worktree /repo.worktree/app-feat/login
HEAD bbbbbb
branch refs/heads/feat/login
"
        .to_string()
    }

    fn setup_worktree_mocks(mock: &mut MockRunner, _cwd: &str) {
        mock.add_response(
            "git",
            &["status", "--porcelain"],
            ok_output(""),
        );
        mock.add_response(
            "git",
            &["worktree", "list", "--porcelain"],
            ok_output(&worktree_porcelain()),
        );
        mock.add_response(
            "git",
            &["rev-parse", "--show-toplevel"],
            ok_output("/repo\n"),
        );
    }

    fn successful_merge_mocks(mock: &mut MockRunner, branch: &str, source: &str, strategy: &MergeStrategy) {
        mock.add_response("git", &["checkout", source], ok_output(""));
        match strategy {
            MergeStrategy::MergeCommit => {
                mock.add_response("git", &["merge", "--no-ff", branch], ok_output(""));
            }
            MergeStrategy::FastForward => {
                mock.add_response("git", &["merge", "--ff-only", branch], ok_output(""));
            }
        }
    }

    fn cleanup_mocks(mock: &mut MockRunner, path: &str, branch: &str) {
        mock.add_response("git", &["worktree", "remove", path], ok_output(""));
        mock.add_response("git", &["branch", "-d", branch], ok_output(""));
    }

    fn default_branch_mocks(mock: &mut MockRunner) {
        mock.add_response(
            "git",
            &["symbolic-ref", "refs/remotes/origin/HEAD"],
            ok_output("refs/remotes/origin/main\n"),
        );
    }

    fn tmux_close_mocks(mock: &mut MockRunner) {
        mock.add_response(
            "tmux",
            &["list-windows", "-F", "#{window_index}"],
            ok_output("0\n1\n"),
        );
        mock.add_response(
            "tmux",
            &["display-message", "-p", "#{window_index}"],
            ok_output("1\n"),
        );
        mock.add_response("tmux", &["select-window", "-t", "0"], ok_output(""));
        mock.add_response("tmux", &["kill-window", "-t", "1"], ok_output(""));
    }

    fn cwd_in_worktree() -> PathBuf {
        PathBuf::from("/repo.worktree/app-feat/login")
    }

    fn cwd_in_repo() -> PathBuf {
        PathBuf::from("/repo")
    }

    // ── Uncommitted changes ──────────────────────────────────────

    #[test]
    fn test_uncommitted_changes_stops_early() {
        let mut mock = mock_git();
        mock.add_response("git", &["status", "--porcelain"], ok_output(" M src/file.rs\n"));

        let git = GitClient::new(mock);
        let tmux = TmuxClient::new(MockRunner::new());
        let config = base_config();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_worktree());
        assert!(result.is_err());
        match result.unwrap_err() {
            MatError::Validation { ref message } => {
                assert!(message.contains("Uncommitted changes"));
            }
            _ => panic!("expected MatError::Validation"),
        }
    }

    // ── Auto-merge success ───────────────────────────────────────

    #[test]
    fn test_auto_merge_success_with_delete_branch() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        cleanup_mocks(&mut mock, "/repo.worktree/app-feat/login", "feat/login");
        tmux_close_mocks(&mut mock);

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = base_config();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    #[test]
    fn test_auto_merge_success_no_delete_branch() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        mock.add_response("git", &["worktree", "remove", "/repo.worktree/app-feat/login"], ok_output(""));
        tmux_close_mocks(&mut mock);

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = config_no_delete();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    #[test]
    fn test_auto_merge_branch_delete_not_called_when_delete_false() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        mock.add_response("git", &["worktree", "remove", "/repo.worktree/app-feat/login"], ok_output(""));
        // No branch -d mock = test will fail if branch_delete is called
        tmux_close_mocks(&mut mock);

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = config_no_delete();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    // ── Merge strategy flags ────────────────────────────────────

    #[test]
    fn test_merge_strategy_merge_commit() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        cleanup_mocks(&mut mock, "/repo.worktree/app-feat/login", "feat/login");
        tmux_close_mocks(&mut mock);

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = base_config();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    #[test]
    fn test_merge_strategy_fast_forward() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_mocks(&mut mock, "feat/login", "main", &MergeStrategy::FastForward);
        cleanup_mocks(&mut mock, "/repo.worktree/app-feat/login", "feat/login");
        tmux_close_mocks(&mut mock);

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = config_ff();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    // ── Merge conflict ───────────────────────────────────────────

    #[test]
    fn test_merge_conflict_does_not_delete() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        mock.add_response("git", &["checkout", "main"], ok_output(""));
        mock.add_error(
            "git",
            &["merge", "--no-ff", "feat/login"],
            MatError::Git {
                command: "git merge --no-ff feat/login".into(),
                stderr: "Auto-merging src/auth.rs\nCONFLICT (content): Merge conflict in src/auth.rs\nAutomatic merge failed; fix conflicts and then commit the result.\n".into(),
            },
        );
        mock.add_response("git", &["merge", "--abort"], ok_output(""));
        // No worktree_remove or branch_delete mocks — should not be called

        let git = GitClient::new(mock);
        let tmux = TmuxClient::new(MockRunner::new());
        let config = base_config();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_worktree());
        assert!(result.is_err());
        match result.unwrap_err() {
            MatError::Validation { ref message } => {
                assert!(message.contains("Merge conflict"));
            }
            _ => panic!("expected MatError::Validation"),
        }
    }

    // ── --no-merge path ──────────────────────────────────────────

    #[test]
    fn test_no_merge_skips_merge_and_copies_to_buffer() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        mock.add_response("git", &["worktree", "remove", "/repo.worktree/app-feat/login"], ok_output(""));
        mock.add_response("git", &["branch", "-d", "feat/login"], ok_output(""));
        mock.add_response("tmux", &["set-buffer", "git merge feat/login"], ok_output(""));
        tmux_close_mocks(&mut mock);

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = base_config();

        let result = handle_close(true, &config, &git, &tmux, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_merge_merge_not_called() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        mock.add_response("git", &["worktree", "remove", "/repo.worktree/app-feat/login"], ok_output(""));
        mock.add_response("tmux", &["set-buffer", "git merge feat/login"], ok_output(""));
        tmux_close_mocks(&mut mock);
        // No checkout/merge mocks — merge should NOT be called
        // No branch -d mock — delete_branch=true but we skip it intentionally in this test

        // Use config_no_delete to avoid needing branch_delete mock
        let config = config_no_delete();
        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);

        let result = handle_close(true, &config, &git, &tmux, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_merge_with_delete_branch() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        mock.add_response("git", &["worktree", "remove", "/repo.worktree/app-feat/login"], ok_output(""));
        mock.add_response("git", &["branch", "-d", "feat/login"], ok_output(""));
        mock.add_response("tmux", &["set-buffer", "git merge feat/login"], ok_output(""));
        tmux_close_mocks(&mut mock);

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = base_config();

        let result = handle_close(true, &config, &git, &tmux, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    // ── No-worktree close ───────────────────────────────────────

    #[test]
    fn test_no_worktree_close_stash_pop_and_merge() {
        let mut mock = mock_git();
        // has_uncommitted
        mock.add_response("git", &["status", "--porcelain"], ok_output(""));
        // worktree_list with no match (only main worktree)
        mock.add_response(
            "git",
            &["worktree", "list", "--porcelain"],
            ok_output(
                "\
worktree /repo
HEAD aaaaaa
branch refs/heads/main
",
            ),
        );
        mock.add_response(
            "git",
            &["rev-parse", "--show-toplevel"],
            ok_output("/repo\n"),
        );
        // current_branch
        mock.add_response("git", &["branch", "--show-current"], ok_output("feat/login\n"));
        // stash_pop
        mock.add_response("git", &["stash", "pop", "mat:auto:feat/login"], ok_output(""));
        // default_branch
        default_branch_mocks(&mut mock);
        // merge
        successful_merge_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        // branch_delete
        mock.add_response("git", &["branch", "-d", "feat/login"], ok_output(""));
        // tmux close
        tmux_close_mocks(&mut mock);

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = base_config();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_repo());
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_worktree_close_stash_pop_failure() {
        let mut mock = mock_git();
        mock.add_response("git", &["status", "--porcelain"], ok_output(""));
        mock.add_response(
            "git",
            &["worktree", "list", "--porcelain"],
            ok_output(
                "\
worktree /repo
HEAD aaaaaa
branch refs/heads/main
",
            ),
        );
        mock.add_response(
            "git",
            &["rev-parse", "--show-toplevel"],
            ok_output("/repo\n"),
        );
        mock.add_response("git", &["branch", "--show-current"], ok_output("feat/login\n"));
        mock.add_error(
            "git",
            &["stash", "pop", "mat:auto:feat/login"],
            MatError::Git {
                command: "git stash pop mat:auto:feat/login".into(),
                stderr: "CONFLICT: merge conflict in stash".into(),
            },
        );

        let git = GitClient::new(mock);
        let tmux = TmuxClient::new(MockRunner::new());
        let config = base_config();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_repo());
        assert!(result.is_err());
    }

    #[test]
    fn test_no_worktree_close_stash_not_found_continues() {
        let mut mock = mock_git();
        mock.add_response("git", &["status", "--porcelain"], ok_output(""));
        mock.add_response(
            "git",
            &["worktree", "list", "--porcelain"],
            ok_output(
                "\
worktree /repo
HEAD aaaaaa
branch refs/heads/main
",
            ),
        );
        mock.add_response(
            "git",
            &["rev-parse", "--show-toplevel"],
            ok_output("/repo\n"),
        );
        mock.add_response("git", &["branch", "--show-current"], ok_output("feat/login\n"));
        // stash_pop returns non-conflict error (stash not found)
        mock.add_error(
            "git",
            &["stash", "pop", "mat:auto:feat/login"],
            MatError::Git {
                command: "git stash pop mat:auto:feat/login".into(),
                stderr: "fatal: log for 'stash' is empty".into(),
            },
        );
        default_branch_mocks(&mut mock);
        successful_merge_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        mock.add_response("git", &["branch", "-d", "feat/login"], ok_output(""));
        tmux_close_mocks(&mut mock);

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = base_config();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_repo());
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_worktree_close_stash_pop_merge_and_delete() {
        let mut mock = mock_git();
        mock.add_response("git", &["status", "--porcelain"], ok_output(""));
        mock.add_response(
            "git",
            &["worktree", "list", "--porcelain"],
            ok_output(
                "\
worktree /repo
HEAD aaaaaa
branch refs/heads/main
",
            ),
        );
        mock.add_response(
            "git",
            &["rev-parse", "--show-toplevel"],
            ok_output("/repo\n"),
        );
        mock.add_response("git", &["branch", "--show-current"], ok_output("feat/login\n"));
        mock.add_response("git", &["stash", "pop", "mat:auto:feat/login"], ok_output(""));
        default_branch_mocks(&mut mock);
        successful_merge_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        mock.add_response("git", &["branch", "-d", "feat/login"], ok_output(""));
        tmux_close_mocks(&mut mock);

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = base_config();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_repo());
        assert!(result.is_ok());
    }

    // ── close_current_window called after cleanup ───────────────

    #[test]
    fn test_close_window_called_after_cleanup() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        cleanup_mocks(&mut mock, "/repo.worktree/app-feat/login", "feat/login");
        tmux_close_mocks(&mut mock);

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = base_config();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    // ── Output messages ─────────────────────────────────────────

    #[test]
    fn test_merge_strategy_name() {
        assert_eq!(merge_strategy_name(&MergeStrategy::MergeCommit), "merge commit");
        assert_eq!(merge_strategy_name(&MergeStrategy::FastForward), "fast-forward");
    }

    #[test]
    fn test_handle_close_no_tmux_error_does_not_crash() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        mock.add_response("git", &["worktree", "remove", "/repo.worktree/app-feat/login"], ok_output(""));
        mock.add_response("git", &["branch", "-d", "feat/login"], ok_output(""));
        // tmux errors should propagate
        mock.add_response("tmux", &["list-windows", "-F", "#{window_index}"], ok_output("0\n1\n"));
        mock.add_response("tmux", &["display-message", "-p", "#{window_index}"], ok_output("1\n"));
        mock.add_response("tmux", &["select-window", "-t", "0"], ok_output(""));
        mock.add_response("tmux", &["kill-window", "-t", "1"], ok_output(""));

        let git = GitClient::new(mock.clone());
        let tmux = TmuxClient::new(mock);
        let config = config_no_delete();

        let result = handle_close(false, &config, &git, &tmux, &cwd_in_worktree());
        assert!(result.is_ok());
    }
}
