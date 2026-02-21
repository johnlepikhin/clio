# CLI Contract: clio

**Binary name**: `clio`
**Parser**: `clap` with derive API

## Global Options

```
clio [OPTIONS] <COMMAND>

Options:
  -c, --config <PATH>   Path to config file (default: $XDG_CONFIG_HOME/clio/config.yaml)
  -h, --help            Print help
  -V, --version         Print version
```

## Subcommands

### `clio show`

Read current clipboard content and print to stdout.

```
clio show [OPTIONS]

Options:
  -h, --help    Print help
```

**Behavior**:
- Text clipboard → print raw text to stdout, exit 0
- Image clipboard → print summary line to stdout:
  `Image: {width}x{height} PNG ({size_kb} KB)`, exit 0
- Empty clipboard → print error to stderr, exit 1

**Exit codes**: 0 = success, 1 = empty/error

---

### `clio copy`

Read text from stdin and set as clipboard content.

```
clio copy [OPTIONS]

Options:
  -h, --help    Print help
```

**Behavior**:
- Read stdin until EOF
- If stdin is empty → print error to stderr, exit 1
- Set system clipboard to the text content
- Save entry to history database (with deduplication)
- Exit 0

**Exit codes**: 0 = success, 1 = empty stdin/error

---

### `clio watch`

Monitor clipboard for changes and save to history.

```
clio watch [OPTIONS]

Options:
  -h, --help    Print help
```

**Behavior**:
- Long-running foreground process
- Poll clipboard every `watch_interval_ms` (default: 500ms)
- On new content: compute hash, deduplicate, save to DB
- On SIGINT/SIGTERM: shut down gracefully
- Prune history if exceeds `max_history` after insert

**Exit codes**: 0 = clean shutdown, 1 = error

---

### `clio history`

Open GTK window with clipboard history.

```
clio history [OPTIONS]

Options:
  -h, --help    Print help
```

**Behavior**:
- Open GTK window showing entries ordered by created_at DESC
- Text entries: show first line / truncated preview
- Image entries: show thumbnail
- Type to filter (case-insensitive substring on text entries)
- Enter/click: set clipboard to selected entry, update timestamp, close
- Delete key: remove selected entry from database
- Escape: close window without action

**Exit codes**: 0 = entry selected or window closed, 1 = error

## Config File Format

**Path**: `$XDG_CONFIG_HOME/clio/config.yaml`

```yaml
# Maximum number of history entries (default: 500)
max_history: 500

# Clipboard polling interval in milliseconds (default: 500)
watch_interval_ms: 500

# Override database path (default: $XDG_DATA_HOME/clio/clio.db)
# db_path: /custom/path/to/clio.db

# Maximum entry size in KB (default: 51200 = 50MB)
# max_entry_size_kb: 51200

# Window dimensions
# window_width: 600
# window_height: 400
```
