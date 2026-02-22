# Feature Specification: Memory Optimization

**Feature Branch**: `009-memory-optimization`
**Created**: 2026-02-22
**Status**: Draft
**Input**: User description: "Необходимо поработать над оптимизацией потребления памяти."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Reduce peak memory when copying large images (Priority: P1)

A user copies a large screenshot (e.g., 1920×1080) to the clipboard while `clio watch` is running. The watch loop reads the RGBA data and encodes it to PNG for storage. Currently, during encoding the RGBA buffer is cloned unnecessarily, doubling peak memory (~18 MB for a single 1080p image). After optimization, the watch loop should process the image without redundant allocations.

**Why this priority**: Image encoding is the single largest memory spike in the application. Every clipboard image copy triggers it, making it the most impactful fix.

**Independent Test**: Copy a large image to clipboard with `clio watch` running and measure peak resident memory (RSS). It should be noticeably lower than before the optimization.

**Acceptance Scenarios**:

1. **Given** `clio watch` is running, **When** user copies a 1920×1080 screenshot, **Then** peak memory during processing does not exceed 12 MB above baseline (vs ~20 MB before).
2. **Given** `clio watch` is running, **When** user copies a small text snippet, **Then** memory consumption remains negligible (under 1 MB above baseline).

---

### User Story 2 - Sequential clipboard/primary sync for images (Priority: P2)

When sync mode is active (`both`), the watch loop reads both CLIPBOARD and PRIMARY selections simultaneously. If both contain large images, this doubles peak memory unnecessarily. The system should process one selection at a time, releasing the first buffer before reading the second.

**Why this priority**: Sync mode is the default configuration. Two large images read simultaneously cause a ~20 MB peak that is easily avoidable with sequential processing.

**Independent Test**: With sync mode `both`, copy a large image and measure that peak memory stays comparable to processing a single image, not doubled.

**Acceptance Scenarios**:

1. **Given** sync mode is `both` and user copies a 1080p image, **When** the watch loop processes the change, **Then** peak memory does not exceed that of single-image processing plus a small constant overhead.
2. **Given** sync mode is `disabled`, **When** user copies a large image, **Then** behavior is unchanged from current implementation.

---

### User Story 3 - Early size rejection for oversized images (Priority: P2)

Currently, the `max_entry_size_kb` check happens after PNG encoding is complete. This means a very large RGBA image is fully encoded to PNG before being rejected. The system should estimate the resulting size and reject oversized entries before performing expensive encoding.

**Why this priority**: Prevents wasted CPU and memory on images that will be discarded anyway. Complements Story 1 by avoiding encoding altogether for entries that exceed the limit.

**Independent Test**: Copy an extremely large image (exceeding `max_entry_size_kb`) and verify that memory spike is minimal and no PNG encoding is performed.

**Acceptance Scenarios**:

1. **Given** `max_entry_size_kb` is 50 MB and user copies a 4K screenshot (~32 MB RGBA), **When** RGBA size exceeds the configured limit, **Then** the entry is skipped without PNG encoding and a log message is emitted.
2. **Given** `max_entry_size_kb` is 50 MB and user copies a normal screenshot (~8 MB RGBA), **When** RGBA size is within limits, **Then** encoding proceeds normally.

---

### User Story 4 - Thumbnail texture caching in history window (Priority: P3)

When the user opens the history window and scrolls through entries, each image thumbnail is decoded from PNG, decompressed to RGBA, scaled, and uploaded as a GPU texture on every display. If the same image appears multiple times or the user scrolls back and forth, the same decoding work repeats. A cache of recently decoded textures should avoid redundant work.

**Why this priority**: Affects interactive performance of the history window. Lower priority because GTK4 ListView already manages off-screen items and this only matters with many image entries.

**Independent Test**: Open history window with 50+ image entries, scroll up and down repeatedly, and measure that memory does not grow on repeated scrolls (stable after initial load).

**Acceptance Scenarios**:

1. **Given** history contains 50 image entries, **When** user scrolls through the list twice, **Then** peak memory on the second scroll is no higher than on the first.
2. **Given** history contains only text entries, **When** user scrolls through the list, **Then** no texture caching overhead is introduced.

---

### Edge Cases

- What happens when clipboard contains a corrupted or zero-size image? The system should skip it gracefully without allocation.
- What happens when the user rapidly copies many large images in sequence? Each should be processed and freed before the next, without accumulating buffers.
- What happens when available system memory is very low? The system should handle allocation failures gracefully and continue operating for text entries.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST NOT create redundant copies of image data during PNG encoding. The RGBA buffer passed to the encoder must be consumed by move, not cloned.
- **FR-002**: System MUST process CLIPBOARD and PRIMARY selections sequentially (not simultaneously) when sync mode is active, releasing the first buffer before reading the second.
- **FR-003**: System MUST estimate entry size from raw RGBA dimensions before PNG encoding and skip entries that would exceed `max_entry_size_kb`, logging a warning.
- **FR-004**: System SHOULD cache decoded thumbnail textures in the history window, keyed by content hash, to avoid redundant PNG decoding on repeated display.
- **FR-005**: System MUST NOT regress in functionality — all existing clipboard operations (text, image, sync, TTL, actions) must continue working correctly.
- **FR-006**: Thumbnail cache SHOULD have a bounded size (e.g., matching `history_page_size`) to prevent unbounded memory growth.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Peak memory during processing of a single 1920×1080 image is reduced by at least 30% compared to current implementation.
- **SC-002**: Peak memory during sync-mode processing of a large image does not exceed 1.5× the single-image peak (vs ~2× currently).
- **SC-003**: All existing tests pass without modification (no regressions).
- **SC-004**: Scrolling through 50 image entries in the history window twice results in stable memory (second pass within 10% of first pass peak).

## Assumptions

- Image entries are the primary source of memory pressure; text entries are negligible in comparison.
- GTK4 ListView already handles off-screen widget recycling; the optimization focuses on the application-level data, not GTK internals.
- The `image` crate's `RgbaImage::from_raw()` can accept ownership of the buffer without cloning.
- Sequential processing of CLIPBOARD and PRIMARY adds negligible latency relative to the 500 ms watch interval.
