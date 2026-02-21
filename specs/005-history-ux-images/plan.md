# Implementation Plan: History UX & Image Previews

**Branch**: `005-history-ux-images` | **Date**: 2026-02-21 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/005-history-ux-images/spec.md`

## Summary

Improve history window UX: focus ListView on open so type-to-filter works immediately, ensure Escape closes window from any widget, and replace 48px image icons with configurable larger previews (default 320px max side, proportional scaling via `Pixbuf::scale_simple`).

## Technical Context

**Language/Version**: Rust (edition 2021, stable toolchain)
**Primary Dependencies**: gtk4-rs 0.9 (ListView, SearchEntry, ScrolledWindow, gdk_pixbuf), rusqlite, serde, serde_yaml
**Storage**: SQLite (existing, unchanged schema)
**Testing**: `cargo test` (unit tests), `cargo clippy`
**Target Platform**: Linux (X11/Wayland)
**Project Type**: CLI + GTK desktop app
**Performance Goals**: Instant UI response, image scaling at display time
**Constraints**: No new crate dependencies (Principle V)
**Scale/Scope**: 3 files modified (window.rs, entry_row.rs, config/types.rs)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust Idioms & Safety | PASS | No unsafe, standard idiomatic Rust |
| II. Single-Binary CLI | PASS | Changes only GTK window mode, no daemon |
| III. XDG Compliance | PASS | Config field uses existing YAML config path |
| IV. Extensible Data Model | PASS | No schema changes needed |
| V. Minimal Dependencies | PASS | No new crates — `gdk_pixbuf::Pixbuf::scale_simple` is transitive via gtk4 |
| VI. Test Discipline | PASS | Unit tests for scaling logic; GUI code keeps logic separate |
| VII. Simplicity & YAGNI | PASS | Minimal changes, one new config field |

**Post-design re-check**: PASS — no violations.

## Project Structure

### Documentation (this feature)

```text
specs/005-history-ux-images/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── cli.md
└── tasks.md
```

### Source Code (files to modify)

```text
src/
├── config/
│   └── types.rs          # Add image_preview_max_px field
├── ui/
│   ├── window.rs         # Focus, escape, thumbnail scaling
│   └── entry_row.rs      # Remove fixed pixel_size(48)
```

**Structure Decision**: Existing project structure. Only 3 source files are modified. No new files or modules.
