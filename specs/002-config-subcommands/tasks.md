# Tasks: Config Management Subcommands

**Input**: Design documents from `/specs/002-config-subcommands/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/cli.md

**Tests**: Not explicitly requested. Unit tests included as part of implementation tasks where constitution (Principle VI) requires them for public interfaces.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Shared infrastructure changes that all config subcommands depend on

- [x] T001 Add `Serialize` derive to `Config` struct in `src/config/types.rs` (add `use serde::Serialize` and `#[derive(Serialize)]`)
- [x] T002 Add `default_config_path()` public function to `src/config/mod.rs` that returns `config_dir().join("config.yaml")`; refactor `load_config()` to use it
- [x] T003 Add `ConfigCommands` enum with variants `Show`, `Init { force: bool }`, `Validate`, `Path` to `src/cli/mod.rs`; add `Config { command: ConfigCommands }` variant to `Commands` enum
- [x] T004 Create `src/cli/config.rs` with stub `run(config_path: &Path, command: &ConfigCommands) -> anyhow::Result<()>` dispatching to each subcommand; register module in `src/cli/mod.rs`
- [x] T005 Add `Commands::Config` dispatch in `src/main.rs` — resolve config_path (override or default), call `cli::config::run()`

**Checkpoint**: `cargo build` succeeds, `clio config show` / `init` / `validate` / `path` are recognized by clap (stubs can print "not yet implemented")

---

## Phase 2: User Story 1 — View Current Configuration (Priority: P1) 🎯 MVP

**Goal**: `clio config show` prints effective config as valid YAML to stdout

**Independent Test**: Run `clio config show` with/without config file; verify valid YAML output

### Implementation

- [x] T006 [US1] Implement `cmd_show()` in `src/cli/config.rs` — call `config::load_config()`, serialize with `serde_yaml::to_string()`, print to stdout
- [x] T007 [US1] Add unit test for Config serialization roundtrip in `src/config/mod.rs` — serialize default Config to YAML, deserialize back, assert fields match

**Checkpoint**: `clio config show` outputs valid YAML; output can be deserialized back to Config

---

## Phase 3: User Story 2 — Initialize Default Config File (Priority: P1)

**Goal**: `clio config init` creates a commented YAML config file at the XDG config path

**Independent Test**: Run `clio config init` in a clean environment, verify file created with valid YAML and comments

### Implementation

- [x] T008 [US2] Add `Config::default_yaml() -> String` method to `src/config/types.rs` — return hardcoded YAML string with inline comments documenting each field and its default value
- [x] T009 [US2] Implement `cmd_init(config_path: &Path, force: bool)` in `src/cli/config.rs` — check if file exists (error unless `--force`), create parent dirs, write `Config::default_yaml()`, print confirmation
- [x] T010 [US2] Add unit test in `src/config/mod.rs` — verify `Config::default_yaml()` parses as valid Config and produces expected default values

**Checkpoint**: `clio config init` creates file; `clio config validate` accepts it; `clio config init` without `--force` refuses to overwrite

---

## Phase 4: User Story 3 — Validate Configuration (Priority: P2)

**Goal**: `clio config validate` checks config file validity and reports errors

**Independent Test**: Run with valid, invalid, and missing config files; verify correct messages and exit codes

### Implementation

- [x] T011 [US3] Add `Config::validate() -> Result<(), Vec<String>>` method to `src/config/types.rs` — check value ranges (e.g., `watch_interval_ms > 0`, `window_width > 0`, `window_height > 0`)
- [x] T012 [US3] Implement `cmd_validate(config_path: &Path)` in `src/cli/config.rs` — handle three cases: (1) file exists and valid → "Configuration is valid.", (2) file missing → report using defaults + valid, (3) parse/validation error → report errors with `std::process::exit(1)`
- [x] T013 [US3] Add unit tests for `Config::validate()` in `src/config/mod.rs` — test valid config passes, invalid values (zero window dimensions, etc.) fail with descriptive messages

**Checkpoint**: `clio config validate` correctly reports valid/invalid configs with proper exit codes

---

## Phase 5: User Story 4 — Show Config File Path (Priority: P3)

**Goal**: `clio config path` prints the resolved config file path

**Independent Test**: Run `clio config path` with and without `--config` flag; verify correct path output

### Implementation

- [x] T014 [US4] Implement `cmd_path(config_path: &Path)` in `src/cli/config.rs` — print the path to stdout with a trailing newline

**Checkpoint**: `clio config path` outputs expected XDG path; `clio --config /tmp/x.yaml config path` outputs `/tmp/x.yaml`

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final validation across all subcommands

- [x] T015 Run `cargo clippy` and fix any warnings
- [x] T016 Run `cargo test` and ensure all tests pass
- [x] T017 Run quickstart.md validation — manually test all four commands per quickstart.md scenarios

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — T001, T002, T003 can run in parallel; T004 depends on T003; T005 depends on T004
- **User Stories (Phase 2–5)**: All depend on Setup (Phase 1) completion
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **US1 (show)**: Depends on T001 (Serialize) + T005 (dispatch). No dependency on other stories.
- **US2 (init)**: Depends on T005 (dispatch). No dependency on other stories.
- **US3 (validate)**: Depends on T005 (dispatch). No dependency on other stories.
- **US4 (path)**: Depends on T002 (default_config_path) + T005 (dispatch). No dependency on other stories.

### Parallel Opportunities

- T001, T002, T003 can run in parallel (different files)
- US1–US4 implementation tasks can proceed in parallel after Setup is complete (different functions in same file, but logically independent)

---

## Parallel Example: Setup Phase

```
# These modify different files and can run in parallel:
T001: src/config/types.rs (add Serialize)
T002: src/config/mod.rs (add default_config_path)
T003: src/cli/mod.rs (add ConfigCommands enum)
```

---

## Implementation Strategy

### MVP First (User Story 1 + 2)

1. Complete Phase 1: Setup (T001–T005)
2. Complete Phase 2: US1 — `config show` (T006–T007)
3. Complete Phase 3: US2 — `config init` (T008–T010)
4. **STOP and VALIDATE**: Both P1 stories work independently
5. Deploy/demo if ready

### Full Delivery

1. Setup → US1 (show) → US2 (init) → US3 (validate) → US4 (path) → Polish
2. Each story adds value without breaking previous stories

---

## Notes

- No new dependencies needed — all tasks use existing crates
- Total: 17 tasks (5 setup, 2 US1, 3 US2, 3 US3, 1 US4, 3 polish)
- All user stories are independent — no cross-story dependencies
- Constitution Principle VI requires unit tests for public interfaces; test tasks are included in each story
