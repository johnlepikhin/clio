# clio

A clipboard manager for Linux with SQLite history and GTK4 UI.

Clio monitors your clipboard in the background, storing text and image entries in a local SQLite database. It deduplicates content, supports search and auto-expiration, and provides a GTK4 window for browsing and restoring past clipboard entries. Works on Linux with X11 and Wayland.

## Features

- Text and image clipboard history with SQLite storage
- Content deduplication by hash
- GTK4 history browser with search, thumbnails, and infinite scroll
- Clipboard sync between PRIMARY and CLIPBOARD selections
- Action rules: auto-expire secrets, strip tracking params, transform text with external commands
- Auto-expiration of old entries
- Headless mode (no GTK4 dependency) for servers and scripts
- Lightweight: the background watcher (`clio watch`) uses ~1.7 MB of private memory (heap + stack), ~10 MB PSS total including shared GTK4/glib libraries

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

Options:

| Flag | Description |
|------|-------------|
| `--ttl <DURATION>` | Auto-expire the entry after this duration (e.g. `30s`, `5m`, `1h`) |
| `--mask-with <TEXT>` | Display this text instead of real content in history UI |

```bash
echo "secret" | clio copy --ttl 30s --mask-with "••••••"
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
| `actions` | `[]` | Action rules for matching entries (see [Action Rules](#action-rules)) |

### Duration format for `max_age` and `ttl`

The `max_age` field accepts human-readable durations:

- `30s` — 30 seconds
- `90m` — 90 minutes
- `12h` — 12 hours
- `30d` — 30 days

Omit `max_age` to keep entries forever.

## Action Rules

Action rules let you automatically process clipboard entries that match certain conditions. Each rule has a name, conditions (matched with AND logic), and actions to apply.

### Conditions

| Field | Description |
|-------|-------------|
| `source_app` | Exact match on the application that owns the clipboard (X11 only) |
| `content_regex` | Regex match against text content (image entries are skipped) |
| `source_title_regex` | Regex match against the window title of the source application (X11 only) |

All conditions are optional, but at least one is required. When multiple are present, all must match.

### Actions

| Field | Default | Description |
|-------|---------|-------------|
| `ttl` | none | Auto-expire the entry after this duration (e.g. `30s`, `5m`) |
| `command` | none | External command that transforms the text via stdin/stdout |
| `command_timeout` | `5s` | Kill the command if it exceeds this duration |
| `mask_with` | none | Display this text instead of real content in history UI |

When multiple rules match, TTL and `mask_with` use last-match-wins; commands chain sequentially (output of one becomes input to the next). If a command fails, the original text is preserved. The `mask_with` action only affects display — the real text is stored in the database and restored to the clipboard when the entry is selected.

### Example

```yaml
actions:
  - name: "Expire passwords quickly"
    conditions:
      source_app: "KeePassXC"
    actions:
      ttl: "30s"

  - name: "Expire API keys"
    conditions:
      content_regex: "^(sk-|ghp_|gho_|ghs_|AKIA|xox[bpas]-|glpat-)[A-Za-z0-9_\\-]+"
    actions:
      ttl: "1m"

  - name: "Strip tracking params"
    conditions:
      content_regex: "^https?://.*[?&](utm_|fbclid|gclid|msclkid)"
    actions:
      command: ["sed", "s/[?&]\\(utm_[^&]*\\|fbclid=[^&]*\\|gclid=[^&]*\\|msclkid=[^&]*\\)//g"]

  - name: "Clean trailing whitespace"
    conditions:
      content_regex: "[ \\t]+$"
    actions:
      command: ["sed", "s/[[:space:]]*$//"]

  - name: "Short-lived GitHub tokens"
    conditions:
      source_title_regex: "GitHub"
    actions:
      ttl: "2m"

  - name: "Mask passwords"
    conditions:
      source_app: "KeePassXC"
    actions:
      ttl: "30s"
      mask_with: "••••••"
```

Run `clio config init` to generate a config file with more commented-out examples.

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
