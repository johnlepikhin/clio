# Research: User-Facing README Documentation

**Date**: 2026-02-22

## Content Inventory

All data below extracted directly from the codebase (v0.2.0) to ensure README accuracy.

### CLI Commands (from `clio --help`)

| Command | Description |
|---------|-------------|
| `show` | Show current clipboard content |
| `copy` | Copy stdin to clipboard |
| `watch` | Watch clipboard for changes |
| `history` | Open history window (requires `ui` feature) |
| `config` | Configuration management (`show`, `init`, `validate`, `path`) |

Global options: `--config <PATH>`, `--help`, `--version`

### Configuration Fields (from `clio config show`)

| Field | Default | Description |
|-------|---------|-------------|
| `max_history` | `500` | Maximum number of clipboard entries retained |
| `watch_interval_ms` | `500` | Polling interval in milliseconds |
| `db_path` | `null` (auto) | Custom SQLite DB path; auto = `~/.local/share/clio/clio.db` |
| `max_entry_size_kb` | `51200` | Max entry size in KB (50 MB) |
| `window_width` | `600` | History window width (px) |
| `window_height` | `400` | History window height (px) |
| `sync_mode` | `both` | Clipboard sync mode |
| `preview_text_chars` | `4096` | Max characters in history preview |
| `history_page_size` | `50` | Entries per page in history |
| `image_preview_max_px` | `640` | Max thumbnail dimension (px) |
| `max_age` | none | Entry expiration (e.g., `30s`, `90m`, `12h`, `30d`) |

### Sync Modes (from `src/config.rs`)

- `both` — sync CLIPBOARD ↔ PRIMARY in both directions (default)
- `to-clipboard` — copy PRIMARY selections to CLIPBOARD
- `to-primary` — copy CLIPBOARD to PRIMARY
- `disabled` — monitor CLIPBOARD only, ignore PRIMARY

### Build Requirements (from `Cargo.toml` + `manifest.scm`)

- Rust stable toolchain (edition 2021)
- GTK4 dev libraries (for UI feature)
- pkg-config
- GNU Guix alternative: `guix shell -m manifest.scm`

### Feature Flags (from `Cargo.toml`)

- `ui` (default) — enables GTK4 history window
- `--no-default-features` — headless build, no GTK4 required

### File Paths (XDG)

- Config: `~/.config/clio/config.yaml`
- Database: `~/.local/share/clio/clio.db`

### History Window Keyboard Shortcuts (from `src/ui/window.rs`)

- **Enter** / click — restore entry to clipboard, close window
- **Delete** — delete selected entry
- **Escape** — close window
- Type to filter — real-time text search

### Content Types Supported

- Text (plain text)
- Images (PNG-encoded RGBA; CLIPBOARD only, PRIMARY is text-only)

## Decisions

### README Structure

- **Decision**: Use a flat structure with sections: intro → quick start → installation → commands → configuration → sync modes → file paths
- **Rationale**: Follows convention for CLI tool READMEs; quick start early to hook users; detailed reference later
- **Alternatives considered**: Separate docs/ directory with multiple pages — rejected as overkill for a single-binary tool

### Language

- **Decision**: English
- **Rationale**: Code and docs convention per CLAUDE.md; wider audience
