mod types;

pub use types::{Config, SyncMode};

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::errors::{AppError, Result};

pub fn default_config_path() -> PathBuf {
    config_dir().join("config.yaml")
}

pub fn load_config(override_path: Option<&Path>) -> Result<Config> {
    let path = match override_path {
        Some(p) => p.to_path_buf(),
        None => default_config_path(),
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

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let restored: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(restored.max_history, config.max_history);
        assert_eq!(restored.watch_interval_ms, config.watch_interval_ms);
        assert_eq!(restored.max_entry_size_kb, config.max_entry_size_kb);
        assert_eq!(restored.window_width, config.window_width);
        assert_eq!(restored.window_height, config.window_height);
        assert_eq!(restored.db_path, config.db_path);
    }

    #[test]
    fn test_default_yaml_parses() {
        let yaml = Config::default_yaml();
        let config: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.max_history, 500);
        assert_eq!(config.watch_interval_ms, 500);
        assert_eq!(config.max_entry_size_kb, 51200);
        assert_eq!(config.window_width, 600);
        assert_eq!(config.window_height, 400);
        assert!(config.db_path.is_none());
    }

    #[test]
    fn test_validate_default_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_config() {
        let mut config = Config::default();
        config.max_history = 0;
        config.window_width = 0;
        let errors = config.validate().unwrap_err();
        assert_eq!(errors.len(), 2);
        assert!(errors[0].contains("max_history"));
        assert!(errors[1].contains("window_width"));
    }

    // T008: SyncMode serde roundtrip (all four kebab-case values)
    #[test]
    fn test_sync_mode_serde_roundtrip() {
        use types::SyncMode;
        let modes = [
            (SyncMode::ToClipboard, "to-clipboard"),
            (SyncMode::ToPrimary, "to-primary"),
            (SyncMode::Both, "both"),
            (SyncMode::Disabled, "disabled"),
        ];
        for (mode, expected_yaml) in &modes {
            let yaml = format!("sync_mode: {expected_yaml}\n");
            let config: Config = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(&config.sync_mode, mode);

            let serialized = serde_yaml::to_string(&config).unwrap();
            assert!(
                serialized.contains(expected_yaml),
                "expected '{expected_yaml}' in serialized output: {serialized}"
            );
        }
    }

    // T011: SyncMode default and equality
    #[test]
    fn test_sync_mode_default_is_both() {
        use types::SyncMode;
        assert_eq!(SyncMode::default(), SyncMode::Both);
    }

    #[test]
    fn test_config_without_sync_mode_defaults_to_both() {
        use types::SyncMode;
        let yaml = "max_history: 100\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.sync_mode, SyncMode::Both);
    }

    // T013: Updated default_yaml() parses and produces SyncMode::Both
    #[test]
    fn test_default_yaml_has_sync_mode() {
        use types::SyncMode;
        let yaml = Config::default_yaml();
        let config: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.sync_mode, SyncMode::Both);
    }

    // T014: Invalid sync_mode fails to parse
    #[test]
    fn test_invalid_sync_mode_fails() {
        let yaml = "sync_mode: invalid\n";
        let result: std::result::Result<Config, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("unknown variant"),
            "expected 'unknown variant' in error, got: {err}"
        );
    }

    #[test]
    fn test_preview_text_chars_default() {
        let config = Config::default();
        assert_eq!(config.preview_text_chars, 4096);
    }

    #[test]
    fn test_history_page_size_default() {
        let config = Config::default();
        assert_eq!(config.history_page_size, 50);
    }

    #[test]
    fn test_preview_text_chars_deserialization() {
        let yaml = "preview_text_chars: 8192\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.preview_text_chars, 8192);
    }

    #[test]
    fn test_history_page_size_deserialization() {
        let yaml = "history_page_size: 100\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.history_page_size, 100);
    }

    #[test]
    fn test_validate_preview_text_chars_zero() {
        let mut config = Config::default();
        config.preview_text_chars = 0;
        let errors = config.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("preview_text_chars")));
    }

    #[test]
    fn test_validate_history_page_size_zero() {
        let mut config = Config::default();
        config.history_page_size = 0;
        let errors = config.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("history_page_size")));
    }

    #[test]
    fn test_default_yaml_has_preview_and_page_size() {
        let yaml = Config::default_yaml();
        let config: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.preview_text_chars, 4096);
        assert_eq!(config.history_page_size, 50);
    }
}
