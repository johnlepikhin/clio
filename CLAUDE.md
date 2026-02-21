# clio Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-21

## Active Technologies
- Rust (edition 2021, stable toolchain) + clap 4 (derive), serde + serde_yaml, directories (002-config-subcommands)
- N/A (filesystem only for config file) (002-config-subcommands)
- Rust (edition 2021, stable toolchain) + arboard 3.6.1 (LinuxClipboardKind, GetExtLinux, SetExtLinux), serde, serde_yaml, clap (003-clipboard-sync)
- SQLite (existing, unchanged) (003-clipboard-sync)
- Rust (edition 2021, stable toolchain) + gtk4-rs (ListView, ScrolledWindow, ListStore), rusqlite, serde, serde_yaml, clap (004-history-preview)
- SQLite (existing, unchanged schema) (004-history-preview)

- Rust (edition 2021, stable toolchain) + clap, arboard, rusqlite, gtk4-rs, serde, (001-clipboard-manager)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust (edition 2021, stable toolchain): Follow standard conventions

## Recent Changes
- 004-history-preview: Added Rust (edition 2021, stable toolchain) + gtk4-rs (ListView, ScrolledWindow, ListStore), rusqlite, serde, serde_yaml, clap
- 003-clipboard-sync: Added Rust (edition 2021, stable toolchain) + arboard 3.6.1 (LinuxClipboardKind, GetExtLinux, SetExtLinux), serde, serde_yaml, clap
- 002-config-subcommands: Added Rust (edition 2021, stable toolchain) + clap 4 (derive), serde + serde_yaml, directories


<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
