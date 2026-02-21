pub mod migrations;
pub mod repository;

use std::path::Path;

use rusqlite::Connection;

use crate::errors::Result;

pub fn init_db(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::run_migrations(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn init_db_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::run_migrations(&mut conn)?;
    Ok(conn)
}
