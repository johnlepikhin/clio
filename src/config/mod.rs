mod types;

pub use types::Config;

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::errors::{AppError, Result};

pub fn load_config(override_path: Option<&Path>) -> Result<Config> {
    let path = match override_path {
        Some(p) => p.to_path_buf(),
        None => config_dir().join("config.yaml"),
    };

    if !path.exists() {
        return Ok(Config::default());
    }

    let contents = std::fs::read_to_string(&path)?;
    let config: Config = serde_yaml::from_str(&contents)
        .map_err(|e| AppError::Config(format!("{}: {}", path.display(), e)))?;
    Ok(config)
}

pub fn config_dir() -> PathBuf {
    let dir = ProjectDirs::from("", "", "clio")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config").join("clio")
        });
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn data_dir() -> PathBuf {
    let dir = ProjectDirs::from("", "", "clio")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("clio")
        });
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.max_history, 500);
        assert_eq!(config.watch_interval_ms, 500);
        assert_eq!(config.max_entry_size_kb, 51200);
        assert_eq!(config.window_width, 600);
        assert_eq!(config.window_height, 400);
        assert!(config.db_path.is_none());
    }

    #[test]
    fn test_load_missing_file() {
        let config = load_config(Some(Path::new("/nonexistent/config.yaml"))).unwrap();
        assert_eq!(config.max_history, 500);
    }

    #[test]
    fn test_load_valid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        std::fs::write(&path, "max_history: 100\nwatch_interval_ms: 200\n").unwrap();

        let config = load_config(Some(&path)).unwrap();
        assert_eq!(config.max_history, 100);
        assert_eq!(config.watch_interval_ms, 200);
        // Unset fields use defaults
        assert_eq!(config.window_width, 600);
    }

    #[test]
    fn test_load_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        std::fs::write(&path, "max_history: [invalid\n").unwrap();

        let result = load_config(Some(&path));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("config.yaml"));
    }

    #[test]
    fn test_serde_deserialization_with_partial_fields() {
        let yaml = "max_history: 250\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.max_history, 250);
        assert_eq!(config.watch_interval_ms, 500); // default
    }

    #[test]
    fn test_serde_deserialization_with_db_path() {
        let yaml = "db_path: /tmp/custom.db\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.db_path.as_deref(), Some("/tmp/custom.db"));
    }
}
