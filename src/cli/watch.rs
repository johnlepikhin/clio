use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rusqlite::Connection;

#[cfg(target_os = "linux")]
use arboard::LinuxClipboardKind;

use anyhow::Context;

use crate::actions;
use crate::clipboard::source_app;
use crate::clipboard::{self, ClipboardContent};
use crate::config::{CompiledRule, Config, SyncMode};
use crate::db::repository;
use crate::models::entry::{compute_hash, ClipboardEntry, EntryContent};

/// Shared state for the watch loop.
struct WatchState<'a> {
    conn: &'a Connection,
    max_history: usize,
    max_age: Option<Duration>,
    max_size: u64,
    prune_interval: Duration,
    last_prune: Cell<Instant>,
    rules: Vec<CompiledRule>,
    has_ttl_rules: bool,
    /// When the current clipboard entry's TTL expires (None = no TTL).
    current_expiry: Cell<Option<Instant>>,
    /// Content hash of the current entry with TTL (to check if user changed clipboard).
    current_expiry_hash: RefCell<Option<Vec<u8>>>,
}

impl WatchState<'_> {
    /// Run prune_expired periodically, independent of clipboard changes.
    fn maybe_prune(&self) {
        if self.max_age.is_none() && !self.has_ttl_rules {
            return;
        }
        if self.last_prune.get().elapsed() < self.prune_interval {
            return;
        }
        if let Err(e) = repository::prune_expired(self.conn, self.max_age) {
            eprintln!("error pruning expired entries: {e}");
        }
        self.last_prune.set(Instant::now());
    }

    /// Build a ClipboardEntry from content, or None if empty.
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

    /// Apply action rules to an entry, mutating it in place.
    /// Returns the TTL duration if a TTL rule matched.
    fn apply_actions(&self, entry: &mut ClipboardEntry) -> Option<Duration> {
        if self.rules.is_empty() {
            return None;
        }

        let result = actions::apply_rules(&self.rules, entry);

        if let Some(transformed) = result.transformed_text {
            let new_hash = compute_hash(transformed.as_bytes());
            entry.content = EntryContent::Text(transformed);
            entry.content_hash = new_hash;
        }

        entry.expires_at = result.expires_at;
        result.ttl
    }

    /// Save entry to DB if within size limit.
    fn save_if_fits(&self, entry: &ClipboardEntry) {
        if entry.content_size_bytes() as u64 > self.max_size {
            eprintln!(
                "skipping entry: size {} KB exceeds limit {} KB",
                entry.content_size_bytes() / 1024,
                self.max_size / 1024
            );
            return;
        }
        if let Err(e) = repository::save_or_update(self.conn, entry, self.max_history, self.max_age) {
            eprintln!("error saving entry: {e}");
        }
    }

    /// Update expiry tracking state after processing an entry.
    fn update_expiry_tracking(&self, ttl: Option<Duration>, content_hash: &[u8]) {
        if let Some(d) = ttl {
            self.current_expiry.set(Some(Instant::now() + d));
            *self.current_expiry_hash.borrow_mut() = Some(content_hash.to_vec());
        } else {
            self.current_expiry.set(None);
            *self.current_expiry_hash.borrow_mut() = None;
        }
    }

    /// Process a clipboard content change: build entry, apply actions, and save.
    fn process_change(&self, content: &ClipboardContent) {
        if let Some(mut entry) = Self::build_entry(content) {
            let ttl = self.apply_actions(&mut entry);
            self.save_if_fits(&entry);
            self.update_expiry_tracking(ttl, &entry.content_hash);
        }
    }

    /// Check if the current TTL entry has expired and restore the previous entry.
    /// Returns Some if clipboard was updated (hash + optional text for primary sync).
    fn check_expiry_and_restore(&self) -> Option<RestoreResult> {
        let expiry = self.current_expiry.get()?;
        if Instant::now() < expiry {
            return None;
        }

        // TTL expired — reset tracking
        self.current_expiry.set(None);
        let expired_hash = self.current_expiry_hash.borrow_mut().take();

        // Prune expired entries from DB
        if let Err(e) = repository::prune_expired(self.conn, self.max_age) {
            eprintln!("error pruning expired entries: {e}");
        }
        self.last_prune.set(Instant::now());

        // Check if the expired entry is still in clipboard
        let current_cb_hash = clipboard::read_clipboard().ok().as_ref().and_then(content_hash);
        let expired_still_in_clipboard = match (&current_cb_hash, &expired_hash) {
            (Some(current), Some(expired)) => current == expired,
            _ => false,
        };

        if !expired_still_in_clipboard {
            return None;
        }

        // Restore previous active entry
        eprintln!("clipboard entry expired, restoring previous");
        match repository::get_latest_active(self.conn) {
            Ok(Some(entry)) => {
                if let Some(text) = entry.content.text() {
                    let _ = clipboard::write_clipboard_text_sync(text);
                    return Some(RestoreResult {
                        clipboard_hash: entry.content_hash.clone(),
                        restored_text: text.to_owned(),
                    });
                }
                None
            }
            Ok(None) => {
                let _ = clipboard::write_clipboard_text_sync("");
                Some(RestoreResult {
                    clipboard_hash: compute_hash("".as_bytes()),
                    restored_text: String::new(),
                })
            }
            Err(e) => {
                eprintln!("error querying latest entry: {e}");
                None
            }
        }
    }
}

struct RestoreResult {
    clipboard_hash: Vec<u8>,
    restored_text: String,
}

/// Compute hash for clipboard content, or None if empty.
fn content_hash(content: &ClipboardContent) -> Option<Vec<u8>> {
    match content {
        ClipboardContent::Text(t) => Some(compute_hash(t.as_bytes())),
        ClipboardContent::Image { rgba_bytes, .. } => Some(compute_hash(rgba_bytes)),
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

pub fn run(conn: &Connection, config: &Config) -> anyhow::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .context("failed to set Ctrl+C handler")?;

    let rules = config.compile_rules();
    let has_ttl_rules = rules.iter().any(|r| r.ttl.is_some());

    if !rules.is_empty() {
        eprintln!("loaded {} action rule(s)", rules.len());
    }

    eprintln!(
        "watching clipboard (interval: {}ms, sync: {})...",
        config.watch_interval_ms, config.sync_mode
    );

    let prune_interval = config.prune_interval;
    let state = WatchState {
        conn,
        max_history: config.max_history,
        max_age: config.max_age,
        max_size: config.max_entry_size_kb * 1024,
        prune_interval,
        last_prune: Cell::new(Instant::now()),
        rules,
        has_ttl_rules,
        current_expiry: Cell::new(None),
        current_expiry_hash: RefCell::new(None),
    };
    let interval = Duration::from_millis(config.watch_interval_ms);
    let sync_mode = config.sync_mode;

    if sync_mode == SyncMode::Disabled {
        return run_disabled(&state, &running, interval);
    }

    run_sync(&state, &running, interval, sync_mode)
}

/// Disabled mode: only monitor CLIPBOARD, no PRIMARY interaction.
fn run_disabled(
    state: &WatchState<'_>,
    running: &Arc<AtomicBool>,
    interval: Duration,
) -> anyhow::Result<()> {
    let mut last_hash: Option<Vec<u8>> = None;

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(interval);
        state.maybe_prune();

        if let Some(result) = state.check_expiry_and_restore() {
            last_hash = Some(result.clipboard_hash);
        }

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

        state.process_change(&content);
        last_hash = Some(hash);
    }

    Ok(())
}

/// Sync-enabled modes: monitor both CLIPBOARD and PRIMARY, sync per mode.
#[cfg(target_os = "linux")]
fn run_sync(
    state: &WatchState<'_>,
    running: &Arc<AtomicBool>,
    interval: Duration,
    sync_mode: SyncMode,
) -> anyhow::Result<()> {
    let mut last_clipboard_hash: Option<Vec<u8>> = None;
    let mut last_primary_hash: Option<Vec<u8>> = None;

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(interval);
        state.maybe_prune();

        if let Some(result) = state.check_expiry_and_restore() {
            last_clipboard_hash = Some(result.clipboard_hash);
            if matches!(sync_mode, SyncMode::Both | SyncMode::ToPrimary) {
                let _ = clipboard::write_selection_text(
                    LinuxClipboardKind::Primary,
                    &result.restored_text,
                );
                last_primary_hash = Some(compute_hash(result.restored_text.as_bytes()));
            }
        }

        let cb_content = clipboard::read_selection(LinuxClipboardKind::Clipboard).ok();
        let pr_content = clipboard::read_selection(LinuxClipboardKind::Primary).ok();

        let cb_hash = cb_content.as_ref().and_then(content_hash);
        let pr_hash = pr_content.as_ref().and_then(content_hash);

        let cb_changed = cb_hash.as_ref().is_some_and(|h| last_clipboard_hash.as_ref() != Some(h));
        let pr_changed = pr_hash.as_ref().is_some_and(|h| last_primary_hash.as_ref() != Some(h));

        if cb_changed {
            if let Some(ref content) = cb_content {
                state.process_change(content);
            }

            last_clipboard_hash = cb_hash.clone();

            if matches!(sync_mode, SyncMode::Both | SyncMode::ToPrimary) {
                if let Some(text) = cb_content.as_ref().and_then(content_text) {
                    let _ = clipboard::write_selection_text(LinuxClipboardKind::Primary, text);
                    last_primary_hash = Some(compute_hash(text.as_bytes()));
                }
            }
        }

        if pr_changed {
            if let Some(ref content) = pr_content {
                if let Some(text) = content_text(content) {
                    let mut entry = ClipboardEntry::from_text(
                        text.to_owned(),
                        source_app::detect_source_app(),
                    );
                    let ttl = state.apply_actions(&mut entry);
                    state.save_if_fits(&entry);

                    // Track TTL for primary entries synced to clipboard
                    if matches!(sync_mode, SyncMode::Both | SyncMode::ToClipboard) {
                        state.update_expiry_tracking(ttl, &entry.content_hash);
                    }
                }
            }

            last_primary_hash = pr_hash;

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
    state: &WatchState<'_>,
    running: &Arc<AtomicBool>,
    interval: Duration,
    _sync_mode: SyncMode,
) -> anyhow::Result<()> {
    run_disabled(state, running, interval)
}
