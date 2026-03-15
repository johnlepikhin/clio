use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;

const HISTORY_BINARY: &str = "clio-history";

pub fn run(config_path: Option<&Path>, db_path: PathBuf) -> anyhow::Result<()> {
    let binary = find_clio_history();
    let mut cmd = Command::new(&binary);
    cmd.arg("--db-path").arg(&db_path);
    if let Some(path) = config_path {
        cmd.arg("--config").arg(path);
    }
    let status = cmd.status().with_context(|| {
        format!(
            "failed to launch {HISTORY_BINARY}; ensure it is installed alongside clio or in PATH"
        )
    })?;
    if !status.success() {
        anyhow::bail!("{HISTORY_BINARY} exited with {status}");
    }
    Ok(())
}

fn find_clio_history() -> PathBuf {
    // 1. Next to current executable
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name(HISTORY_BINARY);
        if sibling.exists() {
            return sibling;
        }
    }
    // 2. In PATH (let OS resolve)
    PathBuf::from(HISTORY_BINARY)
}
