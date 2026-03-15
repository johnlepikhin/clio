use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use rusqlite::{params, Connection};

use crate::errors::{AppError, Result};
use crate::models::entry::{ClipboardEntry, ContentHash, ContentType, EntryContent, Timestamp, TIMESTAMP_FORMAT};

/// Column list for clipboard_entries SELECT queries.
/// Positional indices in `row_to_entry` must match this order.
const ENTRY_COLUMNS: &str = "id, content_type, text_content, blob_content, content_hash, source_app, source_title, created_at, metadata, expires_at, mask_text";

/// Default JSON metadata for entries without explicit metadata.
const DEFAULT_METADATA: &str = "{}";

/// Build the SELECT column list for preview queries that truncate text_content.
/// `text_param` is the SQL parameter placeholder for the preview length (e.g. "?3").
fn preview_columns(text_param: &str) -> String {
    format!(
        "id, content_type, CASE WHEN content_type = 'text' THEN substr(text_content, 1, {text_param}) ELSE text_content END, \
         blob_content, content_hash, source_app, source_title, created_at, metadata, expires_at, mask_text"
    )
}

/// Escape special LIKE characters (`%`, `_`, `\`) for safe use in SQL LIKE patterns.
fn escape_like(query: &str) -> String {
    query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

pub fn insert_entry(conn: &Connection, entry: &ClipboardEntry) -> Result<i64> {
    let mut stmt = conn.prepare_cached(
        "INSERT INTO clipboard_entries (content_type, text_content, blob_content, content_hash, source_app, source_title, metadata, expires_at, mask_text)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    stmt.execute(params![
        entry.content().content_type().as_str(),
        entry.content().text(),
        entry.content().blob(),
        entry.content_hash() as &[u8],
        entry.source_app(),
        entry.source_title(),
        entry.metadata().unwrap_or(DEFAULT_METADATA),
        entry.expires_at(),
        entry.mask_text(),
    ])?;
    Ok(conn.last_insert_rowid())
}

pub fn find_by_hash(conn: &Connection, hash: &ContentHash) -> Result<Option<ClipboardEntry>> {
    let sql = format!("SELECT {ENTRY_COLUMNS} FROM clipboard_entries WHERE content_hash = ?1");
    let mut stmt = conn.prepare_cached(&sql)?;
    let mut rows = stmt.query(params![hash as &[u8]])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_entry(row)?)),
        None => Ok(None),
    }
}

pub fn find_expires_at(conn: &Connection, hash: &ContentHash) -> Result<Option<Option<Timestamp>>> {
    let mut stmt = conn.prepare_cached(
        "SELECT expires_at FROM clipboard_entries WHERE content_hash = ?1"
    )?;
    let mut rows = stmt.query(params![hash as &[u8]])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn update_timestamp_and_expiry(
    conn: &Connection,
    id: i64,
    expires_at: Option<&Timestamp>,
) -> Result<()> {
    conn.execute(
        "UPDATE clipboard_entries SET created_at = strftime('%Y-%m-%dT%H:%M:%f', 'now'), expires_at = ?2 WHERE id = ?1",
        params![id, expires_at],
    )?;
    Ok(())
}

/// Update entry on dedup: refresh timestamp, COALESCE all optional fields.
/// `None` means "keep existing value" (SQL COALESCE returns the first non-NULL).
/// NOTE: This means existing values cannot be cleared to NULL via dedup.
/// For example, an entry with `expires_at` set by `clio copy --ttl` will keep
/// that TTL even when re-copied without `--ttl`. This is intentional: the watch
/// daemon should not silently remove user-configured TTL on re-detection.
fn update_on_dedup(
    conn: &Connection,
    id: i64,
    expires_at: Option<&Timestamp>,
    source_app: Option<&str>,
    source_title: Option<&str>,
    mask_text: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE clipboard_entries
         SET created_at = strftime('%Y-%m-%dT%H:%M:%f', 'now'),
             expires_at = COALESCE(?2, expires_at),
             source_app = COALESCE(?3, source_app),
             source_title = COALESCE(?4, source_title),
             mask_text = COALESCE(?5, mask_text)
         WHERE id = ?1",
        params![id, expires_at, source_app, source_title, mask_text],
    )?;
    Ok(())
}

#[cfg(test)]
pub fn list_entries(conn: &Connection, limit: usize) -> Result<Vec<ClipboardEntry>> {
    let sql = format!("SELECT {ENTRY_COLUMNS} FROM clipboard_entries ORDER BY created_at DESC LIMIT ?1");
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(params![limit as i64], row_to_entry)?;
    collect_entries(rows)
}

#[cfg(test)]
pub fn list_entries_page(
    conn: &Connection,
    limit: usize,
    offset: usize,
) -> Result<Vec<ClipboardEntry>> {
    let sql = format!("SELECT {ENTRY_COLUMNS} FROM clipboard_entries ORDER BY created_at DESC LIMIT ?1 OFFSET ?2");
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(params![limit as i64, offset as i64], row_to_entry)?;
    collect_entries(rows)
}

/// Like `list_entries_page`, but truncates `text_content` to `preview_chars`
/// characters in SQL to avoid transferring large blobs for UI preview.
///
/// **Note:** returned entries contain truncated text but the original `content_hash`
/// (computed from the full text). Do not use the hash for content comparison.
pub fn list_entries_preview(
    conn: &Connection,
    limit: usize,
    offset: usize,
    preview_chars: usize,
) -> Result<Vec<ClipboardEntry>> {
    let sql = format!(
        "SELECT {} FROM clipboard_entries ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
        preview_columns("?3")
    );
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(
        params![limit as i64, offset as i64, preview_chars as i64],
        row_to_entry,
    )?;
    let mut entries = collect_entries(rows)?;
    // Clear hash — preview content is truncated, hash would be misleading.
    for entry in &mut entries {
        entry.content_hash = [0; 32];
    }
    Ok(entries)
}

#[cfg(test)]
pub fn search_entries_page(
    conn: &Connection,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<ClipboardEntry>> {
    let pattern = format!("%{}%", escape_like(query));
    let sql = format!(
        "SELECT {ENTRY_COLUMNS} FROM clipboard_entries WHERE text_content LIKE ?1 ESCAPE '\\' ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
    );
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(params![pattern, limit as i64, offset as i64], row_to_entry)?;
    collect_entries(rows)
}

/// Like `search_entries_page`, but truncates `text_content` to `preview_chars`
/// characters in SQL to avoid transferring large blobs for UI preview.
/// LIKE still matches against the full `text_content`, only the returned column
/// is truncated.
///
/// **Note:** returned entries contain truncated text but the original `content_hash`
/// (computed from the full text). Do not use the hash for content comparison.
pub fn search_entries_preview(
    conn: &Connection,
    query: &str,
    limit: usize,
    offset: usize,
    preview_chars: usize,
) -> Result<Vec<ClipboardEntry>> {
    let pattern = format!("%{}%", escape_like(query));
    let sql = format!(
        "SELECT {} FROM clipboard_entries WHERE text_content LIKE ?1 ESCAPE '\\' ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
        preview_columns("?4")
    );
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(
        params![pattern, limit as i64, offset as i64, preview_chars as i64],
        row_to_entry,
    )?;
    let mut entries = collect_entries(rows)?;
    // Clear hash — preview content is truncated, hash would be misleading.
    for entry in &mut entries {
        entry.content_hash = [0; 32];
    }
    Ok(entries)
}

pub fn delete_entry(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM clipboard_entries WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn get_entry_content(conn: &Connection, id: i64) -> Result<Option<ClipboardEntry>> {
    let sql = format!("SELECT {ENTRY_COLUMNS} FROM clipboard_entries WHERE id = ?1");
    let mut stmt = conn.prepare_cached(&sql)?;
    let mut rows = stmt.query(params![id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_entry(row)?)),
        None => Ok(None),
    }
}

pub fn prune_oldest(conn: &Connection, max_count: usize) -> Result<u64> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM clipboard_entries", [], |row| {
        row.get(0)
    })?;
    if count <= max_count as i64 {
        return Ok(0);
    }
    let to_delete = count - max_count as i64;
    let deleted = conn.execute(
        "DELETE FROM clipboard_entries WHERE id IN (
            SELECT id FROM clipboard_entries ORDER BY created_at ASC LIMIT ?1
        )",
        params![to_delete],
    )?;
    Ok(deleted as u64)
}

pub fn prune_expired(conn: &Connection, max_age: Option<Duration>) -> Result<u64> {
    let mut total_deleted: u64 = 0;

    // Prune entries with per-entry TTL (expires_at)
    let now_ts = Timestamp::now();
    let ttl_deleted = conn.execute(
        "DELETE FROM clipboard_entries WHERE expires_at IS NOT NULL AND expires_at < ?1",
        params![now_ts],
    )?;
    total_deleted += ttl_deleted as u64;

    // Prune entries by global max_age
    if let Some(age) = max_age {
        let chrono_age = ChronoDuration::from_std(age)
            .map_err(|e| AppError::DataIntegrity(format!("max_age duration too large: {e}")))?;
        let cutoff = Utc::now() - chrono_age;
        let cutoff_ts = Timestamp::from_raw(cutoff.format(TIMESTAMP_FORMAT).to_string());
        let deleted = conn.execute(
            "DELETE FROM clipboard_entries WHERE created_at < ?1",
            params![cutoff_ts],
        )?;
        total_deleted += deleted as u64;
    }

    Ok(total_deleted)
}

pub fn get_latest_active(conn: &Connection) -> Result<Option<ClipboardEntry>> {
    let now_ts = Timestamp::now();
    let sql = format!(
        "SELECT {ENTRY_COLUMNS} FROM clipboard_entries WHERE expires_at IS NULL OR expires_at >= ?1 ORDER BY created_at DESC LIMIT 1"
    );
    let mut stmt = conn.prepare_cached(&sql)?;
    let mut rows = stmt.query(params![now_ts])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_entry(row)?)),
        None => Ok(None),
    }
}

pub fn save_or_update(
    conn: &Connection,
    entry: &ClipboardEntry,
    max_history: usize,
) -> Result<i64> {
    let tx = conn.unchecked_transaction()?;
    let id = if let Some(existing) = find_by_hash(&tx, entry.content_hash())? {
        let id = existing
            .id()
            .ok_or_else(|| AppError::DataIntegrity("entry from DB has no id".to_owned()))?;
        update_on_dedup(&tx, id, entry.expires_at(), entry.source_app(), entry.source_title(), entry.mask_text())?;
        id
    } else {
        let id = insert_entry(&tx, entry)?;
        prune_oldest(&tx, max_history)?;
        id
    };
    tx.commit()?;
    Ok(id)
}

fn collect_entries(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<ClipboardEntry>>,
) -> Result<Vec<ClipboardEntry>> {
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

// Column indices matching ENTRY_COLUMNS order.
mod col {
    pub const ID: usize = 0;
    pub const CONTENT_TYPE: usize = 1;
    pub const TEXT_CONTENT: usize = 2;
    pub const BLOB_CONTENT: usize = 3;
    pub const CONTENT_HASH: usize = 4;
    pub const SOURCE_APP: usize = 5;
    pub const SOURCE_TITLE: usize = 6;
    pub const CREATED_AT: usize = 7;
    pub const METADATA: usize = 8;
    pub const EXPIRES_AT: usize = 9;
    pub const MASK_TEXT: usize = 10;
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardEntry> {
    let ct_str: String = row.get(col::CONTENT_TYPE)?;
    let text_content: Option<String> = row.get(col::TEXT_CONTENT)?;
    let blob_content: Option<Vec<u8>> = row.get(col::BLOB_CONTENT)?;

    let content = match ContentType::from_db_str(&ct_str) {
        ContentType::Text => EntryContent::Text(text_content.ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                col::TEXT_CONTENT,
                rusqlite::types::Type::Null,
                "content_type is 'text' but text_content is NULL".into(),
            )
        })?),
        ContentType::Image => EntryContent::Image(blob_content.ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                col::BLOB_CONTENT,
                rusqlite::types::Type::Null,
                "content_type is 'image' but blob_content is NULL".into(),
            )
        })?),
        ContentType::Unknown => {
            log::warn!("unknown content_type in DB row, falling back to text");
            EntryContent::Text(text_content.unwrap_or_default())
        }
    };

    let hash_vec: Vec<u8> = row.get(col::CONTENT_HASH)?;
    let content_hash: ContentHash = hash_vec.try_into().map_err(|v: Vec<u8>| {
        rusqlite::Error::FromSqlConversionFailure(
            col::CONTENT_HASH,
            rusqlite::types::Type::Blob,
            format!("expected 32-byte hash, got {}", v.len()).into(),
        )
    })?;

    Ok(ClipboardEntry {
        id: Some(row.get(col::ID)?),
        content,
        content_hash,
        source_app: row.get(col::SOURCE_APP)?,
        source_title: row.get(col::SOURCE_TITLE)?,
        created_at: row.get(col::CREATED_AT)?,
        metadata: row.get(col::METADATA)?,
        expires_at: row.get(col::EXPIRES_AT)?,
        mask_text: row.get(col::MASK_TEXT)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db_in_memory;

    fn setup() -> Connection {
        init_db_in_memory().unwrap()
    }

    #[test]
    fn test_insert_and_find_by_hash() {
        let conn = setup();
        let entry = ClipboardEntry::from_text("hello".to_string(), None);
        let id = insert_entry(&conn, &entry).unwrap();
        assert!(id > 0);

        let found = find_by_hash(&conn, &entry.content_hash).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.content.text(), Some("hello"));
    }

    #[test]
    fn test_dedup_via_save_or_update() {
        let conn = setup();
        let entry1 = ClipboardEntry::from_text("hello".to_string(), None);
        let id1 = save_or_update(&conn, &entry1, 500).unwrap();

        // Same content should update, not insert
        let entry2 = ClipboardEntry::from_text("hello".to_string(), None);
        let id2 = save_or_update(&conn, &entry2, 500).unwrap();
        assert_eq!(id1, id2);

        let entries = list_entries(&conn, 500).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_list_entries_ordered() {
        let conn = setup();
        save_or_update(
            &conn,
            &ClipboardEntry::from_text("a".to_string(), None),
            500,
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        save_or_update(
            &conn,
            &ClipboardEntry::from_text("b".to_string(), None),
            500,
        )
        .unwrap();

        let entries = list_entries(&conn, 500).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content.text(), Some("b"));
        assert_eq!(entries[1].content.text(), Some("a"));
    }

    #[test]
    fn test_delete_entry() {
        let conn = setup();
        let entry = ClipboardEntry::from_text("to delete".to_string(), None);
        let id = insert_entry(&conn, &entry).unwrap();

        delete_entry(&conn, id).unwrap();
        let found = find_by_hash(&conn, &entry.content_hash).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_prune_oldest() {
        let conn = setup();
        for i in 0..10 {
            let entry = ClipboardEntry::from_text(format!("entry {i}"), None);
            insert_entry(&conn, &entry).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let pruned = prune_oldest(&conn, 5).unwrap();
        assert_eq!(pruned, 5);

        let entries = list_entries(&conn, 100).unwrap();
        assert_eq!(entries.len(), 5);
    }

    #[test]
    fn test_save_or_update_with_pruning() {
        let conn = setup();
        for i in 0..5 {
            let entry = ClipboardEntry::from_text(format!("entry {i}"), None);
            save_or_update(&conn, &entry, 3).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let entries = list_entries(&conn, 100).unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_get_entry_content() {
        let conn = setup();
        let entry = ClipboardEntry::from_text("content".to_string(), None);
        let id = insert_entry(&conn, &entry).unwrap();

        let found = get_entry_content(&conn, id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().content.text(), Some("content"));

        let not_found = get_entry_content(&conn, 9999).unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_list_entries_page() {
        let conn = setup();
        for i in 0..10 {
            let entry = ClipboardEntry::from_text(format!("entry {i}"), None);
            insert_entry(&conn, &entry).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let page1 = list_entries_page(&conn, 3, 0).unwrap();
        assert_eq!(page1.len(), 3);

        let page2 = list_entries_page(&conn, 3, 3).unwrap();
        assert_eq!(page2.len(), 3);

        let ids1: Vec<_> = page1.iter().map(|e| e.id).collect();
        let ids2: Vec<_> = page2.iter().map(|e| e.id).collect();
        for id in &ids1 {
            assert!(!ids2.contains(id));
        }

        let empty = list_entries_page(&conn, 3, 100).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_search_entries_page() {
        let conn = setup();
        for i in 0..5 {
            let entry = ClipboardEntry::from_text(format!("apple {i}"), None);
            insert_entry(&conn, &entry).unwrap();
        }
        for i in 0..3 {
            let entry = ClipboardEntry::from_text(format!("banana {i}"), None);
            insert_entry(&conn, &entry).unwrap();
        }

        let results = search_entries_page(&conn, "apple", 10, 0).unwrap();
        assert_eq!(results.len(), 5);

        let page1 = search_entries_page(&conn, "apple", 2, 0).unwrap();
        assert_eq!(page1.len(), 2);
        let page2 = search_entries_page(&conn, "apple", 2, 2).unwrap();
        assert_eq!(page2.len(), 2);

        let none = search_entries_page(&conn, "cherry", 10, 0).unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn test_list_entries_preview_truncates_text() {
        let conn = setup();
        let long_text = "a".repeat(1000);
        let entry = ClipboardEntry::from_text(long_text, None);
        insert_entry(&conn, &entry).unwrap();

        let entries = list_entries_preview(&conn, 10, 0, 50).unwrap();
        assert_eq!(entries.len(), 1);
        let text = entries[0].content.text().unwrap();
        assert_eq!(text.len(), 50);
    }

    #[test]
    fn test_list_entries_preview_short_text_unchanged() {
        let conn = setup();
        let entry = ClipboardEntry::from_text("short".to_string(), None);
        insert_entry(&conn, &entry).unwrap();

        let entries = list_entries_preview(&conn, 10, 0, 50).unwrap();
        assert_eq!(entries[0].content.text(), Some("short"));
    }

    #[test]
    fn test_search_entries_preview_truncates_text() {
        let conn = setup();
        let long_text = format!("needle {}", "x".repeat(1000));
        let entry = ClipboardEntry::from_text(long_text, None);
        insert_entry(&conn, &entry).unwrap();

        let entries = search_entries_preview(&conn, "needle", 10, 0, 20).unwrap();
        assert_eq!(entries.len(), 1);
        let text = entries[0].content.text().unwrap();
        assert_eq!(text.len(), 20);
    }

    #[test]
    fn test_search_entries_preview_finds_in_full_text() {
        let conn = setup();
        // Keyword is at position 500+, well beyond preview_chars=50
        let long_text = format!("{}keyword_here", "x".repeat(500));
        let entry = ClipboardEntry::from_text(long_text, None);
        insert_entry(&conn, &entry).unwrap();

        // Search should still find it (LIKE matches full text)
        let entries = search_entries_preview(&conn, "keyword_here", 10, 0, 50).unwrap();
        assert_eq!(entries.len(), 1);
        // But returned text is truncated to 50 chars
        assert_eq!(entries[0].content.text().unwrap().len(), 50);
    }

    #[test]
    fn test_prune_expired_no_op_when_none() {
        let conn = setup();
        let entry = ClipboardEntry::from_text("hello".to_string(), None);
        insert_entry(&conn, &entry).unwrap();

        let deleted = prune_expired(&conn, None).unwrap();
        assert_eq!(deleted, 0);

        let entries = list_entries(&conn, 100).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_prune_expired_deletes_old_entries() {
        let conn = setup();

        conn.execute(
            "INSERT INTO clipboard_entries (content_type, text_content, content_hash, created_at)
             VALUES ('text', 'old', X'0000000000000000000000000000000000000000000000000000000000000000', strftime('%Y-%m-%dT%H:%M:%f', 'now', '-2 hours'))",
            [],
        )
        .unwrap();

        let fresh = ClipboardEntry::from_text("fresh".to_string(), None);
        insert_entry(&conn, &fresh).unwrap();

        let entries = list_entries(&conn, 100).unwrap();
        assert_eq!(entries.len(), 2);

        let deleted = prune_expired(&conn, Some(Duration::from_secs(3600))).unwrap();
        assert_eq!(deleted, 1);

        let entries = list_entries(&conn, 100).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content.text(), Some("fresh"));
    }

    #[test]
    fn test_prune_expired_deletes_by_expires_at() {
        let conn = setup();

        // Entry with expired per-entry TTL
        let mut entry = ClipboardEntry::from_text("expiring".to_string(), None);
        entry.set_expires_at(Some(Timestamp::from_raw("2000-01-01T00:00:00.000".to_string())));
        insert_entry(&conn, &entry).unwrap();

        // Entry without TTL
        let fresh = ClipboardEntry::from_text("fresh".to_string(), None);
        insert_entry(&conn, &fresh).unwrap();

        let entries = list_entries(&conn, 100).unwrap();
        assert_eq!(entries.len(), 2);

        // Even without global max_age, per-entry TTL should be pruned
        let deleted = prune_expired(&conn, None).unwrap();
        assert_eq!(deleted, 1);

        let entries = list_entries(&conn, 100).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content.text(), Some("fresh"));
    }

    #[test]
    fn test_prune_expired_keeps_future_expires_at() {
        let conn = setup();

        // Entry with future TTL
        let mut entry = ClipboardEntry::from_text("future".to_string(), None);
        entry.set_expires_at(Some(Timestamp::from_raw("2099-01-01T00:00:00.000".to_string())));
        insert_entry(&conn, &entry).unwrap();

        let deleted = prune_expired(&conn, None).unwrap();
        assert_eq!(deleted, 0);

        let entries = list_entries(&conn, 100).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_save_or_update_preserves_expires_at() {
        let conn = setup();

        let mut entry = ClipboardEntry::from_text("ttl-entry".to_string(), None);
        entry.set_expires_at(Some(Timestamp::from_raw("2099-01-01T00:00:00.000".to_string())));
        let id = save_or_update(&conn, &entry, 500).unwrap();

        let found = get_entry_content(&conn, id).unwrap().unwrap();
        assert_eq!(found.expires_at.as_ref().map(|t| t.as_str()), Some("2099-01-01T00:00:00.000"));
    }

    #[test]
    fn test_save_or_update_updates_expires_at_on_dedup() {
        let conn = setup();

        // First save with no TTL
        let entry1 = ClipboardEntry::from_text("hello".to_string(), None);
        let id1 = save_or_update(&conn, &entry1, 500).unwrap();

        let found1 = get_entry_content(&conn, id1).unwrap().unwrap();
        assert!(found1.expires_at.is_none());

        // Same content, now with TTL
        let mut entry2 = ClipboardEntry::from_text("hello".to_string(), None);
        entry2.set_expires_at(Some(Timestamp::from_raw("2099-01-01T00:00:00.000".to_string())));
        let id2 = save_or_update(&conn, &entry2, 500).unwrap();

        assert_eq!(id1, id2);
        let found2 = get_entry_content(&conn, id2).unwrap().unwrap();
        assert_eq!(found2.expires_at.as_ref().map(|t| t.as_str()), Some("2099-01-01T00:00:00.000"));
    }

    #[test]
    fn test_save_or_update_preserves_existing_expires_at_on_dedup() {
        let conn = setup();

        // First save with TTL
        let mut entry1 = ClipboardEntry::from_text("hello".to_string(), None);
        entry1.set_expires_at(Some(Timestamp::from_raw("2099-01-01T00:00:00.000".to_string())));
        let id1 = save_or_update(&conn, &entry1, 500).unwrap();

        let found1 = get_entry_content(&conn, id1).unwrap().unwrap();
        assert_eq!(found1.expires_at.as_ref().map(|t| t.as_str()), Some("2099-01-01T00:00:00.000"));

        // Same content without TTL (simulates daemon dedup)
        let entry2 = ClipboardEntry::from_text("hello".to_string(), None);
        assert!(entry2.expires_at.is_none());
        let id2 = save_or_update(&conn, &entry2, 500).unwrap();

        assert_eq!(id1, id2);
        // Existing TTL must be preserved
        let found2 = get_entry_content(&conn, id2).unwrap().unwrap();
        assert_eq!(found2.expires_at.as_ref().map(|t| t.as_str()), Some("2099-01-01T00:00:00.000"));
    }

    #[test]
    fn test_get_latest_active_returns_active_entry() {
        let conn = setup();
        let entry = ClipboardEntry::from_text("active".to_string(), None);
        insert_entry(&conn, &entry).unwrap();

        let result = get_latest_active(&conn).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().content.text(), Some("active"));
    }

    #[test]
    fn test_get_latest_active_skips_expired() {
        let conn = setup();

        // Insert expired entry
        let mut expired = ClipboardEntry::from_text("expired".to_string(), None);
        expired.set_expires_at(Some(Timestamp::from_raw("2000-01-01T00:00:00.000".to_string())));
        insert_entry(&conn, &expired).unwrap();

        // Insert active entry (older by created_at but not expired)
        let active = ClipboardEntry::from_text("active".to_string(), None);
        insert_entry(&conn, &active).unwrap();

        let result = get_latest_active(&conn).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().content.text(), Some("active"));
    }

    #[test]
    fn test_get_latest_active_none_when_all_expired() {
        let conn = setup();

        let mut expired = ClipboardEntry::from_text("expired".to_string(), None);
        expired.set_expires_at(Some(Timestamp::from_raw("2000-01-01T00:00:00.000".to_string())));
        insert_entry(&conn, &expired).unwrap();

        let result = get_latest_active(&conn).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_dedup_updates_source_app() {
        let conn = setup();

        let entry1 = ClipboardEntry::from_text("hello".to_string(), Some("Firefox".to_string()));
        let id1 = save_or_update(&conn, &entry1, 500).unwrap();

        let found1 = get_entry_content(&conn, id1).unwrap().unwrap();
        assert_eq!(found1.source_app.as_deref(), Some("Firefox"));

        // Same content from a different app
        let entry2 = ClipboardEntry::from_text("hello".to_string(), Some("Chrome".to_string()));
        let id2 = save_or_update(&conn, &entry2, 500).unwrap();

        assert_eq!(id1, id2);
        let found2 = get_entry_content(&conn, id2).unwrap().unwrap();
        assert_eq!(found2.source_app.as_deref(), Some("Chrome"));
    }

    #[test]
    fn test_dedup_preserves_source_app_when_new_is_none() {
        let conn = setup();

        let entry1 = ClipboardEntry::from_text("hello".to_string(), Some("Firefox".to_string()));
        let id1 = save_or_update(&conn, &entry1, 500).unwrap();

        let found1 = get_entry_content(&conn, id1).unwrap().unwrap();
        assert_eq!(found1.source_app.as_deref(), Some("Firefox"));

        // Same content, no source_app detected
        let entry2 = ClipboardEntry::from_text("hello".to_string(), None);
        let id2 = save_or_update(&conn, &entry2, 500).unwrap();

        assert_eq!(id1, id2);
        // Existing source_app must be preserved
        let found2 = get_entry_content(&conn, id2).unwrap().unwrap();
        assert_eq!(found2.source_app.as_deref(), Some("Firefox"));
    }

    #[test]
    fn test_insert_and_read_source_title() {
        let conn = setup();
        let mut entry = ClipboardEntry::from_text("hello".to_string(), Some("Firefox".to_string()));
        entry.set_source_title(Some("GitHub - Mozilla Firefox".to_string()));
        let id = insert_entry(&conn, &entry).unwrap();

        let found = get_entry_content(&conn, id).unwrap().unwrap();
        assert_eq!(found.source_title.as_deref(), Some("GitHub - Mozilla Firefox"));
        assert_eq!(found.source_app.as_deref(), Some("Firefox"));
    }

    #[test]
    fn test_dedup_updates_mask_text() {
        let conn = setup();

        let mut entry1 = ClipboardEntry::from_text("secret".to_string(), None);
        entry1.set_mask_text(Some("••••••".to_string()));
        let id1 = save_or_update(&conn, &entry1, 500).unwrap();

        let found1 = get_entry_content(&conn, id1).unwrap().unwrap();
        assert_eq!(found1.mask_text.as_deref(), Some("••••••"));

        // Same content with a different mask
        let mut entry2 = ClipboardEntry::from_text("secret".to_string(), None);
        entry2.set_mask_text(Some("***".to_string()));
        let id2 = save_or_update(&conn, &entry2, 500).unwrap();

        assert_eq!(id1, id2);
        let found2 = get_entry_content(&conn, id2).unwrap().unwrap();
        assert_eq!(found2.mask_text.as_deref(), Some("***"));
    }

    #[test]
    fn test_dedup_preserves_mask_text_when_new_is_none() {
        let conn = setup();

        // First save with mask (e.g. via `clio copy --mask-with`)
        let mut entry1 = ClipboardEntry::from_text("secret".to_string(), None);
        entry1.set_mask_text(Some("••••••".to_string()));
        let id1 = save_or_update(&conn, &entry1, 500).unwrap();

        let found1 = get_entry_content(&conn, id1).unwrap().unwrap();
        assert_eq!(found1.mask_text.as_deref(), Some("••••••"));

        // Same content without mask (e.g. watch daemon dedup)
        let entry2 = ClipboardEntry::from_text("secret".to_string(), None);
        assert!(entry2.mask_text.is_none());
        let id2 = save_or_update(&conn, &entry2, 500).unwrap();

        assert_eq!(id1, id2);
        // Mask must be preserved (COALESCE keeps existing value)
        let found2 = get_entry_content(&conn, id2).unwrap().unwrap();
        assert_eq!(found2.mask_text.as_deref(), Some("••••••"));
    }

    #[test]
    fn test_dedup_preserves_source_title_when_new_is_none() {
        let conn = setup();

        let mut entry1 = ClipboardEntry::from_text("hello".to_string(), Some("Firefox".to_string()));
        entry1.set_source_title(Some("GitHub - Mozilla Firefox".to_string()));
        let id1 = save_or_update(&conn, &entry1, 500).unwrap();

        let found1 = get_entry_content(&conn, id1).unwrap().unwrap();
        assert_eq!(found1.source_title.as_deref(), Some("GitHub - Mozilla Firefox"));

        // Same content, no source_title
        let entry2 = ClipboardEntry::from_text("hello".to_string(), None);
        assert!(entry2.source_title.is_none());
        let id2 = save_or_update(&conn, &entry2, 500).unwrap();

        assert_eq!(id1, id2);
        let found2 = get_entry_content(&conn, id2).unwrap().unwrap();
        assert_eq!(found2.source_title.as_deref(), Some("GitHub - Mozilla Firefox"));
    }

    #[test]
    fn test_sqlite_timestamp_roundtrip() {
        // Verify that SQLite strftime-generated timestamps parse correctly in Rust.
        let conn = setup();
        let entry = ClipboardEntry::from_text("ts-test".to_string(), None);
        insert_entry(&conn, &entry).unwrap();

        let found = find_by_hash(&conn, &entry.content_hash).unwrap().unwrap();
        let ts = found.created_at.as_ref().expect("created_at should be set by DB");
        // This would panic if SQLite format doesn't match TIMESTAMP_FORMAT
        let _naive = ts.to_naive();
    }
}
