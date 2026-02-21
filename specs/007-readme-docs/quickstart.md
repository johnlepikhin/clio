# Quickstart: README.md Blueprint

**Date**: 2026-02-22

This document defines the exact structure and content outline for the README.md deliverable.

## README Section Order

```
1. Title + one-liner
2. Overview (2-3 sentences)
3. Quick Start (3 steps: build → watch → history)
4. Installation (prerequisites, build commands, headless mode, Guix)
5. Commands (show, copy, watch, history, config)
6. Configuration (table of all fields, config file path, init command, max_age format)
7. Clipboard Sync (CLIPBOARD vs PRIMARY, 4 modes)
8. File Paths (config, database)
9. License (if applicable)
```

## Section Content Guidelines

### 1. Title + One-liner
- `# clio`
- One sentence: "A clipboard manager for Linux with SQLite history and GTK4 UI."

### 2. Overview
- What: monitors clipboard, stores entries (text + images) in SQLite
- Key features: deduplication, search, auto-expiration, image support
- Platform: Linux (X11/Wayland)

### 3. Quick Start
- Build: `cargo build --release`
- Watch: `clio watch &`
- History: `clio history`
- Minimal — just enough to get running

### 4. Installation
- Prerequisites: Rust toolchain, GTK4 dev libs, pkg-config
- Full build: `cargo build --release`
- Headless: `cargo build --release --no-default-features`
- Guix: `guix shell -m manifest.scm -- cargo build --release`

### 5. Commands
Each command gets: heading, one-line description, usage example(s)
- `clio show` — print current clipboard to stdout
- `clio copy` — read stdin, write to clipboard + save to history
- `clio watch` — poll clipboard, save changes to DB
- `clio history` — GTK4 window with search, scroll, keyboard shortcuts
- `clio config show|init|validate|path` — config management subcommands

### 6. Configuration
- Path: `~/.config/clio/config.yaml`
- Create default: `clio config init`
- Full table from research.md (11 fields)
- Duration format note for `max_age`

### 7. Clipboard Sync
- Brief explanation of CLIPBOARD (Ctrl+C/V) vs PRIMARY (mouse selection)
- Table of 4 modes with descriptions

### 8. File Paths
- Config: `~/.config/clio/config.yaml`
- Database: `~/.local/share/clio/clio.db`

## Mapping to Spec Requirements

| Requirement | README Section |
|-------------|---------------|
| FR-001 (title, overview) | §1, §2 |
| FR-002 (build prerequisites) | §4 |
| FR-003 (quick start) | §3 |
| FR-004 (CLI commands) | §5 |
| FR-005 (config table) | §6 |
| FR-006 (max_age format) | §6 |
| FR-007 (keyboard shortcuts) | §5 (history) |
| FR-008 (file paths) | §8 |
| FR-009 (text + images) | §2, §5 |
| FR-010 (sync modes) | §7 |
