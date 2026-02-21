# Tasks: Clipboard Manager

**Input**: Design documents from `/specs/001-clipboard-manager/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli.md

**Tests**: Included per constitution Principle VI (Test Discipline):
unit tests for all public modules, integration tests for CLI commands.

**Organization**: Tasks grouped by user story (7 stories from spec.md).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1–US7 maps to user stories from spec.md

## Phase 1: Setup

**Purpose**: Project initialization and module structure

- [X] T001 Initialize Rust project with `cargo init --name clio` and
  configure Cargo.toml with all dependencies: clap (derive feature),
  arboard (wayland-data-control feature), rusqlite (bundled feature),
  rusqlite_migration, gtk4, serde + serde_yaml, blake3, image,
  directories, chrono, thiserror, anyhow in Cargo.toml
- [X] T002 Create module directory structure per plan.md: src/cli/,
  src/db/, src/clipboard/, src/config/, src/models/, src/ui/ with
  empty mod.rs files and tests/integration/, tests/unit/ directories
- [X] T003 [P] Configure clippy (clippy.toml or Cargo.toml
  `[lints.clippy]`) and rustfmt (rustfmt.toml with edition=2021)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user
story can be implemented

**CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 Define application error types using thiserror in
  src/errors.rs — variants: ClipboardError, DatabaseError,
  ConfigError, IoError. Re-export from src/lib.rs or src/main.rs
- [X] T005 [P] Implement Configuration struct with serde defaults in
  src/config/types.rs — all fields with `#[serde(default)]`,
  Default trait impl with values from data-model.md (max_history=500,
  watch_interval_ms=500, max_entry_size_kb=51200, window 600x400).
  Implement config loading stub in src/config/mod.rs that returns
  Config::default() (file loading deferred to US7)
- [X] T006 [P] Implement ClipboardEntry domain model in
  src/models/entry.rs — struct with all fields from data-model.md,
  ContentType enum (Text/Image/Unknown), `compute_hash()` method
  using blake3 on content bytes, `from_text()`/`from_image()` factory
  methods. Image helper: encode raw RGBA to PNG bytes using `image`
  crate
- [X] T007 Implement database initialization in src/db/mod.rs —
  `init_db(path)` function that opens rusqlite Connection, sets
  PRAGMAs (journal_mode=WAL, synchronous=NORMAL, busy_timeout=5000,
  foreign_keys=ON), returns Connection. Auto-create parent directories
  using std::fs::create_dir_all
- [X] T008 Implement schema migrations in src/db/migrations.rs —
  define V1 migration SQL from data-model.md using
  rusqlite_migration::Migrations, `run_migrations(&mut conn)` function
- [X] T009 Implement repository CRUD in src/db/repository.rs —
  functions: `insert_entry(conn, entry) -> Result<i64>`,
  `find_by_hash(conn, hash) -> Result<Option<Entry>>`,
  `update_timestamp(conn, id) -> Result<()>`,
  `list_entries(conn, limit) -> Result<Vec<Entry>>`,
  `delete_entry(conn, id) -> Result<()>`,
  `prune_oldest(conn, max_count) -> Result<u64>`,
  `save_or_update(conn, entry, max_history) -> Result<i64>` (combined
  dedup + insert + prune logic per state transitions in data-model.md)
- [X] T010 [P] Implement clipboard abstraction in src/clipboard/mod.rs —
  `read_clipboard() -> Result<ClipboardContent>` (returns enum
  Text(String)/Image{width,height,rgba_bytes}/Empty),
  `write_clipboard_text(text) -> Result<()>`,
  `write_clipboard_image(rgba, width, height) -> Result<()>` using
  arboard::Clipboard
- [X] T011 Define CLI skeleton with clap derive in src/cli/mod.rs —
  `#[derive(Parser)] struct Cli` with global --config option,
  `#[derive(Subcommand)] enum Commands { Show, Copy, Watch, History }`.
  Wire up in src/main.rs: parse args, load config, init DB, dispatch
  to placeholder handlers
- [X] T012 [P] Write unit tests for foundational modules:
  tests/unit/models_test.rs (hash computation, entry creation,
  PNG encoding), tests/unit/db_test.rs (init DB with in-memory
  SQLite, migrations, all repository CRUD operations, dedup logic,
  pruning), tests/unit/config_test.rs (default values, serde
  deserialization)

**Checkpoint**: Foundation ready — `cargo test` passes, `cargo clippy`
clean, binary compiles with `--help` working for all subcommands

---

## Phase 3: User Story 1 — Read Current Clipboard (Priority: P1)

**Goal**: `clio show` prints clipboard contents to stdout

**Independent Test**: Copy text in any app, run `clio show`, verify
output matches

- [X] T013 [US1] Implement `clio show` command handler in
  src/cli/show.rs — read clipboard via clipboard module, match on
  content type: Text → print to stdout, Image → print summary line
  `Image: {w}x{h} PNG ({size_kb} KB)`, Empty → eprintln error + exit
  code 1. Wire into CLI dispatch in src/main.rs
- [X] T014 [US1] Write integration test for `clio show` in
  tests/integration/cli_show.rs — test exit code 0 for text clipboard,
  test summary output format for image, test exit code 1 for empty
  clipboard (use assert_cmd crate for binary testing)

**Checkpoint**: `clio show` works for text and images

---

## Phase 4: User Story 2 — Set Clipboard from stdin (Priority: P1)

**Goal**: `echo "text" | clio copy` sets clipboard and saves to history

**Independent Test**: Pipe text, paste in another app, verify match

- [X] T015 [US2] Implement `clio copy` command handler in
  src/cli/copy.rs — read stdin to string, validate non-empty, compute
  hash, call `save_or_update()` on DB, set clipboard via clipboard
  module. Error on empty stdin with exit code 1
- [X] T016 [US2] Write integration test for `clio copy` in
  tests/integration/cli_copy.rs — test piping text sets clipboard
  (verify via `clio show`), test empty stdin returns exit code 1,
  test entry appears in DB after copy

**Checkpoint**: `echo "hello" | clio copy && clio show` outputs "hello"

---

## Phase 5: User Story 3 — Watch Clipboard (Priority: P1)

**Goal**: `clio watch` monitors clipboard and auto-saves new entries

**Independent Test**: Start watch, copy text in browser, verify entry
in DB

- [X] T017 [US3] Implement source app detection (best-effort) in
  src/clipboard/source_app.rs — on X11: use x11rb to query
  XGetSelectionOwner + WM_CLASS via XFixes. Return Option<String>.
  On Wayland or failure: return None. Add x11rb to Cargo.toml
  dependencies
- [X] T018 [US3] Implement `clio watch` command handler in
  src/cli/watch.rs — polling loop: sleep for watch_interval_ms, read
  clipboard, compare hash with last seen hash (in-memory), if changed:
  attempt source_app detection, create entry, call `save_or_update()`.
  Signal handling: register SIGINT/SIGTERM via ctrlc crate or
  std::sync::atomic flag, break loop on signal. Add ctrlc to
  Cargo.toml if used
- [X] T019 [US3] Implement max_entry_size_kb check in watch loop —
  skip entries whose content size exceeds configured limit (log
  skip to stderr)
- [X] T020 [US3] Write integration test for `clio watch` in
  tests/integration/cli_watch.rs — test: start watch in background
  thread, programmatically set clipboard, sleep, verify DB has entry,
  send SIGINT, verify clean exit

**Checkpoint**: `clio watch` saves clipboard changes to DB, stops on
Ctrl+C

---

## Phase 6: User Story 4 — Browse and Select from History (Priority: P2)

**Goal**: `clio history` opens GTK window, user selects entry to paste

**Independent Test**: Copy several items with watch running, open
history, select older entry, paste — verify match

- [X] T021 [US4] Implement GObject wrapper for ClipboardEntry in
  src/ui/entry_object.rs — glib::Object subclass with properties:
  id (i64), preview_text (String), content_type (String),
  created_at (String), thumbnail (Option<gdk::Texture>). Implement
  glib::Properties derive
- [X] T022 [US4] Implement list item factory in src/ui/entry_row.rs —
  SignalListItemFactory with setup signal: horizontal gtk::Box
  containing Image (48px thumbnail) + vertical Box (Label preview +
  Label meta). Use property expressions for binding
- [X] T023 [US4] Implement history window in src/ui/window.rs — create
  undecorated gtk::Window (configurable width/height from config),
  populate gio::ListStore from DB via gio::spawn_blocking +
  async_channel, create SingleSelection + ListView with factory from
  T022. Handle activate signal (Enter/click): set clipboard to
  selected entry content, update timestamp in DB, close window.
  Handle Escape via action + accelerator: close window
- [X] T024 [US4] Implement GTK application setup in src/ui/mod.rs —
  `run_history_window(config, db_path)` function that initializes
  gtk::Application, creates window from T023, runs main loop.
  Wire into `clio history` handler in src/cli/history.rs
- [X] T025 [US4] Add `get_entry_content(conn, id) -> Result<Content>`
  to src/db/repository.rs for retrieving full content (text or blob)
  by entry id — needed for setting clipboard on selection

**Checkpoint**: `clio history` shows scrollable list, Enter selects
and pastes

---

## Phase 7: User Story 5 — Filter History (Priority: P2)

**Goal**: Type-to-filter in history window narrows displayed entries

**Independent Test**: Open history with many entries, type filter text,
verify only matching entries shown

- [X] T026 [US5] Add SearchEntry and FilterListModel to history window
  in src/ui/window.rs — insert gtk::SearchEntry above ListView, call
  `set_key_capture_widget(list_view)` so keystrokes go to search field.
  Wrap ListStore in FilterListModel with CustomFilter that does
  case-insensitive substring match on preview_text property.
  Connect `search-changed` signal to `filter.changed()`
- [X] T027 [US5] Handle image entries in filter — images with no text
  are hidden when filter is active, shown when filter is cleared.
  CustomFilter returns true for image entries only when filter string
  is empty

**Checkpoint**: Typing in history window filters entries in real time

---

## Phase 8: User Story 6 — Delete History Entry (Priority: P3)

**Goal**: Delete key removes selected entry from history and DB

**Independent Test**: Open history, delete entry, reopen — entry is gone

- [X] T028 [US6] Add EventControllerKey to ListView in
  src/ui/window.rs — on Delete key press: get selected item from
  SingleSelection, call `delete_entry(conn, id)` in DB, remove item
  from ListStore at selected position. Show placeholder label when
  list becomes empty
- [X] T029 [US6] Add `delete_entry_by_id` helper that combines DB
  delete + ListStore removal — integrate with the async_channel
  pattern from T023 for thread-safe DB access

**Checkpoint**: Delete key removes entries, empty list shows placeholder

---

## Phase 9: User Story 7 — Persistent Configuration (Priority: P3)

**Goal**: YAML config file at XDG config path overrides defaults

**Independent Test**: Create config with custom max_history, verify
pruning behavior

- [X] T030 [US7] Implement config file loading in src/config/mod.rs —
  `load_config(override_path: Option<&Path>) -> Result<Config>`:
  resolve XDG config path via directories crate, if file exists read
  and deserialize with serde_yaml, merge with defaults. If file has
  invalid YAML: return ConfigError with clear message. If file absent:
  return Config::default(). Wire into src/main.rs replacing the
  hardcoded default from T005
- [X] T031 [US7] Implement XDG path resolution in src/config/mod.rs —
  `config_dir() -> PathBuf` and `data_dir() -> PathBuf` using
  directories::ProjectDirs. Auto-create directories. Support --config
  CLI override
- [X] T032 [US7] Update unit tests in tests/unit/config_test.rs — test
  loading valid YAML with overrides, test missing file returns defaults,
  test invalid YAML returns descriptive error, test --config path
  override

**Checkpoint**: Config file overrides defaults, invalid YAML shows
clear error

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Final quality pass

- [X] T033 Run `cargo clippy -- -W clippy::all` and fix all warnings
  across all source files
- [X] T034 Run `cargo fmt --check` and fix all formatting issues
- [X] T035 [P] Run full test suite `cargo test` — verify all unit and
  integration tests pass
- [X] T036 [P] Validate quickstart.md scenarios end-to-end: build
  release binary, run `clio show`, `echo test | clio copy`, `clio
  watch` + copy in app + `clio history`
- [X] T037 Review error messages for all failure paths: empty clipboard,
  empty stdin, invalid config, DB lock, disk full — ensure they are
  user-friendly and printed to stderr

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all stories
- **US1 (Phase 3)**: Depends on Phase 2
- **US2 (Phase 4)**: Depends on Phase 2
- **US3 (Phase 5)**: Depends on Phase 2
- **US4 (Phase 6)**: Depends on Phase 2 (and benefits from US3 to
  have entries in DB)
- **US5 (Phase 7)**: Depends on US4 (extends the GTK window)
- **US6 (Phase 8)**: Depends on US4 (extends the GTK window)
- **US7 (Phase 9)**: Depends on Phase 2 (replaces config stub)
- **Polish (Phase 10)**: Depends on all stories being complete

### User Story Dependencies

- **US1, US2, US3**: Independent of each other, all depend only on
  Phase 2. Can start in parallel after foundational is complete
- **US4**: Independent of US1–US3 for implementation, but needs DB
  entries (from US2/US3) for meaningful testing
- **US5, US6**: Depend on US4 (they modify the same GTK window code)
- **US7**: Independent of all other stories

### Within Each User Story

- Models/repository before command handlers
- Command handlers before integration tests
- Core logic before edge case handling

### Parallel Opportunities

- T002 and T003: parallel (different files)
- T005, T006, T010: parallel (independent modules)
- T013 and T014 within US1: sequential (test after impl)
- US1, US2, US3: all parallel after Phase 2
- US5 and US6: parallel after US4 (different keyboard handlers)
- US7: parallel with US4–US6

---

## Parallel Example: After Phase 2

```text
# These three user stories can start simultaneously:
Agent A: T013-T014 (US1: clio show)
Agent B: T015-T016 (US2: clio copy)
Agent C: T017-T020 (US3: clio watch)

# After US4 completes, these can run in parallel:
Agent D: T026-T027 (US5: filter)
Agent E: T028-T029 (US6: delete)

# US7 can run any time after Phase 2:
Agent F: T030-T032 (US7: configuration)
```

---

## Implementation Strategy

### MVP First (US1 + US2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: US1 (clio show)
4. Complete Phase 4: US2 (clio copy)
5. **STOP and VALIDATE**: `echo "test" | clio copy && clio show`
6. Functional CLI clipboard tool usable in shell scripts

### Core Product (Add US3 + US4)

7. Complete Phase 5: US3 (clio watch)
8. Complete Phase 6: US4 (clio history window)
9. **VALIDATE**: Start watch, copy items, browse history
10. Full clipboard manager experience

### Complete Product (Add US5–US7 + Polish)

11. Complete Phase 7: US5 (filter)
12. Complete Phase 8: US6 (delete)
13. Complete Phase 9: US7 (configuration)
14. Complete Phase 10: Polish
15. **VALIDATE**: Run quickstart.md end-to-end

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Constitution mandates: clippy clean, rustfmt clean, unit tests for
  public modules, integration tests for CLI surface
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
