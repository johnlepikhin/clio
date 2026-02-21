# Quickstart: Config Management Subcommands

## Prerequisites

- Rust stable toolchain
- Guix shell: `guix shell -m manifest.scm`

## Build & Test

```bash
guix shell -m manifest.scm -- cargo build
guix shell -m manifest.scm -- cargo test
guix shell -m manifest.scm -- cargo clippy
```

## Usage

### Initialize a config file

```bash
# Create default config at ~/.config/clio/config.yaml
clio config init

# Overwrite existing config
clio config init --force

# Create at custom path
clio --config /tmp/clio.yaml config init
```

### View current config

```bash
# Show effective config (defaults merged with file)
clio config show

# Pipe to file
clio config show > /tmp/backup.yaml
```

### Validate config

```bash
# Validate default config file
clio config validate

# Validate specific file
clio --config /tmp/clio.yaml config validate
```

### Show config path

```bash
# Print where clio looks for config
clio config path
```

## Key files

| File                    | Purpose                                      |
|-------------------------|----------------------------------------------|
| `src/cli/mod.rs`        | CLI struct + `ConfigCommands` enum            |
| `src/cli/config.rs`     | Config subcommand handlers (new)              |
| `src/config/types.rs`   | `Config` struct (add `Serialize`, methods)    |
| `src/config/mod.rs`     | `default_config_path()`, `load_config()`      |
| `src/main.rs`           | Command dispatch (add `Config` branch)        |
