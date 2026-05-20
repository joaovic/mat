use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use crate::error::MatError;

#[derive(Debug, Clone, PartialEq)]
pub enum MergeStrategy {
    MergeCommit,
    FastForward,
}

impl Default for MergeStrategy {
    fn default() -> Self {
        MergeStrategy::MergeCommit
    }
}

impl std::fmt::Display for MergeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeStrategy::MergeCommit => write!(f, "merge-commit"),
            MergeStrategy::FastForward => write!(f, "fast-forward"),
        }
    }
}

impl<'de> Deserialize<'de> for MergeStrategy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "merge-commit" => Ok(MergeStrategy::MergeCommit),
            "fast-forward" => Ok(MergeStrategy::FastForward),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["merge-commit", "fast-forward"],
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TmuxMode {
    Auto,
    Always,
    Never,
}

impl Default for TmuxMode {
    fn default() -> Self {
        TmuxMode::Auto
    }
}

impl std::fmt::Display for TmuxMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TmuxMode::Auto => write!(f, "auto"),
            TmuxMode::Always => write!(f, "always"),
            TmuxMode::Never => write!(f, "never"),
        }
    }
}

impl<'de> Deserialize<'de> for TmuxMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "auto" => Ok(TmuxMode::Auto),
            "always" => Ok(TmuxMode::Always),
            "never" => Ok(TmuxMode::Never),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["auto", "always", "never"],
            )),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TmuxConfig {
    #[serde(default)]
    pub enabled: TmuxMode,
}

impl Default for TmuxConfig {
    fn default() -> Self {
        TmuxConfig {
            enabled: TmuxMode::Auto,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawConfig {
    default_branch: Option<String>,
    delete_branch: Option<bool>,
    merge_strategy: Option<MergeStrategy>,
    worktree_root: Option<String>,
    tmux: Option<TmuxConfig>,
}

impl Default for RawConfig {
    fn default() -> Self {
        RawConfig {
            default_branch: None,
            delete_branch: None,
            merge_strategy: None,
            worktree_root: None,
            tmux: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    Default,
    Global,
    Project,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Default => write!(f, "default"),
            Source::Global => write!(f, "global"),
            Source::Project => write!(f, "project"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub default_branch: String,
    pub delete_branch: bool,
    pub merge_strategy: MergeStrategy,
    pub worktree_root: Option<String>,
    pub tmux: TmuxConfig,
    pub global_path: Option<PathBuf>,
    pub project_path: Option<PathBuf>,
    pub sources: HashMap<String, Source>,
}

impl Default for Config {
    fn default() -> Self {
        let mut sources = HashMap::new();
        sources.insert("default_branch".into(), Source::Default);
        sources.insert("delete_branch".into(), Source::Default);
        sources.insert("merge_strategy".into(), Source::Default);
        sources.insert("worktree_root".into(), Source::Default);
        sources.insert("tmux.enabled".into(), Source::Default);

        Config {
            default_branch: "main".into(),
            delete_branch: true,
            merge_strategy: MergeStrategy::MergeCommit,
            worktree_root: None,
            tmux: TmuxConfig::default(),
            global_path: None,
            project_path: None,
            sources,
        }
    }
}

impl Config {
    pub fn load() -> Result<Config, MatError> {
        let (global_raw, global_path) = load_global_config()?;
        let (project_raw, project_path) = load_project_config()?;

        Ok(merge_configs(global_raw, global_path, project_raw, project_path))
    }

    pub fn effective_values(&self) -> Vec<ConfigEntry> {
        let keys = [
            "default_branch",
            "delete_branch",
            "merge_strategy",
            "worktree_root",
            "tmux.enabled",
        ];

        keys.iter()
            .map(|key| ConfigEntry {
                key: key.to_string(),
                value: self.value_for_key(key),
                source: self.sources.get(*key).cloned().unwrap_or(Source::Default),
                global_path: self.global_path.clone(),
                project_path: self.project_path.clone(),
            })
            .collect()
    }

    pub fn effective_value(&self, key: &str) -> Option<ConfigEntry> {
        let valid_keys = [
            "default_branch",
            "delete_branch",
            "merge_strategy",
            "worktree_root",
            "tmux.enabled",
        ];

        if !valid_keys.contains(&key) {
            return None;
        }

        Some(ConfigEntry {
            key: key.to_string(),
            value: self.value_for_key(key),
            source: self.sources.get(key).cloned().unwrap_or(Source::Default),
            global_path: self.global_path.clone(),
            project_path: self.project_path.clone(),
        })
    }

    fn value_for_key(&self, key: &str) -> String {
        match key {
            "default_branch" => self.default_branch.clone(),
            "delete_branch" => self.delete_branch.to_string(),
            "merge_strategy" => self.merge_strategy.to_string(),
            "worktree_root" => self
                .worktree_root
                .clone()
                .unwrap_or_else(|| "<not set>".to_string()),
            "tmux.enabled" => self.tmux.enabled.to_string(),
            _ => "<unknown>".to_string(),
        }
    }

    pub fn set(key: &str, value: &str, global: bool) -> Result<(), MatError> {
        let path = if global {
            get_global_config_path()?
        } else {
            get_project_config_path()?
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| MatError::Config {
                key: key.into(),
                reason: format!("Failed to create config directory: {}", e),
            })?;
        }

        let content = if path.exists() {
            fs::read_to_string(&path).map_err(|e| MatError::Config {
                key: key.into(),
                reason: format!("Failed to read config file: {}", e),
            })?
        } else {
            String::new()
        };

        let mut doc: toml::Value = content
            .parse()
            .unwrap_or(toml::Value::Table(toml::value::Table::new()));

        let toml_val = parse_toml_value(value);
        set_toml_key(&mut doc, key, toml_val);

        let output = toml::to_string(&doc).map_err(|e| MatError::Config {
            key: key.into(),
            reason: format!("Failed to serialize config: {}", e),
        })?;

        fs::write(&path, output).map_err(|e| MatError::Config {
            key: key.into(),
            reason: format!("Failed to write config file: {}", e),
        })?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
    pub source: Source,
    pub global_path: Option<PathBuf>,
    pub project_path: Option<PathBuf>,
}

impl ConfigEntry {
    pub fn source_annotation(&self) -> String {
        match self.source {
            Source::Default => "(default)".to_string(),
            Source::Global => {
                if let Some(ref path) = self.global_path {
                    format!("(global: {})", path.display())
                } else {
                    "(global)".to_string()
                }
            }
            Source::Project => {
                if let Some(ref path) = self.project_path {
                    format!("(project: {})", path.file_name().unwrap_or_default().to_string_lossy())
                } else {
                    "(project)".to_string()
                }
            }
        }
    }
}

fn load_global_config() -> Result<(Option<RawConfig>, Option<PathBuf>), MatError> {
    let config_dir = dirs::config_dir().ok_or_else(|| MatError::Config {
        key: "global_path".into(),
        reason: "Could not determine config directory (XDG_CONFIG_HOME not set)".into(),
    })?;

    let path = config_dir.join("mat").join("config.toml");

    if !path.exists() {
        return Ok((None, Some(path)));
    }

    let content = fs::read_to_string(&path).map_err(|e| MatError::Config {
        key: "global config".into(),
        reason: format!("Failed to read {}: {}", path.display(), e),
    })?;

    if content.trim().is_empty() {
        return Ok((Some(RawConfig::default()), Some(path)));
    }

    let raw: RawConfig = toml::from_str(&content).map_err(|e| MatError::Config {
        key: "global config".into(),
        reason: format!("Failed to parse {}: {}", path.display(), e),
    })?;

    Ok((Some(raw), Some(path)))
}

fn load_project_config() -> Result<(Option<RawConfig>, Option<PathBuf>), MatError> {
    let repo_root = get_repo_root()?;

    let path = repo_root.join(".mat.toml");

    if !path.exists() {
        return Ok((None, None));
    }

    let content = fs::read_to_string(&path).map_err(|e| MatError::Config {
        key: "project config".into(),
        reason: format!("Failed to read {}: {}", path.display(), e),
    })?;

    if content.trim().is_empty() {
        return Ok((Some(RawConfig::default()), Some(path)));
    }

    let raw: RawConfig = toml::from_str(&content).map_err(|e| MatError::Config {
        key: "project config".into(),
        reason: format!("Failed to parse {}: {}", path.display(), e),
    })?;

    Ok((Some(raw), Some(path)))
}

fn get_repo_root() -> Result<PathBuf, MatError> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| MatError::Config {
            key: "repo_root".into(),
            reason: format!("Failed to run git rev-parse: {}", e),
        })?;

    if !output.status.success() {
        return Err(MatError::Config {
            key: "repo_root".into(),
            reason: "Not a git repository".into(),
        });
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

fn get_global_config_path() -> Result<PathBuf, MatError> {
    let config_dir = dirs::config_dir().ok_or_else(|| MatError::Config {
        key: "global_path".into(),
        reason: "Could not determine config directory".into(),
    })?;
    Ok(config_dir.join("mat").join("config.toml"))
}

fn get_project_config_path() -> Result<PathBuf, MatError> {
    let repo_root = get_repo_root()?;
    Ok(repo_root.join(".mat.toml"))
}

fn merge_configs(
    global_raw: Option<RawConfig>,
    global_path: Option<PathBuf>,
    project_raw: Option<RawConfig>,
    project_path: Option<PathBuf>,
) -> Config {
    let global = global_raw.unwrap_or_default();
    let project = project_raw.unwrap_or_default();

    let mut sources = HashMap::new();

    let default_branch = resolve_str_field(
        &project.default_branch,
        &global.default_branch,
        "main",
        &mut sources,
    );
    let delete_branch = resolve_bool_field(
        &project.delete_branch,
        &global.delete_branch,
        true,
        &mut sources,
    );
    let merge_strategy = resolve_enum_field(
        &project.merge_strategy,
        &global.merge_strategy,
        MergeStrategy::MergeCommit,
        &mut sources,
    );
    let worktree_root = resolve_option_field(
        &project.worktree_root,
        &global.worktree_root,
        &mut sources,
    );
    let tmux_enabled = resolve_tmux_field(
        &project.tmux,
        &global.tmux,
        TmuxMode::Auto,
        &mut sources,
    );

    Config {
        default_branch,
        delete_branch,
        merge_strategy,
        worktree_root,
        tmux: TmuxConfig { enabled: tmux_enabled },
        global_path,
        project_path,
        sources,
    }
}

fn resolve_str_field(
    project: &Option<String>,
    global: &Option<String>,
    default: &str,
    sources: &mut HashMap<String, Source>,
) -> String {
    if let Some(val) = project {
        sources.insert("default_branch".into(), Source::Project);
        val.clone()
    } else if let Some(val) = global {
        sources.insert("default_branch".into(), Source::Global);
        val.clone()
    } else {
        sources.insert("default_branch".into(), Source::Default);
        default.to_string()
    }
}

fn resolve_bool_field(
    project: &Option<bool>,
    global: &Option<bool>,
    default: bool,
    sources: &mut HashMap<String, Source>,
) -> bool {
    if let Some(val) = project {
        sources.insert("delete_branch".into(), Source::Project);
        *val
    } else if let Some(val) = global {
        sources.insert("delete_branch".into(), Source::Global);
        *val
    } else {
        sources.insert("delete_branch".into(), Source::Default);
        default
    }
}

fn resolve_enum_field<T: Clone + PartialEq>(
    project: &Option<T>,
    global: &Option<T>,
    default: T,
    sources: &mut HashMap<String, Source>,
) -> T
where
    T: PartialEq,
{
    if let Some(val) = project {
        sources.insert("merge_strategy".into(), Source::Project);
        val.clone()
    } else if let Some(val) = global {
        sources.insert("merge_strategy".into(), Source::Global);
        val.clone()
    } else {
        sources.insert("merge_strategy".into(), Source::Default);
        default
    }
}

fn resolve_option_field(
    project: &Option<String>,
    global: &Option<String>,
    sources: &mut HashMap<String, Source>,
) -> Option<String> {
    if let Some(val) = project {
        sources.insert("worktree_root".into(), Source::Project);
        Some(val.clone())
    } else if let Some(val) = global {
        sources.insert("worktree_root".into(), Source::Global);
        Some(val.clone())
    } else {
        sources.insert("worktree_root".into(), Source::Default);
        None
    }
}

fn resolve_tmux_field(
    project: &Option<TmuxConfig>,
    global: &Option<TmuxConfig>,
    default: TmuxMode,
    sources: &mut HashMap<String, Source>,
) -> TmuxMode {
    let project_val = project.as_ref().map(|t| &t.enabled);
    let global_val = global.as_ref().map(|t| &t.enabled);

    if let Some(val) = project_val {
        sources.insert("tmux.enabled".into(), Source::Project);
        val.clone()
    } else if let Some(val) = global_val {
        sources.insert("tmux.enabled".into(), Source::Global);
        val.clone()
    } else {
        sources.insert("tmux.enabled".into(), Source::Default);
        default
    }
}

fn parse_toml_value(value: &str) -> toml::Value {
    if let Ok(b) = value.parse::<bool>() {
        return toml::Value::Boolean(b);
    }
    if let Ok(i) = value.parse::<i64>() {
        return toml::Value::Integer(i);
    }
    if let Ok(f) = value.parse::<f64>() {
        return toml::Value::Float(f);
    }
    toml::Value::String(value.to_string())
}

fn set_toml_key(doc: &mut toml::Value, key: &str, value: toml::Value) {
    let parts: Vec<&str> = key.split('.').collect();

    if parts.len() == 1 {
        if let toml::Value::Table(ref mut table) = doc {
            table.insert(parts[0].to_string(), value);
        }
    } else {
        let mut current = doc;
        for i in 0..parts.len() - 1 {
            current = current
                .as_table_mut()
                .unwrap()
                .entry(parts[i].to_string())
                .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
        }
        current
            .as_table_mut()
            .unwrap()
            .insert(parts[parts.len() - 1].to_string(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_strategy_deserialize_merge_commit() {
        let toml_str = "strategy = \"merge-commit\"";
        #[derive(Deserialize)]
        struct Test {
            strategy: MergeStrategy,
        }
        let test: Test = toml::from_str(toml_str).unwrap();
        assert_eq!(test.strategy, MergeStrategy::MergeCommit);
    }

    #[test]
    fn test_merge_strategy_deserialize_fast_forward() {
        let toml_str = "strategy = \"fast-forward\"";
        #[derive(Deserialize)]
        struct Test {
            strategy: MergeStrategy,
        }
        let test: Test = toml::from_str(toml_str).unwrap();
        assert_eq!(test.strategy, MergeStrategy::FastForward);
    }

    #[test]
    fn test_merge_strategy_deserialize_invalid() {
        let toml_str = "strategy = \"invalid\"";
        #[derive(Deserialize)]
        struct Test {
            strategy: MergeStrategy,
        }
        let result: Result<Test, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_tmux_mode_deserialize_auto() {
        let toml_str = "mode = \"auto\"";
        #[derive(Deserialize)]
        struct Test {
            mode: TmuxMode,
        }
        let test: Test = toml::from_str(toml_str).unwrap();
        assert_eq!(test.mode, TmuxMode::Auto);
    }

    #[test]
    fn test_tmux_mode_deserialize_always() {
        let toml_str = "mode = \"always\"";
        #[derive(Deserialize)]
        struct Test {
            mode: TmuxMode,
        }
        let test: Test = toml::from_str(toml_str).unwrap();
        assert_eq!(test.mode, TmuxMode::Always);
    }

    #[test]
    fn test_tmux_mode_deserialize_never() {
        let toml_str = "mode = \"never\"";
        #[derive(Deserialize)]
        struct Test {
            mode: TmuxMode,
        }
        let test: Test = toml::from_str(toml_str).unwrap();
        assert_eq!(test.mode, TmuxMode::Never);
    }

    #[test]
    fn test_merge_strategy_default() {
        assert_eq!(MergeStrategy::default(), MergeStrategy::MergeCommit);
    }

    #[test]
    fn test_merge_strategy_display() {
        assert_eq!(MergeStrategy::MergeCommit.to_string(), "merge-commit");
        assert_eq!(MergeStrategy::FastForward.to_string(), "fast-forward");
    }

    #[test]
    fn test_tmux_mode_default() {
        assert_eq!(TmuxMode::default(), TmuxMode::Auto);
    }

    #[test]
    fn test_tmux_mode_display() {
        assert_eq!(TmuxMode::Auto.to_string(), "auto");
        assert_eq!(TmuxMode::Always.to_string(), "always");
        assert_eq!(TmuxMode::Never.to_string(), "never");
    }

    #[test]
    fn test_config_default_values() {
        let config = Config::default();
        assert_eq!(config.default_branch, "main");
        assert!(config.delete_branch);
        assert_eq!(config.merge_strategy, MergeStrategy::MergeCommit);
        assert_eq!(config.worktree_root, None);
        assert_eq!(config.tmux.enabled, TmuxMode::Auto);
    }

    #[test]
    fn test_config_default_sources() {
        let config = Config::default();
        assert_eq!(
            config.sources.get("default_branch").unwrap(),
            &Source::Default
        );
        assert_eq!(
            config.sources.get("delete_branch").unwrap(),
            &Source::Default
        );
        assert_eq!(
            config.sources.get("merge_strategy").unwrap(),
            &Source::Default
        );
        assert_eq!(
            config.sources.get("worktree_root").unwrap(),
            &Source::Default
        );
        assert_eq!(
            config.sources.get("tmux.enabled").unwrap(),
            &Source::Default
        );
    }

    #[test]
    fn test_merge_configs_project_overrides_global() {
        let global = RawConfig {
            default_branch: Some("global-main".into()),
            delete_branch: Some(false),
            merge_strategy: Some(MergeStrategy::FastForward),
            worktree_root: Some("/global/path".into()),
            tmux: Some(TmuxConfig {
                enabled: TmuxMode::Always,
            }),
        };

        let project = RawConfig {
            default_branch: Some("project-develop".into()),
            delete_branch: None,
            merge_strategy: None,
            worktree_root: None,
            tmux: None,
        };

        let config = merge_configs(Some(global), None, Some(project), None);

        assert_eq!(config.default_branch, "project-develop");
        assert_eq!(config.sources.get("default_branch").unwrap(), &Source::Project);

        assert!(!config.delete_branch);
        assert_eq!(config.sources.get("delete_branch").unwrap(), &Source::Global);

        assert_eq!(config.merge_strategy, MergeStrategy::FastForward);
        assert_eq!(
            config.sources.get("merge_strategy").unwrap(),
            &Source::Global
        );

        assert_eq!(
            config.worktree_root,
            Some("/global/path".to_string())
        );
        assert_eq!(config.sources.get("worktree_root").unwrap(), &Source::Global);

        assert_eq!(config.tmux.enabled, TmuxMode::Always);
        assert_eq!(
            config.sources.get("tmux.enabled").unwrap(),
            &Source::Global
        );
    }

    #[test]
    fn test_merge_configs_unset_fields_default_to_global() {
        let global = RawConfig {
            default_branch: Some("global-main".into()),
            delete_branch: Some(true),
            merge_strategy: Some(MergeStrategy::MergeCommit),
            worktree_root: None,
            tmux: None,
        };

        let project = RawConfig {
            default_branch: Some("project-develop".into()),
            delete_branch: None,
            merge_strategy: None,
            worktree_root: None,
            tmux: None,
        };

        let config = merge_configs(Some(global), None, Some(project), None);

        assert_eq!(config.default_branch, "project-develop");
        assert!(config.delete_branch);
        assert_eq!(config.merge_strategy, MergeStrategy::MergeCommit);
    }

    #[test]
    fn test_merge_configs_both_empty_returns_defaults() {
        let config = merge_configs(None, None, None, None);

        assert_eq!(config.default_branch, "main");
        assert!(config.delete_branch);
        assert_eq!(config.merge_strategy, MergeStrategy::MergeCommit);
        assert_eq!(config.worktree_root, None);
        assert_eq!(config.tmux.enabled, TmuxMode::Auto);
    }

    #[test]
    fn test_merge_configs_global_only() {
        let global = RawConfig {
            default_branch: Some("develop".into()),
            delete_branch: Some(false),
            merge_strategy: Some(MergeStrategy::FastForward),
            worktree_root: Some("/wt".into()),
            tmux: Some(TmuxConfig {
                enabled: TmuxMode::Never,
            }),
        };

        let config = merge_configs(Some(global), None, None, None);

        assert_eq!(config.default_branch, "develop");
        assert_eq!(config.sources.get("default_branch").unwrap(), &Source::Global);
        assert!(!config.delete_branch);
        assert_eq!(config.merge_strategy, MergeStrategy::FastForward);
        assert_eq!(config.tmux.enabled, TmuxMode::Never);
    }

    #[test]
    fn test_parse_toml_value_bool() {
        assert_eq!(parse_toml_value("true"), toml::Value::Boolean(true));
        assert_eq!(parse_toml_value("false"), toml::Value::Boolean(false));
    }

    #[test]
    fn test_parse_toml_value_integer() {
        assert_eq!(parse_toml_value("42"), toml::Value::Integer(42));
    }

    #[test]
    fn test_parse_toml_value_float() {
        assert_eq!(parse_toml_value("2.5"), toml::Value::Float(2.5));
    }

    #[test]
    fn test_parse_toml_value_string() {
        assert_eq!(
            parse_toml_value("hello"),
            toml::Value::String("hello".into())
        );
        assert_eq!(
            parse_toml_value("merge-commit"),
            toml::Value::String("merge-commit".into())
        );
    }

    #[test]
    fn test_set_toml_key_simple() {
        let mut doc = toml::Value::Table(toml::value::Table::new());
        set_toml_key(&mut doc, "default_branch", toml::Value::String("develop".into()));
        assert_eq!(doc["default_branch"].as_str(), Some("develop"));
    }

    #[test]
    fn test_set_toml_key_nested() {
        let mut doc = toml::Value::Table(toml::value::Table::new());
        set_toml_key(&mut doc, "tmux.enabled", toml::Value::String("never".into()));
        assert_eq!(doc["tmux"]["enabled"].as_str(), Some("never"));
    }

    #[test]
    fn test_set_toml_key_overwrites_existing() {
        let mut doc = toml::Value::Table(toml::value::Table::new());
        set_toml_key(&mut doc, "default_branch", toml::Value::String("main".into()));
        set_toml_key(
            &mut doc,
            "default_branch",
            toml::Value::String("develop".into()),
        );
        assert_eq!(doc["default_branch"].as_str(), Some("develop"));
    }

    #[test]
    fn test_config_entry_source_annotation_default() {
        let entry = ConfigEntry {
            key: "default_branch".into(),
            value: "main".into(),
            source: Source::Default,
            global_path: None,
            project_path: None,
        };
        assert_eq!(entry.source_annotation(), "(default)");
    }

    #[test]
    fn test_config_entry_source_annotation_global() {
        let entry = ConfigEntry {
            key: "delete_branch".into(),
            value: "true".into(),
            source: Source::Global,
            global_path: Some(PathBuf::from("/home/user/.config/mat/config.toml")),
            project_path: None,
        };
        assert_eq!(
            entry.source_annotation(),
            "(global: /home/user/.config/mat/config.toml)"
        );
    }

    #[test]
    fn test_config_entry_source_annotation_project() {
        let entry = ConfigEntry {
            key: "merge_strategy".into(),
            value: "fast-forward".into(),
            source: Source::Project,
            global_path: None,
            project_path: Some(PathBuf::from("/repo/.mat.toml")),
        };
        assert_eq!(entry.source_annotation(), "(project: .mat.toml)");
    }

    #[test]
    fn test_effective_values_returns_all_keys() {
        let config = Config::default();
        let values = config.effective_values();
        assert_eq!(values.len(), 5);

        let keys: Vec<String> = values.iter().map(|e| e.key.clone()).collect();
        assert!(keys.contains(&"default_branch".to_string()));
        assert!(keys.contains(&"delete_branch".to_string()));
        assert!(keys.contains(&"merge_strategy".to_string()));
        assert!(keys.contains(&"worktree_root".to_string()));
        assert!(keys.contains(&"tmux.enabled".to_string()));
    }

    #[test]
    fn test_effective_value_returns_valid_key() {
        let config = Config::default();
        let entry = config.effective_value("default_branch").unwrap();
        assert_eq!(entry.value, "main");
        assert_eq!(entry.source, Source::Default);
    }

    #[test]
    fn test_effective_value_returns_none_for_invalid_key() {
        let config = Config::default();
        assert!(config.effective_value("nonexistent").is_none());
    }

    #[test]
    fn test_source_display() {
        assert_eq!(Source::Default.to_string(), "default");
        assert_eq!(Source::Global.to_string(), "global");
        assert_eq!(Source::Project.to_string(), "project");
    }

    #[test]
    fn test_config_clone() {
        let config1 = Config::default();
        let config2 = config1.clone();
        assert_eq!(config1.default_branch, config2.default_branch);
        assert_eq!(config1.delete_branch, config2.delete_branch);
    }

    #[test]
    fn test_tmux_config_default() {
        let tc = TmuxConfig::default();
        assert_eq!(tc.enabled, TmuxMode::Auto);
    }

    #[test]
    fn test_merge_strategy_partial_eq() {
        assert_eq!(MergeStrategy::MergeCommit, MergeStrategy::MergeCommit);
        assert_ne!(MergeStrategy::MergeCommit, MergeStrategy::FastForward);
    }

    #[test]
    fn test_tmux_mode_partial_eq() {
        assert_eq!(TmuxMode::Auto, TmuxMode::Auto);
        assert_ne!(TmuxMode::Auto, TmuxMode::Always);
    }

    #[test]
    fn test_raw_config_default_all_none() {
        let raw = RawConfig::default();
        assert!(raw.default_branch.is_none());
        assert!(raw.delete_branch.is_none());
        assert!(raw.merge_strategy.is_none());
        assert!(raw.worktree_root.is_none());
        assert!(raw.tmux.is_none());
    }

    #[test]
    fn test_parse_toml_config_simple() {
        let toml_str = r#"
default_branch = "develop"
delete_branch = false
merge_strategy = "fast-forward"
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(raw.default_branch.unwrap(), "develop");
        assert!(!raw.delete_branch.unwrap());
        assert_eq!(raw.merge_strategy.unwrap(), MergeStrategy::FastForward);
        assert!(raw.worktree_root.is_none());
        assert!(raw.tmux.is_none());
    }

    #[test]
    fn test_parse_toml_config_with_tmux() {
        let toml_str = r#"
[tmux]
enabled = "always"
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        assert!(raw.default_branch.is_none());
        let tmux = raw.tmux.unwrap();
        assert_eq!(tmux.enabled, TmuxMode::Always);
    }

    #[test]
    fn test_parse_toml_config_with_worktree_root() {
        let toml_str = r#"
worktree_root = "/tmp/worktrees/{app}/{type}"
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            raw.worktree_root.unwrap(),
            "/tmp/worktrees/{app}/{type}"
        );
    }

    #[test]
    fn test_merge_configs_project_overrides_delete_branch() {
        let global = RawConfig {
            default_branch: None,
            delete_branch: Some(true),
            merge_strategy: None,
            worktree_root: None,
            tmux: None,
        };

        let project = RawConfig {
            default_branch: None,
            delete_branch: Some(false),
            merge_strategy: None,
            worktree_root: None,
            tmux: None,
        };

        let config = merge_configs(Some(global), None, Some(project), None);
        assert!(!config.delete_branch);
        assert_eq!(config.sources.get("delete_branch").unwrap(), &Source::Project);
    }

    #[test]
    fn test_merge_configs_project_tmux_overrides_global() {
        let global = RawConfig {
            default_branch: None,
            delete_branch: None,
            merge_strategy: None,
            worktree_root: None,
            tmux: Some(TmuxConfig { enabled: TmuxMode::Always }),
        };

        let project = RawConfig {
            default_branch: None,
            delete_branch: None,
            merge_strategy: None,
            worktree_root: None,
            tmux: Some(TmuxConfig { enabled: TmuxMode::Never }),
        };

        let config = merge_configs(Some(global), None, Some(project), None);
        assert_eq!(config.tmux.enabled, TmuxMode::Never);
        assert_eq!(
            config.sources.get("tmux.enabled").unwrap(),
            &Source::Project
        );
    }

    #[test]
    fn test_malformed_toml_returns_config_error() {
        let result: Result<RawConfig, toml::de::Error> = toml::from_str("garbage [[[");
        assert!(result.is_err());
    }

    #[test]
    fn test_effective_value_delete_branch() {
        let config = Config::default();
        let entry = config.effective_value("delete_branch").unwrap();
        assert_eq!(entry.value, "true");
    }

    #[test]
    fn test_effective_value_merge_strategy() {
        let config = Config::default();
        let entry = config.effective_value("merge_strategy").unwrap();
        assert_eq!(entry.value, "merge-commit");
    }

    #[test]
    fn test_effective_value_worktree_root() {
        let config = Config::default();
        let entry = config.effective_value("worktree_root").unwrap();
        assert_eq!(entry.value, "<not set>");
    }

    #[test]
    fn test_effective_value_tmux_enabled() {
        let config = Config::default();
        let entry = config.effective_value("tmux.enabled").unwrap();
        assert_eq!(entry.value, "auto");
    }
}
