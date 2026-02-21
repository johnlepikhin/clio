# Quickstart: History UX & Image Previews

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
# Open history — list is focused, start typing to filter
clio history
```

### Configure image preview size

```bash
# Edit ~/.config/clio/config.yaml:
#   image_preview_max_px: 200    # show images up to 200px on longest side

# Verify config
clio config validate

# Show effective config
clio config show
```

### Test scenarios

```bash
# 1. Type-to-filter with list focus:
#    - Run clio history
#    - DO NOT click anything — start typing
#    - Verify filter text appears in search field at top
#    - Verify entries are filtered
#    - Click search field, edit text — verify filtering updates

# 2. Escape closes from anywhere:
#    - Open clio history
#    - Press Escape with list focused — window closes
#    - Reopen, click search field to focus it
#    - Press Escape — window closes

# 3. Larger image previews:
#    - Copy a large image (e.g., screenshot) to clipboard
#    - Let clio watch save it
#    - Open clio history
#    - Verify image is displayed at up to 320px (not tiny 48px icon)

# 4. Small image at original size:
#    - Copy a small image (e.g., 64x64 icon) to clipboard
#    - Open clio history
#    - Verify it displays at original size, not upscaled

# 5. Custom max size:
#    - Set image_preview_max_px: 150 in config
#    - Open clio history
#    - Verify large images are capped at 150px on longest side
```

## Key files

| File | Purpose |
|------|---------|
| `src/config/types.rs` | `image_preview_max_px` config field |
| `src/ui/window.rs` | Focus management, escape handling, thumbnail scaling |
| `src/ui/entry_row.rs` | Remove fixed pixel_size(48) |
