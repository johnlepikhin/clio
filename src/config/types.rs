use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub max_history: usize,
    pub watch_interval_ms: u64,
    pub db_path: Option<String>,
    pub max_entry_size_kb: u64,
    pub window_width: i32,
    pub window_height: i32,
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
        }
    }
}
