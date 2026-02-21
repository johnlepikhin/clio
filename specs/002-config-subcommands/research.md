# Research: Config Management Subcommands

## R1: Adding `Serialize` to the existing `Config` struct

**Decision**: Add `Serialize` derive to `Config` in `src/config/types.rs`.

**Rationale**: `config show` needs to serialize the effective config to YAML. The `serde_yaml::to_string()` call requires `Serialize`. Adding the derive is trivial and has no runtime cost.

**Alternatives considered**:
- Manual `Display` impl with custom formatting — rejected, too much maintenance burden and wouldn't produce valid YAML.
- Separate "display" struct — rejected, violates YAGNI (Principle VII).

## R2: Default YAML template with comments

**Decision**: Implement a `Config::default_yaml() -> String` method that returns a hardcoded YAML string with inline comments.

**Rationale**: `serde_yaml::to_string()` cannot produce comments. A handwritten template ensures helpful documentation for new users. This is the same approach used in `voice-type`.

**Alternatives considered**:
- Serialize default + post-process to inject comments — fragile, comments would drift from field names.
- Use a templating crate — rejected, violates Principle V (minimal dependencies).

## R3: Config validation approach

**Decision**: Attempt to load and deserialize the config file. If it parses successfully, report valid. Errors from `serde_yaml` and I/O propagate naturally via existing `AppError`.

**Rationale**: The current `Config` struct with `#[serde(default)]` already validates types at parse time. Range validation (e.g., `max_history > 0`) can be added as a `Config::validate()` method. No new dependencies needed.

**Alternatives considered**:
- `validator` crate with derive macros — rejected, adds a dependency (Principle V).
- Custom validation framework — rejected, YAGNI (Principle VII).

## R4: Config path resolution

**Decision**: Add a `config::default_config_path() -> PathBuf` public function (extracts the existing logic from `load_config`). The `config path` command prints this or the `--config` override.

**Rationale**: The path resolution logic already exists in `config::config_dir().join("config.yaml")`. Extracting it makes it reusable for both `load_config` and the new `config path` command.

## R5: No new dependencies needed

**Decision**: No new crates required. All functionality uses existing dependencies (`clap`, `serde`, `serde_yaml`, `directories`).

**Rationale**: Constitution Principle V (Minimal Dependencies) — all needed capabilities are already available.
