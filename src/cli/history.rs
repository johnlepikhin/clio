use std::path::PathBuf;

use crate::config::Config;
use crate::errors::Result;

pub fn run(config: &Config, db_path: PathBuf) -> Result<()> {
    crate::ui::run_history_window(config, db_path)
}
