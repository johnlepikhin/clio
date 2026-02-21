# Tasks: History Row Visual Separators

**Input**: Design documents from `/specs/006-history-row-separators/`
**Prerequisites**: plan.md (required), spec.md (required), research.md

**Tests**: Not requested. GUI styling is exempt from automated tests per constitution.

**Organization**: Both user stories are satisfied by a single code change, so tasks are minimal.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths in descriptions

---

## Phase 1: User Story 1 - Distinguish adjacent entries at a glance (Priority: P1) MVP

**Goal**: Each history entry is visually separated from its neighbors by a thin horizontal line.

**Independent Test**: Open the history window with 5+ entries (mix of text and image). Verify each entry has a visible horizontal line separator below it. Verify the selection highlight does not conflict with separators. Verify both light and dark GTK themes render separators correctly.

### Implementation for User Story 1

- [x] T001 [US1] Enable built-in ListView row separators by adding `list_view.set_show_separators(true)` after ListView creation in `src/ui/window.rs`
- [x] T002 [US1] Verify build passes: run `cargo build`, `cargo clippy`, and `cargo test` with no errors or warnings

**Checkpoint**: User Story 1 is complete. Separators are visible between all entry types and consistent across themes.

---

## Phase 2: User Story 2 - Comfortable reading during long scrolling sessions (Priority: P2)

**Goal**: Separators remain consistent during scrolling through 50+ entries and across lazy-loaded pages and filtered search results.

**Independent Test**: Load 50+ entries, scroll to trigger lazy loading. Verify separators remain consistent with no flicker or layout shifts. Type a search query and verify filtered results also display separators.

### Implementation for User Story 2

No additional code changes needed. GTK4's `show-separators` property applies to all rows uniformly, including dynamically appended rows from lazy loading and search result refreshes. US1's single change (T001) satisfies US2 automatically.

**Checkpoint**: Both user stories are satisfied by the single change in T001.

---

## Phase 3: Polish & Cross-Cutting Concerns

- [ ] T003 Manual verification: test with 1 entry, 0 entries (empty list), 50+ entries (lazy load), and search-filtered results
- [ ] T004 Manual verification: test with both light and dark GTK themes

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (US1)**: No dependencies — can start immediately
- **Phase 2 (US2)**: Automatically satisfied by Phase 1
- **Phase 3 (Polish)**: Depends on Phase 1 completion

### User Story Dependencies

- **User Story 1 (P1)**: No dependencies
- **User Story 2 (P2)**: Fully covered by US1 implementation — no separate work needed

### Parallel Opportunities

- T001 and T002 are sequential (T002 verifies T001)
- T003 and T004 can run in parallel after T002

---

## Implementation Strategy

### MVP (User Story 1)

1. Add `list_view.set_show_separators(true)` in `src/ui/window.rs` (T001)
2. Run build verification (T002)
3. Manual QA (T003, T004)
4. Done — both user stories complete

This is a single-line change with zero risk. No new dependencies, no schema changes, no new files.

---

## Notes

- Total tasks: 4 (2 implementation, 2 manual verification)
- Tasks per user story: US1 = 2 tasks, US2 = 0 additional tasks (covered by US1)
- Parallel opportunities: T003 + T004 can run in parallel
- MVP scope: T001 + T002
