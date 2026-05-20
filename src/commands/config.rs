use crate::config::Config;
use crate::display::print_success;
use crate::error::MatError;

pub fn handle_config_list() -> Result<(), MatError> {
    let config = Config::load()?;
    let entries = config.effective_values();

    for entry in &entries {
        let annotation = entry.source_annotation();
        println!("{} = {}  {}", entry.key, entry.value, annotation);
    }

    Ok(())
}

pub fn handle_config_get(key: &str) -> Result<(), MatError> {
    let valid_keys = [
        "default_branch",
        "delete_branch",
        "merge_strategy",
        "worktree_root",
        "tmux.enabled",
    ];

    if !valid_keys.contains(&key) {
        return Err(MatError::Config {
            key: key.into(),
            reason: format!(
                "Unknown config key '{}'. Valid keys: {}",
                key,
                valid_keys.join(", ")
            ),
        });
    }

    let config = Config::load()?;

    match config.effective_value(key) {
        Some(entry) => {
            let annotation = entry.source_annotation();
            println!("{} = {}  {}", entry.key, entry.value, annotation);
            Ok(())
        }
        None => Err(MatError::Config {
            key: key.into(),
            reason: "Could not retrieve effective value".into(),
        }),
    }
}

pub fn handle_config_set(key: &str, value: &str, global: bool) -> Result<(), MatError> {
    let valid_keys = [
        "default_branch",
        "delete_branch",
        "merge_strategy",
        "worktree_root",
        "tmux.enabled",
    ];

    if !valid_keys.contains(&key) {
        return Err(MatError::Config {
            key: key.into(),
            reason: format!(
                "Unknown config key '{}'. Valid keys: {}",
                key,
                valid_keys.join(", ")
            ),
        });
    }

    if key == "delete_branch" {
        match value {
            "true" | "false" => {}
            _ => {
                return Err(MatError::Config {
                    key: key.into(),
                    reason: "delete_branch must be 'true' or 'false'".into(),
                });
            }
        }
    }

    if key == "merge_strategy" {
        match value {
            "merge-commit" | "fast-forward" => {}
            _ => {
                return Err(MatError::Config {
                    key: key.into(),
                    reason: "merge_strategy must be 'merge-commit' or 'fast-forward'".into(),
                });
            }
        }
    }

    if key == "tmux.enabled" {
        match value {
            "auto" | "always" | "never" => {}
            _ => {
                return Err(MatError::Config {
                    key: key.into(),
                    reason: "tmux.enabled must be 'auto', 'always', or 'never'".into(),
                });
            }
        }
    }

    Config::set(key, value, global)?;

    let file_name = if global {
        "~/.config/mat/config.toml"
    } else {
        ".mat.toml"
    };
    print_success(&format!("Set {} = {} in {}", key, value, file_name));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_keys_list() {
        let valid_keys = [
            "default_branch",
            "delete_branch",
            "merge_strategy",
            "worktree_root",
            "tmux.enabled",
        ];
        assert!(valid_keys.contains(&"default_branch"));
        assert!(valid_keys.contains(&"delete_branch"));
        assert!(valid_keys.contains(&"merge_strategy"));
        assert!(valid_keys.contains(&"worktree_root"));
        assert!(valid_keys.contains(&"tmux.enabled"));
        assert!(!valid_keys.contains(&"invalid"));
    }

    #[test]
    fn test_handle_config_get_unknown_key_returns_error() {
        let result = handle_config_get("nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            MatError::Config { key, .. } => assert_eq!(key, "nonexistent"),
            _ => panic!("expected MatError::Config"),
        }
    }

    #[test]
    fn test_handle_config_set_delete_branch_validation() {
        let result = handle_config_set("delete_branch", "invalid", false);
        assert!(result.is_err());
        match result.unwrap_err() {
            MatError::Config { key, .. } => assert_eq!(key, "delete_branch"),
            _ => panic!("expected MatError::Config"),
        }
    }

    #[test]
    fn test_handle_config_set_merge_strategy_validation() {
        let result = handle_config_set("merge_strategy", "invalid", false);
        assert!(result.is_err());
        match result.unwrap_err() {
            MatError::Config { key, .. } => assert_eq!(key, "merge_strategy"),
            _ => panic!("expected MatError::Config"),
        }
    }

    #[test]
    fn test_handle_config_set_tmux_enabled_validation() {
        let result = handle_config_set("tmux.enabled", "invalid", false);
        assert!(result.is_err());
        match result.unwrap_err() {
            MatError::Config { key, .. } => assert_eq!(key, "tmux.enabled"),
            _ => panic!("expected MatError::Config"),
        }
    }
}
