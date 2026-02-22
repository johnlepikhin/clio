# Data Model: Memory Optimization

**Feature**: 009-memory-optimization
**Date**: 2026-02-22

## Entities

No new entities or schema changes. This feature modifies internal data flow only.

## Modified Data Flows

### ClipboardContent::Image

Current:
```
ClipboardContent::Image { rgba_bytes: Vec<u8> }
  → from_image(w, h, &rgba_bytes)       // borrows
    → encode_rgba_to_png(w, h, &[u8])   // borrows, then clones via .to_vec()
```

After:
```
ClipboardContent::Image { rgba_bytes: Vec<u8> }
  → from_image(w, h, rgba_bytes)         // moves ownership
    → encode_rgba_to_png(w, h, Vec<u8>)  // moves ownership to RgbaImage::from_raw()
```

### Watch Loop Poll Cycle

Current:
```
cb_content = read_selection(Clipboard)   // allocated
pr_content = read_selection(Primary)     // allocated (both held)
process(cb_content, pr_content)          // peak: 2× image size
drop both
```

After:
```
cb_content = read_selection(Clipboard)   // allocated
process(cb_content)                      // peak: 1× image size
drop cb_content                          // freed
pr_content = read_selection(Primary)     // allocated
process(pr_content)                      // peak: 1× image size
drop pr_content                          // freed
```

### Thumbnail Cache (UI only)

New transient state in history window:
```
thumbnail_cache: HashMap<Vec<u8>, gdk::Texture>
  key: content_hash (SHA-256, 32 bytes)
  value: decoded & scaled texture
  capacity: history_page_size entries
  lifetime: cleared on page navigation
```
