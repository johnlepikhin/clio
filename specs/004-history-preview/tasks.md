# Tasks: History Preview & Lazy Loading

**Input**: Design documents from `/specs/004-history-preview/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/cli.md

**Tests**: Not explicitly requested. Unit tests included where constitution (Principle VI) requires them for public interfaces.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Config fields and repository functions shared by all user stories

- [X] T001 [P] Add `preview_text_chars: usize` (default 4096) and `history_page_size: usize` (default 50) fields to `Config` struct in `src/config/types.rs` — add to struct, Default impl, default_yaml(), and validate() (must be > 0)
- [X] T002 [P] Add `list_entries_page(conn, limit, offset)` function to `src/db/repository.rs` — `SELECT ... ORDER BY created_at DESC LIMIT ?1 OFFSET ?2`; keep existing `list_entries()` unchanged
- [X] T003 [P] Add `search_entries_page(conn, query, limit, offset)` function to `src/db/repository.rs` — `SELECT ... WHERE text_content LIKE '%' || ?1 || '%' ORDER BY created_at DESC LIMIT ?2 OFFSET ?3`
- [X] T004 [P] Add `truncate_preview(text: &str, max_chars: usize) -> (String, bool)` helper function to `src/ui/window.rs` — character-based truncation using `chars().take(max_chars)`; return `(truncated_text, was_truncated)`

**Checkpoint**: `cargo build` succeeds. New config fields parse from YAML. Paginated queries work. Truncation helper is available.

---

## Phase 2: User Story 1 — Text Preview in History Window (Priority: P1) 🎯 MVP

**Goal**: Text entries in history show a truncated preview (default 4 KB) with `…` if truncated. Multiline text is displayed preserving line breaks.

**Independent Test**: Open `clio history` with short, multiline, and very long text entries. Short entries display fully, long entries are truncated with `…`, multiline entries show line breaks.

### Implementation

- [X] T005 [US1] Modify entry-building loop in `src/ui/window.rs` `build_window()`: call `truncate_preview(text, config.preview_text_chars)` when creating `EntryObject`, append `…` if `was_truncated`
- [X] T006 [US1] Modify `src/ui/entry_row.rs` `connect_bind`: remove single-line restriction (remove `lines().next()` and 120-char truncation); display the full `preview_text()` as-is (it's already truncated by T005)
- [X] T007 [US1] Modify `src/ui/entry_row.rs` `connect_setup`: remove `set_ellipsize()` and `set_max_width_chars()` from preview label; set `set_wrap(true)` with `set_wrap_mode(pango::WrapMode::WordChar)` to allow natural multiline display
- [X] T008 [US1] Add unit test for `truncate_preview` in `src/ui/window.rs` — test short text (no truncation), long text (truncation at char boundary), exact boundary, multi-byte UTF-8 characters, emoji

**Checkpoint**: History window shows truncated text previews with `…` and multiline display. Images still show thumbnails.

---

## Phase 3: User Story 2 — Lazy Loading of History Entries (Priority: P1)

**Goal**: Load only `history_page_size` entries initially. Load more on scroll. Filtering searches entire DB.

**Independent Test**: With 200+ entries, open `clio history`. Only 50 entries initially. Scroll loads more. Type a filter — results come from entire DB.

### Implementation

- [X] T009 [US2] Rewrite `build_window()` initial load in `src/ui/window.rs`: replace `repository::list_entries(&conn, config.max_history)` with `repository::list_entries_page(&conn, config.history_page_size, 0)`; track `offset` in a `Rc<RefCell<usize>>` for scroll pagination
- [X] T010 [US2] Add scroll-to-load in `src/ui/window.rs`: connect to `ScrolledWindow`'s `vadjustment` `value-changed` signal; when scroll position exceeds 80% of total content height and more entries may exist, call `list_entries_page(page_size, offset)` and append results to `ListStore`; update offset; stop loading when a page returns fewer entries than `page_size`
- [X] T011 [US2] Replace in-memory `CustomFilter` in `src/ui/window.rs` with DB-side filtering: on `search_changed`, clear `ListStore`, reset offset, call `search_entries_page(query, page_size, 0)` to populate store; when query is empty, reload unfiltered paginated entries; connect scroll-to-load for filtered results too
- [X] T012 [US2] Add unit tests for `list_entries_page` and `search_entries_page` in `src/db/repository.rs` — test pagination (offset 0, offset N), empty results, search matching/not-matching

**Checkpoint**: `clio history` loads entries lazily. Scroll loads more. Filter searches full DB. All existing functionality (select, delete, escape) still works.

---

## Phase 4: User Story 3 — Image Entries Displayed As-Is (Priority: P2)

**Goal**: Image entries in history display as thumbnails without any truncation or preview limit applied to them.

**Independent Test**: Copy an image, let `clio watch` save it, open `clio history`. Image shows as thumbnail alongside text entries.

### Implementation

- [X] T013 [US3] Verify in `src/ui/window.rs` that image entries bypass `truncate_preview` — the entry-building loop must only apply text truncation when `content_type == Text`; images load `blob_content` and create thumbnail texture as before (verified: T005 handles this correctly)

**Checkpoint**: Mix of text and image entries display correctly — text is truncated with previews, images show thumbnails.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Final validation across all changes

- [X] T014 Add unit tests for new config fields in `src/config/mod.rs` — test `preview_text_chars` and `history_page_size` deserialization, default values, validation (0 rejected)
- [X] T015 Run `cargo clippy` and fix any warnings
- [X] T016 Run `cargo test` and ensure all tests pass (existing + new) — 43 tests passed
- [ ] T017 Verify quickstart.md scenarios — build and manually test `clio history` with text truncation, multiline, lazy loading, scroll, filtering

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — T001–T004 can all run in parallel (different files/functions)
- **US1 (Phase 2)**: Depends on T001 (config fields) and T004 (truncation helper)
- **US2 (Phase 3)**: Depends on T001 (config fields), T002, T003 (paginated queries), and T004 (truncation helper used in the new loading loop)
- **US3 (Phase 4)**: Depends on US1 (T005 establishes the entry-building pattern that US3 verifies)
- **Polish (Phase 5)**: Depends on all user stories

### User Story Dependencies

- **US1 (text preview)**: Depends on Setup. Core display logic.
- **US2 (lazy loading)**: Depends on Setup. Rewrites the loading/filtering logic in window.rs. Should be done after US1 since US1 changes the entry-building loop that US2 will further modify.
- **US3 (images as-is)**: Depends on US1 (verifies image handling in the new entry-building flow).

### Parallel Opportunities

- T001, T002, T003, T004 are all parallelizable (different files/sections)
- T014 (config tests) can run in parallel with US1/US2/US3 implementation

---

## Implementation Strategy

### MVP First (User Story 1)

1. Complete Phase 1: Setup (T001–T004)
2. Complete Phase 2: US1 — text preview + multiline (T005–T008)
3. **STOP and VALIDATE**: `clio history` shows truncated previews with multiline

### Full Delivery

1. Setup → US1 (text preview) → US2 (lazy loading + filtering) → US3 (image verify) → Polish
2. US1 must be done before US2 because US2 rewrites the loading loop that US1 modifies

---

## Notes

- No new dependencies — uses existing gtk4-rs, rusqlite, std library
- Total: 17 tasks (4 setup, 4 US1, 4 US2, 1 US3, 4 polish)
- Key risk: GTK4 scroll signal handling — validated in research.md (R2)
- `str::floor_char_boundary()` requires Rust 1.82+ (stable)
