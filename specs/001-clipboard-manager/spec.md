# Feature Specification: Clipboard Manager

**Feature Branch**: `001-clipboard-manager`
**Created**: 2026-02-21
**Status**: Draft
**Input**: User description: "Rust clipboard manager with SQLite history,
YAML config, CLI interface (clap), GTK history window, arboard for
clipboard access"

## Clarifications

### Session 2026-02-21

- Q: How does history get populated without a daemon? → A: Add a
  `clio watch` command — a long-running process that monitors the
  system clipboard and automatically saves new entries to the database.
- Q: Duplicate content handling strategy? → A: Deduplicate by content
  — if an identical entry already exists in the history, update its
  timestamp instead of creating a new entry.
- Q: Default max history size? → A: 500 entries by default,
  overridable via `max_history` in YAML config.
- Q: Polling interval for `clio watch`? → A: 500ms by default,
  overridable via `watch_interval_ms` in YAML config.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read Current Clipboard (Priority: P1)

A user runs a CLI command to see the current clipboard contents printed
to stdout. This is the most fundamental operation — it proves the tool
can access the system clipboard and output data. For text content the
raw text is printed. For image content a summary line is printed
(e.g., dimensions and format) since raw image bytes are not useful in
a terminal.

**Why this priority**: Core clipboard access is the foundation for
every other feature. Without it nothing else works.

**Independent Test**: Run the command after copying text in any
application; verify stdout matches the copied text.

**Acceptance Scenarios**:

1. **Given** the system clipboard contains text, **When** the user
   runs `clio show`, **Then** the text content is printed to stdout
   and the process exits with code 0.
2. **Given** the system clipboard contains an image, **When** the user
   runs `clio show`, **Then** a human-readable summary of the image
   (dimensions, format) is printed to stdout.
3. **Given** the system clipboard is empty, **When** the user runs
   `clio show`, **Then** a message indicating an empty clipboard is
   printed to stderr and the process exits with a non-zero code.

---

### User Story 2 - Set Clipboard from stdin (Priority: P1)

A user pipes text into the CLI to set the clipboard contents. This
enables scripting workflows (e.g., `echo "hello" | clio copy`). The
new content is also saved as an entry in the history database.

**Why this priority**: Together with US1 this completes the basic
read/write cycle, making the tool usable in shell pipelines.

**Independent Test**: Pipe text into the command, then paste in
another application; verify the pasted text matches.

**Acceptance Scenarios**:

1. **Given** stdin contains text, **When** the user runs
   `clio copy`, **Then** the system clipboard is set to that text
   and a new history entry is created with the current timestamp.
2. **Given** stdin is empty (EOF immediately), **When** the user runs
   `clio copy`, **Then** the command prints an error to stderr and
   exits with a non-zero code without modifying the clipboard.

---

### User Story 3 - Watch Clipboard (Priority: P1)

A user starts a long-running process that monitors the system clipboard
for changes. Whenever new content appears in the clipboard (copied from
any application), the watcher automatically saves it as a new entry in
the history database. The watcher runs in the foreground by default and
can be stopped with Ctrl+C. It is intended to be launched at session
startup (e.g., via systemd user service, autostart entry, or shell
profile).

**Why this priority**: Without automatic clipboard monitoring, the
history database is only populated by explicit `clio copy` calls,
making the history window largely useless for typical clipboard
manager workflows.

**Independent Test**: Start `clio watch`, copy text in a browser,
then run `clio history` — verify the copied text appears as an entry.

**Acceptance Scenarios**:

1. **Given** `clio watch` is running, **When** the user copies text in
   any application, **Then** a new entry is created in the database
   with the text content and current timestamp.
2. **Given** `clio watch` is running, **When** the user copies an
   image in any application, **Then** a new entry is created in the
   database with the image content and current timestamp.
3. **Given** `clio watch` is running and the clipboard content matches
   an existing entry in the history, **When** the watcher detects it,
   **Then** no new entry is created; instead the existing entry's
   timestamp is updated to the current time.
4. **Given** `clio watch` is running, **When** the user presses
   Ctrl+C, **Then** the process shuts down gracefully.

---

### User Story 4 - Browse and Select from History (Priority: P2)

A user opens a GTK window that displays the clipboard history as a
scrollable list ordered by most-recent first. Text entries show a
preview of the content. Image entries show a thumbnail. The user
selects an entry (click or Enter) to set it as the current clipboard
content; the selected entry's timestamp is updated so it becomes the
newest in the history. The window closes after selection.

**Why this priority**: The history window is the primary interactive
feature and the main reason users install a clipboard manager.

**Independent Test**: Copy several items, open history window, select
an older entry, paste elsewhere — verify the pasted content matches
the selected entry.

**Acceptance Scenarios**:

1. **Given** the history contains multiple entries, **When** the user
   runs `clio history`, **Then** a GTK window opens showing entries
   ordered by timestamp descending.
2. **Given** the history window is open and an entry is focused,
   **When** the user presses Enter or clicks the entry, **Then** the
   system clipboard is set to that entry's content, the entry's
   timestamp is updated to now, and the window closes.
3. **Given** the history window is open, **When** the user presses
   Escape, **Then** the window closes without modifying the clipboard.

---

### User Story 5 - Filter History (Priority: P2)

While the history window is open, the user types characters to filter
entries in real time. Only text entries whose content contains the
typed substring (case-insensitive) remain visible. Clearing the filter
restores the full list.

**Why this priority**: Filtering makes the history usable when it
grows large. It is a key usability feature that ships together with
the history window.

**Independent Test**: Open history, type a filter string, verify only
matching entries are shown; clear the filter, verify all entries
reappear.

**Acceptance Scenarios**:

1. **Given** the history window is open with 50 entries, **When** the
   user types "foo", **Then** only entries whose text content contains
   "foo" (case-insensitive) are displayed.
2. **Given** a filter is active, **When** the user clears the input
   field, **Then** all entries are shown again.
3. **Given** a filter is active and only image entries remain hidden
   because they have no text, **When** the user clears the filter,
   **Then** all entries including images reappear.

---

### User Story 6 - Delete History Entry (Priority: P3)

While the history window is open, the user can delete a selected entry
from the history. The entry is permanently removed from the database.

**Why this priority**: Deletion is important for privacy (removing
passwords, sensitive data) but is less frequently used than browsing
and selecting.

**Independent Test**: Open history, delete an entry, close and reopen
history — verify the entry is gone.

**Acceptance Scenarios**:

1. **Given** the history window is open and an entry is focused,
   **When** the user presses the Delete key, **Then** the entry is
   removed from the database and disappears from the list.
2. **Given** the history contains one entry, **When** the user deletes
   it, **Then** the list becomes empty and a placeholder message is
   shown.

---

### User Story 7 - Persistent Configuration (Priority: P3)

The application reads its configuration from a YAML file located in
the XDG config directory. The configuration controls settings such as
maximum history size, database path override, and default behavior
flags. If the config file does not exist the application uses sensible
defaults.

**Why this priority**: Configuration is needed for customization but
the application MUST work out of the box without any config file.

**Independent Test**: Create a config file with a custom max-history
value, add more entries than the limit, verify oldest entries are
pruned.

**Acceptance Scenarios**:

1. **Given** no configuration file exists, **When** the user runs any
   command, **Then** the application works with default settings.
2. **Given** the default max history size is 500 entries (overridable
   via `max_history` in config), **When** the history exceeds the
   configured limit, **Then** the oldest entries are pruned
   automatically.
3. **Given** a config file contains invalid YAML, **When** the user
   runs any command, **Then** the application prints a clear error
   message indicating the config problem and exits with a non-zero
   code.

---

### Edge Cases

- What happens when the clipboard contains a format that is neither
  plain text nor a recognized image (e.g., rich text, file list)?
  The system stores the raw bytes with content type "unknown" and
  displays a placeholder label in the history window.
- What happens when the SQLite database file is locked by another
  process? The application retries briefly, then exits with an error
  message explaining the lock conflict.
- What happens when the database file does not exist on first run?
  The application creates the database and runs initial schema
  migrations automatically.
- What happens when disk space is exhausted while saving an entry?
  The application reports a storage error to stderr and exits with a
  non-zero code without corrupting the database.
- What happens when a very large image (e.g., 50 MB) is in the
  clipboard? The system stores it but the history window shows only
  a thumbnail; the config can set a max entry size to skip oversized
  items.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST read current clipboard content (text and
  images) and output it to stdout via a `show` subcommand.
- **FR-002**: System MUST accept text from stdin and set the system
  clipboard via a `copy` subcommand, saving the content as a history
  entry.
- **FR-003**: System MUST display a GTK window listing clipboard
  history via a `history` subcommand, ordered by timestamp descending.
- **FR-004**: System MUST set the system clipboard to a selected
  history entry when the user clicks or presses Enter on it, updating
  the entry's timestamp to the current time.
- **FR-005**: System MUST support real-time text filtering in the
  history window — only entries whose text content matches the filter
  substring (case-insensitive) are displayed.
- **FR-006**: System MUST allow deletion of individual history entries
  via the Delete key in the history window.
- **FR-007**: System MUST persist clipboard entries in a local SQLite
  database located per XDG Base Directory Specification
  (`$XDG_DATA_HOME/clio/`).
- **FR-008**: System MUST read configuration from a YAML file at
  `$XDG_CONFIG_HOME/clio/config.yaml`, falling back to defaults when
  the file is absent.
- **FR-009**: Each clipboard entry MUST store: content (text or image
  blob), content type indicator, creation timestamp, and optional
  source application name.
- **FR-010**: The entry data model MUST support extensible metadata
  flags (e.g., private/sensitive marker, time-to-live expiry).
- **FR-011**: System MUST automatically create the database and config
  directories on first run if they do not exist.
- **FR-012**: System MUST handle schema evolution via versioned
  database migrations.
- **FR-013**: The CLI MUST be implemented with `clap` and support
  `--help` and `--version` flags for all subcommands.
- **FR-014**: System MUST provide a `watch` subcommand that runs a
  long-lived process monitoring the system clipboard for changes and
  automatically saving new entries to the database.
- **FR-015**: System MUST deduplicate by content — when new clipboard
  content matches an existing entry in the history (by content
  equality), the existing entry's timestamp MUST be updated to the
  current time instead of creating a duplicate entry. This applies to
  both the `watch` and `copy` commands.
- **FR-016**: The `watch` command MUST shut down gracefully on SIGINT
  (Ctrl+C) and SIGTERM signals.
- **FR-017**: The `watch` command MUST poll the clipboard at a
  configurable interval (default: 500ms, overridable via
  `watch_interval_ms` in YAML config).

### Key Entities

- **ClipboardEntry**: A single clipboard capture. Attributes: unique
  identifier, content (text string or image blob), content type
  (text/image/unknown), creation timestamp (updated on re-selection),
  optional source application name, extensible metadata (key-value
  flags for private marker, TTL, etc.).
- **Configuration**: Application settings loaded from YAML. Attributes:
  max history size (default: 500), watch polling interval (default:
  500ms), database path override, max entry size limit, UI preferences
  (window size). All settings MUST have sensible defaults so the
  application works without a config file.

## Assumptions

- The application targets Linux with X11 or Wayland display servers.
- GTK4 is available on the target system.
- Most commands are short-lived CLI invocations. The exception is
  `clio watch`, which is a long-running foreground process that
  monitors the clipboard. It is NOT a daemon — it runs in the
  foreground and is intended to be managed by the user's session
  (e.g., systemd user service, autostart, or shell profile).
- Source application detection is best-effort and may not be available
  on all display server configurations.
- Image entries are stored as PNG-encoded blobs.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can read and set clipboard content via CLI in
  under 1 second end-to-end.
- **SC-002**: The history window opens and displays up to 500 entries
  in under 2 seconds.
- **SC-003**: Filtering 500 history entries updates the displayed list
  within 200 milliseconds of the last keystroke.
- **SC-004**: Users can find and select a previous clipboard entry
  from history in under 5 seconds (open window, optional filter,
  select).
- **SC-005**: The application works correctly without any configuration
  file, using sensible defaults for all settings.
- **SC-006**: All CLI subcommands return appropriate exit codes (0 for
  success, non-zero for errors) and are usable in shell scripts.

## Out of scope

Пользователь должен иметь возможность в конфиге настроить различное поведение для различных записей, попадающих в историю.

Например, если значение было скопировано из какого-то приложения-менеджера паролей — например, определить по WM_CLASS и
так далее, — то пользователь должен иметь возможность указать для записи флаг ""приватная запись", должен иметь
возможность выставлять тайм-аут существования записи в истории.

Или, например, если вставленный текст содержит некоторую строчку, то какие-то другие правила должны срабатывать.
