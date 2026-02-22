# Implementation Plan: Memory Optimization

**Branch**: `009-memory-optimization` | **Date**: 2026-02-22 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/009-memory-optimization/spec.md`

## Summary

Optimize peak memory consumption in clio's image processing pipeline. The main bottleneck is redundant RGBA buffer cloning during PNG encoding (~18 MB peak for a single 1080p image). Additional improvements: sequential clipboard/primary reads, early size rejection before encoding, and thumbnail texture caching in the history window.

## Technical Context

**Language/Version**: Rust (edition 2021, stable toolchain)
**Primary Dependencies**: image 0.25 (PNG), arboard (clipboard), gtk4-rs (UI), rusqlite (SQLite)
**Storage**: SQLite (unchanged)
**Testing**: `cargo test --no-default-features` (unit), `guix shell -m manifest.scm -- cargo test` (full)
**Target Platform**: Linux (X11/Wayland)
**Project Type**: CLI + desktop app
**Performance Goals**: Reduce peak image processing memory by ≥30%
**Constraints**: No new dependencies, no schema changes, no API changes
**Scale/Scope**: 3 source files modified, ~50 lines changed

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust Idioms & Safety | PASS | Ownership-based optimization is idiomatic Rust. No unsafe code. |
| II. Single-Binary CLI Architecture | PASS | No architectural changes. |
| III. XDG Compliance | PASS | Not applicable — no file path changes. |
| IV. Extensible Data Model | PASS | No schema changes. |
| V. Minimal Dependencies | PASS | No new dependencies. |
| VI. Test Discipline | PASS | Existing tests verify no regressions. New unit test for early size check. |
| VII. Simplicity & YAGNI | PASS | Each change is minimal and directly addresses measured memory waste. |

## Project Structure

### Documentation (this feature)

```text
specs/009-memory-optimization/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── models/
│   └── entry.rs         # encode_rgba_to_png signature change, from_image ownership
├── cli/
│   └── watch.rs         # Sequential reads, early size check
└── ui/
    └── window.rs        # Thumbnail texture cache
```

**Structure Decision**: No structural changes. All modifications are within existing files following current project layout.

## Design Decisions

### D1: Ownership transfer for `encode_rgba_to_png`

Change signature from `rgba_bytes: &[u8]` to `rgba_bytes: Vec<u8>`. This allows `RgbaImage::from_raw()` to consume the buffer without cloning. The caller chain (`ClipboardContent::Image` → `from_image` → `encode_rgba_to_png`) already owns the data.

**Impact**: Eliminates ~8 MB allocation per 1080p image encoding.

### D2: Sequential clipboard/primary processing

Restructure the poll iteration in `watch.rs` to:
1. Read CLIPBOARD → hash → process → drop buffer
2. Read PRIMARY → hash → process → drop buffer

Instead of reading both into variables simultaneously.

**Impact**: Halves peak memory when both selections contain images.

### D3: Early RGBA size rejection

Before calling `encode_rgba_to_png`, check if `rgba_bytes.len()` exceeds `max_entry_size_kb * 1024`. Since PNG is always smaller than RGBA, this is a safe lower bound. Keep the existing post-encoding check as defense-in-depth.

**Impact**: Avoids ~8 MB PNG encoding allocation for oversized images.

### D4: Bounded thumbnail cache

Add `HashMap<Vec<u8>, gdk::Texture>` to the window state, keyed by content hash (32 bytes). Capacity bounded to `history_page_size`. Cleared on page navigation. Only used in the UI feature gate.

**Impact**: Eliminates redundant PNG decode + scale on repeated scrolls.

## Complexity Tracking

No constitution violations. No complexity justifications needed.
