# Tasks: Clipboard & Paste Buffer Synchronization

**Input**: Design documents from `/specs/003-clipboard-sync/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/cli.md

**Tests**: Not explicitly requested. Unit tests included where constitution (Principle VI) requires them for public interfaces.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Shared types and clipboard infrastructure that all user stories depend on

- [X] T001 [P] Add `SyncMode` enum with variants `ToClipboard`, `ToPrimary`, `Both`, `Disabled` to `src/config/types.rs` — derive `Debug, Clone, Serialize, Deserialize, PartialEq`, use `#[serde(rename_all = "kebab-case")]`, impl `Default` returning `Both`
- [X] T002 [P] Add `sync_mode: SyncMode` field to `Config` struct in `src/config/types.rs` — use `#[serde(default)]` on the field
- [X] T003 [P] Add `read_selection(kind: LinuxClipboardKind)` function to `src/clipboard/mod.rs` — import `GetExtLinux` and `LinuxClipboardKind` from arboard, read text/image from the specified selection; refactor existing `read_clipboard()` to call `read_selection(LinuxClipboardKind::Clipboard)`
- [X] T004 [P] Add `write_selection_text(kind: LinuxClipboardKind, text: &str)` function to `src/clipboard/mod.rs` — write text to the specified selection with `.wait()`; refactor existing `write_clipboard_text()` to call it with `LinuxClipboardKind::Clipboard`

**Checkpoint**: `cargo build` succeeds. SyncMode is parseable from YAML. Clipboard read/write functions accept a selection kind parameter.

---

## Phase 2: User Story 1 — Bidirectional Sync by Default (Priority: P1) 🎯 MVP

**Goal**: `clio watch` with default config monitors both PRIMARY and CLIPBOARD, syncs changes bidirectionally, records history from both, prevents infinite loops.

**Independent Test**: Run `clio watch` with default config; Ctrl+C text → verify middle-click works; select text → verify Ctrl+V works.

### Implementation

- [X] T005 [US1] Rewrite `src/cli/watch.rs` polling loop: replace single `last_hash` with `last_clipboard_hash` and `last_primary_hash`; each cycle read both selections, detect changes, save changed entries to history
- [X] T006 [US1] Add sync logic in `src/cli/watch.rs`: when CLIPBOARD changes and sync mode allows (`Both` or `ToPrimary`), write content to PRIMARY and update `last_primary_hash`; when PRIMARY changes and sync mode allows (`Both` or `ToClipboard`), write content to CLIPBOARD and update `last_clipboard_hash`
- [X] T007 [US1] Update `clio watch` startup message in `src/cli/watch.rs` to include sync mode: `watching clipboard (interval: {N}ms, sync: {mode})...`
- [X] T008 [US1] Add unit test for `SyncMode` serde roundtrip in `src/config/mod.rs` — verify all four values serialize/deserialize correctly in kebab-case

**Checkpoint**: `clio watch` with default config syncs both directions. No infinite loops. History records from both selections.

---

## Phase 3: User Story 2 — Configurable Sync Direction (Priority: P1)

**Goal**: Each of the four sync modes produces the correct behavior — only the configured direction(s) are active.

**Independent Test**: Set `sync_mode: disabled` in config, run `clio watch`, verify no sync happens; repeat for `to-clipboard` and `to-primary`.

### Implementation

- [X] T009 [US2] Ensure disabled mode in `src/cli/watch.rs`: when `SyncMode::Disabled`, skip PRIMARY reading entirely, only monitor CLIPBOARD (existing single-selection behavior)
- [X] T010 [US2] Ensure directional modes in `src/cli/watch.rs`: when `SyncMode::ToClipboard`, only sync PRIMARY→CLIPBOARD (not reverse); when `SyncMode::ToPrimary`, only sync CLIPBOARD→PRIMARY (not reverse); both selections are still monitored and saved to history
- [X] T011 [US2] Add unit tests for `SyncMode` default and equality in `src/config/mod.rs` — verify `SyncMode::default() == SyncMode::Both`, verify config without sync_mode field deserializes to `Both`

**Checkpoint**: All four modes produce correct directional behavior. Disabled mode is backward-compatible with pre-sync behavior.

---

## Phase 4: User Story 3 — Validate Sync Configuration (Priority: P2)

**Goal**: `clio config show/validate/init` correctly handle the sync_mode field.

**Independent Test**: Run `clio config show` and verify `sync_mode: both` appears; set invalid value, run `clio config validate`.

### Implementation

- [X] T012 [US3] Update `Config::default_yaml()` in `src/config/types.rs` — add `sync_mode: both` with a comment listing all four valid values
- [X] T013 [US3] Add unit test in `src/config/mod.rs` — verify updated `default_yaml()` parses correctly and produces `SyncMode::Both`
- [X] T014 [US3] Add unit test in `src/config/mod.rs` — verify that a YAML string with an invalid `sync_mode: invalid` fails to parse with a descriptive error

**Checkpoint**: `clio config show` displays sync_mode. `clio config init` generates template with sync_mode documented. Invalid values produce clear serde errors.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Final validation across all changes

- [X] T015 Run `cargo clippy` and fix any warnings
- [X] T016 Run `cargo test` and ensure all tests pass (existing 23 + new)
- [X] T017 Verify quickstart.md scenarios — build and manually test `clio watch` with each sync mode

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — T001–T004 can all run in parallel (different files/functions)
- **US1 (Phase 2)**: Depends on all Setup tasks (T001–T004)
- **US2 (Phase 3)**: Depends on US1 (T005–T006 establish the sync loop that US2 configures)
- **US3 (Phase 4)**: Depends on T001–T002 (SyncMode type). Independent of US1/US2.
- **Polish (Phase 5)**: Depends on all user stories

### User Story Dependencies

- **US1 (bidirectional sync)**: Depends on Setup. Core sync logic.
- **US2 (configurable direction)**: Depends on US1 (directional filtering is applied to the sync loop built in US1).
- **US3 (config validation)**: Depends on Setup only. Can run in parallel with US1/US2.

### Parallel Opportunities

- T001, T002, T003, T004 are all parallelizable (different files/sections)
- US3 (T012–T014) can run in parallel with US1/US2 (different files)

---

## Parallel Example: Setup Phase

```
# All four tasks modify different files/sections and can run in parallel:
T001: src/config/types.rs (SyncMode enum)
T002: src/config/types.rs (Config.sync_mode field) — same file but different section, sequential with T001
T003: src/clipboard/mod.rs (read_selection)
T004: src/clipboard/mod.rs (write_selection_text) — same file, sequential with T003
```

Actually T001+T002 are in same file and T003+T004 are in same file, so:
- Parallel pair A: T001+T002 (config/types.rs)
- Parallel pair B: T003+T004 (clipboard/mod.rs)
- A and B run in parallel with each other.

---

## Implementation Strategy

### MVP First (User Story 1)

1. Complete Phase 1: Setup (T001–T004)
2. Complete Phase 2: US1 — bidirectional sync (T005–T008)
3. **STOP and VALIDATE**: `clio watch` syncs both directions by default
4. Deploy/demo if ready

### Full Delivery

1. Setup → US1 (bidirectional) → US2 (directional modes) → US3 (config validation) → Polish
2. US3 can run in parallel with US1/US2 since it only touches config files

---

## Notes

- No new dependencies — uses existing arboard types
- Total: 17 tasks (4 setup, 4 US1, 3 US2, 3 US3, 3 polish)
- Key risk: infinite sync loops — addressed by dual hash tracking (R2 in research.md)
- Wayland support: best-effort via arboard's data-control protocol support
