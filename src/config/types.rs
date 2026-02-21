use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyncMode {
    ToClipboard,
    ToPrimary,
    Both,
    Disabled,
}

impl Default for SyncMode {
    fn default() -> Self {
        Self::Both
    }
}

impl fmt::Display for SyncMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ToClipboard => write!(f, "to-clipboard"),
            Self::ToPrimary => write!(f, "to-primary"),
            Self::Both => write!(f, "both"),
            Self::Disabled => write!(f, "disabled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub max_history: usize,
    pub watch_interval_ms: u64,
    pub db_path: Option<String>,
    pub max_entry_size_kb: u64,
    pub window_width: i32,
    pub window_height: i32,
    pub sync_mode: SyncMode,
    pub preview_text_chars: usize,
    pub history_page_size: usize,
    pub image_preview_max_px: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_history: 500,
            watch_interval_ms: 500,
            db_path: None,
            max_entry_size_kb: 51200,
            window_width: 600,
            window_height: 400,
            sync_mode: SyncMode::default(),
            preview_text_chars: 4096,
            history_page_size: 50,
            image_preview_max_px: 640,
        }
    }
}

impl Config {
    /// Generate a default configuration file with explanatory comments.
    #[must_use]
    pub fn default_yaml() -> String {
        r#"# Clio clipboard manager configuration
# See `clio config show` for current effective values.

# Maximum number of clipboard entries to keep in history.
max_history: 500

# Polling interval in milliseconds for `clio watch`.
watch_interval_ms: 500

# Custom database path (omit to use XDG default).
# db_path: /path/to/custom.db

# Maximum clipboard entry size in kilobytes (default 50 MB).
max_entry_size_kb: 51200

# GTK history window dimensions.
window_width: 600
window_height: 400

# Synchronization between PRIMARY selection (mouse) and CLIPBOARD (Ctrl+C/V).
# Values: both (default), to-clipboard, to-primary, disabled
sync_mode: both

# Maximum characters of text shown in history list preview (default 4096).
preview_text_chars: 4096

# Number of entries loaded per page in the history window (default 50).
history_page_size: 50

# Maximum image preview size in pixels (longest side, default 640).
image_preview_max_px: 640
"#
        .to_owned()
    }

    /// Validate configuration values.
    /// Returns `Ok(())` if valid, or `Err` with a list of error messages.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.max_history == 0 {
            errors.push("max_history must be greater than 0".to_owned());
        }
        if self.watch_interval_ms == 0 {
            errors.push("watch_interval_ms must be greater than 0".to_owned());
        }
        if self.max_entry_size_kb == 0 {
            errors.push("max_entry_size_kb must be greater than 0".to_owned());
        }
        if self.window_width <= 0 {
            errors.push("window_width must be greater than 0".to_owned());
        }
        if self.window_height <= 0 {
            errors.push("window_height must be greater than 0".to_owned());
        }
        if self.preview_text_chars == 0 {
            errors.push("preview_text_chars must be greater than 0".to_owned());
        }
        if self.history_page_size == 0 {
            errors.push("history_page_size must be greater than 0".to_owned());
        }
        if self.image_preview_max_px <= 0 {
            errors.push("image_preview_max_px must be greater than 0".to_owned());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
