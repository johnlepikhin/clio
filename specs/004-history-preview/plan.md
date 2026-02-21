# Implementation Plan: History Preview & Lazy Loading

**Branch**: `004-history-preview` | **Date**: 2026-02-21 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-history-preview/spec.md`

## Summary

Add configurable text preview truncation (default 4 KB) with ellipsis and multiline display in the history window. Replace the current load-all-at-once approach with paginated loading (default 50 entries), loading more on scroll. Refactor filtering from in-memory `CustomFilter` to database-side `LIKE` queries with paginated results. Two new config fields: `preview_text_bytes` and `history_page_size`.

## Technical Context

**Language/Version**: Rust (edition 2021, stable toolchain)
**Primary Dependencies**: gtk4-rs (ListView, ScrolledWindow, ListStore), rusqlite, serde, serde_yaml, clap
**Storage**: SQLite (existing, unchanged schema)
**Testing**: cargo test (unit tests for truncation helper, paginated queries)
**Target Platform**: Linux (X11/Wayland)
**Project Type**: CLI application with GTK4 history window
**Performance Goals**: Window opens < 1s with 10k entries; scroll load < 500ms per page
**Constraints**: No new dependencies (Principle V); no schema changes
**Scale/Scope**: 2 new config fields, ~4 files modified, ~2 new repository functions

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust Idioms & Safety | PASS | Standard string truncation, no unsafe |
| II. Single-Binary CLI | PASS | Extends existing `history` command window |
| III. XDG Compliance | N/A | Config path unchanged |
| IV. Extensible Data Model | N/A | No schema changes — reads existing columns |
| V. Minimal Dependencies | PASS | No new crates needed |
| VI. Test Discipline | PASS | Unit tests for truncation, paginated queries, config validation |
| VII. Simplicity & YAGNI | PASS | Only specified config fields and behaviors |

**Post-design re-check**: All gates still pass. No violations.

## Project Structure

### Documentation (this feature)

```text
specs/004-history-preview/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/cli.md     # Phase 1 output
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── cli/
│   └── ...
├── config/
│   ├── types.rs         # MODIFY: add preview_text_bytes, history_page_size fields
│   └── mod.rs           # MODIFY: add tests for new config fields
├── db/
│   └── repository.rs    # MODIFY: add list_entries_page(), search_entries_page()
├── ui/
│   ├── window.rs        # MODIFY: paginated loading, scroll-to-load, DB-side filtering
│   ├── entry_row.rs     # MODIFY: multiline preview with truncation + ellipsis
│   └── entry_object.rs  # UNCHANGED
└── ...
```

**Structure Decision**: Existing single project layout. No new files. Four source files modified.

## Complexity Tracking

No violations. No complexity justifications needed.
