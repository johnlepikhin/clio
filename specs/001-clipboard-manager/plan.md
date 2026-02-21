# Implementation Plan: Clipboard Manager

**Branch**: `001-clipboard-manager` | **Date**: 2026-02-21 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-clipboard-manager/spec.md`

## Summary

Build a Rust CLI clipboard manager (`clio`) that persists clipboard
history in SQLite, provides CLI commands for reading/writing the
clipboard and monitoring it for changes, and offers a GTK4 popup window
for browsing, filtering, and selecting from history. Key crates:
`clap` (CLI), `arboard` (clipboard), `rusqlite` (storage), `gtk4-rs`
(GUI), `blake3` (dedup hashing), `image` (PNG encoding).

## Technical Context

**Language/Version**: Rust (edition 2021, stable toolchain)
**Primary Dependencies**: clap, arboard, rusqlite, gtk4-rs, serde,
serde_yaml, blake3, image, directories, chrono, thiserror, anyhow,
rusqlite_migration
**Storage**: SQLite via rusqlite (WAL mode)
**Testing**: cargo test (unit + integration, in-memory SQLite)
**Target Platform**: Linux (X11/Wayland)
**Project Type**: CLI + desktop-app (hybrid)
**Performance Goals**: CLI ops <1s, history window <2s for 500 entries,
filter update <200ms
**Constraints**: Single-user local app, no network, offline-only
**Scale/Scope**: 1 user, up to 500 history entries (configurable)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust Idioms & Safety | PASS | thiserror + anyhow, no unwrap in non-test code, clippy clean |
| II. Single-Binary CLI | PASS (with justified violation) | `clio watch` is long-running — see Complexity Tracking |
| III. XDG Compliance | PASS | `directories` crate for path resolution |
| IV. Extensible Data Model | PASS | JSON metadata column, versioned migrations |
| V. Minimal Dependencies | PASS (with justified additions) | `blake3`, `image`, `rusqlite_migration` — see Complexity Tracking |
| VI. Test Discipline | PASS | Unit tests for all public modules, integration tests for CLI, in-memory SQLite |
| VII. Simplicity & YAGNI | PASS | No abstractions beyond what spec requires |

**Post-Phase 1 re-check**: All gates still pass. Additional
dependencies (`blake3`, `image`, `rusqlite_migration`) justified in
research.md and Complexity Tracking below.

## Project Structure

### Documentation (this feature)

```text
specs/001-clipboard-manager/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── cli.md
└── tasks.md
```

### Source Code (repository root)

```text
src/
├── main.rs              # Entry point, clap CLI parsing
├── cli/
│   ├── mod.rs           # CLI command definitions (clap derive)
│   ├── show.rs          # clio show implementation
│   ├── copy.rs          # clio copy implementation
│   ├── watch.rs         # clio watch implementation
│   └── history.rs       # clio history (launches GTK window)
├── db/
│   ├── mod.rs           # Database connection, initialization, PRAGMAs
│   ├── migrations.rs    # Schema migrations (rusqlite_migration)
│   └── repository.rs    # ClipboardEntry CRUD operations
├── clipboard/
│   ├── mod.rs           # Clipboard read/write via arboard
│   └── source_app.rs    # Source app detection (X11 best-effort)
├── config/
│   ├── mod.rs           # Configuration loading, defaults
│   └── types.rs         # Config struct with serde
├── models/
│   └── entry.rs         # ClipboardEntry domain model + hashing
└── ui/
    ├── mod.rs           # GTK application setup
    ├── window.rs        # History window layout
    ├── entry_row.rs     # List item factory for entry rows
    └── entry_object.rs  # GObject wrapper for ClipboardEntry

tests/
├── integration/
│   ├── cli_show.rs      # Integration tests for clio show
│   ├── cli_copy.rs      # Integration tests for clio copy
│   └── cli_watch.rs     # Integration tests for clio watch
└── unit/
    ├── db_test.rs       # Repository unit tests (in-memory SQLite)
    ├── config_test.rs   # Config loading tests
    └── models_test.rs   # Entry model + hashing tests
```

**Structure Decision**: Single Rust project with flat module
organization. No workspace needed — the project is a single binary.
Modules are organized by responsibility: `cli/` for command handlers,
`db/` for persistence, `clipboard/` for system clipboard interaction,
`config/` for settings, `models/` for domain types, `ui/` for GTK
window. Tests split into `integration/` (full CLI binary tests) and
`unit/` (module-level tests).

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| `clio watch` is long-running (Principle II: "short-lived CLI command") | Core requirement — clipboard history cannot be populated without monitoring. All clipboard managers require a watcher process. | Alternative: only save via `clio copy` — makes history useless for typical workflows where users copy via Ctrl+C in any app. |
| `blake3` crate (Principle V: not in approved list) | Content deduplication (FR-015) requires hashing. BLAKE3 is 2-10x faster than SHA-256, pure Rust, 32-byte digest. | Alternative: direct content comparison — O(N*M) for large BLOBs, impractical with image entries up to 50MB. |
| `image` crate (Principle V: not in approved list) | arboard returns raw RGBA pixels. PNG encoding required for storage efficiency (4-10x size reduction). Also needed for thumbnail generation in history window. | Alternative: store raw RGBA — wastes significant disk space; no thumbnail without image processing. |
| `rusqlite_migration` crate (Principle V: not in approved list) | Schema versioning (FR-012) requires migration framework. rusqlite_migration is minimal (uses user_version pragma, no extra tables). | Alternative: manual migration code — error-prone, no rollback tracking, reinvents what the crate does in ~100 lines. |
