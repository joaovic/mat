use std::env;
use std::path::{Path, PathBuf};

use crate::config::Config;

#[derive(Debug, Clone, PartialEq)]
pub struct Names {
    pub branch_name: String,
    pub worktree_name: String,
    pub window_name: String,
    pub worktree_path: PathBuf,
}

/// Converts a path to its platform-native string form.
/// On Windows, forward slashes are replaced with backslashes.
pub fn normalize_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    if cfg!(target_os = "windows") {
        raw.replace('/', "\\")
    } else {
        raw.to_string()
    }
}

pub fn get_app_name() -> String {
    env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "app".to_string())
}

pub fn generate_names(
    app_name: &str,
    task_type: &str,
    task_name: &str,
    config: &Config,
    repo_dir: &Path,
) -> Names {
    let branch_name = format!("{}/{}", task_type, task_name);
    let worktree_name = format!("{}-{}/{}", app_name, task_type, task_name);
    let window_name = worktree_name.clone();

    let worktree_path = if let Some(ref root) = config.worktree_root {
        let expanded = root
            .replace("{app}", app_name)
            .replace("{type}", task_type)
            .replace("{name}", task_name);
        PathBuf::from(expanded)
    } else {
        let worktree_root = PathBuf::from(format!("{}.worktree", repo_dir.display()));
        worktree_root.join(&worktree_name)
    };

    Names {
        branch_name,
        worktree_name,
        window_name,
        worktree_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_worktree_name_includes_type() {
        let names = generate_names("dashboard", "feat", "login", &default_config(), Path::new("/repo"));
        assert_eq!(names.worktree_name, "dashboard-feat/login");
    }

    #[test]
    fn test_branch_name_format() {
        let names = generate_names("dashboard", "feat", "login", &default_config(), Path::new("/repo"));
        assert_eq!(names.branch_name, "feat/login");
    }

    #[test]
    fn test_window_name_matches_worktree_name() {
        let names = generate_names("dashboard", "feat", "login", &default_config(), Path::new("/repo"));
        assert_eq!(names.window_name, "dashboard-feat/login");
    }

    #[test]
    fn test_different_task_type_produces_different_worktree_name() {
        let names1 = generate_names("dashboard", "feat", "login", &default_config(), Path::new("/repo"));
        let names2 = generate_names("dashboard", "fix", "login", &default_config(), Path::new("/repo"));
        assert_ne!(names1.worktree_name, names2.worktree_name);
    }

    #[test]
    fn test_worktree_path_defaults_to_repo_dot_worktree() {
        let names = generate_names("dashboard", "feat", "login", &default_config(), Path::new("/repo"));
        assert_eq!(
            names.worktree_path,
            PathBuf::from("/repo.worktree/dashboard-feat/login")
        );
    }

    #[test]
    fn test_worktree_path_substitutes_template_variables() {
        let config = Config {
            worktree_root: Some("/tmp/{app}/{type}/{name}".into()),
            ..Config::default()
        };
        let names = generate_names("dashboard", "feat", "login", &config, Path::new("/repo"));
        assert_eq!(
            names.worktree_path,
            PathBuf::from("/tmp/dashboard/feat/login")
        );
    }

    #[test]
    fn test_worktree_path_handles_partial_template() {
        let config = Config {
            worktree_root: Some("/custom/{app}".into()),
            ..Config::default()
        };
        let names = generate_names("project", "fix", "bug", &config, Path::new("/repo"));
        assert_eq!(names.worktree_path, PathBuf::from("/custom/project"));
    }

    #[test]
    fn test_get_app_name_returns_basename_of_cwd() {
        let name = get_app_name();
        assert!(!name.is_empty());
        assert!(!name.contains('/'), "app name should not contain path separators");
    }

    #[test]
    fn test_generate_names_with_app_name_only() {
        let names = generate_names("myapp", "feat", "feature-1", &default_config(), Path::new("/home/user/project"));
        assert_eq!(names.branch_name, "feat/feature-1");
        assert_eq!(names.worktree_name, "myapp-feat/feature-1");
        assert_eq!(
            names.worktree_path,
            PathBuf::from("/home/user/project.worktree/myapp-feat/feature-1")
        );
    }

    #[test]
    fn test_branch_name_does_not_include_app_name() {
        let names1 = generate_names("app1", "feat", "login", &default_config(), Path::new("/r1"));
        let names2 = generate_names("app2", "feat", "login", &default_config(), Path::new("/r2"));
        assert_eq!(names1.branch_name, names2.branch_name);
    }

    #[test]
    fn test_names_struct_debug_and_clone() {
        let names = generate_names("a", "b", "c", &default_config(), Path::new("/r"));
        let cloned = names.clone();
        assert_eq!(names, cloned);
        assert!(format!("{:?}", names).contains("a-b/c"));
    }

    #[test]
    fn test_worktree_path_default_uses_platform_separator() {
        let names = generate_names("x", "y", "z", &default_config(), Path::new("/repo"));
        let path_str = names.worktree_path.to_string_lossy().to_string();
        let expected_end = format!("x-y{}z", std::path::MAIN_SEPARATOR);
        assert!(path_str.ends_with(&expected_end));
    }

    #[test]
    fn test_worktree_path_components_are_valid() {
        let names = generate_names("myapp", "feat", "login", &default_config(), Path::new("/repo"));
        let components: Vec<_> = names.worktree_path.components().collect();
        let last = components.last().unwrap();
        let display = format!("{}", last.as_os_str().to_string_lossy());
        assert_eq!(display, "login", "last component should be the task name");
    }

    #[test]
    fn test_worktree_path_produces_valid_pathbuf() {
        let names = generate_names("myapp", "feat", "login", &default_config(), Path::new("/repo"));
        let parent = names.worktree_path.parent().unwrap();
        assert!(parent.to_string_lossy().contains("myapp-feat"), "parent dir should contain app-type");
    }

    #[test]
    fn test_normalize_path_uses_platform_separator() {
        let p = Path::new("/repo/worktree");
        let s = normalize_path(p);
        assert_eq!(s, format!("{}repo{}worktree", std::path::MAIN_SEPARATOR, std::path::MAIN_SEPARATOR));
    }

    #[test]
    fn test_normalize_path_with_trailing_name() {
        let p = Path::new("/repo.worktree/app-feat/login");
        let s = normalize_path(p);
        let sep = std::path::MAIN_SEPARATOR;
        assert!(s.contains(&format!("app-feat{}login", sep)));
    }

    #[test]
    fn test_generate_names_does_not_panic_with_empty_values() {
        let names = generate_names("", "", "", &default_config(), Path::new("/r"));
        assert_eq!(names.branch_name, "/");
        assert_eq!(names.worktree_name, "-/");
    }
}
