use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rusqlite::Connection;

#[cfg(target_os = "linux")]
use arboard::LinuxClipboardKind;

use anyhow::Context;
use chrono::{NaiveDateTime, Utc};
use log::{debug, error, info, warn};

use crate::actions;
use crate::clipboard::source_app;
use crate::clipboard::{self, ClipboardContent};
use crate::config::{CompiledRule, Config, SyncMode};
use crate::db::repository;
use crate::models::entry::{compute_hash, ClipboardEntry, ContentHash, EntryContent, TIMESTAMP_FORMAT};

/// Ask glibc to return free heap pages to the OS.
/// Cost: ~1μs per call. Prevents RSS growth from transient allocations.
#[cfg(target_os = "linux")]
fn trim_heap() {
    unsafe {
        libc::malloc_trim(0);
    }
}

/// Limit glibc malloc arenas to avoid VSZ bloat from per-thread arenas.
/// Each arena reserves ~64MB of virtual address space (PROT_NONE, no RSS cost)
/// but inflates VSZ. With 2 arenas (main + 1 worker) VSZ stays bounded.
#[cfg(target_os = "linux")]
fn limit_malloc_arenas() {
    unsafe {
        // M_ARENA_MAX = -8
        libc::mallopt(-8, 2);
    }
}

#[cfg(not(target_os = "linux"))]
fn trim_heap() {}

/// Reap finished child processes to prevent zombie accumulation.
/// `spawn_clipboard_server()` detaches children without waiting.
///
/// WARNING: `waitpid(-1)` reaps ALL child processes. If any code path needs
/// to call `Child::wait()` on a spawned process, either collect it before
/// this function runs, or switch to tracking specific PIDs.
#[cfg(target_os = "linux")]
fn reap_zombies() {
    loop {
        let ret = unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) };
        if ret > 0 {
            continue;
        }
        if ret == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
            continue;
        }
        break;
    }
}

#[cfg(not(target_os = "linux"))]
fn reap_zombies() {}

/// Post-iteration OS-level cleanup: release heap pages and reap zombie children.
fn post_iteration_cleanup() {
    trim_heap();
    reap_zombies();
}

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
    current_expiry_hash: RefCell<Option<ContentHash>>,
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
            error!("pruning expired entries: {e}");
        }
        self.last_prune.set(Instant::now());
    }

    /// Build a ClipboardEntry from content, or None if empty.
    /// Rejects oversized images early (before PNG encoding) based on RGBA size.
    fn build_entry(&self, content: ClipboardContent) -> Option<ClipboardEntry> {
        let info = source_app::detect_source_app();
        debug!(
            "source app: class={:?}, title={:?}",
            info.class, info.title
        );
        match content {
            ClipboardContent::Text(t) => {
                debug!("clipboard text, {} bytes", t.len());
                let mut entry = ClipboardEntry::from_text(t, info.class);
                entry.source_title = info.title;
                Some(entry)
            }
            ClipboardContent::Image {
                width,
                height,
                rgba_bytes,
            } => {
                if rgba_bytes.len() as u64 > self.max_size {
                    warn!(
                        "skipping image: RGBA size {} KB exceeds limit {} KB",
                        rgba_bytes.len() / 1024,
                        self.max_size / 1024
                    );
                    return None;
                }
                let mut entry = ClipboardEntry::from_image(width, height, rgba_bytes, info.class).ok()?;
                entry.source_title = info.title;
                Some(entry)
            }
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
        entry.mask_text = result.mask_with;
        result.ttl
    }

    /// Save entry to DB if within size limit.
    fn save_if_fits(&self, entry: &ClipboardEntry) {
        debug!("saving entry, size={} bytes", entry.content_size_bytes());
        if entry.content_size_bytes() as u64 > self.max_size {
            warn!(
                "skipping entry: size {} KB exceeds limit {} KB",
                entry.content_size_bytes() / 1024,
                self.max_size / 1024
            );
            return;
        }
        if let Err(e) = repository::save_or_update(self.conn, entry, self.max_history, self.max_age) {
            error!("saving entry: {e}");
        }
    }

    /// Update expiry tracking state after processing an entry.
    fn update_expiry_tracking(&self, ttl: Option<Duration>, content_hash: &ContentHash) {
        if let Some(d) = ttl {
            debug!("TTL set: {d:?}");
            self.current_expiry.set(Some(Instant::now() + d));
            *self.current_expiry_hash.borrow_mut() = Some(*content_hash);
        } else {
            self.current_expiry.set(None);
            *self.current_expiry_hash.borrow_mut() = None;
        }
    }

    /// If no TTL came from action rules, check if the DB entry has an `expires_at`
    /// (e.g. set by `clio copy --ttl`) and pick it up for clipboard clearing.
    fn pick_up_db_expiry(&self, content_hash: &ContentHash) -> Option<Duration> {
        let entry = repository::find_by_hash(self.conn, content_hash)
            .inspect_err(|e| error!("looking up expiry: {e}"))
            .ok()??;
        let expires_str = entry.expires_at.as_deref()?;
        let expires = NaiveDateTime::parse_from_str(expires_str, TIMESTAMP_FORMAT).ok()?;
        let now = Utc::now().naive_utc();
        if expires > now {
            let remaining = (expires - now).to_std().ok()?;
            Some(remaining)
        } else {
            Some(Duration::ZERO)
        }
    }

    /// Apply actions, save to DB, pick up DB expiry if needed, and update tracking.
    fn apply_save_and_track(&self, entry: &mut ClipboardEntry) {
        let mut ttl = self.apply_actions(entry);
        self.save_if_fits(entry);
        if ttl.is_none() {
            ttl = self.pick_up_db_expiry(&entry.content_hash);
        }
        self.update_expiry_tracking(ttl, &entry.content_hash);
    }

    /// Process a clipboard content change: build entry, apply actions, and save.
    fn process_change(&self, content: ClipboardContent) {
        if let Some(mut entry) = self.build_entry(content) {
            self.apply_save_and_track(&mut entry);
        }
    }

    /// Process a change and optionally extract text for syncing (avoids an extra clone).
    /// When `need_sync` is true and the entry is text, returns the text by move.
    fn process_change_with_sync(
        &self,
        content: ClipboardContent,
        need_sync: bool,
    ) -> Option<String> {
        if let Some(mut entry) = self.build_entry(content) {
            self.apply_save_and_track(&mut entry);
            if need_sync {
                if let EntryContent::Text(t) = entry.content {
                    return Some(t);
                }
            }
        }
        None
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
            error!("pruning expired entries: {e}");
        }
        self.last_prune.set(Instant::now());

        // Check if the expired entry is still in clipboard
        let current_cb_hash = clipboard::read_clipboard().ok().and_then(|c| c.content_hash());
        let expired_still_in_clipboard = match (&current_cb_hash, &expired_hash) {
            (Some(current), Some(expired)) => current == expired,
            _ => false,
        };

        if !expired_still_in_clipboard {
            return None;
        }

        // Restore previous active entry
        info!("clipboard entry expired, restoring previous");
        match repository::get_latest_active(self.conn) {
            Ok(Some(entry)) => match &entry.content {
                EntryContent::Text(text) => {
                    let _ = clipboard::write_clipboard_text_sync(text);
                    Some(RestoreResult {
                        clipboard_hash: entry.content_hash,
                        restored_text: text.to_owned(),
                    })
                }
                EntryContent::Image(blob) => {
                    if let Ok(img) = image::load_from_memory(blob) {
                        let rgba = img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        let _ = clipboard::write_clipboard_image_sync(w, h, rgba.into_raw());
                        Some(RestoreResult {
                            clipboard_hash: entry.content_hash,
                            restored_text: String::new(),
                        })
                    } else {
                        None
                    }
                }
            },
            Ok(None) => {
                let _ = clipboard::write_clipboard_text_sync("");
                Some(RestoreResult {
                    clipboard_hash: compute_hash("".as_bytes()),
                    restored_text: String::new(),
                })
            }
            Err(e) => {
                error!("querying latest entry: {e}");
                None
            }
        }
    }
}

struct RestoreResult {
    clipboard_hash: ContentHash,
    restored_text: String,
}

/// Result of processing a single selection (CLIPBOARD or PRIMARY).
#[cfg(target_os = "linux")]
struct SelectionResult {
    hash: Option<ContentHash>,
    /// Text to sync to the other selection, if applicable.
    sync_text: Option<String>,
}

impl WatchState<'_> {
    /// Read a selection, process if changed, return new hash and optional sync text.
    #[cfg(target_os = "linux")]
    fn poll_clipboard(
        &self,
        last_hash: &Option<ContentHash>,
        should_sync: bool,
    ) -> SelectionResult {
        let content = clipboard::read_selection(LinuxClipboardKind::Clipboard).ok();
        let hash = content.as_ref().and_then(|c| c.content_hash());
        let changed = hash.as_ref().is_some_and(|h| last_hash.as_ref() != Some(h));

        if !changed {
            return SelectionResult { hash: *last_hash, sync_text: None };
        }

        let sync_text = if let Some(content) = content {
            self.process_change_with_sync(content, should_sync)
        } else {
            None
        };

        SelectionResult { hash, sync_text }
    }

    /// Read PRIMARY, process text if changed, return new hash and optional sync text.
    #[cfg(target_os = "linux")]
    fn poll_primary(
        &self,
        last_hash: &Option<ContentHash>,
        should_sync: bool,
    ) -> SelectionResult {
        let content = clipboard::read_selection(LinuxClipboardKind::Primary).ok();
        let hash = content.as_ref().and_then(|c| c.content_hash());
        let changed = hash.as_ref().is_some_and(|h| last_hash.as_ref() != Some(h));

        if !changed {
            return SelectionResult { hash: *last_hash, sync_text: None };
        }

        let sync_text = if let Some(content) = content {
            self.process_change_with_sync(content, should_sync)
        } else {
            None
        };

        SelectionResult { hash, sync_text }
    }
}


pub fn run(conn: &Connection, config: &Config) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    limit_malloc_arenas();

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
    let mut last_hash: Option<ContentHash> = None;

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

/// Sync-enabled modes: monitor both CLIPBOARD and PRIMARY, sync per mode.
#[cfg(target_os = "linux")]
fn run_sync(
    state: &WatchState<'_>,
    running: &Arc<AtomicBool>,
    interval: Duration,
    sync_mode: SyncMode,
) -> anyhow::Result<()> {
    let mut last_clipboard_hash: Option<ContentHash> = None;
    let mut last_primary_hash: Option<ContentHash> = None;
    let mut primary_handle: Option<std::thread::JoinHandle<()>> = None;
    let mut clipboard_handle: Option<std::thread::JoinHandle<()>> = None;

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(interval);
        state.maybe_prune();

        if let Some(result) = state.check_expiry_and_restore() {
            last_clipboard_hash = Some(result.clipboard_hash);
            if matches!(sync_mode, SyncMode::Both | SyncMode::ToPrimary) {
                let new_handle = clipboard::write_selection_text(
                    LinuxClipboardKind::Primary,
                    &result.restored_text,
                );
                if let Some(old) = primary_handle.take() {
                    let _ = old.join();
                }
                primary_handle = new_handle;
                last_primary_hash = Some(compute_hash(result.restored_text.as_bytes()));
            }
        }

        // Process CLIPBOARD first; its buffer is freed before PRIMARY read.
        let sync_to_primary = matches!(sync_mode, SyncMode::Both | SyncMode::ToPrimary);
        let cb = state.poll_clipboard(&last_clipboard_hash, sync_to_primary);
        last_clipboard_hash = cb.hash;
        let synced_to_primary = if let Some(text) = cb.sync_text {
            let new_handle =
                clipboard::write_selection_text(LinuxClipboardKind::Primary, &text);
            if let Some(old) = primary_handle.take() {
                let _ = old.join();
            }
            primary_handle = new_handle;
            last_primary_hash = Some(compute_hash(text.as_bytes()));
            true
        } else {
            false
        };

        // Skip PRIMARY poll if we just synced text to it — we'd read back
        // the same content we wrote, wasting an allocation.
        if !synced_to_primary {
            let sync_to_clipboard = matches!(sync_mode, SyncMode::Both | SyncMode::ToClipboard);
            let pr = state.poll_primary(&last_primary_hash, sync_to_clipboard);
            last_primary_hash = pr.hash;
            if let Some(text) = pr.sync_text {
                let new_handle =
                    clipboard::write_selection_text(LinuxClipboardKind::Clipboard, &text);
                if let Some(old) = clipboard_handle.take() {
                    let _ = old.join();
                }
                clipboard_handle = new_handle;
                last_clipboard_hash = Some(compute_hash(text.as_bytes()));
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
) -> anyhow::Result<()> {
    run_disabled(state, running, interval)
}
