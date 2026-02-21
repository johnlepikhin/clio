# Data Model: History Preview & Lazy Loading

## Schema Changes

**None.** This feature reads existing columns — no migrations required.

## New Configuration Fields

Added to `Config` struct in `src/config/types.rs`:

| Field | Type | Default | Validation | Description |
|-------|------|---------|------------|-------------|
| `preview_text_bytes` | `usize` | 4096 | > 0 | Maximum bytes of text shown in history list preview |
| `history_page_size` | `usize` | 50 | > 0 | Number of entries loaded per page (initial + scroll) |

## New Repository Functions

### `list_entries_page(conn, limit, offset) -> Vec<ClipboardEntry>`

```sql
SELECT id, content_type, text_content, blob_content, content_hash, source_app, created_at, metadata
FROM clipboard_entries
ORDER BY created_at DESC
LIMIT ?1 OFFSET ?2
```

### `search_entries_page(conn, query, limit, offset) -> Vec<ClipboardEntry>`

```sql
SELECT id, content_type, text_content, blob_content, content_hash, source_app, created_at, metadata
FROM clipboard_entries
WHERE text_content LIKE '%' || ?1 || '%'
ORDER BY created_at DESC
LIMIT ?2 OFFSET ?3
```

## Text Preview Truncation

Applied at UI layer when building `EntryObject`:

```
fn truncate_preview(text: &str, max_bytes: usize) -> (String, bool)
```

- Returns `(truncated_text, was_truncated)`
- Uses `str::floor_char_boundary(max_bytes)` for safe UTF-8 truncation
- If `was_truncated`, caller appends `…` to display text

## Entities Unchanged

- `ClipboardEntry` — no field changes
- `ContentType` — no changes
- `EntryObject` (GTK) — no property changes, just different data passed to `preview_text`
