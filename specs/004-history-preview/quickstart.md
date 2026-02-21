# Quickstart: History Preview & Lazy Loading

## Prerequisites

- Rust stable toolchain
- Guix shell: `guix shell -m manifest.scm`
- X11 or Wayland session

## Build & Test

```bash
guix shell -m manifest.scm -- cargo build
guix shell -m manifest.scm -- cargo test
guix shell -m manifest.scm -- cargo clippy
```

## Usage

### Default behavior

```bash
# Open history — loads first 50 entries, text truncated to 4 KB preview
clio history
```

### Configure preview size

```bash
# Create config with defaults
clio config init

# Edit ~/.config/clio/config.yaml:
#   preview_text_bytes: 8192    # show up to 8 KB of text
#   history_page_size: 100      # load 100 entries per page

# Verify config
clio config validate

# Show effective config
clio config show
```

### Test scenarios

```bash
# 1. Text preview truncation:
#    - Copy a very long text (> 4 KB) to clipboard
#    - Run clio watch in background, then clio history
#    - Verify the long entry shows truncated text with "…" at the end

# 2. Multiline text:
#    - Copy multiline text (e.g., a code snippet)
#    - Open clio history
#    - Verify lines are shown with line breaks, not just first line

# 3. Lazy loading:
#    - Ensure 200+ entries in history
#    - Open clio history
#    - Verify only 50 entries initially
#    - Scroll down — verify more entries load

# 4. Filtering with lazy loading:
#    - With 200+ entries, type a filter query
#    - Verify results come from the entire database, not just loaded entries
```

## Key files

| File | Purpose |
|------|---------|
| `src/config/types.rs` | `preview_text_bytes` + `history_page_size` config fields |
| `src/db/repository.rs` | `list_entries_page()` + `search_entries_page()` queries |
| `src/ui/window.rs` | Paginated loading, scroll-to-load, DB-side filtering |
| `src/ui/entry_row.rs` | Multiline text preview with truncation + ellipsis |
