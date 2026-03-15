# clio Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-21

## Active Technologies
- Rust (edition 2021, stable toolchain) + clap 4 (derive), serde + serde_yaml, directories (002-config-subcommands)
- N/A (filesystem only for config file) (002-config-subcommands)
- Rust (edition 2021, stable toolchain) + arboard 3.6.1 (LinuxClipboardKind, GetExtLinux, SetExtLinux), serde, serde_yaml, clap (003-clipboard-sync)
- SQLite (existing, unchanged) (003-clipboard-sync)
- Rust (edition 2021, stable toolchain) + gtk4-rs (ListView, ScrolledWindow, ListStore), rusqlite, serde, serde_yaml, clap (004-history-preview)
- SQLite (existing, unchanged schema) (004-history-preview)
- Rust (edition 2021, stable toolchain) + gtk4-rs 0.9 (ListView, SearchEntry, ScrolledWindow, gdk_pixbuf), rusqlite, serde, serde_yaml (005-history-ux-images)
- N/A (no data model changes) (006-history-row-separators)
- N/A (Markdown documentation only) (007-readme-docs)

- Rust (edition 2021, stable toolchain) + clap, arboard, rusqlite, gtk4-rs, serde, (001-clipboard-manager)

## Project Structure

```text
Cargo.toml          # workspace root, clio lib+bin (no GTK)
src/
  lib.rs            # public modules for clio library
  main.rs           # CLI binary entry point
clio-history/       # separate crate: GTK4 history viewer
  Cargo.toml
  src/
    main.rs
    ui/             # GTK4 UI (moved from src/ui/)
```

## Commands

cargo test -p clio                                          # tests without GTK4
guix shell -m manifest.scm -- cargo test -p clio-history    # UI tests (needs GTK4)
cargo clippy -p clio
guix shell -m manifest.scm -- cargo clippy -p clio-history

## Code Style

Rust (edition 2021, stable toolchain): Follow standard conventions

## Recent Changes
- 009-memory-optimization: Internal optimization, no new dependencies
- 007-readme-docs: Added N/A (Markdown documentation only)
- 006-history-row-separators: Added Rust (edition 2021, stable toolchain) + gtk4-rs 0.9


<!-- MANUAL ADDITIONS START -->

## Tools

- **ast-index**: Actively use the `/ast-index` skill for codebase navigation — finding structs, functions, implementations, usages, module dependencies, and project structure. Prefer it over manual Grep/Glob when searching for symbols or understanding code relationships.

<!-- MANUAL ADDITIONS END -->
