# Quickstart: clio

## Prerequisites

- Rust stable toolchain (rustup)
- GTK4 development libraries:
  ```bash
  # Fedora
  sudo dnf install gtk4-devel

  # Ubuntu/Debian
  sudo apt install libgtk-4-dev

  # Arch
  sudo pacman -S gtk4
  ```

## Build

```bash
git clone <repo-url> clio
cd clio
cargo build --release
```

The binary is at `target/release/clio`.

## Basic Usage

### Read clipboard

```bash
# Show current clipboard text
clio show

# Pipe clipboard to another command
clio show | wc -l
```

### Write clipboard

```bash
# Set clipboard from command output
echo "Hello, world" | clio copy

# Copy file contents to clipboard
cat README.md | clio copy
```

### Start clipboard watcher

```bash
# Run in foreground (Ctrl+C to stop)
clio watch

# Run via systemd user service (recommended)
# See below for service file
```

### Browse history

```bash
# Open GTK history window
clio history
# Type to filter, Enter to select, Delete to remove, Escape to close
```

## Configuration

Create `~/.config/clio/config.yaml` (optional — defaults work fine):

```yaml
max_history: 500
watch_interval_ms: 500
```

## Autostart with systemd

Create `~/.config/systemd/user/clio-watch.service`:

```ini
[Unit]
Description=Clio Clipboard Watcher
After=graphical-session.target

[Service]
ExecStart=/usr/local/bin/clio watch
Restart=on-failure
RestartSec=5

[Install]
WantedBy=graphical-session.target
```

Enable and start:

```bash
systemctl --user enable --now clio-watch.service
```

## Verify Installation

```bash
# 1. Start watcher
clio watch &

# 2. Copy something in any app (Ctrl+C)

# 3. Check history
clio history
# → You should see the copied content in the list

# 4. Stop watcher
kill %1
```
