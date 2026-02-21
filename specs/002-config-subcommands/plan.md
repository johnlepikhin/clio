# Implementation Plan: Config Management Subcommands

**Branch**: `002-config-subcommands` | **Date**: 2026-02-21 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-config-subcommands/spec.md`

## Summary

Add a `clio config` subcommand group with four commands: `show`, `init`, `validate`, and `path`. Follows the same pattern as `voice-type` — nested clap subcommands dispatched from `main.rs`, handlers in a new `src/cli/config.rs` module. Requires adding `Serialize` to `Config` and new helper methods.

## Technical Context

**Language/Version**: Rust (edition 2021, stable toolchain)
**Primary Dependencies**: clap 4 (derive), serde + serde_yaml, directories
**Storage**: N/A (filesystem only for config file)
**Testing**: cargo test (unit + integration)
**Target Platform**: Linux (X11/Wayland)
**Project Type**: CLI application
**Performance Goals**: N/A (instant CLI commands)
**Constraints**: No new dependencies (Principle V)
**Scale/Scope**: 4 subcommands, ~3 files modified, ~1 new file

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust Idioms & Safety | PASS | Standard derive macros, proper error handling via `thiserror`/`anyhow` |
| II. Single-Binary CLI | PASS | New subcommands via `clap` derive, short-lived commands |
| III. XDG Compliance | PASS | Config path resolved via `directories` crate, `config path` exposes it |
| IV. Extensible Data Model | N/A | No DB schema changes |
| V. Minimal Dependencies | PASS | No new crates required |
| VI. Test Discipline | PASS | Unit tests for new public functions, CLI contract tests planned |
| VII. Simplicity & YAGNI | PASS | Only specified subcommands, no over-engineering |

**Post-design re-check**: All gates still pass. No violations.

## Project Structure

### Documentation (this feature)

```text
specs/002-config-subcommands/
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
│   ├── mod.rs           # MODIFY: add ConfigCommands enum, Config variant to Commands
│   ├── config.rs        # NEW: config subcommand handlers (show, init, validate, path)
│   ├── copy.rs
│   ├── history.rs
│   ├── show.rs
│   └── watch.rs
├── config/
│   ├── mod.rs           # MODIFY: add default_config_path() public function
│   └── types.rs         # MODIFY: add Serialize derive, default_yaml(), validate()
├── main.rs              # MODIFY: add Commands::Config dispatch
└── errors.rs
```

**Structure Decision**: Single project layout (existing). One new file `src/cli/config.rs`, three files modified. No structural changes.

## Complexity Tracking

No violations. No complexity justifications needed.
