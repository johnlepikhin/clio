# Research: Memory Optimization

**Feature**: 009-memory-optimization
**Date**: 2026-02-22

## R1: Eliminating RGBA clone in `encode_rgba_to_png`

**Decision**: Change `encode_rgba_to_png` signature from `rgba_bytes: &[u8]` to `rgba_bytes: Vec<u8>` so `RgbaImage::from_raw()` can take ownership directly, eliminating the `.to_vec()` clone.

**Rationale**: `image::RgbaImage::from_raw(width, height, buf)` accepts `Vec<u8>` by ownership. Current code passes `&[u8]` and then clones via `.to_vec()`, doubling RGBA memory (~8 MB for 1080p). By passing the owned `Vec<u8>` through from the caller (`ClipboardEntry::from_image`), the clone is eliminated entirely.

**Alternatives considered**:
- Keep `&[u8]` and accept the clone — rejected, trivial fix with large impact.
- Use `Cow<[u8]>` — overcomplicates API for no benefit since callers always have owned data.

## R2: Sequential CLIPBOARD/PRIMARY reads

**Decision**: Restructure the watch loop poll to process CLIPBOARD first (read → hash → save → drop buffer), then PRIMARY, instead of reading both into variables simultaneously.

**Rationale**: Lines 305-306 in `watch.rs` hold `cb_content` and `pr_content` at the same time. For two large images this is ~16 MB peak. Sequential processing halves peak to ~8 MB. The 500 ms watch interval provides ample time headroom.

**Alternatives considered**:
- Keep simultaneous reads — rejected, easy win with zero functional impact.
- Use threads for parallel processing — overengineered, adds complexity for no benefit.

## R3: Early size check before PNG encoding

**Decision**: Check RGBA byte length against `max_entry_size_kb` before calling `encode_rgba_to_png`. Since PNG is always smaller than RGBA, an RGBA buffer exceeding the limit guarantees the PNG will also exceed it.

**Rationale**: Currently `save_if_fits()` checks size after PNG encoding. For a 4K image (~32 MB RGBA), this wastes both CPU and memory. RGBA size is a conservative upper bound — if RGBA exceeds the limit, PNG certainly will too.

**Alternatives considered**:
- Check estimated PNG size (RGBA ÷ compression ratio) — fragile, compression ratio varies.
- Keep current post-encoding check — rejected, easy fix that avoids unnecessary work.
- Note: Post-encoding check must remain as a safety net for edge cases where RGBA is below limit but PNG somehow isn't (shouldn't happen, but defense-in-depth).

## R4: Thumbnail texture caching strategy

**Decision**: Use a bounded `HashMap<Vec<u8>, gdk::Texture>` keyed by content hash, with capacity equal to `history_page_size`. Evict all entries on page change.

**Rationale**: GTK4 ListView recycles widgets but the `create_thumbnail_texture` function re-decodes PNG on every bind. A page-sized cache covers all visible entries. Evicting on page change keeps memory bounded and implementation simple.

**Alternatives considered**:
- LRU cache with configurable size — overengineered for this use case (YAGNI).
- No cache, rely on GTK4 — GTK4 recycles widgets but not application-level data.
- Store pre-scaled thumbnails in database — adds schema complexity, violates Principle IV (migrations).

## R5: Caller chain for `encode_rgba_to_png`

**Decision**: Propagate ownership change through the call chain: `ClipboardContent::Image { rgba_bytes: Vec<u8> }` → `ClipboardEntry::from_image(width, height, rgba_bytes: Vec<u8>)` → `encode_rgba_to_png(width, height, rgba_bytes: Vec<u8>)`.

**Rationale**: `ClipboardContent::Image` already owns a `Vec<u8>` (from `arboard`'s `into_owned()`). `from_image` currently borrows it as `&[u8]` and passes to `encode_rgba_to_png(&[u8])`. Changing both to take `Vec<u8>` by move eliminates the clone chain.

**Alternatives considered**:
- Only change `encode_rgba_to_png` — insufficient, caller still borrows.
- Use `Arc<Vec<u8>>` — unnecessary complexity, buffer is only used once.
