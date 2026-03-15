use std::io::Read;
use std::time::Duration;

use anyhow::bail;
use log::debug;
use rusqlite::Connection;

use crate::clipboard;
use crate::config::Config;
use crate::db::repository;
use crate::models::ClipboardEntry;
use crate::models::entry::Timestamp;

pub fn run(
    conn: &Connection,
    config: &Config,
    ttl: Option<Duration>,
    mask_with: Option<String>,
) -> anyhow::Result<()> {
    let max_bytes = config.max_entry_size_bytes();
    let mut input = String::new();
    std::io::stdin().take(max_bytes + 1).read_to_string(&mut input)?;
    if input.len() as u64 > max_bytes {
        bail!("stdin exceeds max_entry_size_kb ({} KB)", config.max_entry_size_kb);
    }
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

    entry.set_mask_text(mask_with);

    entry.set_expires_at(ttl.map(Timestamp::after));

    repository::save_or_update(conn, &entry, config.max_history)?;
    debug!("entry saved to database");

    Ok(())
}
