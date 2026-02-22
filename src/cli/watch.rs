use std::cell::Cell;
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
    fn apply_actions(&self, entry: &mut ClipboardEntry) {
        if self.rules.is_empty() {
            return;
        }

        let result = actions::apply_rules(&self.rules, entry);

        if let Some(transformed) = result.transformed_text {
            let new_hash = compute_hash(transformed.as_bytes());
            entry.content = EntryContent::Text(transformed);
            entry.content_hash = new_hash;
        }

        entry.expires_at = result.expires_at;
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

    /// Process a clipboard content change: build entry, apply actions, and save.
    fn process_change(&self, content: &ClipboardContent) {
        if let Some(mut entry) = Self::build_entry(content) {
            self.apply_actions(&mut entry);
            self.save_if_fits(&entry);
        }
    }
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

    let prune_interval = Duration::from_millis(config.watch_interval_ms * 120)
        .clamp(Duration::from_secs(30), Duration::from_secs(300));
    let state = WatchState {
        conn,
        max_history: config.max_history,
        max_age: config.max_age,
        max_size: config.max_entry_size_kb * 1024,
        prune_interval,
        last_prune: Cell::new(Instant::now()),
        rules,
        has_ttl_rules,
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
                    state.apply_actions(&mut entry);
                    state.save_if_fits(&entry);
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
