# CLI Contract: History Preview & Lazy Loading

## Config changes

### New fields in config.yaml

```yaml
# Maximum bytes of text shown in history list preview (default 4 KB).
preview_text_bytes: 4096

# Number of entries loaded per page in the history window (default 50).
history_page_size: 50
```

### `clio config show` (updated output)

Both new fields appear in the YAML output alongside existing fields.

### `clio config validate` (updated behavior)

| Scenario | stdout / stderr | Exit code |
|----------|-----------------|-----------|
| Valid preview_text_bytes and history_page_size | `Configuration is valid.` | 0 |
| preview_text_bytes = 0 | `preview_text_bytes must be greater than 0` | 1 |
| history_page_size = 0 | `history_page_size must be greater than 0` | 1 |

### `clio config init` (updated template)

The default config template includes both new fields with explanatory comments.

## `clio history` (updated behavior)

### Initial load

- Loads `history_page_size` entries (default 50) from the database, most recent first.
- Text entries display up to `preview_text_bytes` bytes with `…` appended if truncated.
- Multiline text entries display preserving line breaks (no single-line restriction).
- Image entries display as thumbnails (unchanged behavior).

### Scroll-to-load

- When the user scrolls past the loaded entries, the next `history_page_size` batch is loaded from DB.
- Loading stops when all database entries have been loaded.

### Filtering

- Typing in the search entry triggers a database `LIKE` query across all entries (not just loaded ones).
- Filtered results are also loaded in pages of `history_page_size`.
- Clearing the filter resets to the normal paginated view.
