# Data Model: Config Management Subcommands

## Entities

### Config (modified)

Existing struct in `src/config/types.rs`. Changes:

| Field              | Type              | Default  | Notes                      |
|--------------------|-------------------|----------|----------------------------|
| `max_history`      | `usize`           | 500      | Max entries in DB           |
| `watch_interval_ms`| `u64`             | 500      | Polling interval (ms)       |
| `db_path`          | `Option<String>`  | None     | Custom SQLite DB path       |
| `max_entry_size_kb`| `u64`             | 51200    | Max entry size (KB)         |
| `window_width`     | `i32`             | 600      | GTK window width            |
| `window_height`    | `i32`             | 400      | GTK window height           |

**Modifications required**:
- Add `Serialize` derive (currently only `Deserialize`)
- Add `Config::default_yaml() -> String` static method
- Add `Config::validate() -> Result<()>` method

### ConfigCommands (new)

New clap subcommand enum in `src/cli/mod.rs`:

| Variant    | Args          | Description                          |
|------------|---------------|--------------------------------------|
| `Show`     | —             | Print effective config as YAML       |
| `Init`     | `--force`     | Create default config file           |
| `Validate` | —             | Check config file validity           |
| `Path`     | —             | Print resolved config file path      |

## State Transitions

No state transitions. Config subcommands are stateless read/write operations on the filesystem.

## Database Changes

None. This feature does not modify the SQLite schema.
