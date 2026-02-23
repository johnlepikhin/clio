pub mod migrations;
pub mod repository;

use std::path::Path;

use rusqlite::Connection;

use crate::errors::Result;

pub fn init_db(path: &Path) -> Result<Connection> {
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
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::run_migrations(&mut conn)?;
    Ok(conn)
}

/// Open DB for UI use: skip migrations and foreign_keys, enable mmap.
/// Assumes `init_db` has already been called (e.g. by `clio watch`).
#[cfg(feature = "ui")]
pub fn init_db_ui(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "busy_timeout", 1000)?;
    conn.pragma_update(None, "mmap_size", 67_108_864)?; // 64 MB
    Ok(conn)
}

#[cfg(test)]
pub fn init_db_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::run_migrations(&mut conn)?;
    Ok(conn)
}
