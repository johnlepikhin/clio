use rusqlite::{params, Connection};

use crate::errors::Result;
use crate::models::entry::{ClipboardEntry, ContentType};

pub fn insert_entry(conn: &Connection, entry: &ClipboardEntry) -> Result<i64> {
    conn.execute(
        "INSERT INTO clipboard_entries (content_type, text_content, blob_content, content_hash, source_app, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            entry.content_type.as_str(),
            entry.text_content,
            entry.blob_content,
            entry.content_hash,
            entry.source_app,
            entry.metadata.as_deref().unwrap_or("{}"),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn find_by_hash(conn: &Connection, hash: &[u8]) -> Result<Option<ClipboardEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, content_type, text_content, blob_content, content_hash, source_app, created_at, metadata
         FROM clipboard_entries WHERE content_hash = ?1",
    )?;
    let mut rows = stmt.query(params![hash])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_entry(row)?)),
        None => Ok(None),
    }
}

pub fn update_timestamp(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE clipboard_entries SET created_at = strftime('%Y-%m-%dT%H:%M:%f', 'now') WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn list_entries(conn: &Connection, limit: usize) -> Result<Vec<ClipboardEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, content_type, text_content, blob_content, content_hash, source_app, created_at, metadata
         FROM clipboard_entries ORDER BY created_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], row_to_entry)?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

pub fn list_entries_page(
    conn: &Connection,
    limit: usize,
    offset: usize,
) -> Result<Vec<ClipboardEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, content_type, text_content, blob_content, content_hash, source_app, created_at, metadata
         FROM clipboard_entries ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
    )?;
    let rows = stmt.query_map(params![limit as i64, offset as i64], row_to_entry)?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

pub fn search_entries_page(
    conn: &Connection,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<ClipboardEntry>> {
    let pattern = format!("%{query}%");
    let mut stmt = conn.prepare(
        "SELECT id, content_type, text_content, blob_content, content_hash, source_app, created_at, metadata
         FROM clipboard_entries
         WHERE text_content LIKE ?1
         ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
    )?;
    let rows = stmt.query_map(params![pattern, limit as i64, offset as i64], row_to_entry)?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

pub fn delete_entry(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM clipboard_entries WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn get_entry_content(conn: &Connection, id: i64) -> Result<Option<ClipboardEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, content_type, text_content, blob_content, content_hash, source_app, created_at, metadata
         FROM clipboard_entries WHERE id = ?1",
    )?;
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

pub fn save_or_update(
    conn: &Connection,
    entry: &ClipboardEntry,
    max_history: usize,
) -> Result<i64> {
    if let Some(existing) = find_by_hash(conn, &entry.content_hash)? {
        let id = existing.id.expect("DB entry must have id");
        update_timestamp(conn, id)?;
        return Ok(id);
    }
    let id = insert_entry(conn, entry)?;
    prune_oldest(conn, max_history)?;
    Ok(id)
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardEntry> {
    Ok(row_to_entry_unchecked(row))
}

fn row_to_entry_unchecked(row: &rusqlite::Row<'_>) -> ClipboardEntry {
    let ct_str: String = row.get_unwrap(1);
    ClipboardEntry {
        id: Some(row.get_unwrap(0)),
        content_type: ContentType::from_str(&ct_str),
        text_content: row.get_unwrap(2),
        blob_content: row.get_unwrap(3),
        content_hash: row.get_unwrap(4),
        source_app: row.get_unwrap(5),
        created_at: row.get_unwrap(6),
        metadata: row.get_unwrap(7),
    }
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
        assert_eq!(found.text_content.as_deref(), Some("hello"));
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
        assert_eq!(entries[0].text_content.as_deref(), Some("b"));
        assert_eq!(entries[1].text_content.as_deref(), Some("a"));
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
        assert_eq!(found.unwrap().text_content.as_deref(), Some("content"));

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

        // First page
        let page1 = list_entries_page(&conn, 3, 0).unwrap();
        assert_eq!(page1.len(), 3);

        // Second page
        let page2 = list_entries_page(&conn, 3, 3).unwrap();
        assert_eq!(page2.len(), 3);

        // Pages should not overlap
        let ids1: Vec<_> = page1.iter().map(|e| e.id).collect();
        let ids2: Vec<_> = page2.iter().map(|e| e.id).collect();
        for id in &ids1 {
            assert!(!ids2.contains(id));
        }

        // Beyond all entries
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

        // Search matching
        let results = search_entries_page(&conn, "apple", 10, 0).unwrap();
        assert_eq!(results.len(), 5);

        // Search with pagination
        let page1 = search_entries_page(&conn, "apple", 2, 0).unwrap();
        assert_eq!(page1.len(), 2);
        let page2 = search_entries_page(&conn, "apple", 2, 2).unwrap();
        assert_eq!(page2.len(), 2);

        // Search not matching
        let none = search_entries_page(&conn, "cherry", 10, 0).unwrap();
        assert!(none.is_empty());
    }
}
