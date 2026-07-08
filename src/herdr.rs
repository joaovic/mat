use crate::error::MatError;
use crate::git::CommandRunner;

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

    pub fn create_workspace(&self, path: &str, label: &str) -> Result<String, MatError> {
        let output = self.run_herdr(&["workspace", "create", "--cwd", path, "--label", label])?;
        let id = output.stdout.trim().to_string();
        Ok(id)
    }

    pub fn find_workspace_by_path(&self, path: &str) -> Result<Option<String>, MatError> {
        let output = match self.run_herdr(&["workspace", "list"]) {
            Ok(o) => o,
            Err(MatError::Herdr { ref stderr, .. }) if stderr.contains("server is not running") => {
                return Ok(None);
            }
            Err(e) => return Err(e),
        };

        for line in output.stdout.lines() {
            if line.contains(path) {
                let id = line.split_whitespace().next().unwrap_or("").to_string();
                if !id.is_empty() {
                    return Ok(Some(id));
                }
            }
        }
        Ok(None)
    }

    pub fn close_workspace(&self, id: &str) -> Result<(), MatError> {
        self.run_herdr(&["workspace", "close", id])?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn list_workspaces(&self) -> Result<Vec<String>, MatError> {
        let output = self.run_herdr(&["workspace", "list"])?;
        let ids = output
            .stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                l.split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string()
            })
            .filter(|id| !id.is_empty())
            .collect();
        Ok(ids)
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

    #[test]
    fn test_create_workspace_returns_id() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["workspace", "create", "--cwd", "/path", "--label", "my-label"],
            ok_output("ws-123\n"),
        );
        let herdr = client_with(mock);
        assert_eq!(herdr.create_workspace("/path", "my-label").unwrap(), "ws-123");
    }

    #[test]
    fn test_list_workspaces_parses_ids() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["workspace", "list"],
            ok_output("ws-1  /path1  label1\nws-2  /path2  label2\n"),
        );
        let herdr = client_with(mock);
        assert_eq!(herdr.list_workspaces().unwrap(), vec!["ws-1", "ws-2"]);
    }

    #[test]
    fn test_close_workspace_passes_id() {
        let mut mock = mock_herdr();
        mock.add_response("herdr", &["workspace", "close", "ws-1"], ok_output(""));
        let herdr = client_with(mock);
        herdr.close_workspace("ws-1").unwrap();
    }

    #[test]
    fn test_find_workspace_by_path_finds_matching() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["workspace", "list"],
            ok_output("ws-1  /some/path  label1\nws-2  /target/path  label2\n"),
        );
        let herdr = client_with(mock);
        let result = herdr.find_workspace_by_path("/target/path").unwrap();
        assert_eq!(result, Some("ws-2".to_string()));
    }

    #[test]
    fn test_find_workspace_by_path_returns_none_when_not_found() {
        let mut mock = mock_herdr();
        mock.add_response(
            "herdr",
            &["workspace", "list"],
            ok_output("ws-1  /some/path  label1\n"),
        );
        let herdr = client_with(mock);
        let result = herdr.find_workspace_by_path("/other/path").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_workspace_by_path_returns_none_when_server_not_running() {
        let mut mock = mock_herdr();
        mock.add_error(
            "herdr",
            &["workspace", "list"],
            MatError::Herdr {
                command: "herdr workspace list".into(),
                stderr: "Error: server is not running".into(),
            },
        );
        let herdr = client_with(mock);
        let result = herdr.find_workspace_by_path("/path").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_create_workspace_returns_error_on_failure() {
        let mut mock = mock_herdr();
        mock.add_error(
            "herdr",
            &["workspace", "create", "--cwd", "/bad", "--label", "test"],
            MatError::Herdr {
                command: "herdr workspace create".into(),
                stderr: "failed to create workspace".into(),
            },
        );
        let herdr = client_with(mock);
        let err = herdr.create_workspace("/bad", "test").unwrap_err();
        match err {
            MatError::Herdr { ref stderr, .. } => {
                assert!(stderr.contains("failed to create workspace"));
            }
            _ => panic!("expected MatError::Herdr"),
        }
    }

    #[test]
    fn test_list_workspaces_returns_error_on_failure() {
        let mut mock = mock_herdr();
        mock.add_error(
            "herdr",
            &["workspace", "list"],
            MatError::Herdr {
                command: "herdr workspace list".into(),
                stderr: "server error".into(),
            },
        );
        let herdr = client_with(mock);
        let err = herdr.list_workspaces().unwrap_err();
        match err {
            MatError::Herdr { .. } => {}
            _ => panic!("expected MatError::Herdr"),
        }
    }

    #[test]
    fn test_close_workspace_returns_error_on_failure() {
        let mut mock = mock_herdr();
        mock.add_error(
            "herdr",
            &["workspace", "close", "ws-99"],
            MatError::Herdr {
                command: "herdr workspace close ws-99".into(),
                stderr: "workspace not found".into(),
            },
        );
        let herdr = client_with(mock);
        let err = herdr.close_workspace("ws-99").unwrap_err();
        match err {
            MatError::Herdr { ref stderr, .. } => {
                assert!(stderr.contains("workspace not found"));
            }
            _ => panic!("expected MatError::Herdr"),
        }
    }
}
