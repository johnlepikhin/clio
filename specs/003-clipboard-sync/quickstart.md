# Quickstart: Clipboard & Paste Buffer Synchronization

## Prerequisites

- Rust stable toolchain
- Guix shell: `guix shell -m manifest.scm`
- X11 or Wayland (with data-control v2+) session

## Build & Test

```bash
guix shell -m manifest.scm -- cargo build
guix shell -m manifest.scm -- cargo test
guix shell -m manifest.scm -- cargo clippy
```

## Usage

### Default behavior (bidirectional sync)

```bash
# Start watching — both selections are synced
clio watch

# In another terminal: copy text, then middle-click paste should work
# Select text with mouse, then Ctrl+V should work
```

### Configure sync direction

```bash
# Create config with defaults
clio config init

# Edit sync_mode in ~/.config/clio/config.yaml:
#   sync_mode: to-clipboard   # PRIMARY → CLIPBOARD only
#   sync_mode: to-primary     # CLIPBOARD → PRIMARY only
#   sync_mode: both           # bidirectional (default)
#   sync_mode: disabled       # no sync

# Verify config
clio config validate

# Show effective config
clio config show
```

### Disable sync (clipboard-only mode)

```bash
# Edit config: sync_mode: disabled
# Then watch only records CLIPBOARD history, no PRIMARY monitoring
clio watch
```

## Key files

| File                     | Purpose                                          |
|--------------------------|--------------------------------------------------|
| `src/config/types.rs`    | `SyncMode` enum + `Config.sync_mode` field       |
| `src/clipboard/mod.rs`   | PRIMARY read/write functions                     |
| `src/cli/watch.rs`       | Sync logic in polling loop                       |
