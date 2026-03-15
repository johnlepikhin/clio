pub mod migrations;
pub mod repository;

use std::path::Path;

use log::debug;
pub use rusqlite::Connection;

use crate::errors::Result;

/// SQLite busy timeout for CLI operations (ms).
const BUSY_TIMEOUT_CLI_MS: u32 = 5000;
/// SQLite busy timeout for UI operations (ms) — shorter to keep UI responsive.
const BUSY_TIMEOUT_UI_MS: u32 = 1000;
/// SQLite mmap size for UI: 64 MB for faster reads.
const MMAP_SIZE_UI: u32 = 67_108_864;

pub fn init_db(path: &Path) -> Result<Connection> {
    debug!("opening database at {}", path.display());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
    }
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "busy_timeout", BUSY_TIMEOUT_CLI_MS)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::run_migrations(&mut conn)?;
    Ok(conn)
}

/// Open DB for UI use: skip migrations and foreign_keys, enable mmap.
/// Assumes `init_db` has already been called (e.g. by `clio watch`).
pub fn init_db_ui(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "busy_timeout", BUSY_TIMEOUT_UI_MS)?;
    conn.pragma_update(None, "mmap_size", MMAP_SIZE_UI)?;
    Ok(conn)
}

#[cfg(test)]
pub fn init_db_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::run_migrations(&mut conn)?;
    Ok(conn)
}
