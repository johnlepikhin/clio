# Implementation Plan: Clipboard & Paste Buffer Synchronization

**Branch**: `003-clipboard-sync` | **Date**: 2026-02-21 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/003-clipboard-sync/spec.md`

## Summary

Add configurable synchronization between X11 PRIMARY selection (paste buffer / mouse selection) and CLIPBOARD selection (Ctrl+C/V). A new `SyncMode` enum in the config controls direction: `both` (default), `to-clipboard`, `to-primary`, `disabled`. The `clio watch` loop is extended to monitor both selections, sync content according to the mode, and prevent infinite loops via dual hash tracking.

## Technical Context

**Language/Version**: Rust (edition 2021, stable toolchain)
**Primary Dependencies**: arboard 3.6.1 (LinuxClipboardKind, GetExtLinux, SetExtLinux), serde, serde_yaml, clap
**Storage**: SQLite (existing, unchanged)
**Testing**: cargo test (unit tests for SyncMode serde, clipboard functions)
**Target Platform**: Linux (X11/Wayland with data-control v2+)
**Project Type**: CLI application
**Performance Goals**: Sync within one polling interval (< 500ms default)
**Constraints**: No new dependencies (Principle V); no infinite sync loops
**Scale/Scope**: 1 new enum, ~3 files modified, ~1 new function set

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust Idioms & Safety | PASS | Standard derive macros, proper error handling |
| II. Single-Binary CLI | PASS | Extends existing `watch` command, no daemon |
| III. XDG Compliance | N/A | Config path unchanged |
| IV. Extensible Data Model | N/A | No schema changes |
| V. Minimal Dependencies | PASS | Uses existing arboard types only |
| VI. Test Discipline | PASS | Unit tests for SyncMode serde and validation |
| VII. Simplicity & YAGNI | PASS | Only the four specified modes, no extras |

**Post-design re-check**: All gates still pass. No violations.

## Project Structure

### Documentation (this feature)

```text
specs/003-clipboard-sync/
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
│   ├── watch.rs         # MODIFY: dual-selection polling, sync logic, loop prevention
│   └── ...
├── clipboard/
│   └── mod.rs           # MODIFY: add read/write functions parameterized by LinuxClipboardKind
├── config/
│   ├── types.rs         # MODIFY: add SyncMode enum, add sync_mode field to Config
│   └── mod.rs           # MODIFY: update default_yaml() template and tests
└── ...
```

**Structure Decision**: Existing single project layout. No new files. Three source files modified.

## Complexity Tracking

No violations. No complexity justifications needed.
