use crate::error::MatError;
use crate::git::CommandRunner;

pub struct TmuxClient<R: CommandRunner> {
    runner: R,
}

impl<R: CommandRunner> TmuxClient<R> {
    pub fn new(runner: R) -> Self {
        TmuxClient { runner }
    }

    fn run_tmux(&self, args: &[&str]) -> Result<crate::git::CommandOutput, MatError> {
        match self.runner.run("tmux", args) {
            Ok(output) => Ok(output),
            Err(MatError::Git { command, stderr }) => Err(MatError::Tmux { command, stderr }),
            Err(e) => Err(e),
        }
    }

    pub fn new_window(&self, path: &str) -> Result<i32, MatError> {
        let output =
            self.run_tmux(&["new-window", "-c", path, "-P", "-F", "#{window_index}"])?;
        let index = output.stdout.trim().parse::<i32>().map_err(|e| {
            MatError::Tmux {
                command: "tmux new-window".into(),
                stderr: format!("Failed to parse window index: {}", e),
            }
        })?;
        Ok(index)
    }

    pub fn rename_window(&self, name: &str) -> Result<(), MatError> {
        self.run_tmux(&["rename-window", name])?;
        Ok(())
    }

    pub fn list_windows(&self) -> Result<Vec<i32>, MatError> {
        let output = self.run_tmux(&["list-windows", "-F", "#{window_index}"])?;
        let windows = output
            .stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                l.trim().parse::<i32>().map_err(|e| MatError::Tmux {
                    command: "tmux list-windows".into(),
                    stderr: format!("Failed to parse window index: {}", e),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(windows)
    }

    pub fn select_window(&self, target: &str) -> Result<(), MatError> {
        self.run_tmux(&["select-window", "-t", target])?;
        Ok(())
    }

    pub fn kill_window(&self, index: i32) -> Result<(), MatError> {
        self.run_tmux(&["kill-window", "-t", &index.to_string()])?;
        Ok(())
    }

    pub fn set_buffer(&self, text: &str) -> Result<(), MatError> {
        self.run_tmux(&["set-buffer", text])?;
        Ok(())
    }

    pub fn send_keys(&self, target: &str, keys: &str) -> Result<(), MatError> {
        self.run_tmux(&["send-keys", "-t", target, keys, "Enter"])?;
        Ok(())
    }

    #[cfg(test)]
    pub fn display_message(&self, format: &str) -> Result<String, MatError> {
        let output = self.run_tmux(&["display-message", "-p", format])?;
        Ok(output.stdout.trim().to_string())
    }

    #[cfg(test)]
    pub fn get_prefix(&self) -> Result<String, MatError> {
        let output = self.run_tmux(&["show-options", "-g", "prefix"])?;
        let prefix = output
            .stdout
            .trim()
            .strip_prefix("prefix ")
            .map(|s| s.replace("C-", "Ctrl-"))
            .unwrap_or_else(|| "Ctrl-b".to_string());
        Ok(prefix)
    }

    #[cfg(test)]
    pub fn is_running(&self) -> Result<bool, MatError> {
        match self.runner.run("tmux", &["list-sessions"]) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn current_window_index(&self) -> Result<i32, MatError> {
        let output = self.run_tmux(&["display-message", "-p", "#{window_index}"])?;
        let index = output.stdout.trim().parse::<i32>().map_err(|e| {
            MatError::Tmux {
                command: "tmux display-message".into(),
                stderr: format!("Failed to parse window index: {}", e),
            }
        })?;
        Ok(index)
    }

    pub fn close_current_window(&self) -> Result<(), MatError> {
        let windows = self.list_windows()?;
        let current = self.current_window_index()?;

        if windows.len() > 1 {
            let target = windows.iter().find(|&&w| w != current).copied().unwrap_or(0);
            self.select_window(&target.to_string())?;
            self.kill_window(current)?;
        } else {
            // Only window: kill-window may exit non-zero on PSMUX when killing
            // the session's last window. Fall back to kill-session.
            if self.kill_window(current).is_err() {
                self.run_tmux(&["kill-session"])?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{CommandOutput, MockRunner};

    fn mock_tmux() -> MockRunner {
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

    fn client_with(mock: MockRunner) -> TmuxClient<MockRunner> {
        TmuxClient::new(mock)
    }

    #[test]
    fn test_new_window_constructs_correct_args() {
        let mut mock = mock_tmux();
        mock.add_response(
            "tmux",
            &["new-window", "-c", "/path", "-P", "-F", "#{window_index}"],
            ok_output("2\n"),
        );
        let tmux = client_with(mock);
        assert_eq!(tmux.new_window("/path").unwrap(), 2);
    }

    #[test]
    fn test_rename_window_passes_name() {
        let mut mock = mock_tmux();
        mock.add_response(
            "tmux",
            &["rename-window", "dashboard-feat/login"],
            ok_output(""),
        );
        let tmux = client_with(mock);
        tmux.rename_window("dashboard-feat/login").unwrap();
    }

    #[test]
    fn test_list_windows_parses_indices() {
        let mut mock = mock_tmux();
        mock.add_response(
            "tmux",
            &["list-windows", "-F", "#{window_index}"],
            ok_output("0\n1\n2\n"),
        );
        let tmux = client_with(mock);
        assert_eq!(tmux.list_windows().unwrap(), vec![0, 1, 2]);
    }

    #[test]
    fn test_select_window_passes_target() {
        let mut mock = mock_tmux();
        mock.add_response("tmux", &["select-window", "-t", "0"], ok_output(""));
        let tmux = client_with(mock);
        tmux.select_window("0").unwrap();
    }

    #[test]
    fn test_kill_window_passes_index() {
        let mut mock = mock_tmux();
        mock.add_response("tmux", &["kill-window", "-t", "1"], ok_output(""));
        let tmux = client_with(mock);
        tmux.kill_window(1).unwrap();
    }

    #[test]
    fn test_set_buffer_stores_text() {
        let mut mock = mock_tmux();
        mock.add_response(
            "tmux",
            &["set-buffer", "cd /path"],
            ok_output(""),
        );
        let tmux = client_with(mock);
        tmux.set_buffer("cd /path").unwrap();
    }

    #[test]
    fn test_send_keys_passes_target_and_keys() {
        let mut mock = mock_tmux();
        mock.add_response(
            "tmux",
            &["send-keys", "-t", "%0", "cd /path", "Enter"],
            ok_output(""),
        );
        let tmux = client_with(mock);
        tmux.send_keys("%0", "cd /path").unwrap();
    }

    #[test]
    fn test_display_message_queries_format() {
        let mut mock = mock_tmux();
        mock.add_response(
            "tmux",
            &["display-message", "-p", "#{window_index}"],
            ok_output("2\n"),
        );
        let tmux = client_with(mock);
        assert_eq!(tmux.display_message("#{window_index}").unwrap(), "2");
    }

    #[test]
    fn test_get_prefix_parses_cb() {
        let mut mock = mock_tmux();
        mock.add_response(
            "tmux",
            &["show-options", "-g", "prefix"],
            ok_output("prefix C-b\n"),
        );
        let tmux = client_with(mock);
        assert_eq!(tmux.get_prefix().unwrap(), "Ctrl-b");
    }

    #[test]
    fn test_get_prefix_falls_back_when_unparseable() {
        let mut mock = mock_tmux();
        mock.add_response(
            "tmux",
            &["show-options", "-g", "prefix"],
            ok_output("garbage\n"),
        );
        let tmux = client_with(mock);
        assert_eq!(tmux.get_prefix().unwrap(), "Ctrl-b");
    }

    #[test]
    fn test_is_running_returns_true_when_tmux_reachable() {
        let mut mock = mock_tmux();
        mock.add_response("tmux", &["list-sessions"], ok_output(""));
        let tmux = client_with(mock);
        assert!(tmux.is_running().unwrap());
    }

    #[test]
    fn test_is_running_returns_false_when_tmux_unreachable() {
        let mut mock = mock_tmux();
        mock.add_error(
            "tmux",
            &["list-sessions"],
            MatError::Tmux {
                command: "tmux list-sessions".into(),
                stderr: "no server running".into(),
            },
        );
        let tmux = client_with(mock);
        assert!(!tmux.is_running().unwrap());
    }

    #[test]
    fn test_current_window_index_parses_output() {
        let mut mock = mock_tmux();
        mock.add_response(
            "tmux",
            &["display-message", "-p", "#{window_index}"],
            ok_output("1\n"),
        );
        let tmux = client_with(mock);
        assert_eq!(tmux.current_window_index().unwrap(), 1);
    }

    #[test]
    fn test_close_current_window_switches_before_kill() {
        let mut mock = mock_tmux();
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
        let tmux = client_with(mock);
        tmux.close_current_window().unwrap();
    }

    #[test]
    fn test_close_current_window_kills_only_window() {
        let mut mock = mock_tmux();
        mock.add_response(
            "tmux",
            &["list-windows", "-F", "#{window_index}"],
            ok_output("0\n"),
        );
        mock.add_response(
            "tmux",
            &["display-message", "-p", "#{window_index}"],
            ok_output("0\n"),
        );
        mock.add_response("tmux", &["kill-window", "-t", "0"], ok_output(""));
        let tmux = client_with(mock);
        tmux.close_current_window().unwrap();
    }

    #[test]
    fn test_close_current_window_falls_back_to_kill_session() {
        let mut mock = mock_tmux();
        mock.add_response(
            "tmux",
            &["list-windows", "-F", "#{window_index}"],
            ok_output("0\n"),
        );
        mock.add_response(
            "tmux",
            &["display-message", "-p", "#{window_index}"],
            ok_output("0\n"),
        );
        mock.add_error(
            "tmux",
            &["kill-window", "-t", "0"],
            MatError::Tmux {
                command: "tmux kill-window -t 0".into(),
                stderr: "kill-window failed on last window".into(),
            },
        );
        mock.add_response("tmux", &["kill-session"], ok_output(""));
        let tmux = client_with(mock);
        tmux.close_current_window().unwrap();
    }

    #[test]
    fn test_new_window_returns_tmux_error_on_failure() {
        let mut mock = mock_tmux();
        mock.add_error(
            "tmux",
            &["new-window", "-c", "/bad", "-P", "-F", "#{window_index}"],
            MatError::Tmux {
                command: "tmux new-window -c /bad".into(),
                stderr: "no server running".into(),
            },
        );
        let tmux = client_with(mock);
        let err = tmux.new_window("/bad").unwrap_err();
        match err {
            MatError::Tmux { ref stderr, .. } => {
                assert!(stderr.contains("no server running"));
            }
            _ => panic!("expected MatError::Tmux"),
        }
    }

    #[test]
    fn test_list_windows_returns_tmux_error_on_failure() {
        let mut mock = mock_tmux();
        mock.add_error(
            "tmux",
            &["list-windows", "-F", "#{window_index}"],
            MatError::Tmux {
                command: "tmux list-windows".into(),
                stderr: "no server running".into(),
            },
        );
        let tmux = client_with(mock);
        let err = tmux.list_windows().unwrap_err();
        match err {
            MatError::Tmux { .. } => {}
            _ => panic!("expected MatError::Tmux"),
        }
    }

    #[test]
    fn test_rename_window_returns_tmux_error_on_failure() {
        let mut mock = mock_tmux();
        mock.add_error(
            "tmux",
            &["rename-window", "test"],
            MatError::Tmux {
                command: "tmux rename-window test".into(),
                stderr: "can't find window".into(),
            },
        );
        let tmux = client_with(mock);
        let err = tmux.rename_window("test").unwrap_err();
        match err {
            MatError::Tmux { ref stderr, .. } => {
                assert!(stderr.contains("can't find window"));
            }
            _ => panic!("expected MatError::Tmux"),
        }
    }

    #[test]
    fn test_set_buffer_returns_tmux_error_on_failure() {
        let mut mock = mock_tmux();
        mock.add_error(
            "tmux",
            &["set-buffer", "text"],
            MatError::Tmux {
                command: "tmux set-buffer text".into(),
                stderr: "no server".into(),
            },
        );
        let tmux = client_with(mock);
        let err = tmux.set_buffer("text").unwrap_err();
        match err {
            MatError::Tmux { .. } => {}
            _ => panic!("expected MatError::Tmux"),
        }
    }

    #[test]
    fn test_display_message_returns_tmux_error_on_failure() {
        let mut mock = mock_tmux();
        mock.add_error(
            "tmux",
            &["display-message", "-p", "#{window_index}"],
            MatError::Tmux {
                command: "tmux display-message".into(),
                stderr: "no server".into(),
            },
        );
        let tmux = client_with(mock);
        let err = tmux.display_message("#{window_index}").unwrap_err();
        match err {
            MatError::Tmux { .. } => {}
            _ => panic!("expected MatError::Tmux"),
        }
    }

    #[test]
    fn test_select_window_returns_tmux_error_on_failure() {
        let mut mock = mock_tmux();
        mock.add_error(
            "tmux",
            &["select-window", "-t", "99"],
            MatError::Tmux {
                command: "tmux select-window -t 99".into(),
                stderr: "index out of bounds".into(),
            },
        );
        let tmux = client_with(mock);
        let err = tmux.select_window("99").unwrap_err();
        match err {
            MatError::Tmux { .. } => {}
            _ => panic!("expected MatError::Tmux"),
        }
    }

    #[test]
    fn test_kill_window_returns_tmux_error_on_failure() {
        let mut mock = mock_tmux();
        mock.add_error(
            "tmux",
            &["kill-window", "-t", "99"],
            MatError::Tmux {
                command: "tmux kill-window -t 99".into(),
                stderr: "can't find window".into(),
            },
        );
        let tmux = client_with(mock);
        let err = tmux.kill_window(99).unwrap_err();
        match err {
            MatError::Tmux { .. } => {}
            _ => panic!("expected MatError::Tmux"),
        }
    }

    #[test]
    fn test_current_window_index_returns_tmux_error_on_failure() {
        let mut mock = mock_tmux();
        mock.add_error(
            "tmux",
            &["display-message", "-p", "#{window_index}"],
            MatError::Tmux {
                command: "tmux display-message".into(),
                stderr: "no server".into(),
            },
        );
        let tmux = client_with(mock);
        let err = tmux.current_window_index().unwrap_err();
        match err {
            MatError::Tmux { .. } => {}
            _ => panic!("expected MatError::Tmux"),
        }
    }

    #[test]
    fn test_get_prefix_returns_tmux_error_on_failure() {
        let mut mock = mock_tmux();
        mock.add_error(
            "tmux",
            &["show-options", "-g", "prefix"],
            MatError::Tmux {
                command: "tmux show-options -g prefix".into(),
                stderr: "no server".into(),
            },
        );
        let tmux = client_with(mock);
        let err = tmux.get_prefix().unwrap_err();
        match err {
            MatError::Tmux { .. } => {}
            _ => panic!("expected MatError::Tmux"),
        }
    }
}
