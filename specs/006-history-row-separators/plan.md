# Implementation Plan: History Row Visual Separators

**Branch**: `006-history-row-separators` | **Date**: 2026-02-21 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/006-history-row-separators/spec.md`

## Summary

Add thin horizontal line separators between clipboard history entries in the GTK4 ListView. GTK4 provides a built-in `show-separators` property on ListView that renders theme-aware horizontal dividers between rows. This is a single-line code change.

## Technical Context

**Language/Version**: Rust (edition 2021, stable toolchain)
**Primary Dependencies**: gtk4-rs 0.9
**Storage**: N/A (no data model changes)
**Testing**: `cargo test` + `cargo clippy` (GUI exempt from automated tests per constitution; manual verification)
**Target Platform**: Linux (X11/Wayland)
**Project Type**: Desktop app (GTK4 clipboard manager)
**Performance Goals**: N/A (no performance impact — CSS-only change)
**Constraints**: No new dependencies allowed
**Scale/Scope**: 1 file, 1 line of code

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust Idioms & Safety | PASS | No new unsafe code |
| II. Single-Binary CLI Architecture | PASS | No architecture changes |
| III. XDG Compliance | PASS | No file system changes |
| IV. Extensible Data Model | PASS | No schema changes |
| V. Minimal Dependencies | PASS | No new dependencies — uses built-in GTK4 property |
| VI. Test Discipline | PASS | GUI styling exempt from automated tests per constitution |
| VII. Simplicity & YAGNI | PASS | Single property setter, minimal possible implementation |

All gates pass. No violations.

## Project Structure

### Documentation (this feature)

```text
specs/006-history-row-separators/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── spec.md              # Feature specification
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
src/
└── ui/
    └── window.rs        # Only file modified — add set_show_separators(true)
```

**Structure Decision**: No new files or directories. Single property change in existing `window.rs` where the ListView is constructed.

## Design

### Approach

GTK4's `ListView` widget has a built-in `show-separators` property. When set to `true`, GTK automatically:
1. Adds the `.separators` CSS class to the listview
2. Renders a thin horizontal line between rows using the theme's `@borders` color
3. Respects both light and dark themes automatically

### Change

In `src/ui/window.rs`, after creating the `ListView` (line ~114), add:

```rust
list_view.set_show_separators(true);
```

This satisfies all functional requirements:
- **FR-001**: Thin horizontal line between entries — provided by GTK built-in
- **FR-002**: Consistent for all entry types — separator is between rows, independent of content
- **FR-003**: Does not conflict with selection highlight — GTK handles this natively
- **FR-004**: Works with light/dark themes — uses `@borders` theme color
- **FR-005**: Consistent with lazy loading — separator is a row property, not content-dependent

### Alternatives Rejected

| Alternative | Why Rejected |
|-------------|-------------|
| Custom CSS with CssProvider | Unnecessary complexity — built-in property does exactly what's needed |
| CSS border-bottom on row widget | Requires creating CssProvider infrastructure for a one-liner |
| Gtk Separator widget between rows | ListView doesn't support inserting non-data widgets between rows |
| Zebra striping | Not chosen by user (clarification session) |
| Card-style layout | Not chosen by user (clarification session) |
