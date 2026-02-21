# Feature Specification: User-Facing README Documentation

**Feature Branch**: `007-readme-docs`
**Created**: 2026-02-22
**Status**: Draft
**Input**: User description: "Необходимо написать пользовательскую документацию о проекте в файле README.md"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - New User Learns What Clio Is and Installs It (Priority: P1)

A potential user discovers the clio repository and wants to quickly understand what the project does, whether it fits their needs, and how to get it running.

**Why this priority**: Without a clear introduction and installation guide, no one can start using the project. This is the entry point for all users.

**Independent Test**: Can be tested by showing the README to a person unfamiliar with the project and confirming they can explain what clio does and build it within 5 minutes of reading.

**Acceptance Scenarios**:

1. **Given** a user opens the repository page, **When** they read the README, **Then** they understand that clio is a clipboard manager for Linux with SQLite history and GTK4 UI within the first 2-3 sentences.
2. **Given** a user wants to install clio, **When** they follow the installation section, **Then** they find clear build instructions for both full (with GTK4) and headless modes.
3. **Given** a user has built clio, **When** they look for how to start, **Then** they find a "Quick Start" section showing the minimal steps to begin using clio (`clio watch` + `clio history`).

---

### User Story 2 - User Discovers All Available Commands (Priority: P1)

A user who has installed clio wants to understand the full set of commands and what each one does.

**Why this priority**: Equal to P1 because a CLI tool is useless without command documentation. Users need a reference for all subcommands.

**Independent Test**: Can be tested by asking a user to perform specific tasks (watch clipboard, view history, show current clipboard, copy from stdin) using only the README as a guide.

**Acceptance Scenarios**:

1. **Given** a user reads the commands section, **When** they look for how to start the background watcher, **Then** they find the `clio watch` command with a brief explanation.
2. **Given** a user reads the commands section, **When** they look for how to browse history, **Then** they find the `clio history` command with keyboard shortcuts documented.
3. **Given** a user reads the commands section, **When** they want to pipe text into the clipboard, **Then** they find examples for `clio copy` with stdin.
4. **Given** a user reads the commands section, **When** they want to see current clipboard contents, **Then** they find the `clio show` command.
5. **Given** a user reads the commands section, **When** they want to manage configuration, **Then** they find `clio config` subcommands (`show`, `init`, `validate`, `path`).

---

### User Story 3 - User Configures Clio (Priority: P2)

A user wants to customize clio's behavior — change polling interval, set entry expiration, adjust history window size, or configure clipboard sync mode.

**Why this priority**: Configuration is important but not blocking — clio works with sensible defaults. Users typically configure after initial usage.

**Independent Test**: Can be tested by asking a user to change a specific setting (e.g., set max_age to 30 minutes) using only the README.

**Acceptance Scenarios**:

1. **Given** a user reads the configuration section, **When** they look for available options, **Then** they find a table of all config fields with defaults and descriptions.
2. **Given** a user wants to set up entry expiration, **When** they read the config reference, **Then** they find the `max_age` option with example values (`30m`, `12h`, `30d`).
3. **Given** a user wants to know where the config file lives, **When** they read the configuration section, **Then** they find the default path and how to create it with `clio config init`.

---

### User Story 4 - User Understands Clipboard Sync Modes (Priority: P3)

A Linux user wants to understand and configure how clio handles CLIPBOARD and PRIMARY selections.

**Why this priority**: This is a Linux-specific power-user feature. Most users will use the default (`both`) without changes.

**Independent Test**: Can be tested by asking a Linux user to explain the four sync modes after reading the section.

**Acceptance Scenarios**:

1. **Given** a user reads the sync section, **When** they look for sync mode options, **Then** they find all four modes (`both`, `to-clipboard`, `to-primary`, `disabled`) with brief explanations of each.

---

### Edge Cases

- What happens if a user tries to build without GTK4 libraries installed? The README should mention the `--no-default-features` headless build option.
- What happens if the config file doesn't exist? The README should note that clio works with built-in defaults and mention `clio config init`.
- What about non-Linux platforms? The README should state Linux as the primary target platform.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: README MUST contain a project title, one-line description, and a brief (2-3 sentence) overview explaining what clio does.
- **FR-002**: README MUST include build prerequisites (Rust toolchain, GTK4 dev libraries) and build commands for both full and headless modes.
- **FR-003**: README MUST include a "Quick Start" section with minimal steps: build, start watcher, open history.
- **FR-004**: README MUST document all user-facing CLI commands (`show`, `copy`, `watch`, `history`, `config show/init/validate/path`) with brief descriptions and usage examples.
- **FR-005**: README MUST include a configuration reference table listing all config fields, their defaults, and descriptions.
- **FR-006**: README MUST document the `max_age` duration format with examples (`30s`, `90m`, `12h`, `30d`).
- **FR-007**: README MUST document keyboard shortcuts for the history window (Enter to restore, Delete to remove, Escape to close, type to filter).
- **FR-008**: README MUST mention default file paths (config and database).
- **FR-009**: README MUST mention that clio supports both text and image clipboard content.
- **FR-010**: README MUST briefly explain clipboard sync modes for Linux (CLIPBOARD vs PRIMARY) and list all four options.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A new user can understand what clio does and build it by reading only the README, without consulting source code.
- **SC-002**: All CLI subcommands are documented — running `clio --help` produces no subcommand absent from the README.
- **SC-003**: All configuration options are documented — every field in the default config YAML is present in the README's config table.
- **SC-004**: A user can set up `clio watch` and open `clio history` by following the Quick Start section in under 5 minutes.

## Assumptions

- Target audience is Linux users familiar with the terminal and building Rust projects.
- README is written in English (code and docs language per project convention).
- README uses standard GitHub-flavored Markdown.
- No badges, CI status, or contribution guidelines are needed at this stage — focus is on user documentation.
- GNU Guix build environment (`guix shell -m manifest.scm`) is mentioned as an alternative for Guix users.
