# Research: Clipboard Manager

**Date**: 2026-02-21
**Feature**: 001-clipboard-manager

## R1: Clipboard Access (arboard)

**Decision**: Use `arboard` v3.6+ with `wayland-data-control` feature.

**Rationale**: Maintained by 1Password, supports text, images (raw RGBA),
HTML, and file lists. Works on both X11 and Wayland. Image data is
returned as raw RGBA pixels via `ImageData { width, height, bytes }` —
we encode to PNG for storage.

**Alternatives considered**:
- `clipboard-rs` — has monitoring support but less maintained
- `wl-clipboard-rs` — Wayland-only, no X11 support

**Key findings**:
- No built-in clipboard change detection API. Polling is required.
- On Linux, clipboard data vanishes when the owning process exits.
  `clio watch` must call `set().wait()` or re-set clipboard after
  reading to keep data alive if needed.
- `wayland-data-control` feature must be explicitly enabled in
  `Cargo.toml` for Wayland support.
- Image format: arboard returns raw RGBA; we must encode to PNG
  (via `image` crate or manual encoder) before storing in SQLite.

## R2: SQLite Migrations & Storage (rusqlite)

**Decision**: Use `rusqlite` with `rusqlite_migration` crate for schema
versioning. Store images as PNG BLOBs directly in SQLite for simplicity.

**Rationale**: `rusqlite_migration` uses SQLite `user_version` pragma,
has minimal dependencies, and is designed for embedded SQLite-only apps.
Storing BLOBs in SQLite is simpler than managing external files; typical
clipboard images are screenshots (100KB–5MB) well within SQLite's
comfortable range.

**Alternatives considered**:
- `refinery` — multi-DB support unnecessary, heavier
- External file storage for images — adds complexity (orphan cleanup,
  path management) without clear benefit for typical image sizes

**Key findings**:
- WAL mode: Set `PRAGMA journal_mode = WAL` before migrations.
  Sticky setting, persists across restarts.
- Recommended PRAGMAs: `synchronous=NORMAL`, `busy_timeout=5000`,
  `foreign_keys=ON`, `cache_size=-64000` (64MB).
- Content deduplication: Use content hash (BLAKE3, 32 bytes) stored
  as indexed column. Hash comparison is O(log N) vs full BLOB scan.
  BLAKE3 is fastest cryptographic hash in Rust ecosystem.
- For very large images (>10MB), incremental BLOB I/O via
  `Connection::blob_open()` is available but unlikely needed for v1.

## R3: GTK4 History Window (gtk4-rs)

**Decision**: Use `gtk4::ListView` with `FilterListModel`,
`SingleSelection`, and `SignalListItemFactory`.

**Rationale**: `ListView` uses virtual scrolling (recycles widgets),
handles hundreds of items efficiently. `FilterListModel` with
`SearchEntry::set_key_capture_widget()` provides native type-to-filter
UX without custom key handling.

**Alternatives considered**:
- `ListBox` — creates one widget per row, poor performance with many items
- `ColumnView` — table-oriented, overkill for a simple list

**Key findings**:
- Row widget: horizontal `gtk::Box` with `gtk::Image` (thumbnail) +
  vertical `gtk::Box` (preview label + meta label).
- `SignalListItemFactory` with property expressions for data binding.
- Keyboard: `SearchEntry::set_key_capture_widget` for filter input,
  `ListView::connect_activate` for Enter, `EventControllerKey` for
  Delete, action+accelerator for Escape.
- SQLite ↔ GTK bridge: load data via `gio::spawn_blocking`, push to
  `gio::ListStore` via `async_channel` on main thread. Never call
  rusqlite on the GLib main thread.
- Window style: undecorated `gtk::Window` for popup feel. Consider
  `gtk4-layer-shell` for Wayland overlay in future.

## R4: Source Application Detection

**Decision**: Best-effort detection on X11 only. Not available on
Wayland. Store as optional field.

**Rationale**: X11 exposes clipboard owner window via XFixes extension,
from which `WM_CLASS` and `_NET_WM_PID` can be queried. Wayland
protocols intentionally do not expose source application identity — this
is a deliberate security design decision.

**Alternatives considered**:
- Wayland compositor-specific hacks — non-portable, fragile
- Skip entirely — useful metadata when available, worth the effort

**Key findings**:
- X11 path: `x11rb` crate with XFixes for `SelectionNotifyEvent`,
  then `WmClass::get()` and `_NET_WM_PID` property query.
- Wayland: impossible via `ext-data-control-v1` or `wlr-data-control`.
  The protocol exposes only MIME types and data, not source identity.
- Failure modes on X11: Electron/Wine may have wrong PID, Flatpak
  apps have sandboxed PID namespace. `WM_CLASS` is more reliable
  than PID.
- Implementation: optional — if detection fails, store `None`. Do not
  block clipboard capture on detection failure.

## R5: Image Encoding for Storage

**Decision**: Use the `image` crate to encode arboard's raw RGBA data
to PNG before storing in SQLite BLOB.

**Rationale**: `arboard` returns raw RGBA pixels. PNG is lossless,
widely supported, and typically much smaller than raw RGBA. The `image`
crate is the standard Rust image processing library.

**Alternatives considered**:
- Store raw RGBA — wastes 4-10x more space
- Use `png` crate directly — less ergonomic, `image` is already needed
  for thumbnail generation

**Key findings**:
- Encoding: `image::RgbaImage::from_raw(w, h, bytes)` then
  `img.write_to(&mut cursor, ImageFormat::Png)`.
- Thumbnail generation for history window: resize to ~48px height
  using `image::imageops::resize` with `FilterType::Triangle`.
- Constitution note: `image` crate is an additional dependency beyond
  the approved list. Justified because PNG encoding/decoding is
  required for image storage and no approved crate covers this.

## R6: Content Hashing for Deduplication

**Decision**: Use BLAKE3 hash for content deduplication.

**Rationale**: BLAKE3 is 2-10x faster than SHA-256, has a pure-Rust
implementation, and produces 32-byte digests suitable for indexed
SQLite column lookup.

**Alternatives considered**:
- SHA-256 — slower, no advantage for dedup use case
- xxHash — non-cryptographic, higher collision risk
- Direct content comparison — O(N*M) with large BLOBs, impractical

**Key findings**:
- Hash computed on raw content bytes (text UTF-8 or PNG-encoded image).
- Stored as 32-byte BLOB in `content_hash` column with UNIQUE index.
- On clipboard change: compute hash → check index → update timestamp
  if exists, insert if new.
- Constitution note: `blake3` crate is an additional dependency.
  Justified because no existing approved crate provides hashing, and
  content deduplication is a core requirement (FR-015).
