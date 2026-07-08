use std::env;
use std::path::Path;

use crate::config::{Config, HerdrMode};
use crate::display::{print_error, print_info, print_success, print_tip, print_warning};
use crate::error::MatError;
use crate::config::MergeStrategy;
use crate::git::{CommandRunner, GitClient};
use crate::naming;
use crate::herdr::HerdrClient;

fn should_use_herdr(config: &Config) -> bool {
    match config.herdr.enabled {
        HerdrMode::Never => false,
        HerdrMode::Always => true,
        HerdrMode::Auto => false,
    }
}

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
    main_worktree: Option<&str>,
) -> Result<bool, MatError> {
    print_info(&format!("Merging {} into {}...", branch, source));

    let merge_result = match main_worktree {
        Some(path) => git.merge_from(branch, strategy.clone(), path),
        None => {
            git.checkout(source)?;
            git.merge(branch, strategy.clone())
        }
    };

    match merge_result {
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

            match main_worktree {
                Some(path) => {
                    let _ = git.abort_merge_from(path);
                }
                None => {
                    let _ = git.abort_merge();
                }
            }

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
    herdr: &HerdrClient<R>,
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

    let source_branch = {
        let mat_source_key = format!("branch.{}.mat-source", branch_name);
        git.config_get(&mat_source_key)
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                git.default_branch()
                    .unwrap_or_else(|_| config.default_branch.clone())
            })
    };

    print_info(&format!(
        "Current branch: {} (from {})",
        branch_name, source_branch
    ));

    let main_worktree_path = worktrees
        .iter()
        .find(|wt| wt.is_main)
        .map(|wt| naming::normalize_path(&wt.path));

    // Step 1: Change to main worktree directory first (so shell ends up in valid directory)
    if let Some(ref main_path) = main_worktree_path {
        if let Err(e) = env::set_current_dir(main_path) {
            print_error(&format!("Failed to change to main directory: {}", e));
        }
    }

    // Step 2: Checkout source branch
    print_info(&format!("Checking out {}...", source_branch));
    git.checkout(&source_branch)?;
    print_success(&format!("Switched to {}", source_branch));

    // Step 3: Merge the task branch into source
    let merge_success = if no_merge {
        false
    } else {
        do_merge(
            git,
            &branch_name,
            &source_branch,
            &config.merge_strategy,
            None,
        )?
    };

    // Step 4: Close herdr workspace (before worktree remove — releases Windows worktree lock)
    let use_herdr = should_use_herdr(config);

    if use_herdr {
        let path_str = naming::normalize_path(&current_dir);
        if let Ok(Some(ws_id)) = herdr.find_workspace_by_path(&path_str) {
            herdr.close_workspace(&ws_id)?;
            print_success("Herdr workspace closed");
        }
    }

    // Step 5: Change to original directory to release worktree lock, then delete worktree
    if let Some(ref path) = maybe_worktree_path {
        // Restore original cwd (where `mat create` was issued) to avoid Windows worktree lock
        let cwd_key = format!("branch.{}.mat-cwd", branch_name);
        if let Ok(original_cwd) = git.config_get(&cwd_key) {
            if !original_cwd.is_empty() {
                if let Err(e) = env::set_current_dir(&original_cwd) {
                    print_error(&format!("Failed to change to original directory: {}", e));
                }
            }
        }

        let path_str = naming::normalize_path(path);
        print_info(&format!("Deleting worktree..."));
        git.worktree_remove(&path_str)?;
        print_success("Worktree deleted");
    }

    // Step 6: Delete the task branch
    if config.delete_branch && (merge_success || no_merge) {
        print_info(&format!("Deleting branch: {}", branch_name));
        git.branch_delete(&branch_name)?;
        print_success("Branch deleted");
    }

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

    if maybe_worktree_path.is_some() && !use_herdr {
        print_tip("Worktree directory deleted. Type 'exit' to close this tab.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{HerdrConfig, MergeStrategy};
    use crate::git::{CommandOutput, MockRunner};
    use std::path::PathBuf;

    fn ok_output(stdout: &str) -> CommandOutput {
        CommandOutput {
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    fn err_output(stderr: &str) -> CommandOutput {
        CommandOutput {
            stdout: String::new(),
            stderr: stderr.to_string(),
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
            &["rev-parse", "--git-common-dir"],
            ok_output("/repo/.git\n"),
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

    fn successful_merge_from_mocks(mock: &mut MockRunner, branch: &str, source: &str, strategy: &MergeStrategy) {
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
        mock.add_response(
            "git",
            &["config", "--get", &format!("branch.{}.mat-cwd", branch)],
            ok_output("/original/cwd\n"),
        );
        mock.add_response("git", &["worktree", "remove", path], ok_output(""));
        mock.add_response("git", &["branch", "-d", branch], ok_output(""));
    }

    fn default_branch_mocks(mock: &mut MockRunner) {
        mock.add_error(
            "git",
            &["config", "--get", "branch.feat/login.mat-source"],
            MatError::Git {
                command: "git config --get branch.feat/login.mat-source".into(),
                stderr: "error: key not found\n".into(),
            },
        );
        mock.add_response(
            "git",
            &["symbolic-ref", "refs/remotes/origin/HEAD"],
            ok_output("refs/remotes/origin/main\n"),
        );
    }

    fn herdr_close_mocks(mock: &mut MockRunner, path: &str, ws_id: &str) {
        mock.add_response(
            "herdr",
            &["workspace", "list"],
            ok_output(&format!("{}\t{}\tlabel", ws_id, path)),
        );
        mock.add_response(
            "herdr",
            &["workspace", "close", ws_id],
            ok_output(""),
        );
    }

    fn cwd_in_worktree() -> PathBuf {
        PathBuf::from("/repo.worktree/app-feat/login")
    }

    fn cwd_in_repo() -> PathBuf {
        PathBuf::from("/repo")
    }

    fn config_herdr_never() -> Config {
        Config {
            herdr: crate::config::HerdrConfig {
                enabled: HerdrMode::Never,
            },
            ..Config::default()
        }
    }

    fn config_herdr_always() -> Config {
        Config {
            herdr: crate::config::HerdrConfig {
                enabled: HerdrMode::Always,
            },
            ..Config::default()
        }
    }

    // ── Uncommitted changes ──────────────────────────────────────

    #[test]
    fn test_uncommitted_changes_stops_early() {
        let mut mock = mock_git();
        mock.add_response("git", &["status", "--porcelain"], ok_output(" M src/file.rs\n"));

        let git = GitClient::new(mock);
        let herdr = HerdrClient::new(MockRunner::new());
        let config = base_config();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_worktree());
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
        successful_merge_from_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        cleanup_mocks(&mut mock, "/repo.worktree/app-feat/login", "feat/login");
        herdr_close_mocks(&mut mock, "/repo.worktree/app-feat/login", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr_always();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    #[test]
    fn test_auto_merge_success_no_delete_branch() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_from_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        mock.add_response(
            "git",
            &["config", "--get", "branch.feat/login.mat-cwd"],
            ok_output("/original/cwd\n"),
        );
        mock.add_response("git", &["worktree", "remove", "/repo.worktree/app-feat/login"], ok_output(""));
        herdr_close_mocks(&mut mock, "/repo.worktree/app-feat/login", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = Config {
            delete_branch: false,
            herdr: HerdrConfig { enabled: HerdrMode::Always },
            ..Config::default()
        };

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    #[test]
    fn test_auto_merge_branch_delete_not_called_when_delete_false() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_from_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        mock.add_response("git", &["worktree", "remove", "/repo.worktree/app-feat/login"], ok_output(""));
        // No branch -d mock = test will fail if branch_delete is called
        herdr_close_mocks(&mut mock, "/repo.worktree/app-feat/login", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = Config {
            delete_branch: false,
            herdr: HerdrConfig { enabled: HerdrMode::Always },
            ..Config::default()
        };

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    // ── Merge strategy flags ────────────────────────────────────

    #[test]
    fn test_merge_strategy_merge_commit() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_from_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        cleanup_mocks(&mut mock, "/repo.worktree/app-feat/login", "feat/login");
        herdr_close_mocks(&mut mock, "/repo.worktree/app-feat/login", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr_always();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    #[test]
    fn test_merge_strategy_fast_forward() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_from_mocks(&mut mock, "feat/login", "main", &MergeStrategy::FastForward);
        cleanup_mocks(&mut mock, "/repo.worktree/app-feat/login", "feat/login");
        herdr_close_mocks(&mut mock, "/repo.worktree/app-feat/login", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = Config {
            merge_strategy: MergeStrategy::FastForward,
            herdr: HerdrConfig { enabled: HerdrMode::Always },
            ..Config::default()
        };

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_worktree());
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
        let herdr = HerdrClient::new(MockRunner::new());
        let config = base_config();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_worktree());
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
    fn test_no_merge_skips_merge_and_closes_herdr() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        mock.add_response("git", &["checkout", "main"], ok_output(""));
        mock.add_response(
            "git",
            &["config", "--get", "branch.feat/login.mat-cwd"],
            ok_output("/original/cwd\n"),
        );
        mock.add_response("git", &["worktree", "remove", "/repo.worktree/app-feat/login"], ok_output(""));
        mock.add_response("git", &["branch", "-d", "feat/login"], ok_output(""));
        herdr_close_mocks(&mut mock, "/repo.worktree/app-feat/login", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr_always();

        let result = handle_close(true, &config, &git, &herdr, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_merge_merge_not_called() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        mock.add_response("git", &["checkout", "main"], ok_output(""));
        mock.add_response(
            "git",
            &["config", "--get", "branch.feat/login.mat-cwd"],
            ok_output("/original/cwd\n"),
        );
        mock.add_response("git", &["worktree", "remove", "/repo.worktree/app-feat/login"], ok_output(""));
        herdr_close_mocks(&mut mock, "/repo.worktree/app-feat/login", "ws-1");

        // Use no_delete + herdr always to avoid needing branch_delete mock
        let config = Config {
            delete_branch: false,
            herdr: HerdrConfig { enabled: HerdrMode::Always },
            ..Config::default()
        };
        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);

        let result = handle_close(true, &config, &git, &herdr, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_merge_with_delete_branch() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        mock.add_response("git", &["checkout", "main"], ok_output(""));
        mock.add_response(
            "git",
            &["config", "--get", "branch.feat/login.mat-cwd"],
            ok_output("/original/cwd\n"),
        );
        mock.add_response("git", &["worktree", "remove", "/repo.worktree/app-feat/login"], ok_output(""));
        mock.add_response("git", &["branch", "-d", "feat/login"], ok_output(""));
        herdr_close_mocks(&mut mock, "/repo.worktree/app-feat/login", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr_always();

        let result = handle_close(true, &config, &git, &herdr, &cwd_in_worktree());
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
            &["rev-parse", "--git-common-dir"],
            ok_output("/repo/.git\n"),
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
        // herdr close
        herdr_close_mocks(&mut mock, "/repo", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr_always();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_repo());
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
            &["rev-parse", "--git-common-dir"],
            ok_output("/repo/.git\n"),
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
        let herdr = HerdrClient::new(MockRunner::new());
        let config = base_config();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_repo());
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
            &["rev-parse", "--git-common-dir"],
            ok_output("/repo/.git\n"),
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
        herdr_close_mocks(&mut mock, "/repo", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr_always();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_repo());
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
            &["rev-parse", "--git-common-dir"],
            ok_output("/repo/.git\n"),
        );
        mock.add_response("git", &["branch", "--show-current"], ok_output("feat/login\n"));
        mock.add_response("git", &["stash", "pop", "mat:auto:feat/login"], ok_output(""));
        default_branch_mocks(&mut mock);
        successful_merge_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        mock.add_response("git", &["branch", "-d", "feat/login"], ok_output(""));
        herdr_close_mocks(&mut mock, "/repo", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr_always();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_repo());
        assert!(result.is_ok());
    }

    // ── close workspace called after cleanup ────────────────────

    #[test]
    fn test_close_workspace_called_after_cleanup() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_from_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        cleanup_mocks(&mut mock, "/repo.worktree/app-feat/login", "feat/login");
        herdr_close_mocks(&mut mock, "/repo.worktree/app-feat/login", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr_always();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    // ── Herdr guard behavior ──────────────────────────────────

    #[test]
    fn test_close_skips_herdr_when_config_never() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_from_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        cleanup_mocks(&mut mock, "/repo.worktree/app-feat/login", "feat/login");
        // No herdr mocks needed — herdr should not be touched

        let git = GitClient::new(mock);
        let herdr = HerdrClient::new(MockRunner::new());
        let config = config_herdr_never();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    #[test]
    fn test_close_uses_herdr_when_config_always() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_from_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        cleanup_mocks(&mut mock, "/repo.worktree/app-feat/login", "feat/login");
        herdr_close_mocks(&mut mock, "/repo.worktree/app-feat/login", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr_always();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    // ── Source branch from mat config ─────────────────────────────

    #[test]
    fn test_close_uses_mat_source_branch_from_config() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        // Config returns "develop" as the source branch
        mock.add_response(
            "git",
            &["config", "--get", "branch.feat/login.mat-source"],
            ok_output("develop\n"),
        );
        // default_branch should NOT be called since config has the value
        successful_merge_from_mocks(&mut mock, "feat/login", "develop", &MergeStrategy::MergeCommit);
        cleanup_mocks(&mut mock, "/repo.worktree/app-feat/login", "feat/login");
        herdr_close_mocks(&mut mock, "/repo.worktree/app-feat/login", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr_always();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_worktree());
        assert!(result.is_ok());
    }

    // ── Output messages ─────────────────────────────────────────

    #[test]
    fn test_merge_strategy_name() {
        assert_eq!(merge_strategy_name(&MergeStrategy::MergeCommit), "merge commit");
        assert_eq!(merge_strategy_name(&MergeStrategy::FastForward), "fast-forward");
    }

    #[test]
    fn test_handle_close_no_herdr_error_does_not_crash() {
        let mut mock = mock_git();
        setup_worktree_mocks(&mut mock, "/repo.worktree/app-feat/login");
        default_branch_mocks(&mut mock);
        successful_merge_from_mocks(&mut mock, "feat/login", "main", &MergeStrategy::MergeCommit);
        mock.add_response(
            "git",
            &["config", "--get", "branch.feat/login.mat-cwd"],
            ok_output("/original/cwd\n"),
        );
        mock.add_response("git", &["worktree", "remove", "/repo.worktree/app-feat/login"], ok_output(""));
        mock.add_response("git", &["branch", "-d", "feat/login"], ok_output(""));
        // herdr errors should propagate
        herdr_close_mocks(&mut mock, "/repo.worktree/app-feat/login", "ws-1");

        let git = GitClient::new(mock.clone());
        let herdr = HerdrClient::new(mock);
        let config = config_herdr_always();

        let result = handle_close(false, &config, &git, &herdr, &cwd_in_worktree());
        assert!(result.is_ok());
    }
}
