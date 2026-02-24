use std::io::Read;
use std::time::Duration;

use anyhow::bail;
use chrono::Utc;
use log::{debug, warn};
use rusqlite::Connection;

use crate::clipboard;
use crate::config::Config;
use crate::db::repository;
use crate::models::entry::TIMESTAMP_FORMAT;
use crate::models::ClipboardEntry;

pub fn run(
    conn: &Connection,
    config: &Config,
    ttl: Option<Duration>,
    mask_with: Option<String>,
) -> anyhow::Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    debug!("read {} bytes from stdin", input.len());

    if input.is_empty() {
        bail!("stdin is empty");
    }

    clipboard::write_clipboard_text_sync(&input)?;
    debug!("clipboard written");

    // Also set PRIMARY selection so middle-click paste works immediately.
    #[cfg(target_os = "linux")]
    clipboard::write_selection_text(arboard::LinuxClipboardKind::Primary, &input);
    let mut entry = ClipboardEntry::from_text(input, None);

    entry.mask_text = mask_with;

    entry.expires_at = ttl.map(|d| {
        let chrono_d = match chrono::Duration::from_std(d) {
            Ok(d) => d,
            Err(_) => {
                warn!("TTL duration {d:?} too large, entry will not expire");
                chrono::Duration::MAX
            }
        };
        let expires = Utc::now() + chrono_d;
        expires.format(TIMESTAMP_FORMAT).to_string()
    });

    repository::save_or_update(conn, &entry, config.max_history, config.max_age)?;
    debug!("entry saved to database");

    Ok(())
}
