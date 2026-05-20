#[cfg(test)]
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use crate::config::MergeStrategy;
use crate::error::MatError;

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, MatError>;
}

pub struct RealRunner;

impl CommandRunner for RealRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, MatError> {
        let cmd_str = format!("{} {}", program, args.join(" "));
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|e| MatError::Git {
                command: cmd_str.clone(),
                stderr: e.to_string(),
            })?;

        let result = CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        };

        if !output.status.success() {
            return Err(MatError::Git {
                command: cmd_str,
                stderr: result.stderr.clone(),
            });
        }

        Ok(result)
    }
}

#[cfg(test)]
#[derive(Clone)]
pub struct MockRunner {
    responses: HashMap<String, Result<CommandOutput, MatError>>,
}

#[cfg(test)]
impl MockRunner {
    pub fn new() -> Self {
        MockRunner {
            responses: HashMap::new(),
        }
    }

    pub fn add_response(&mut self, program: &str, args: &[&str], output: CommandOutput) {
        let key = Self::make_key(program, args);
        self.responses.insert(key, Ok(output));
    }

    pub fn add_error(&mut self, program: &str, args: &[&str], error: MatError) {
        let key = Self::make_key(program, args);
        self.responses.insert(key, Err(error));
    }

    fn make_key(program: &str, args: &[&str]) -> String {
        let mut key = String::from(program);
        for arg in args {
            key.push(' ');
            key.push_str(arg);
        }
        key
    }
}

#[cfg(test)]
impl CommandRunner for MockRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, MatError> {
        let key = Self::make_key(program, args);
        self.responses.get(&key).cloned().unwrap_or_else(|| {
            Err(MatError::Git {
                command: key,
                stderr: "no mock response configured".into(),
            })
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub commit: String,
    pub is_main: bool,
}

pub struct GitClient<R: CommandRunner> {
    runner: R,
}

impl<R: CommandRunner> GitClient<R> {
    pub fn new(runner: R) -> Self {
        GitClient { runner }
    }

    #[cfg(test)]
    pub fn is_repo(&self) -> Result<bool, MatError> {
        match self.runner.run("git", &["rev-parse", "--git-dir"]) {
            Ok(_) => Ok(true),
            Err(MatError::Git { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub fn current_branch(&self) -> Result<String, MatError> {
        let output = self.runner.run("git", &["branch", "--show-current"])?;
        Ok(output.stdout.trim().to_string())
    }

    pub fn default_branch(&self) -> Result<String, MatError> {
        let output = self
            .runner
            .run("git", &["symbolic-ref", "refs/remotes/origin/HEAD"])?;
        let branch = output
            .stdout
            .trim()
            .strip_prefix("refs/remotes/origin/")
            .unwrap_or(output.stdout.trim())
            .to_string();
        Ok(branch)
    }

    pub fn has_uncommitted_changes(&self) -> Result<bool, MatError> {
        let output = self.runner.run("git", &["status", "--porcelain"])?;
        Ok(!output.stdout.trim().is_empty())
    }

    pub fn worktree_add(&self, path: &str, branch: &str, source: &str) -> Result<String, MatError> {
        let output = self
            .runner
            .run("git", &["worktree", "add", "-b", branch, path, source])?;
        Ok(output.stdout.trim().to_string())
    }

    pub fn worktree_list(&self) -> Result<Vec<WorktreeInfo>, MatError> {
        let output = self
            .runner
            .run("git", &["worktree", "list", "--porcelain"])?;

        let repo_root = self
            .runner
            .run("git", &["rev-parse", "--show-toplevel"])
            .ok()
            .map(|o| PathBuf::from(o.stdout.trim().to_string()));

        let mut worktrees = Vec::new();
        let mut current_path: Option<PathBuf> = None;
        let mut current_branch: Option<String> = None;
        let mut current_commit: Option<String> = None;

        for line in output.stdout.lines() {
            if line.is_empty() {
                if let (Some(path), Some(commit)) = (&current_path, &current_commit) {
                    let branch = current_branch
                        .clone()
                        .unwrap_or_else(|| "detached".to_string());
                    let is_main = repo_root.as_ref().map_or(false, |root| *path == *root);
                    worktrees.push(WorktreeInfo {
                        path: path.clone(),
                        branch,
                        commit: commit.clone(),
                        is_main,
                    });
                }
                current_path = None;
                current_branch = None;
                current_commit = None;
                continue;
            }

            if let Some(path) = line.strip_prefix("worktree ") {
                current_path = Some(PathBuf::from(path));
            } else if let Some(branch_ref) = line.strip_prefix("branch ") {
                current_branch = Some(
                    branch_ref
                        .strip_prefix("refs/heads/")
                        .unwrap_or(branch_ref)
                        .to_string(),
                );
            } else if let Some(commit) = line.strip_prefix("HEAD ") {
                current_commit = Some(commit.to_string());
            } else if line == "detached" {
                current_branch = Some("detached".to_string());
            }
        }

        if let (Some(path), Some(commit)) = (&current_path, &current_commit) {
            let branch = current_branch
                .clone()
                .unwrap_or_else(|| "detached".to_string());
            let is_main = repo_root.as_ref().map_or(false, |root| *path == *root);
            worktrees.push(WorktreeInfo {
                path: path.clone(),
                branch,
                commit: commit.clone(),
                is_main,
            });
        }

        Ok(worktrees)
    }

    pub fn worktree_remove(&self, path: &str) -> Result<(), MatError> {
        self.runner.run("git", &["worktree", "remove", path])?;
        Ok(())
    }

    pub fn checkout(&self, branch: &str) -> Result<(), MatError> {
        self.runner.run("git", &["checkout", branch])?;
        Ok(())
    }

    pub fn checkout_b(&self, branch: &str, source: &str) -> Result<(), MatError> {
        self.runner
            .run("git", &["checkout", "-b", branch, source])?;
        Ok(())
    }

    pub fn merge(&self, branch: &str, strategy: MergeStrategy) -> Result<(), MatError> {
        match strategy {
            MergeStrategy::MergeCommit => {
                self.runner.run("git", &["merge", "--no-ff", branch])?;
            }
            MergeStrategy::FastForward => {
                self.runner.run("git", &["merge", "--ff-only", branch])?;
            }
        }
        Ok(())
    }

    pub fn branch_delete(&self, branch: &str) -> Result<(), MatError> {
        self.runner.run("git", &["branch", "-d", branch])?;
        Ok(())
    }

    pub fn stash_push(&self, message: &str, include_untracked: bool) -> Result<(), MatError> {
        let stash_msg = format!("mat:auto:{}", message);
        let mut args = vec!["stash", "push", "-m", stash_msg.as_str()];
        if include_untracked {
            args.push("--include-untracked");
        }
        self.runner.run("git", &args)?;
        Ok(())
    }

    pub fn stash_pop(&self, stash_ref: &str) -> Result<(), MatError> {
        self.runner.run("git", &["stash", "pop", stash_ref])?;
        Ok(())
    }

    pub fn abort_merge(&self) -> Result<(), MatError> {
        self.runner.run("git", &["merge", "--abort"])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_git() -> MockRunner {
        MockRunner::new()
    }

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

    fn client_with(mock: MockRunner) -> GitClient<MockRunner> {
        GitClient::new(mock)
    }

    #[test]
    fn test_mock_runner_returns_canned_output() {
        let mut mock = mock_git();
        mock.add_response("git", &["status"], ok_output("clean"));
        let git = client_with(mock);
        let output = git.runner.run("git", &["status"]).unwrap();
        assert_eq!(output.stdout, "clean");
    }

    #[test]
    fn test_mock_runner_returns_error_for_unconfigured_command() {
        let mock = mock_git();
        let git = client_with(mock);
        let err = git.runner.run("git", &["unknown"]).unwrap_err();
        match err {
            MatError::Git { command, .. } => assert_eq!(command, "git unknown"),
            _ => panic!("expected MatError::Git"),
        }
    }

    #[test]
    fn test_is_repo_returns_true_when_rev_parse_succeeds() {
        let mut mock = mock_git();
        mock.add_response("git", &["rev-parse", "--git-dir"], ok_output(".git"));
        let git = client_with(mock);
        assert!(git.is_repo().unwrap());
    }

    #[test]
    fn test_is_repo_returns_false_when_not_a_repo() {
        let mut mock = mock_git();
        mock.add_error(
            "git",
            &["rev-parse", "--git-dir"],
            MatError::Git {
                command: "git rev-parse --git-dir".into(),
                stderr: "fatal: not a git repository".into(),
            },
        );
        let git = client_with(mock);
        assert!(!git.is_repo().unwrap());
    }

    #[test]
    fn test_current_branch_returns_branch_name() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &["branch", "--show-current"],
            ok_output("feat/login\n"),
        );
        let git = client_with(mock);
        assert_eq!(git.current_branch().unwrap(), "feat/login");
    }

    #[test]
    fn test_current_branch_returns_empty_when_detached() {
        let mut mock = mock_git();
        mock.add_response("git", &["branch", "--show-current"], ok_output("\n"));
        let git = client_with(mock);
        assert!(git.current_branch().unwrap().is_empty());
    }

    #[test]
    fn test_default_branch_parses_origin_head() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &["symbolic-ref", "refs/remotes/origin/HEAD"],
            ok_output("refs/remotes/origin/main\n"),
        );
        let git = client_with(mock);
        assert_eq!(git.default_branch().unwrap(), "main");
    }

    #[test]
    fn test_has_uncommitted_changes_returns_true_when_dirty() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &["status", "--porcelain"],
            ok_output(" M src/main.rs\n"),
        );
        let git = client_with(mock);
        assert!(git.has_uncommitted_changes().unwrap());
    }

    #[test]
    fn test_has_uncommitted_changes_returns_false_when_clean() {
        let mut mock = mock_git();
        mock.add_response("git", &["status", "--porcelain"], ok_output(""));
        let git = client_with(mock);
        assert!(!git.has_uncommitted_changes().unwrap());
    }

    #[test]
    fn test_worktree_add_constructs_correct_args() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &["worktree", "add", "-b", "feat/login", "/tmp/wt", "main"],
            ok_output("/tmp/wt\n"),
        );
        let git = client_with(mock);
        let result = git.worktree_add("/tmp/wt", "feat/login", "main").unwrap();
        assert_eq!(result, "/tmp/wt");
    }

    #[test]
    fn test_worktree_list_parses_porcelain_output() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &["worktree", "list", "--porcelain"],
            ok_output(
                "\
worktree /home/user/project
HEAD abc123def456
branch refs/heads/main

worktree /home/user/project.worktree/app-feat/login
HEAD def789abc012
branch refs/heads/feat/login
",
            ),
        );
        mock.add_response(
            "git",
            &["rev-parse", "--show-toplevel"],
            ok_output("/home/user/project\n"),
        );
        let git = client_with(mock);
        let worktrees = git.worktree_list().unwrap();
        assert_eq!(worktrees.len(), 2);
        assert!(worktrees[0].is_main);
        assert_eq!(worktrees[0].branch, "main");
        assert_eq!(worktrees[0].commit, "abc123def456");
        assert_eq!(worktrees[0].path, PathBuf::from("/home/user/project"));
        assert!(!worktrees[1].is_main);
        assert_eq!(worktrees[1].branch, "feat/login");
        assert_eq!(worktrees[1].commit, "def789abc012");
        assert_eq!(
            worktrees[1].path,
            PathBuf::from("/home/user/project.worktree/app-feat/login")
        );
    }

    #[test]
    fn test_worktree_list_handles_detached_head() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &["worktree", "list", "--porcelain"],
            ok_output(
                "\
worktree /home/user/project
HEAD abc123def456
branch refs/heads/main

worktree /home/user/project.worktree/detached-feat
HEAD def789abc012
detached
",
            ),
        );
        mock.add_response(
            "git",
            &["rev-parse", "--show-toplevel"],
            ok_output("/home/user/project\n"),
        );
        let git = client_with(mock);
        let worktrees = git.worktree_list().unwrap();
        assert_eq!(worktrees.len(), 2);
        assert_eq!(worktrees[1].branch, "detached");
    }

    #[test]
    fn test_worktree_remove_invokes_correct_command() {
        let mut mock = mock_git();
        mock.add_response("git", &["worktree", "remove", "/tmp/wt"], ok_output(""));
        let git = client_with(mock);
        git.worktree_remove("/tmp/wt").unwrap();
    }

    #[test]
    fn test_checkout_switches_branch() {
        let mut mock = mock_git();
        mock.add_response("git", &["checkout", "main"], ok_output(""));
        let git = client_with(mock);
        git.checkout("main").unwrap();
    }

    #[test]
    fn test_checkout_b_creates_new_branch() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &["checkout", "-b", "feat/login", "main"],
            ok_output(""),
        );
        let git = client_with(mock);
        git.checkout_b("feat/login", "main").unwrap();
    }

    #[test]
    fn test_merge_with_merge_commit_passes_no_ff() {
        let mut mock = mock_git();
        mock.add_response("git", &["merge", "--no-ff", "feat/login"], ok_output(""));
        let git = client_with(mock);
        git.merge("feat/login", MergeStrategy::MergeCommit).unwrap();
    }

    #[test]
    fn test_merge_with_fast_forward_passes_ff_only() {
        let mut mock = mock_git();
        mock.add_response("git", &["merge", "--ff-only", "feat/login"], ok_output(""));
        let git = client_with(mock);
        git.merge("feat/login", MergeStrategy::FastForward).unwrap();
    }

    #[test]
    fn test_stash_push_uses_mat_auto_prefix() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &["stash", "push", "-m", "mat:auto:feat/login"],
            ok_output(""),
        );
        let git = client_with(mock);
        git.stash_push("feat/login", false).unwrap();
    }

    #[test]
    fn test_stash_push_with_untracked_includes_flag() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &[
                "stash",
                "push",
                "-m",
                "mat:auto:feat/login",
                "--include-untracked",
            ],
            ok_output(""),
        );
        let git = client_with(mock);
        git.stash_push("feat/login", true).unwrap();
    }

    #[test]
    fn test_stash_pop_uses_provided_ref() {
        let mut mock = mock_git();
        mock.add_response("git", &["stash", "pop", "stash@{0}"], ok_output(""));
        let git = client_with(mock);
        git.stash_pop("stash@{0}").unwrap();
    }

    #[test]
    fn test_branch_delete_uses_d_flag() {
        let mut mock = mock_git();
        mock.add_response("git", &["branch", "-d", "feat/login"], ok_output(""));
        let git = client_with(mock);
        git.branch_delete("feat/login").unwrap();
    }

    #[test]
    fn test_is_repo_propagates_io_error() {
        let mut mock = mock_git();
        mock.add_error(
            "git",
            &["rev-parse", "--git-dir"],
            MatError::Io(std::sync::Arc::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "git binary not found",
            ))),
        );
        let git = client_with(mock);
        let err = git.is_repo().unwrap_err();
        match err {
            MatError::Io(_) => {}
            _ => panic!("expected MatError::Io"),
        }
    }

    #[test]
    fn test_all_methods_return_git_error_on_nonzero_exit() {
        let mut mock = mock_git();
        mock.add_error(
            "git",
            &["worktree", "remove", "/bad/path"],
            MatError::Git {
                command: "git worktree remove /bad/path".into(),
                stderr: "fatal: '/bad/path' not found".into(),
            },
        );
        let git = client_with(mock);
        let err = git.worktree_remove("/bad/path").unwrap_err();
        match err {
            MatError::Git { ref stderr, .. } => {
                assert!(stderr.contains("not found"));
            }
            _ => panic!("expected MatError::Git"),
        }
    }

    #[test]
    fn test_current_branch_returns_git_error_on_failure() {
        let mut mock = mock_git();
        mock.add_error(
            "git",
            &["branch", "--show-current"],
            MatError::Git {
                command: "git branch --show-current".into(),
                stderr: "fatal: not a git repository".into(),
            },
        );
        let git = client_with(mock);
        let err = git.current_branch().unwrap_err();
        match err {
            MatError::Git { ref stderr, .. } => {
                assert!(stderr.contains("not a git repository"));
            }
            _ => panic!("expected MatError::Git"),
        }
    }

    #[test]
    fn test_default_branch_parses_non_origin_ref() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &["symbolic-ref", "refs/remotes/origin/HEAD"],
            ok_output("refs/remotes/origin/develop\n"),
        );
        let git = client_with(mock);
        assert_eq!(git.default_branch().unwrap(), "develop");
    }

    #[test]
    fn test_default_branch_returns_whole_output_if_no_prefix() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &["symbolic-ref", "refs/remotes/origin/HEAD"],
            ok_output("main\n"),
        );
        let git = client_with(mock);
        assert_eq!(git.default_branch().unwrap(), "main");
    }

    #[test]
    fn test_worktree_list_parses_single_entry() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &["worktree", "list", "--porcelain"],
            ok_output(
                "\
worktree /repo
HEAD deadbeef
branch refs/heads/main
",
            ),
        );
        mock.add_response(
            "git",
            &["rev-parse", "--show-toplevel"],
            ok_output("/repo\n"),
        );
        let git = client_with(mock);
        let worktrees = git.worktree_list().unwrap();
        assert_eq!(worktrees.len(), 1);
        assert!(worktrees[0].is_main);
    }

    #[test]
    fn test_worktree_list_parses_multiple_entries() {
        let mut mock = mock_git();
        mock.add_response(
            "git",
            &["worktree", "list", "--porcelain"],
            ok_output(
                "\
worktree /repo
HEAD aaa
branch refs/heads/main

worktree /repo.worktree/wt1
HEAD bbb
branch refs/heads/feat/a

worktree /repo.worktree/wt2
HEAD ccc
branch refs/heads/fix/b
",
            ),
        );
        mock.add_response(
            "git",
            &["rev-parse", "--show-toplevel"],
            ok_output("/repo\n"),
        );
        let git = client_with(mock);
        let worktrees = git.worktree_list().unwrap();
        assert_eq!(worktrees.len(), 3);
        assert!(worktrees[0].is_main);
        assert!(!worktrees[1].is_main);
        assert!(!worktrees[2].is_main);
        assert_eq!(worktrees[1].branch, "feat/a");
        assert_eq!(worktrees[2].branch, "fix/b");
    }

    #[test]
    fn test_merge_strategy_debug_and_clone() {
        let mc = MergeStrategy::MergeCommit;
        let ff = MergeStrategy::FastForward;
        assert_eq!(format!("{:?}", mc), "MergeCommit");
        assert_eq!(format!("{:?}", ff), "FastForward");
        assert_eq!(mc.clone(), mc);
        assert_eq!(ff.clone(), ff);
    }

    #[test]
    fn test_worktree_info_debug_and_clone() {
        let info = WorktreeInfo {
            path: PathBuf::from("/test"),
            branch: "main".into(),
            commit: "abc".into(),
            is_main: true,
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
        assert!(format!("{:?}", info).contains("/test"));
    }

    #[test]
    fn test_git_client_new() {
        let mock = mock_git();
        let git = GitClient::new(mock);
        assert!(!git.is_repo().unwrap());
    }

    #[test]
    fn test_real_runner_executes_git_successfully() {
        let runner = RealRunner;
        let result = runner.run("true", &[]);
        assert!(result.is_ok());
        assert!(result.unwrap().stdout.is_empty());
    }

    #[test]
    fn test_real_runner_returns_error_for_nonexistent_command() {
        let runner = RealRunner;
        let result = runner.run("nonexistent_cmd_xyz", &[]);
        assert!(result.is_err());
    }
}
