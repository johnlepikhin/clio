# Feature Specification: Config Management Subcommands

**Feature Branch**: `002-config-subcommands`
**Created**: 2026-02-21
**Status**: Draft
**Input**: User description: "Серия сабкоманд для управления конфиг-файлом, по образцу voice-type"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - View Current Configuration (Priority: P1)

A user wants to see the effective configuration (defaults merged with their config file) to understand how clio is currently configured. They run `clio config show` and see the full YAML output on stdout.

**Why this priority**: Most common config operation — users need to inspect settings before changing them.

**Independent Test**: Run `clio config show` with and without a config file; verify YAML output contains all fields with correct values.

**Acceptance Scenarios**:

1. **Given** a config file exists with custom `max_history: 100`, **When** user runs `clio config show`, **Then** stdout contains valid YAML with `max_history: 100` and all other fields at their defaults.
2. **Given** no config file exists, **When** user runs `clio config show`, **Then** stdout contains valid YAML with all default values.
3. **Given** a custom `--config` path, **When** user runs `clio --config /tmp/my.yaml config show`, **Then** the output reflects that file's contents.

---

### User Story 2 - Initialize Default Config File (Priority: P1)

A new user wants to create a starter config file with sensible defaults and explanatory comments. They run `clio config init` and get a commented YAML file at the default config path.

**Why this priority**: Essential for onboarding — users need a config file to start customizing.

**Independent Test**: Run `clio config init`, verify file is created at the expected XDG path with valid YAML and helpful comments.

**Acceptance Scenarios**:

1. **Given** no config file exists, **When** user runs `clio config init`, **Then** a default config file is created at the XDG config path and a confirmation message is printed.
2. **Given** a config file already exists, **When** user runs `clio config init`, **Then** the command refuses to overwrite and displays an error with instructions.
3. **Given** a config file already exists, **When** user runs `clio config init --force`, **Then** the existing file is overwritten with the default config.
4. **Given** the config directory does not exist, **When** user runs `clio config init`, **Then** the directory is created automatically before writing the file.

---

### User Story 3 - Validate Configuration (Priority: P2)

A user has edited their config file and wants to verify it parses correctly and all values are within valid ranges. They run `clio config validate` and get either a success message or a list of errors.

**Why this priority**: Important for debugging but less frequently used than show/init.

**Independent Test**: Run `clio config validate` with valid and invalid config files; verify correct output and exit codes.

**Acceptance Scenarios**:

1. **Given** a valid config file, **When** user runs `clio config validate`, **Then** stdout shows "Configuration is valid." and exit code is 0.
2. **Given** a config file with invalid YAML syntax, **When** user runs `clio config validate`, **Then** an error message indicates the parse error with line number and exit code is non-zero.
3. **Given** a config file with out-of-range values (e.g., `max_history: -1`), **When** user runs `clio config validate`, **Then** an error message identifies the invalid field and exit code is non-zero.
4. **Given** no config file exists, **When** user runs `clio config validate`, **Then** a message indicates no config file was found and defaults are valid (exit code 0).

---

### User Story 4 - Show Config File Path (Priority: P3)

A user wants to know where clio looks for its config file. They run `clio config path` and see the resolved path on stdout.

**Why this priority**: Convenience utility; helps users locate the config file quickly, especially in non-standard XDG setups.

**Independent Test**: Run `clio config path` and verify the output matches the expected XDG-resolved path.

**Acceptance Scenarios**:

1. **Given** default XDG configuration, **When** user runs `clio config path`, **Then** stdout shows the absolute path to the default config file location.
2. **Given** a custom `--config` flag, **When** user runs `clio --config /tmp/my.yaml config path`, **Then** stdout shows `/tmp/my.yaml`.

---

### Edge Cases

- What happens when the config file contains unknown keys? The system ignores unknown keys and proceeds (forward compatibility).
- What happens when the config file has incorrect permissions (not readable)? The system displays a clear "permission denied" error.
- What happens when `config init --force` is used on a path where the user has no write permission? The system displays a clear I/O error.
- What happens when YAML is valid but a value has the wrong type (e.g., `max_history: "abc"`)? A type-error message identifies the field.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a `config` subcommand group under the main `clio` command.
- **FR-002**: System MUST implement `config show` — serialize the effective Config to YAML and print to stdout.
- **FR-003**: System MUST implement `config init` — write a default config file with explanatory comments to the XDG config path.
- **FR-004**: `config init` MUST refuse to overwrite an existing file unless `--force` flag is provided.
- **FR-005**: `config init` MUST create parent directories if they don't exist.
- **FR-006**: System MUST implement `config validate` — load and validate the config file, reporting errors to stderr with a non-zero exit code.
- **FR-007**: `config validate` with no config file MUST report that defaults are valid (not an error).
- **FR-008**: System MUST implement `config path` — print the resolved config file path to stdout.
- **FR-009**: All config subcommands MUST respect the global `--config` flag for custom config file paths.
- **FR-010**: The Config struct MUST support both serialization and deserialization (currently only Deserialize is derived).

### Assumptions

- The generated default config file uses YAML format with inline comments explaining each field and its default value.
- Unknown YAML keys are silently ignored (serde default behavior without `#[serde(deny_unknown_fields)]`).
- Validation checks that values parse correctly and are within reasonable ranges (e.g., positive integers where expected).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All four subcommands (`show`, `init`, `validate`, `path`) are accessible via `clio config <subcommand>` and documented in `clio config --help`.
- **SC-002**: `clio config init` creates a valid, parseable config file that `clio config validate` accepts without errors.
- **SC-003**: `clio config show` output can be piped to a file and used as a valid config file (`clio config show > /tmp/test.yaml && clio --config /tmp/test.yaml config validate`).
- **SC-004**: Invalid config files produce actionable error messages that identify the problematic field or syntax error.
