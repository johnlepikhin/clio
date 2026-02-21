# Research: History Preview & Lazy Loading

## R1: Text Truncation Strategy

**Decision**: Byte-based truncation with UTF-8 boundary adjustment using `str::floor_char_boundary()` (stable since Rust 1.82).

**Rationale**: The spec requires byte-based truncation (default 4096 bytes). Rust strings are UTF-8, so we need to avoid cutting in the middle of a multi-byte character. `str::floor_char_boundary(n)` returns the largest byte index ≤ n that is a valid char boundary — exactly what we need.

**Alternatives considered**:
- Character-based truncation (`chars().take(n)`) — discarded because spec explicitly says bytes.
- Manual byte scanning — unnecessary since `floor_char_boundary` is in std.

## R2: GTK4 ListView Scroll-to-Load Pattern

**Decision**: Connect to `ScrolledWindow`'s `vadjustment` `value-changed` signal. When scroll position approaches the end (e.g., within 80% of total height), load the next page from DB and append to `ListStore`.

**Rationale**: GTK4's `ListView` uses a virtual list model — it only creates widgets for visible items. But the backing `ListStore` still holds all model objects. By loading entries incrementally into the `ListStore`, we keep memory proportional to loaded entries, not total DB size.

**Alternatives considered**:
- Custom `gio::ListModel` implementation with on-demand item creation — much more complex, requires implementing `n_items()` which needs total count, and `item()` which would need to query DB per item. Overkill for this use case.
- `gtk4::SliceListModel` — doesn't help with lazy DB loading since it wraps an existing model.

## R3: Database-Side Filtering

**Decision**: Replace in-memory `CustomFilter` with SQL `LIKE` query. New `search_entries_page()` function in repository.

**Rationale**: The clarification requires filtering to search the entire database, not just loaded entries. The current approach (`CustomFilter` on `ListStore`) only filters loaded items. Moving to SQL `LIKE '%query%'` searches all entries and returns results paginated.

**Alternatives considered**:
- SQLite FTS5 (full-text search) — overkill for simple substring matching. Would add complexity and require schema changes.
- Load all entries into memory, then filter — defeats the purpose of lazy loading.

## R4: Multiline Text Display in ListView

**Decision**: Remove `set_ellipsize()` and `set_max_width_chars()` from the preview label. Set `set_wrap(true)` and `set_wrap_mode(WrapMode::WordChar)` or simply allow natural line breaks by not constraining height. The label should display the full truncated preview with line breaks preserved.

**Rationale**: The current `entry_row.rs` takes only the first line and truncates to 120 chars. The spec requires showing multiline text with line breaks. GTK4 `Label` naturally handles `\n` in text — we just need to stop the single-line restriction.

**Alternatives considered**:
- `TextView` (read-only) instead of `Label` — heavier widget, unnecessary for display-only.
- Custom widget — overkill.

## R5: Pagination Query Pattern

**Decision**: Use `LIMIT ? OFFSET ?` in SQL queries. The `list_entries_page(limit, offset)` function returns a page of entries. The offset tracks how many entries have been loaded so far.

**Rationale**: Simple, efficient for moderate datasets. SQLite handles `LIMIT/OFFSET` well with an index on `created_at DESC`.

**Alternatives considered**:
- Cursor-based pagination (WHERE created_at < last_timestamp) — more efficient for very large offsets but adds complexity. Not needed for typical clipboard history sizes (hundreds to low thousands).
