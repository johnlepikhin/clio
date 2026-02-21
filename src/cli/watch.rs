use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rusqlite::Connection;

#[cfg(target_os = "linux")]
use arboard::LinuxClipboardKind;

use anyhow::Context;

use crate::clipboard::source_app;
use crate::clipboard::{self, ClipboardContent};
use crate::config::SyncMode;
use crate::config::Config;
use crate::db::repository;
use crate::models::entry::{compute_hash, ClipboardEntry};

/// Compute hash for clipboard content, or None if empty.
fn content_hash(content: &ClipboardContent) -> Option<Vec<u8>> {
    match content {
        ClipboardContent::Text(t) => Some(compute_hash(t.as_bytes())),
        ClipboardContent::Image { rgba_bytes, .. } => Some(compute_hash(rgba_bytes)),
        ClipboardContent::Empty => None,
    }
}

/// Build a ClipboardEntry from content for saving to history.
fn build_entry(content: &ClipboardContent) -> Option<ClipboardEntry> {
    match content {
        ClipboardContent::Text(t) => {
            Some(ClipboardEntry::from_text(t.clone(), source_app::detect_source_app()))
        }
        ClipboardContent::Image {
            width,
            height,
            rgba_bytes,
        } => ClipboardEntry::from_image(*width, *height, rgba_bytes, source_app::detect_source_app()).ok(),
        ClipboardContent::Empty => None,
    }
}

/// Extract text from clipboard content for syncing.
fn content_text(content: &ClipboardContent) -> Option<&str> {
    match content {
        ClipboardContent::Text(t) => Some(t.as_str()),
        _ => None,
    }
}

fn save_entry(
    conn: &Connection,
    entry: &ClipboardEntry,
    max_history: usize,
    max_age: Option<Duration>,
) {
    if let Err(e) = repository::save_or_update(conn, entry, max_history, max_age) {
        eprintln!("error saving entry: {e}");
    }
}

pub fn run(conn: &Connection, config: &Config) -> anyhow::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .context("failed to set Ctrl+C handler")?;

    eprintln!(
        "watching clipboard (interval: {}ms, sync: {})...",
        config.watch_interval_ms, config.sync_mode
    );

    let interval = Duration::from_millis(config.watch_interval_ms);
    let max_size = config.max_entry_size_kb * 1024;
    let sync_mode = config.sync_mode;

    if sync_mode == SyncMode::Disabled {
        return run_disabled(conn, config, &running, interval, max_size);
    }

    run_sync(conn, config, &running, interval, max_size, sync_mode)
}

/// Disabled mode: only monitor CLIPBOARD, no PRIMARY interaction.
fn run_disabled(
    conn: &Connection,
    config: &Config,
    running: &Arc<AtomicBool>,
    interval: Duration,
    max_size: u64,
) -> anyhow::Result<()> {
    let mut last_hash: Option<Vec<u8>> = None;

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(interval);

        let content = match clipboard::read_clipboard() {
            Ok(c) => c,
            Err(_) => continue,
        };

        let hash = match content_hash(&content) {
            Some(h) => h,
            None => continue,
        };

        if last_hash.as_ref() == Some(&hash) {
            continue;
        }

        if let Some(entry) = build_entry(&content) {
            if entry.content_size_bytes() as u64 > max_size {
                eprintln!(
                    "skipping entry: size {} KB exceeds limit {} KB",
                    entry.content_size_bytes() / 1024,
                    config.max_entry_size_kb
                );
            } else {
                save_entry(conn, &entry, config.max_history, config.max_age);
            }
        }

        last_hash = Some(hash);
    }

    Ok(())
}

/// Sync-enabled modes: monitor both CLIPBOARD and PRIMARY, sync per mode.
#[cfg(target_os = "linux")]
fn run_sync(
    conn: &Connection,
    config: &Config,
    running: &Arc<AtomicBool>,
    interval: Duration,
    max_size: u64,
    sync_mode: SyncMode,
) -> anyhow::Result<()> {
    let mut last_clipboard_hash: Option<Vec<u8>> = None;
    let mut last_primary_hash: Option<Vec<u8>> = None;

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(interval);

        // Read both selections
        let cb_content = clipboard::read_selection(LinuxClipboardKind::Clipboard).ok();
        let pr_content = clipboard::read_selection(LinuxClipboardKind::Primary).ok();

        // Compute hashes
        let cb_hash = cb_content.as_ref().and_then(content_hash);
        let pr_hash = pr_content.as_ref().and_then(content_hash);

        let cb_changed = cb_hash.as_ref().is_some_and(|h| last_clipboard_hash.as_ref() != Some(h));
        let pr_changed = pr_hash.as_ref().is_some_and(|h| last_primary_hash.as_ref() != Some(h));

        // Process CLIPBOARD change
        if cb_changed {
            if let Some(ref content) = cb_content {
                if let Some(entry) = build_entry(content) {
                    if entry.content_size_bytes() as u64 <= max_size {
                        save_entry(conn, &entry, config.max_history, config.max_age);
                    }
                }
            }

            last_clipboard_hash = cb_hash.clone();

            // Sync CLIPBOARD → PRIMARY (Both or ToPrimary)
            if matches!(sync_mode, SyncMode::Both | SyncMode::ToPrimary) {
                if let Some(text) = cb_content.as_ref().and_then(content_text) {
                    let _ = clipboard::write_selection_text(LinuxClipboardKind::Primary, text);
                    last_primary_hash = Some(compute_hash(text.as_bytes()));
                }
            }
        }

        // Process PRIMARY change
        if pr_changed {
            if let Some(ref content) = pr_content {
                if let Some(text) = content_text(content) {
                    let entry = ClipboardEntry::from_text(
                        text.to_owned(),
                        source_app::detect_source_app(),
                    );
                    if entry.content_size_bytes() as u64 <= max_size {
                        save_entry(conn, &entry, config.max_history, config.max_age);
                    }
                }
            }

            last_primary_hash = pr_hash;

            // Sync PRIMARY → CLIPBOARD (Both or ToClipboard)
            if matches!(sync_mode, SyncMode::Both | SyncMode::ToClipboard) {
                if let Some(text) = pr_content.as_ref().and_then(content_text) {
                    let _ = clipboard::write_selection_text(LinuxClipboardKind::Clipboard, text);
                    last_clipboard_hash = Some(compute_hash(text.as_bytes()));
                }
            }
        }
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn run_sync(
    conn: &Connection,
    config: &Config,
    running: &Arc<AtomicBool>,
    interval: Duration,
    max_size: u64,
    _sync_mode: SyncMode,
) -> anyhow::Result<()> {
    run_disabled(conn, config, running, interval, max_size)
}
