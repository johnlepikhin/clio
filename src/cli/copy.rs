use std::io::Read;
use std::time::Duration;

use anyhow::bail;
use chrono::Utc;
use rusqlite::Connection;

use crate::clipboard;
use crate::config::Config;
use crate::db::repository;
use crate::models::entry::TIMESTAMP_FORMAT;
use crate::models::ClipboardEntry;

pub fn run(conn: &Connection, config: &Config, ttl: Option<Duration>) -> anyhow::Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    if input.is_empty() {
        bail!("stdin is empty");
    }

    clipboard::write_clipboard_text_sync(&input)?;
    let mut entry = ClipboardEntry::from_text(input, None);

    entry.expires_at = ttl.map(|d| {
        let chrono_d = match chrono::Duration::from_std(d) {
            Ok(d) => d,
            Err(_) => {
                eprintln!("warning: TTL duration {d:?} too large, entry will not expire");
                chrono::Duration::MAX
            }
        };
        let expires = Utc::now() + chrono_d;
        expires.format(TIMESTAMP_FORMAT).to_string()
    });

    repository::save_or_update(conn, &entry, config.max_history, config.max_age)?;

    Ok(())
}
