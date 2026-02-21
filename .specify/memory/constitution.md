<!--
Sync Impact Report
===================
Version change: 0.0.0 → 1.0.0 (initial adoption)
Modified principles: N/A (initial version)
Added sections:
  - Core Principles (7 principles)
  - Technology Stack
  - Development Workflow
  - Governance
Removed sections: N/A
Templates requiring updates:
  - .specify/templates/plan-template.md ✅ no changes needed
  - .specify/templates/spec-template.md ✅ no changes needed
  - .specify/templates/tasks-template.md ✅ no changes needed
Follow-up TODOs: none
-->

# Clio Constitution

## Core Principles

### I. Rust Idioms & Safety

All code MUST be written in idiomatic Rust. Unsafe blocks are
prohibited unless justified in writing with a comment explaining
why safe alternatives are insufficient. The project MUST compile
with no warnings under `#[warn(clippy::all)]`. Error handling
MUST use `thiserror` for library errors and `anyhow` for
application-level propagation — no `.unwrap()` on fallible
operations in non-test code.

### II. Single-Binary CLI Architecture

Clio is a single compiled binary with no daemon process. Every
invocation is a short-lived CLI command that performs one action
and exits. Subcommands MUST be implemented via `clap` with derive
macros. The binary MUST be usable non-interactively (piping,
scripting) for text operations and MUST support a GTK window mode
for interactive history browsing.

### III. XDG Compliance

All persistent files MUST follow the XDG Base Directory
Specification:
- Configuration: `$XDG_CONFIG_HOME/clio/` (default `~/.config/clio/`)
- Data (SQLite DB): `$XDG_DATA_HOME/clio/` (default `~/.local/share/clio/`)

The application MUST create directories automatically if they do
not exist. Hardcoded paths are prohibited — always resolve via
XDG environment variables with standard fallbacks.

### IV. Extensible Data Model

The clipboard entry schema MUST support:
- Content storage (text and images via BLOB)
- Timestamp of creation/selection
- Source application name (optional)
- Extensible metadata via a JSON column or flags table

Schema changes MUST be managed through versioned migrations.
Adding new entry metadata (e.g., private flag, TTL) MUST NOT
require breaking changes to existing records.

### V. Minimal Dependencies

Every external crate MUST be justified. The core dependency set:
- `clap` — CLI parsing
- `rusqlite` — SQLite access
- `arboard` — clipboard interaction
- `gtk4` / `gtk4-rs` — GUI window
- `serde` + `serde_yaml` — configuration
- `dirs` or `directories` — XDG path resolution
- `chrono` — timestamp handling
- `thiserror` / `anyhow` — error handling

Adding a dependency outside this list requires explicit
justification that no existing crate or standard library
feature covers the need.

### VI. Test Discipline

All public module interfaces MUST have unit tests. Integration
tests MUST cover the full CLI command surface (stdout output,
exit codes). SQLite operations MUST be tested against an
in-memory database. GUI code is exempt from automated testing
but MUST be structured to keep business logic separate from
GTK widget code so the logic layer remains testable.

### VII. Simplicity & YAGNI

Start with the minimum viable implementation. Do not add
abstractions, traits, or generics unless there are at least two
concrete use cases. Configuration options MUST have sensible
defaults. Features not described in the specification MUST NOT
be implemented speculatively.

## Technology Stack

- **Language**: Rust (edition 2021, stable toolchain)
- **CLI**: `clap` with derive API
- **Storage**: SQLite via `rusqlite` (WAL mode for concurrency)
- **Clipboard**: `arboard`
- **GUI**: `gtk4-rs`
- **Config format**: YAML (`serde_yaml`)
- **Build**: `cargo`, standard `Cargo.toml` workspace if needed
- **Linting**: `clippy` + `rustfmt`
- **Testing**: `cargo test` (unit + integration)
- **Target platform**: Linux (X11/Wayland)

## Development Workflow

- All changes MUST pass `cargo clippy` and `cargo test` before
  being committed.
- `cargo fmt --check` MUST report no diffs.
- Commits follow Conventional Commits format
  (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`).
- Each feature MUST have a specification created via the Specify
  workflow before implementation begins.
- Code reviews MUST verify compliance with this constitution.

## Governance

This constitution is the highest-authority document for the Clio
project. All specifications, plans, and task lists MUST comply
with the principles defined here.

**Amendment procedure**:
1. Propose change with rationale.
2. Update constitution with new version number.
3. Propagate changes to dependent templates and documents.
4. Document changes in the Sync Impact Report comment.

**Versioning**: MAJOR.MINOR.PATCH semantic versioning.
- MAJOR: Principle removal or incompatible redefinition.
- MINOR: New principle or material expansion.
- PATCH: Clarifications, wording, non-semantic refinements.

**Compliance**: Every PR and code review MUST verify adherence
to these principles. Violations MUST be resolved before merge.

**Version**: 1.0.0 | **Ratified**: 2026-02-21 | **Last Amended**: 2026-02-21
