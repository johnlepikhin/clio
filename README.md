# clio

A clipboard manager for Linux with SQLite history and GTK4 UI.

Clio monitors your clipboard in the background, storing text and image entries in a local SQLite database. It deduplicates content, supports search and auto-expiration, and provides a GTK4 window for browsing and restoring past clipboard entries. Works on Linux with X11 and Wayland.

## Quick Start

```bash
cargo build --release
./target/release/clio watch &
./target/release/clio history
```

## Installation

**Platform**: Linux (X11/Wayland)

### Prerequisites

- [Rust](https://rustup.rs/) stable toolchain
- GTK4 development libraries
- pkg-config

On Debian/Ubuntu:

```bash
sudo apt install libgtk-4-dev pkg-config
```

### Build

```bash
cargo build --release
```

The binary is at `./target/release/clio`.

### Headless build (no GTK4)

If you only need the background watcher and CLI commands without the history window:

```bash
cargo build --release --no-default-features
```

### GNU Guix

```bash
guix shell -m manifest.scm -- cargo build --release
```

## Commands

### `clio show`

Print the current clipboard content to stdout.

```bash
clio show
```

Text content is printed as-is. For images, prints a summary line like `Image: 1920x1080 PNG (245 KB)`.

### `clio copy`

Read from stdin and write to the clipboard. The entry is also saved to history.

```bash
echo "hello" | clio copy
cat file.txt | clio copy
```

### `clio watch`

Start the background clipboard watcher. Polls the clipboard at a configurable interval and saves new entries to the database.

```bash
clio watch
```

Handles text and image content. Runs until interrupted with Ctrl+C. Duplicate content is detected by hash — re-copying the same text bumps the timestamp instead of creating a new entry.

### `clio history`

Open a GTK4 window for browsing and restoring clipboard history. Requires the `ui` feature (enabled by default).

```bash
clio history
```

**Keyboard shortcuts**:

| Key | Action |
|-----|--------|
| Enter | Restore selected entry to clipboard and close |
| Delete | Delete the selected entry from history |
| Escape | Close the window |
| Type any text | Filter entries by text content |

The history window shows text previews and image thumbnails with infinite scroll.

### `clio config`

Configuration management subcommands.

```bash
clio config show       # Print current effective configuration
clio config init       # Create default config file (use --force to overwrite)
clio config validate   # Validate config file and report errors
clio config path       # Print the resolved config file path
```

## Configuration

Config file location: `~/.config/clio/config.yaml`

All fields have sensible defaults — the config file is optional. To create one:

```bash
clio config init
```

### Options

| Field | Default | Description |
|-------|---------|-------------|
| `max_history` | `500` | Maximum number of clipboard entries to retain |
| `watch_interval_ms` | `500` | Clipboard polling interval in milliseconds |
| `db_path` | auto | Custom SQLite database path (default: `~/.local/share/clio/clio.db`) |
| `max_entry_size_kb` | `51200` | Skip entries larger than this (in KB; default is 50 MB) |
| `window_width` | `600` | History window width in pixels |
| `window_height` | `400` | History window height in pixels |
| `sync_mode` | `both` | Clipboard sync mode (see [Clipboard Sync](#clipboard-sync)) |
| `preview_text_chars` | `4096` | Maximum characters shown in history entry preview |
| `history_page_size` | `50` | Number of entries loaded per page (infinite scroll) |
| `image_preview_max_px` | `640` | Maximum thumbnail dimension in pixels (longest side) |
| `max_age` | none | Auto-expire entries older than this duration |

### Duration format for `max_age`

The `max_age` field accepts human-readable durations:

- `30s` — 30 seconds
- `90m` — 90 minutes
- `12h` — 12 hours
- `30d` — 30 days

Omit `max_age` to keep entries forever.

## Clipboard Sync

Linux has two clipboard selections:

- **CLIPBOARD** — used by Ctrl+C / Ctrl+V
- **PRIMARY** — used by mouse selection / middle-click paste

Clio can sync between them. Set `sync_mode` in the config:

| Mode | Description |
|------|-------------|
| `both` | Sync in both directions (default) |
| `to-clipboard` | Copy PRIMARY selections to CLIPBOARD |
| `to-primary` | Copy CLIPBOARD to PRIMARY |
| `disabled` | Monitor CLIPBOARD only, ignore PRIMARY |

## File Paths

| Purpose | Default Path |
|---------|-------------|
| Configuration | `~/.config/clio/config.yaml` |
| Database | `~/.local/share/clio/clio.db` |

Both paths follow the [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/latest/). Directories are created automatically on first use. Use `--config <PATH>` to override the config file location.
