# Data Model: Clipboard & Paste Buffer Synchronization

## Entities

### SyncMode (new)

Configuration enum controlling synchronization direction between PRIMARY and CLIPBOARD selections.

| Value          | Direction           | Description                          |
|----------------|---------------------|--------------------------------------|
| `to-clipboard` | PRIMARY → CLIPBOARD | Mouse selection syncs to Ctrl+V      |
| `to-primary`   | CLIPBOARD → PRIMARY | Ctrl+C syncs to middle-click         |
| `both`         | ↔ bidirectional     | Full sync in both directions         |
| `disabled`     | —                   | No synchronization                   |

**Default**: `both`

**Serialization**: Kebab-case in YAML (`to-clipboard`, not `ToClipboard`).

### Config (modified)

Existing struct in `src/config/types.rs`. One new field:

| Field       | Type       | Default | Notes                               |
|-------------|------------|---------|--------------------------------------|
| `sync_mode` | `SyncMode` | `both`  | Clipboard sync direction             |

All existing fields remain unchanged.

### Watch State (modified, runtime only)

The `clio watch` command currently tracks a single `last_hash`. This extends to:

| Field                | Type            | Purpose                               |
|----------------------|-----------------|---------------------------------------|
| `last_clipboard_hash`| `Option<Vec<u8>>`| Last seen CLIPBOARD content hash     |
| `last_primary_hash`  | `Option<Vec<u8>>`| Last seen PRIMARY content hash       |

These are runtime values, not persisted.

## Database Changes

None. The sync mode is a runtime behavior setting, not stored in SQLite. Clipboard history entries from both selections use the existing `clipboard_entries` table unchanged.

## State Transitions

```
Poll cycle:
  1. Read CLIPBOARD → compute hash
  2. Read PRIMARY → compute hash
  3. Compare hashes to last known values
  4. If CLIPBOARD changed AND sync mode allows (both | to-primary):
     → Write CLIPBOARD content to PRIMARY
     → Update last_primary_hash to new hash
  5. If PRIMARY changed AND sync mode allows (both | to-clipboard):
     → Write PRIMARY content to CLIPBOARD
     → Update last_clipboard_hash to new hash
  6. Save changed content to history (from whichever changed)
  7. Update last_clipboard_hash and last_primary_hash
```
