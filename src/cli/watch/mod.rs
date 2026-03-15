mod state;

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rusqlite::Connection;

#[cfg(target_os = "linux")]
use arboard::LinuxClipboardKind;

use anyhow::Context;
use log::{info, warn};

use arboard::Clipboard;

use crate::clipboard;
use crate::config::{Config, SyncMode};
use crate::models::entry::{compute_hash, ContentHash};
use crate::platform;

use state::WatchState;

/// Post-iteration OS-level cleanup: release heap pages and reap zombie children.
fn post_iteration_cleanup() {
    platform::trim_heap();
    platform::reap_zombies();
}

pub fn run(conn: &Connection, config: &Config) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    platform::limit_malloc_arenas();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .context("failed to set Ctrl+C handler")?;

    let rules = config.compile_rules();
    let has_ttl_rules = rules.iter().any(|r| r.ttl.is_some());

    if !rules.is_empty() {
        info!("loaded {} action rule(s)", rules.len());
    }

    info!(
        "watching clipboard (interval: {}ms, sync: {})",
        config.watch_interval.as_millis(), config.sync_mode
    );

    let prune_interval = config.prune_interval;
    let state = WatchState {
        conn,
        max_history: config.max_history,
        max_age: config.max_age,
        max_entry_size_bytes: config.max_entry_size_bytes(),
        prune_interval,
        last_prune: Cell::new(Instant::now()),
        rules,
        has_ttl_rules,
        expiry: state::ExpiryTracker::new(),
    };
    let interval = config.watch_interval;
    let sync_mode = config.sync_mode;

    let mut cb = clipboard::open_clipboard()
        .context("failed to open clipboard")?;

    if sync_mode == SyncMode::Disabled {
        return run_disabled(&state, &running, interval, &mut cb);
    }

    run_sync(&state, &running, interval, sync_mode, &mut cb)
}

/// Disabled mode: only monitor CLIPBOARD, no PRIMARY interaction.
fn run_disabled(
    state: &WatchState<'_>,
    running: &Arc<AtomicBool>,
    interval: Duration,
    cb: &mut Clipboard,
) -> anyhow::Result<()> {
    let mut last_hash: Option<ContentHash> = None;

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(interval);
        state.maybe_prune();

        if let Some(result) = state.check_expiry_and_restore(cb) {
            last_hash = Some(result.clipboard_hash);
        }

        let content = match clipboard::read_clipboard_with(cb) {
            Ok(c) => c,
            Err(e) => {
                warn!("clipboard read error: {e}, reconnecting");
                if let Ok(new_cb) = clipboard::open_clipboard() {
                    *cb = new_cb;
                }
                continue;
            }
        };

        let hash = match content.content_hash() {
            Some(h) => h,
            None => continue,
        };

        if last_hash.as_ref() == Some(&hash) {
            continue;
        }

        state.process_change(content);
        last_hash = Some(hash);

        post_iteration_cleanup();
    }

    Ok(())
}

/// Write text to a selection, joining any previous handle first.
#[cfg(target_os = "linux")]
fn sync_to_selection(
    kind: LinuxClipboardKind,
    text: &str,
    handle: &mut Option<std::thread::JoinHandle<()>>,
) -> Option<ContentHash> {
    let new_handle = clipboard::write_selection_text(kind, text);
    if let Some(old) = handle.take() {
        let _ = old.join();
    }
    *handle = new_handle;
    Some(compute_hash(text.as_bytes()))
}

/// Sync-enabled modes: monitor both CLIPBOARD and PRIMARY, sync per mode.
#[cfg(target_os = "linux")]
fn run_sync(
    state: &WatchState<'_>,
    running: &Arc<AtomicBool>,
    interval: Duration,
    sync_mode: SyncMode,
    cb: &mut Clipboard,
) -> anyhow::Result<()> {
    let mut last_clipboard_hash: Option<ContentHash> = None;
    let mut last_primary_hash: Option<ContentHash> = None;
    let mut primary_handle: Option<std::thread::JoinHandle<()>> = None;
    let mut clipboard_handle: Option<std::thread::JoinHandle<()>> = None;

    let sync_to_primary = matches!(sync_mode, SyncMode::Both | SyncMode::ToPrimary);
    let sync_to_clipboard = matches!(sync_mode, SyncMode::Both | SyncMode::ToClipboard);

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(interval);
        state.maybe_prune();

        if let Some(result) = state.check_expiry_and_restore(cb) {
            last_clipboard_hash = Some(result.clipboard_hash);
            if sync_to_primary {
                last_primary_hash = sync_to_selection(
                    LinuxClipboardKind::Primary,
                    &result.restored_text,
                    &mut primary_handle,
                );
            }
        }

        // Process CLIPBOARD; sync text to PRIMARY if configured.
        let cb_result = state.poll_selection(cb, LinuxClipboardKind::Clipboard, &last_clipboard_hash, sync_to_primary);
        last_clipboard_hash = cb_result.hash;

        let synced = if let Some(text) = cb_result.sync_text {
            last_primary_hash = sync_to_selection(LinuxClipboardKind::Primary, &text, &mut primary_handle);
            true
        } else {
            false
        };

        // Skip PRIMARY poll if we just synced to it — avoids reading back our own write.
        if !synced {
            let primary_result = state.poll_selection(cb, LinuxClipboardKind::Primary, &last_primary_hash, sync_to_clipboard);
            last_primary_hash = primary_result.hash;
            if let Some(text) = primary_result.sync_text {
                last_clipboard_hash = sync_to_selection(LinuxClipboardKind::Clipboard, &text, &mut clipboard_handle);
            }
        }

        post_iteration_cleanup();
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn run_sync(
    state: &WatchState<'_>,
    running: &Arc<AtomicBool>,
    interval: Duration,
    _sync_mode: SyncMode,
    cb: &mut Clipboard,
) -> anyhow::Result<()> {
    if !matches!(_sync_mode, SyncMode::Disabled) {
        warn!("clipboard sync is not supported on this platform, running without sync");
    }
    run_disabled(state, running, interval, cb)
}
