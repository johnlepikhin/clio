use std::io::Read;

use rusqlite::Connection;

use crate::clipboard;
use crate::config::Config;
use crate::db::repository;
use crate::errors::Result;
use crate::models::ClipboardEntry;

pub fn run(conn: &Connection, config: &Config) -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    if input.is_empty() {
        eprintln!("error: stdin is empty");
        std::process::exit(1);
    }

    let entry = ClipboardEntry::from_text(input.clone(), None);
    repository::save_or_update(conn, &entry, config.max_history)?;
    clipboard::write_clipboard_text(&input)?;

    Ok(())
}
