# Data Model: Clipboard Manager

**Date**: 2026-02-21
**Feature**: 001-clipboard-manager

## Entities

### ClipboardEntry

Represents a single clipboard capture stored in the history.

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| id | INTEGER | PRIMARY KEY, AUTOINCREMENT | Unique entry identifier |
| content_type | TEXT | NOT NULL | One of: "text", "image", "unknown" |
| text_content | TEXT | NULL | Text content (NULL for non-text entries) |
| blob_content | BLOB | NULL | Binary content — PNG-encoded image or raw bytes |
| content_hash | BLOB(32) | NOT NULL, UNIQUE | BLAKE3 hash of content for deduplication |
| source_app | TEXT | NULL | WM_CLASS of source application (best-effort) |
| created_at | TEXT | NOT NULL | ISO-8601 timestamp, updated on re-selection |
| metadata | TEXT | NULL | JSON object for extensible flags (private, TTL, etc.) |

**Indexes**:
- `idx_entries_hash` on `content_hash` (UNIQUE) — deduplication lookup
- `idx_entries_created` on `created_at DESC` — history ordering

**Notes**:
- `text_content` and `blob_content` are mutually exclusive based on
  `content_type`. Text entries use `text_content`; image entries use
  `blob_content` with PNG-encoded data.
- `content_hash` is computed from raw content bytes: UTF-8 bytes for
  text, PNG-encoded bytes for images.
- `metadata` stores a JSON object. Initial keys: `private` (bool),
  `ttl_seconds` (integer). Schema allows adding new keys without
  migration.
- `created_at` uses ISO-8601 text format (SQLite has no native datetime)
  for human readability and `ORDER BY` compatibility.

### Configuration

Application settings loaded from YAML at
`$XDG_CONFIG_HOME/clio/config.yaml`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| max_history | integer | 500 | Maximum number of entries to retain |
| watch_interval_ms | integer | 500 | Clipboard polling interval in ms |
| db_path | string | (XDG default) | Override path for SQLite database |
| max_entry_size_kb | integer | 51200 | Max single entry size in KB (50MB) |
| window_width | integer | 600 | History window width in pixels |
| window_height | integer | 400 | History window height in pixels |

**Notes**:
- All fields are optional in YAML. Missing fields use defaults.
- `db_path` overrides the XDG-derived default path.
- `max_entry_size_kb` prevents storing extremely large clipboard
  content; entries exceeding this are silently skipped by `watch`.

## Schema Migration: V1

```sql
CREATE TABLE IF NOT EXISTS clipboard_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content_type TEXT NOT NULL CHECK(content_type IN ('text', 'image', 'unknown')),
    text_content TEXT,
    blob_content BLOB,
    content_hash BLOB NOT NULL,
    source_app TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    metadata TEXT DEFAULT '{}'
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_hash
    ON clipboard_entries(content_hash);

CREATE INDEX IF NOT EXISTS idx_entries_created
    ON clipboard_entries(created_at DESC);
```

## State Transitions

```
[New clipboard content detected]
       │
       ▼
  Compute BLAKE3 hash
       │
       ▼
  Hash exists in DB? ──Yes──▶ UPDATE created_at = now()
       │                         (dedup: move to top of stack)
       No
       │
       ▼
  Entry count >= max_history? ──Yes──▶ DELETE oldest entries
       │
       No
       │
       ▼
  INSERT new entry
```

## Relationships

- One `Configuration` instance (singleton, loaded at startup).
- Many `ClipboardEntry` records, ordered by `created_at DESC`.
- No foreign key relationships between entities.
