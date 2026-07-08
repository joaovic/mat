use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum MatError {
    Git { command: String, stderr: String },
    Herdr { command: String, stderr: String },
    Config { key: String, reason: String },
    Validation { message: String },
    Io(Arc<std::io::Error>),
    Glob { message: String },
    PatternError { message: String },
    SettingsNotFound,
}

impl fmt::Display for MatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatError::Git { command, stderr } => {
                write!(f, "Git command '{}' failed: {}", command, stderr.trim())
            }
            MatError::Herdr { command, stderr } => {
                write!(f, "Herdr command '{}' failed: {}", command, stderr.trim())
            }
            MatError::Config { key, reason } => {
                write!(f, "Config error for '{}': {}", key, reason)
            }
            MatError::Validation { message } => {
                write!(f, "{}", message)
            }
            MatError::Io(err) => {
                write!(f, "IO error: {}", err.as_ref())
            }
            MatError::Glob { message } => {
                write!(f, "Glob error: {}", message)
            }
            MatError::PatternError { message } => {
                write!(f, "Pattern error: {}", message)
            }
            MatError::SettingsNotFound => {
                write!(f, "Settings file not found")
            }
        }
    }
}

impl From<std::io::Error> for MatError {
    fn from(err: std::io::Error) -> Self {
        MatError::Io(Arc::new(err))
    }
}

impl From<glob::GlobError> for MatError {
    fn from(err: glob::GlobError) -> Self {
        MatError::Glob {
            message: err.to_string(),
        }
    }
}

impl From<glob::PatternError> for MatError {
    fn from(err: glob::PatternError) -> Self {
        MatError::PatternError {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_error_display_shows_command_and_stderr() {
        let err = MatError::Git {
            command: "git push".into(),
            stderr: "fatal: not a git repository\n".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("git push"));
        assert!(msg.contains("fatal: not a git repository"));
    }

    #[test]
    fn test_herdr_error_display_shows_command_and_stderr() {
        let err = MatError::Herdr {
            command: "herdr workspace create".into(),
            stderr: "no server running\n".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("herdr workspace create"));
        assert!(msg.contains("no server running"));
    }

    #[test]
    fn test_config_error_display_shows_key_and_reason() {
        let err = MatError::Config {
            key: "default_branch".into(),
            reason: "invalid value".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("default_branch"));
        assert!(msg.contains("invalid value"));
    }

    #[test]
    fn test_validation_error_display_shows_message() {
        let err = MatError::Validation {
            message: "Task type is required".into(),
        };
        let msg = err.to_string();
        assert_eq!(msg, "Task type is required");
    }

    #[test]
    fn test_io_error_display_contains_error_text() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = MatError::Io(Arc::new(io_err));
        let msg = err.to_string();
        assert!(msg.contains("IO error"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let mat_err: MatError = io_err.into();
        let msg = mat_err.to_string();
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn test_git_error_stderr_trimmed() {
        let err = MatError::Git {
            command: "git status".into(),
            stderr: "  trailing spaces  \n".into(),
        };
        let msg = err.to_string();
        assert!(!msg.ends_with("  \n"));
        assert!(msg.contains("trailing spaces"));
    }
}
