use serde::Deserialize;

use crate::error::MatError;
use crate::git::CommandRunner;

#[derive(Debug)]
pub struct WorktreeCreate {
    pub workspace_id: String,
    #[allow(dead_code)]
    pub branch: String,
    pub path: String,
}

pub struct HerdrClient<R: CommandRunner> {
    runner: R,
}

impl<R: CommandRunner> HerdrClient<R> {
    pub fn new(runner: R) -> Self {
        HerdrClient { runner }
    }

    fn run_herdr(&self, args: &[&str]) -> Result<crate::git::CommandOutput, MatError> {
        match self.runner.run("herdr", args) {
            Ok(output) => Ok(output),
            Err(MatError::Git { command, stderr }) => Err(MatError::Herdr { command, stderr }),
            Err(e) => Err(e),
        }
    }

    pub fn create_worktree(
        &self,
        cwd: &str,
        branch: &str,
        base: &str,
        label: &str,
        path: Option<&str>,
    ) -> Result<WorktreeCreate, MatError> {
        let mut args: Vec<&str> = vec![
            "worktree",
            "create",
            "--cwd",
            cwd,
            "--branch",
            branch,
            "--base",
            base,
            "--label",
            label,
            "--no-focus",
            "--json",
        ];
        if let Some(p) = path {
            args.push("--path");
            args.push(p);
        }
        let output = self.run_herdr(&args)?;

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct CreateOutput {
            result: CreateResult,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct CreateResult {
            workspace: WorkspacePart,
            worktree: WorktreePart,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct WorkspacePart {
            workspace_id: String,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct WorktreePart {
            branch: String,
            path: String,
        }

        let parsed: CreateOutput = serde_json::from_str(&output.stdout).map_err(|e| {
            MatError::Herdr {
                command: format!("herdr {}", args.join(" ")),
                stderr: format!("failed to parse JSON: {}", e),
            }
        })?;

        Ok(WorktreeCreate {
            workspace_id: parsed.result.workspace.workspace_id,
            branch: parsed.result.worktree.branch,
            path: parsed.result.worktree.path,
        })
    }

    pub fn remove_worktree(&self, workspace_id: &str, force: bool) -> Result<(), MatError> {
        let mut args: Vec<&str> = vec!["worktree", "remove", "--workspace", workspace_id];
        if force {
            args.push("--force");
        }
        self.run_herdr(&args)?;
        Ok(())
    }

    pub fn current_workspace(&self) -> Result<String, MatError> {
        let output = self.run_herdr(&["pane", "list"])?;

        #[derive(Debug, Deserialize)]
        struct PaneItem {
            workspace_id: String,
            focused: Option<bool>,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct PaneListResult {
            panes: Vec<PaneItem>,
        }

        #[derive(Debug, Deserialize)]
        struct PaneListOutput {
            result: PaneListResult,
        }

        let parsed: PaneListOutput =
            serde_json::from_str(&output.stdout).map_err(|e| MatError::Herdr {
                command: "herdr pane list".into(),
                stderr: format!("failed to parse JSON: {}", e),
            })?;

        let focused = parsed
            .result
            .panes
            .iter()
            .find(|p| p.focused.unwrap_or(false))
            .or_else(|| parsed.result.panes.first());

        match focused {
            Some(p) => Ok(p.workspace_id.clone()),
            None => Err(MatError::Herdr {
                command: "herdr pane list".into(),
                stderr: "no panes found".into(),
            }),
        }
    }

    pub fn tab_create(
        &self,
        workspace_id: &str,
        label: Option<&str>,
    ) -> Result<(String, String), MatError> {
        let mut args = vec!["tab", "create", "--workspace", workspace_id];
        if let Some(l) = label {
            args.push("--label");
            args.push(l);
        }
        let output = self.run_herdr(&args)?;

        #[derive(Debug, Deserialize)]
        struct TabPart {
            tab_id: String,
        }

        #[derive(Debug, Deserialize)]
        struct PanePart {
            pane_id: String,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct TabCreateResult {
            tab: TabPart,
            root_pane: PanePart,
        }

        #[derive(Debug, Deserialize)]
        struct TabCreateOutput {
            result: TabCreateResult,
        }

        let parsed: TabCreateOutput =
            serde_json::from_str(&output.stdout).map_err(|e| MatError::Herdr {
                command: format!("herdr {}", args.join(" ")),
                stderr: format!("failed to parse JSON: {}", e),
            })?;

        Ok((parsed.result.tab.tab_id, parsed.result.root_pane.pane_id))
    }

    pub fn pane_split(
        &self,
        pane_id: &str,
        direction: &str,
        no_focus: bool,
    ) -> Result<String, MatError> {
        let mut args = vec!["pane", "split", pane_id, "--direction", direction];
        if no_focus {
            args.push("--no-focus");
        }
        let output = self.run_herdr(&args)?;

        #[derive(Debug, Deserialize)]
        struct PanePart {
            pane_id: String,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "snake_case")]
        struct PaneSplitResult {
            pane: PanePart,
        }

        #[derive(Debug, Deserialize)]
        struct PaneSplitOutput {
            result: PaneSplitResult,
        }

        let parsed: PaneSplitOutput =
            serde_json::from_str(&output.stdout).map_err(|e| MatError::Herdr {
                command: format!("herdr {}", args.join(" ")),
                stderr: format!("failed to parse JSON: {}", e),
            })?;

        Ok(parsed.result.pane.pane_id)
    }

    pub fn tab_focus(&self, tab_id: &str) -> Result<(), MatError> {
        self.run_herdr(&["tab", "focus", tab_id])?;
        Ok(())
    }

    pub fn pane_run(&self, pane_id: &str, command: &str) -> Result<(), MatError> {
        self.run_herdr(&["pane", "run", pane_id, command])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{CommandOutput, MockRunner};

    fn mock_herdr() -> MockRunner {
        MockRunner::new()
    }

    fn ok_output(stdout: &str) -> CommandOutput {
        CommandOutput {
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    fn client_with(mock: MockRunner) -> HerdrClient<MockRunner> {
        HerdrClient::new(mock)
    }

    fn create_worktree_json(ws_id: &str, branch: &str, path: &str) -> String {
        format!(
            r#"{{"result":{{"workspace":{{"workspace_id":"{}"}},"worktree":{{"branch":"{}","path":"{}"}}}}}}"#,
            ws_id, branch, path
        )
    }

    #[test]
    fn test_create_worktree_returns_structured_data() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                "/repo",
                "--branch",
                "feat/login",
                "--base",
                "main",
                "--label",
                "app-feat/login",
                "--no-focus",
                "--json",
            ],
            ok_output(&create_worktree_json("w1", "feat/login", "/home/.herdr/worktrees/mat/wt1")),
        );
        let herdr = client_with(mock);
        let result = herdr
            .create_worktree("/repo", "feat/login", "main", "app-feat/login", None)
            .unwrap();
        assert_eq!(result.workspace_id, "w1");
        assert_eq!(result.branch, "feat/login");
        assert_eq!(result.path, "/home/.herdr/worktrees/mat/wt1");
    }

    #[test]
    fn test_create_worktree_passes_custom_path() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                "/repo",
                "--branch",
                "feat/login",
                "--base",
                "main",
                "--label",
                "app-feat/login",
                "--no-focus",
                "--json",
                "--path",
                "/custom/path",
            ],
            ok_output(&create_worktree_json("w2", "feat/login", "/custom/path")),
        );
        let herdr = client_with(mock);
        let result = herdr
            .create_worktree("/repo", "feat/login", "main", "app-feat/login", Some("/custom/path"))
            .unwrap();
        assert_eq!(result.path, "/custom/path");
    }

    #[test]
    fn test_create_worktree_returns_error_on_herdr_failure() {
        let mut mock = mock_herdr();
        mock.add_error(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                "/repo",
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
        let herdr = client_with(mock);
        let err = herdr
            .create_worktree("/repo", "feat/login", "main", "app-feat/login", None)
            .unwrap_err();
        match err {
            MatError::Herdr { ref stderr, .. } => {
                assert!(stderr.contains("server not running"));
            }
            _ => panic!("expected MatError::Herdr"),
        }
    }

    #[test]
    fn test_remove_worktree_passes_workspace_id() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["worktree", "remove", "--workspace", "w1"],
            ok_output(""),
        );
        let herdr = client_with(mock);
        herdr.remove_worktree("w1", false).unwrap();
    }

    #[test]
    fn test_remove_worktree_with_force() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["worktree", "remove", "--workspace", "w2", "--force"],
            ok_output(""),
        );
        let herdr = client_with(mock);
        herdr.remove_worktree("w2", true).unwrap();
    }

    #[test]
    fn test_remove_worktree_returns_error_on_failure() {
        let mut mock = mock_herdr();
        mock.add_error(
            "herdr",
            &["worktree", "remove", "--workspace", "w99"],
            MatError::Herdr {
                command: "herdr worktree remove".into(),
                stderr: "workspace not found".into(),
            },
        );
        let herdr = client_with(mock);
        let err = herdr.remove_worktree("w99", false).unwrap_err();
        match err {
            MatError::Herdr { ref stderr, .. } => {
                assert!(stderr.contains("workspace not found"));
            }
            _ => panic!("expected MatError::Herdr"),
        }
    }

    #[test]
    fn test_current_workspace_parses_pane_list() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["pane", "list"],
            ok_output(r#"{"result":{"panes":[{"pane_id":"1-1","workspace_id":"2","tab_id":"1:1","label":"zsh","focused":true}]}}"#),
        );
        let herdr = client_with(mock);
        let ws_id = herdr.current_workspace().unwrap();
        assert_eq!(ws_id, "2");
    }

    #[test]
    fn test_current_workspace_falls_back_to_first_pane() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["pane", "list"],
            ok_output(r#"{"result":{"panes":[{"pane_id":"2-1","workspace_id":"5","tab_id":"2:1","label":"bash"}]}}"#),
        );
        let herdr = client_with(mock);
        let ws_id = herdr.current_workspace().unwrap();
        assert_eq!(ws_id, "5");
    }

    #[test]
    fn test_current_workspace_returns_error_on_empty_list() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["pane", "list"],
            ok_output(r#"{"result":{"panes":[]}}"#),
        );
        let herdr = client_with(mock);
        let err = herdr.current_workspace().unwrap_err();
        match err {
            MatError::Herdr { ref stderr, .. } => assert!(stderr.contains("no panes found")),
            _ => panic!("expected MatError::Herdr"),
        }
    }

    #[test]
    fn test_tab_create_returns_tab_and_pane_ids() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["tab", "create", "--workspace", "1"],
            ok_output(r#"{"result":{"tab":{"tab_id":"1:2"},"root_pane":{"pane_id":"1-3"}}}"#),
        );
        let herdr = client_with(mock);
        let (tab_id, pane_id) = herdr.tab_create("1", None).unwrap();
        assert_eq!(tab_id, "1:2");
        assert_eq!(pane_id, "1-3");
    }

    #[test]
    fn test_tab_create_with_label() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["tab", "create", "--workspace", "1", "--label", "my-task"],
            ok_output(r#"{"result":{"tab":{"tab_id":"1:3"},"root_pane":{"pane_id":"1-4"}}}"#),
        );
        let herdr = client_with(mock);
        let (tab_id, pane_id) = herdr.tab_create("1", Some("my-task")).unwrap();
        assert_eq!(tab_id, "1:3");
        assert_eq!(pane_id, "1-4");
    }

    #[test]
    fn test_pane_split_returns_new_pane_id() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["pane", "split", "1-3", "--direction", "right", "--no-focus"],
            ok_output(r#"{"result":{"pane":{"pane_id":"1-5"}}}"#),
        );
        let herdr = client_with(mock);
        let new_id = herdr.pane_split("1-3", "right", true).unwrap();
        assert_eq!(new_id, "1-5");
    }

    #[test]
    fn test_pane_split_without_no_focus() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["pane", "split", "1-3", "--direction", "down"],
            ok_output(r#"{"result":{"pane":{"pane_id":"1-6"}}}"#),
        );
        let herdr = client_with(mock);
        let new_id = herdr.pane_split("1-3", "down", false).unwrap();
        assert_eq!(new_id, "1-6");
    }

    #[test]
    fn test_tab_focus_sends_tab_id() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["tab", "focus", "1:2"],
            ok_output(""),
        );
        let herdr = client_with(mock);
        herdr.tab_focus("1:2").unwrap();
    }

    #[test]
    fn test_pane_run_sends_command() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["pane", "run", "1-3", "cd /repo && opencode ."],
            ok_output(""),
        );
        let herdr = client_with(mock);
        herdr.pane_run("1-3", "cd /repo && opencode .").unwrap();
    }

    #[test]
    fn test_create_worktree_parse_error_on_invalid_json() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                "/repo",
                "--branch",
                "feat/login",
                "--base",
                "main",
                "--label",
                "app-feat/login",
                "--no-focus",
                "--json",
            ],
            ok_output("not json"),
        );
        let herdr = client_with(mock);
        let err = herdr
            .create_worktree("/repo", "feat/login", "main", "app-feat/login", None)
            .unwrap_err();
        match err {
            MatError::Herdr { ref stderr, .. } => {
                assert!(stderr.contains("failed to parse JSON"));
            }
            _ => panic!("expected MatError::Herdr with parse error"),
        }
    }
}
