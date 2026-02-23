use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::errors::Result;

pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    let migrations = Migrations::new(vec![
        M::up(
            "CREATE TABLE IF NOT EXISTS clipboard_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type TEXT NOT NULL CHECK(content_type IN ('text', 'image', 'unknown')),
                text_content TEXT,
                blob_content BLOB,
                content_hash BLOB NOT NULL,
                source_app TEXT,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
                metadata TEXT DEFAULT '{}'
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_hash
                ON clipboard_entries(content_hash);

            CREATE INDEX IF NOT EXISTS idx_entries_created
                ON clipboard_entries(created_at DESC);",
        ),
        M::up(
            "ALTER TABLE clipboard_entries ADD COLUMN expires_at TEXT;

            CREATE INDEX IF NOT EXISTS idx_entries_expires
                ON clipboard_entries(expires_at)
                WHERE expires_at IS NOT NULL;",
        ),
        M::up("ALTER TABLE clipboard_entries ADD COLUMN source_title TEXT;"),
    ]);
    migrations.to_latest(conn)?;
    Ok(())
}
