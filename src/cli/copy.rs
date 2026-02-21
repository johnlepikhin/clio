use std::io::Read;

use anyhow::bail;
use rusqlite::Connection;

use crate::clipboard;
use crate::config::Config;
use crate::db::repository;
use crate::models::ClipboardEntry;

pub fn run(conn: &Connection, config: &Config) -> anyhow::Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    if input.is_empty() {
        bail!("stdin is empty");
    }

    clipboard::write_clipboard_text_sync(&input)?;
    let entry = ClipboardEntry::from_text(input, None);
    repository::save_or_update(conn, &entry, config.max_history, config.max_age)?;

    Ok(())
}
