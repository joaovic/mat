use clap::{error::ErrorKind, Parser, Subcommand};

use crate::error::MatError;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Create {
        task_type: String,
        task_name: String,
        source: Option<String>,
        no_worktree: bool,
        use_tmux: bool,
    },
    Close {
        no_merge: bool,
    },
    ConfigList,
    ConfigGet {
        key: String,
    },
    ConfigSet {
        key: String,
        value: String,
        global: bool,
    },
}

#[derive(Parser)]
#[command(name = "mat", version = "0.2.0")]
#[command(about = "Multi-Agent Task - Create TMUX window + Git worktree for new features", long_about = None)]
struct Cli {
    #[arg(help = "Task type (e.g., feat, fix, chore, refactor)")]
    task_type: Option<String>,

    #[arg(help = "Task name (e.g., increase-counter)")]
    task_name: Option<String>,

    #[arg(short, long, help = "Base branch to create worktree from")]
    source: Option<String>,

    #[arg(long, help = "Skip worktree creation, only create branch")]
    no_worktree: bool,

    #[arg(long, help = "Force tmux window creation even outside tmux")]
    use_tmux: bool,

    #[arg(short = 'c', long, help = "Close the current task worktree (deprecated, use 'mat close')")]
    close: bool,

    #[arg(long, help = "Skip merge on close")]
    no_merge: bool,

    #[command(subcommand)]
    command: Option<MatCommand>,
}

#[derive(Subcommand)]
enum MatCommand {
    /// Close the current task worktree
    Close {
        #[arg(long, help = "Skip merge on close")]
        no_merge: bool,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// List all config values with sources
    List,
    /// Get a config value
    Get {
        key: String,
    },
    /// Set a config value
    Set {
        key: String,
        value: String,
        #[arg(long, help = "Write to global config instead of project config")]
        global: bool,
    },
}

pub fn parse() -> Result<Command, MatError> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            if matches!(e.kind(), ErrorKind::DisplayVersion | ErrorKind::DisplayHelp) {
                eprintln!("{e}");
                std::process::exit(0);
            }
            return Err(MatError::Validation {
                message: e.to_string(),
            });
        }
    };
    cli_to_command(cli)
}

fn cli_to_command(cli: Cli) -> Result<Command, MatError> {
    if cli.close {
        eprintln!("Warning: --close is deprecated, use 'mat close' instead");
        return Ok(Command::Close {
            no_merge: cli.no_merge,
        });
    }

    match cli.command {
        Some(MatCommand::Close { no_merge }) => Ok(Command::Close { no_merge }),
        Some(MatCommand::Config { command }) => match command {
            ConfigCommands::List => Ok(Command::ConfigList),
            ConfigCommands::Get { key } => Ok(Command::ConfigGet { key }),
            ConfigCommands::Set { key, value, global } => Ok(Command::ConfigSet { key, value, global }),
        },
        None => {
            let task_type = cli.task_type.ok_or_else(|| MatError::Validation {
                message: "Task type is required".into(),
            })?;
            let task_name = cli.task_name.ok_or_else(|| MatError::Validation {
                message: "Task name is required".into(),
            })?;
            Ok(Command::Create {
                task_type,
                task_name,
                source: cli.source,
                no_worktree: cli.no_worktree,
                use_tmux: cli.use_tmux,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_feat_login() {
        let cli = Cli::try_parse_from(["mat", "feat", "login"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(
            cmd,
            Command::Create {
                task_type: "feat".into(),
                task_name: "login".into(),
                source: None,
                no_worktree: false,
                use_tmux: false,
            }
        );
    }

    #[test]
    fn test_create_fix_bug_no_worktree() {
        let cli = Cli::try_parse_from(["mat", "fix", "bug", "--no-worktree"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(
            cmd,
            Command::Create {
                task_type: "fix".into(),
                task_name: "bug".into(),
                source: None,
                no_worktree: true,
                use_tmux: false,
            }
        );
    }

    #[test]
    fn test_create_with_source() {
        let cli = Cli::try_parse_from(["mat", "feat", "login", "--source", "develop"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(
            cmd,
            Command::Create {
                task_type: "feat".into(),
                task_name: "login".into(),
                source: Some("develop".into()),
                no_worktree: false,
                use_tmux: false,
            }
        );
    }

    #[test]
    fn test_create_with_short_source() {
        let cli = Cli::try_parse_from(["mat", "feat", "login", "-s", "staging"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert!(matches!(
            cmd,
            Command::Create {
                source: Some(s),
                ..
            } if s == "staging"
        ));
    }

    #[test]
    fn test_create_with_use_tmux() {
        let cli = Cli::try_parse_from(["mat", "feat", "login", "--use-tmux"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert!(matches!(
            cmd,
            Command::Create { use_tmux: true, .. }
        ));
    }

    #[test]
    fn test_close_subcommand() {
        let cli = Cli::try_parse_from(["mat", "close"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(cmd, Command::Close { no_merge: false });
    }

    #[test]
    fn test_close_subcommand_with_no_merge() {
        let cli = Cli::try_parse_from(["mat", "close", "--no-merge"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(cmd, Command::Close { no_merge: true });
    }

    #[test]
    fn test_close_deprecated_flag() {
        let cli = Cli::try_parse_from(["mat", "--close"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(cmd, Command::Close { no_merge: false });
    }

    #[test]
    fn test_close_deprecated_short_flag() {
        let cli = Cli::try_parse_from(["mat", "-c"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(cmd, Command::Close { no_merge: false });
    }

    #[test]
    fn test_close_deprecated_flag_with_no_merge() {
        let cli = Cli::try_parse_from(["mat", "--close", "--no-merge"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(cmd, Command::Close { no_merge: true });
    }

    #[test]
    fn test_config_list() {
        let cli = Cli::try_parse_from(["mat", "config", "list"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(cmd, Command::ConfigList);
    }

    #[test]
    fn test_config_get() {
        let cli = Cli::try_parse_from(["mat", "config", "get", "default_branch"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(
            cmd,
            Command::ConfigGet {
                key: "default_branch".into()
            }
        );
    }

    #[test]
    fn test_config_set() {
        let cli =
            Cli::try_parse_from(["mat", "config", "set", "merge_strategy", "fast-forward"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(
            cmd,
            Command::ConfigSet {
                key: "merge_strategy".into(),
                value: "fast-forward".into(),
                global: false,
            }
        );
    }

    #[test]
    fn test_config_set_global() {
        let cli =
            Cli::try_parse_from(["mat", "config", "set", "--global", "default_branch", "develop"])
                .unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(
            cmd,
            Command::ConfigSet {
                key: "default_branch".into(),
                value: "develop".into(),
                global: true,
            }
        );
    }

    #[test]
    fn test_create_missing_task_type_returns_validation_error() {
        let cli = Cli::try_parse_from(["mat"]).unwrap();
        let result = cli_to_command(cli);
        assert!(result.is_err());
        match result.unwrap_err() {
            MatError::Validation { message } => {
                assert!(message.contains("Task type is required"));
            }
            _ => panic!("Expected MatError::Validation"),
        }
    }

    #[test]
    fn test_create_missing_task_name_returns_validation_error() {
        let cli = Cli::try_parse_from(["mat", "feat"]).unwrap();
        let result = cli_to_command(cli);
        assert!(result.is_err());
        match result.unwrap_err() {
            MatError::Validation { message } => {
                assert!(message.contains("Task name is required"));
            }
            _ => panic!("Expected MatError::Validation"),
        }
    }

    #[test]
    fn test_deprecation_warning_printed_for_long_flag() {
        let cli = Cli::try_parse_from(["mat", "--close"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(cmd, Command::Close { no_merge: false });
    }

    #[test]
    fn test_deprecation_warning_printed_for_short_flag() {
        let cli = Cli::try_parse_from(["mat", "-c"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(cmd, Command::Close { no_merge: false });
    }

    #[test]
    fn test_config_set_validates_key_value_positional() {
        let cli =
            Cli::try_parse_from(["mat", "config", "set", "delete_branch", "true"]).unwrap();
        let cmd = cli_to_command(cli).unwrap();
        assert_eq!(
            cmd,
            Command::ConfigSet {
                key: "delete_branch".into(),
                value: "true".into(),
                global: false,
            }
        );
    }
}
