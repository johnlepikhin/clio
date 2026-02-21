# Data Model: History UX & Image Previews

## Config Changes

### New Field

| Field | Type | Default | Validation |
|-------|------|---------|------------|
| `image_preview_max_px` | i32 | 320 | Must be > 0 |

Added to `Config` struct in `src/config/types.rs`, `Default` impl, `default_yaml()`, and `validate()`.

## Modified Functions

### `create_thumbnail_texture` (src/ui/window.rs)

**Current signature**: `fn create_thumbnail_texture(png_bytes: &[u8]) -> Option<Texture>`

**New signature**: `fn create_thumbnail_texture(png_bytes: &[u8], max_px: i32) -> Option<Texture>`

**Scaling logic**:
1. Load pixbuf from bytes via `PixbufLoader`
2. Get `src_w = pixbuf.width()`, `src_h = pixbuf.height()`
3. If `src_w <= max_px && src_h <= max_px` → use original pixbuf (no scaling)
4. Otherwise: `scale = max_px as f64 / max(src_w, src_h) as f64`; `dst_w = (src_w * scale) as i32`; `dst_h = (src_h * scale) as i32`
5. Call `pixbuf.scale_simple(dst_w, dst_h, InterpType::Bilinear)`
6. Convert to `Texture::for_pixbuf(&scaled)`

## No Schema Changes

The SQLite schema (`clipboard_entries` table) is not modified. Image blob data is stored and retrieved as-is. Scaling happens at display time only.
