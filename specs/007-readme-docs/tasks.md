# Tasks: User-Facing README Documentation

**Input**: Design documents from `/specs/007-readme-docs/`
**Prerequisites**: plan.md, spec.md, research.md, quickstart.md

**Tests**: Not requested. Completeness verified manually via SC-002/SC-003.

**Organization**: Tasks write a single file (README.md) section by section, grouped by user story. Since all tasks target the same file, parallelism is limited to research within stories.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- All tasks target `README.md` at repository root

---

## Phase 1: Setup

**Purpose**: Create file scaffold with title and overview

- [x] T001 Create README.md with title, one-liner, and overview section (FR-001). Content: project name, "A clipboard manager for Linux with SQLite history and GTK4 UI", 2-3 sentence overview mentioning text+image support, deduplication, search, auto-expiration. Data source: research.md § Content Inventory, quickstart.md §1-2

---

## Phase 2: User Story 1 - New User Learns What Clio Is and Installs It (Priority: P1) MVP

**Goal**: A new user can understand clio and get it running by reading the README alone.

**Independent Test**: Show README to someone unfamiliar with the project; they should be able to explain what clio does and build it within 5 minutes.

### Implementation for User Story 1

- [x] T002 [US1] Write Quick Start section in README.md (FR-003). Three numbered steps: (1) build with `cargo build --release`, (2) start watcher `clio watch`, (3) open history `clio history`. Minimal, no explanation — just commands. Data source: quickstart.md §3
- [x] T003 [US1] Write Installation section in README.md (FR-002). Subsections: Prerequisites (Rust stable, GTK4 dev libs, pkg-config), Build (`cargo build --release`), Headless build (`cargo build --release --no-default-features`), Guix (`guix shell -m manifest.scm -- cargo build --release`). Mention Linux as primary target. Data source: research.md § Build Requirements, § Feature Flags

**Checkpoint**: User Story 1 complete — a reader can understand and install clio.

---

## Phase 3: User Story 2 - User Discovers All Available Commands (Priority: P1)

**Goal**: All CLI commands documented with descriptions and usage examples.

**Independent Test**: Ask someone to perform: show clipboard, copy from stdin, start watcher, browse history, manage config — using only the README.

### Implementation for User Story 2

- [x] T004 [US2] Write Commands section header and `clio show` subsection in README.md (FR-004). Description + example output for text and image. Mention text+image support (FR-009). Data source: research.md § CLI Commands
- [x] T005 [US2] Write `clio copy` subsection in README.md (FR-004). Description + pipe examples (`echo "text" | clio copy`, `cat file.txt | clio copy`). Data source: research.md § CLI Commands
- [x] T006 [US2] Write `clio watch` subsection in README.md (FR-004). Description, usage, mention configurable interval and sync mode. Data source: research.md § CLI Commands
- [x] T007 [US2] Write `clio history` subsection in README.md (FR-004, FR-007). Description, keyboard shortcuts table (Enter, Delete, Escape, type-to-filter), mention image thumbnails and infinite scroll. Data source: research.md § History Window Keyboard Shortcuts
- [x] T008 [US2] Write `clio config` subsection in README.md (FR-004). Document all 4 subcommands: `show`, `init [--force]`, `validate`, `path`. Brief description + example for each. Data source: research.md § CLI Commands

**Checkpoint**: User Story 2 complete — all CLI commands documented.

---

## Phase 4: User Story 3 - User Configures Clio (Priority: P2)

**Goal**: Full configuration reference with all fields, defaults, and descriptions.

**Independent Test**: Ask someone to change `max_age` to 30 minutes using only the README.

### Implementation for User Story 3

- [x] T009 [US3] Write Configuration section in README.md (FR-005, FR-006, FR-008). Include: config file path (`~/.config/clio/config.yaml`), how to create (`clio config init`), note that all fields have defaults. Write table with all 11 fields from research.md § Configuration Fields. Add note on `max_age` duration format with examples (`30s`, `90m`, `12h`, `30d`). Mention database path (`~/.local/share/clio/clio.db`). Data source: research.md § Configuration Fields, § File Paths

**Checkpoint**: User Story 3 complete — all config options documented.

---

## Phase 5: User Story 4 - User Understands Clipboard Sync Modes (Priority: P3)

**Goal**: Explain CLIPBOARD vs PRIMARY and document all sync modes.

**Independent Test**: Ask a Linux user to explain the four sync modes after reading this section.

### Implementation for User Story 4

- [x] T010 [US4] Write Clipboard Sync section in README.md (FR-010). Brief explanation of CLIPBOARD (Ctrl+C/V) vs PRIMARY (mouse selection). Table with 4 modes: `both` (default), `to-clipboard`, `to-primary`, `disabled` — each with one-line description. Data source: research.md § Sync Modes

**Checkpoint**: User Story 4 complete — sync modes documented.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final review and validation

- [x] T011 Review full README.md for completeness against spec requirements FR-001 through FR-010 and success criteria SC-001 through SC-004. Verify all 5 CLI commands, 11 config fields, 4 sync modes, and 4 keyboard shortcuts are present. Fix any gaps.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — creates file scaffold
- **Phase 2 (US1)**: Depends on Phase 1 — adds Quick Start and Installation
- **Phase 3 (US2)**: Depends on Phase 1 — adds Commands section (can run parallel to Phase 2 conceptually, but same file)
- **Phase 4 (US3)**: Depends on Phase 1 — adds Configuration section
- **Phase 5 (US4)**: Depends on Phase 1 — adds Sync section
- **Phase 6 (Polish)**: Depends on all prior phases

### User Story Dependencies

- **US1 (P1)**: No dependencies on other stories
- **US2 (P1)**: No dependencies on other stories
- **US3 (P2)**: No dependencies on other stories
- **US4 (P3)**: No dependencies on other stories

All stories are independent but target the same file, so they execute sequentially.

### Parallel Opportunities

Limited — all tasks write to the same `README.md`. However, content research for each story can be done in parallel since all data is already in `research.md`.

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. T001: Create file scaffold
2. T002-T003: Quick Start + Installation
3. **STOP**: Validate — can a reader understand and build clio?

### Incremental Delivery

1. T001 → File exists with overview
2. T002-T003 → User can install (MVP)
3. T004-T008 → User can discover all commands
4. T009 → User can configure
5. T010 → User understands sync
6. T011 → Final validation

---

## Notes

- All content data is pre-extracted in research.md — no codebase reading needed during implementation
- quickstart.md provides the section order blueprint
- Single-file deliverable: README.md at repository root
- Total: 11 tasks, 4 user stories
