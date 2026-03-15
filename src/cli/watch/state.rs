use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use arboard::Clipboard;
use chrono::Utc;
use log::{debug, error, info, warn};
use rusqlite::Connection;

#[cfg(target_os = "linux")]
use arboard::LinuxClipboardKind;

use crate::actions;
use crate::clipboard::source_app;
use crate::clipboard::{self, ClipboardContent};
use crate::config::CompiledRule;
use crate::db::repository;
use crate::models::entry::{compute_hash, ClipboardEntry, ContentHash, EntryContent};

/// Tracks per-entry TTL expiry for clipboard clearing.
pub(super) struct ExpiryTracker {
    /// When the current clipboard entry's TTL expires (None = no TTL).
    current_expiry: Cell<Option<Instant>>,
    /// Content hash of the current entry with TTL.
    current_expiry_hash: RefCell<Option<ContentHash>>,
}

impl ExpiryTracker {
    pub(super) fn new() -> Self {
        Self {
            current_expiry: Cell::new(None),
            current_expiry_hash: RefCell::new(None),
        }
    }

    /// Update expiry tracking state after processing an entry.
    pub(super) fn update(&self, ttl: Option<Duration>, content_hash: &ContentHash) {
        if let Some(d) = ttl {
            debug!("TTL set: {d:?}");
            self.current_expiry.set(Some(Instant::now() + d));
            *self.current_expiry_hash.borrow_mut() = Some(*content_hash);
        } else {
            self.current_expiry.set(None);
            *self.current_expiry_hash.borrow_mut() = None;
        }
    }

    /// Check if the current TTL has expired. Returns the expired hash if so.
    pub(super) fn check_expired(&self) -> Option<Option<ContentHash>> {
        let expiry = self.current_expiry.get()?;
        if Instant::now() < expiry {
            return None;
        }
        self.current_expiry.set(None);
        Some(self.current_expiry_hash.borrow_mut().take())
    }
}

/// Shared state for the watch loop.
pub(super) struct WatchState<'a> {
    pub(super) conn: &'a Connection,
    pub(super) max_history: usize,
    pub(super) max_age: Option<Duration>,
    pub(super) max_entry_size_bytes: u64,
    pub(super) prune_interval: Duration,
    pub(super) last_prune: Cell<Instant>,
    pub(super) rules: Vec<CompiledRule>,
    pub(super) has_ttl_rules: bool,
    pub(super) expiry: ExpiryTracker,
}

impl WatchState<'_> {
    /// Run prune_expired periodically, independent of clipboard changes.
    pub(super) fn maybe_prune(&self) {
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
                entry.set_source_title(info.title);
                Some(entry)
            }
            ClipboardContent::Image {
                width,
                height,
                rgba_bytes,
            } => {
                if rgba_bytes.len() as u64 > self.max_entry_size_bytes {
                    warn!(
                        "skipping image: RGBA size {} KB exceeds limit {} KB",
                        rgba_bytes.len() / 1024,
                        self.max_entry_size_bytes / 1024
                    );
                    return None;
                }
                let mut entry = ClipboardEntry::from_image(width, height, rgba_bytes, info.class)
                    .inspect_err(|e| warn!("PNG encoding failed: {e}"))
                    .ok()?;
                entry.set_source_title(info.title);
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
            entry.set_content(EntryContent::Text(transformed));
        }

        entry.set_expires_at(result.expires_at);
        entry.set_mask_text(result.mask_with);
        result.ttl
    }

    /// Save entry to DB if within size limit.
    fn save_if_fits(&self, entry: &ClipboardEntry) {
        debug!("saving entry, size={} bytes", entry.content_size_bytes());
        if entry.content_size_bytes() as u64 > self.max_entry_size_bytes {
            warn!(
                "skipping entry: size {} KB exceeds limit {} KB",
                entry.content_size_bytes() / 1024,
                self.max_entry_size_bytes / 1024
            );
            return;
        }
        if let Err(e) = repository::save_or_update(self.conn, entry, self.max_history) {
            error!("saving entry: {e}");
        }
    }

    /// If no TTL came from action rules, check if the DB entry has an `expires_at`
    /// (e.g. set by `clio copy --ttl`) and pick it up for clipboard clearing.
    fn pick_up_db_expiry(&self, content_hash: &ContentHash) -> Option<Duration> {
        let expires_at = repository::find_expires_at(self.conn, content_hash)
            .inspect_err(|e| error!("looking up expiry: {e}"))
            .ok()??;
        let expires_ts = expires_at.as_ref()?;
        let expires = expires_ts.to_naive();
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
            ttl = self.pick_up_db_expiry(entry.content_hash());
        }
        self.expiry.update(ttl, entry.content_hash());
    }

    /// Process a clipboard content change: build entry, apply actions, and save.
    pub(super) fn process_change(&self, content: ClipboardContent) {
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
                if let EntryContent::Text(t) = entry.into_content() {
                    return Some(t);
                }
            }
        }
        None
    }

    /// Check if the current TTL entry has expired and restore the previous entry.
    /// Returns Some if clipboard was updated (hash + optional text for primary sync).
    pub(super) fn check_expiry_and_restore(&self, cb: &mut Clipboard) -> Option<RestoreResult> {
        let expired_hash = self.expiry.check_expired()?;

        // Prune expired entries from DB
        if let Err(e) = repository::prune_expired(self.conn, self.max_age) {
            error!("pruning expired entries: {e}");
        }
        self.last_prune.set(Instant::now());

        // Check if the expired entry is still in clipboard
        let current_cb_hash = clipboard::read_clipboard_with(cb).ok().and_then(|c| c.content_hash());
        let expired_still_in_clipboard = match (&current_cb_hash, &expired_hash) {
            (Some(current), Some(expired)) => current == expired,
            _ => false,
        };

        if !expired_still_in_clipboard {
            return None;
        }

        // Restore previous active entry.
        // NOTE: Similar logic exists in clipboard::restore_or_clear_clipboard(),
        // but here we need the hash+text for RestoreResult, so we inline it.
        info!("clipboard entry expired, restoring previous");
        match repository::get_latest_active(self.conn) {
            Ok(Some(entry)) => {
                if let Err(e) = clipboard::write_entry_to_clipboard(entry.content()) {
                    error!("writing restored entry to clipboard: {e}");
                    return None;
                }
                let hash = *entry.content_hash();
                let restored_text = entry.content().text().unwrap_or_default().to_owned();
                Some(RestoreResult {
                    clipboard_hash: hash,
                    restored_text,
                })
            }
            Ok(None) => {
                if let Err(e) = clipboard::write_clipboard_text_sync("") {
                    error!("clearing clipboard: {e}");
                    return None;
                }
                Some(RestoreResult {
                    clipboard_hash: compute_hash("".as_bytes()),
                    restored_text: String::new(),
                })
            }
            Err(e) => {
                error!("restoring previous entry: {e}");
                None
            }
        }
    }
}

pub(super) struct RestoreResult {
    pub(super) clipboard_hash: ContentHash,
    pub(super) restored_text: String,
}

/// Result of processing a single selection (CLIPBOARD or PRIMARY).
#[cfg(target_os = "linux")]
pub(super) struct SelectionResult {
    pub(super) hash: Option<ContentHash>,
    /// Text to sync to the other selection, if applicable.
    pub(super) sync_text: Option<String>,
}

impl WatchState<'_> {
    /// Read a selection, process if changed, reconnect once on error.
    #[cfg(target_os = "linux")]
    pub(super) fn poll_selection(
        &self,
        cb: &mut Clipboard,
        kind: LinuxClipboardKind,
        last_hash: &Option<ContentHash>,
        should_sync: bool,
    ) -> SelectionResult {
        let content = match clipboard::read_selection_with(cb, kind) {
            Ok(c) => Some(c),
            Err(e) => {
                warn!("clipboard read error ({kind:?}): {e}, reconnecting");
                if let Ok(new_cb) = clipboard::open_clipboard() {
                    *cb = new_cb;
                }
                clipboard::read_selection_with(cb, kind).ok()
            }
        };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db_in_memory;

    fn test_state(conn: &Connection) -> WatchState<'_> {
        WatchState {
            conn,
            max_history: 100,
            max_age: None,
            max_entry_size_bytes: 1024 * 1024, // 1 MB
            prune_interval: Duration::from_secs(60),
            last_prune: Cell::new(Instant::now()),
            rules: vec![],
            has_ttl_rules: false,
            expiry: ExpiryTracker::new(),
        }
    }

    #[test]
    fn build_entry_rejects_oversized_image() {
        let conn = init_db_in_memory().unwrap();
        let mut state = test_state(&conn);
        state.max_entry_size_bytes = 64; // very small limit

        // 2x2 RGBA = 16 bytes, but we set limit to 64 and use a bigger image
        // 10x10 RGBA = 400 bytes > 64
        let rgba = vec![255u8; 4 * 10 * 10];
        let content = ClipboardContent::Image {
            width: 10,
            height: 10,
            rgba_bytes: rgba,
        };

        assert!(state.build_entry(content).is_none());
    }

    #[test]
    fn build_entry_accepts_text() {
        let conn = init_db_in_memory().unwrap();
        let state = test_state(&conn);

        let content = ClipboardContent::Text("hello".into());
        let entry = state.build_entry(content);

        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.content().text(), Some("hello"));
    }

    #[test]
    fn build_entry_returns_none_for_empty() {
        let conn = init_db_in_memory().unwrap();
        let state = test_state(&conn);

        assert!(state.build_entry(ClipboardContent::Empty).is_none());
    }

    #[test]
    fn save_if_fits_skips_oversized_entry() {
        let conn = init_db_in_memory().unwrap();
        let mut state = test_state(&conn);
        state.max_entry_size_bytes = 4; // smaller than "hello" (5 bytes)

        let entry = ClipboardEntry::from_text("hello".into(), None);
        // Should not panic, just skip
        state.save_if_fits(&entry);

        // Verify nothing was saved
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM clipboard_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn apply_actions_with_no_rules_returns_none() {
        let conn = init_db_in_memory().unwrap();
        let state = test_state(&conn);
        assert!(state.rules.is_empty());

        let mut entry = ClipboardEntry::from_text("test".into(), None);
        let ttl = state.apply_actions(&mut entry);

        assert!(ttl.is_none());
    }

    #[test]
    fn expiry_tracker_update_and_check_cycle() {
        let tracker = ExpiryTracker::new();
        let hash = crate::models::entry::compute_hash(b"test");

        // Initially no expiry
        assert!(tracker.check_expired().is_none());

        // Set a very short TTL so it expires immediately
        tracker.update(Some(Duration::from_millis(0)), &hash);

        // Should detect expiration and return the hash
        let expired = tracker.check_expired();
        assert!(expired.is_some());
        assert_eq!(expired.unwrap(), Some(hash));

        // After consuming, should be None again
        assert!(tracker.check_expired().is_none());
    }

    #[test]
    fn expiry_tracker_clear_on_none_ttl() {
        let tracker = ExpiryTracker::new();
        let hash = crate::models::entry::compute_hash(b"test");

        // Set expiry
        tracker.update(Some(Duration::from_secs(60)), &hash);

        // Clear it by passing None
        tracker.update(None, &hash);

        // Should have no expiry
        assert!(tracker.check_expired().is_none());
    }
}
