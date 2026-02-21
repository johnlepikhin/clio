# Tasks: History UX & Image Previews

**Input**: Design documents from `/specs/005-history-ux-images/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/cli.md

**Tests**: Not explicitly requested. Unit tests included where constitution (Principle VI) requires them for public interfaces.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Config field shared by US3 (image previews)

- [X] T001 [P] Add `image_preview_max_px: i32` (default 320) field to `Config` struct in `src/config/types.rs` — add to struct, Default impl, default_yaml(), and validate() (must be > 0)

**Checkpoint**: `cargo build` succeeds. New config field parses from YAML.

---

## Phase 2: User Story 1 — Type-to-Filter with List Focus (Priority: P1) 🎯 MVP

**Goal**: History window opens with focus on the entry list. Typing immediately filters entries with text shown in the search field.

**Independent Test**: Open `clio history`, start typing without clicking. Verify filter text appears in search field, entries are filtered. Click search field, edit text, verify re-filtering.

### Implementation

- [X] T002 [US1] Add `list_view.grab_focus()` call after `window.present()` in `src/ui/window.rs` — moves initial focus from SearchEntry to ListView so `set_key_capture_widget` becomes active and forwards keystrokes to the search field

**Checkpoint**: Type-to-filter works from the moment the window opens. Search field shows typed text. Clicking search field allows direct editing.

---

## Phase 3: User Story 2 — Escape Closes Window from Anywhere (Priority: P1)

**Goal**: Pressing Escape closes the history window regardless of which widget has focus.

**Independent Test**: Open `clio history`, focus search field, press Escape — window closes. Repeat with list focused.

### Implementation

- [X] T003 [US2] Change the Escape `EventControllerKey` in `src/ui/window.rs` to use `PropagationPhase::Capture` on the window — currently the escape controller uses bubble phase (default), but `SearchEntry` intercepts Escape in bubble phase to clear text, preventing the window close; switching to capture phase makes the window-level handler fire first, closing the window immediately

**Checkpoint**: Single Escape press closes window from both list focus and search field focus.

---

## Phase 4: User Story 3 — Larger Image Previews (Priority: P2)

**Goal**: Image entries display proportionally scaled previews (default max 320px on longest side) instead of tiny icons. Configurable via `image_preview_max_px`.

**Independent Test**: Copy large and small images to clipboard. Open history. Large images scale to fit 320px max side. Small images show at original size. Change config, verify new size applied.

### Implementation

- [X] T004 [US3] Modify `create_thumbnail_texture` in `src/ui/window.rs` to accept `max_px: i32` parameter and scale images using `Pixbuf::scale_simple(dst_w, dst_h, InterpType::Bilinear)` — if both dimensions <= max_px use original, otherwise compute `scale = max_px / max(src_w, src_h)` and scale both dimensions proportionally
- [X] T005 [US3] Update all call sites of `create_thumbnail_texture` in `src/ui/window.rs` (`append_entries_to_store`) to pass `config.image_preview_max_px` as the `max_px` argument
- [X] T006 [US3] Remove `thumbnail.set_pixel_size(48)` from `src/ui/entry_row.rs` `connect_setup` — this is a no-op for paintable sources and the texture will already have correct dimensions from scaling
- [X] T007 [US3] Add unit test for `create_thumbnail_texture` scaling logic in `src/ui/window.rs` — extract scaling calculation into a testable pure function `compute_thumbnail_size(src_w, src_h, max_px) -> (i32, i32)` and test: small image (no scaling), landscape (width > max), portrait (height > max), square, exact boundary

**Checkpoint**: History shows large image previews. Small images unchanged. Config override works.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Final validation across all changes

- [X] T008 Add unit tests for `image_preview_max_px` config field in `src/config/mod.rs` — test deserialization, default value (320), validation (0 and negative rejected)
- [X] T009 Run `cargo clippy` and fix any warnings
- [X] T010 Run `cargo test` and ensure all tests pass (existing + new)
- [ ] T011 Verify quickstart.md scenarios — build and manually test focus, escape, image previews

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — T001 standalone
- **US1 (Phase 2)**: No dependencies on Setup — T002 modifies only window.rs focus logic
- **US2 (Phase 3)**: No dependencies on Setup — T003 modifies only escape controller in window.rs
- **US3 (Phase 4)**: Depends on T001 (config field for max_px)
- **Polish (Phase 5)**: Depends on all user stories

### User Story Dependencies

- **US1 (focus)**: Independent. Only adds `grab_focus()` call.
- **US2 (escape)**: Independent. Only changes escape controller propagation phase.
- **US3 (images)**: Depends on T001 (config field). Modifies thumbnail creation and entry_row.rs.

### Parallel Opportunities

- T001 (config) and T002 (focus) and T003 (escape) are all parallelizable (different code sections, no conflicts)
- T004, T005, T006 must be sequential within US3 (T005 depends on T004 signature change; T006 is independent file but logically part of the same change)
- T008 (config tests) can run in parallel with US1/US2/US3

---

## Implementation Strategy

### MVP First (User Stories 1 + 2)

1. Complete T001 (config), T002 (focus), T003 (escape) — can be done in parallel
2. **STOP and VALIDATE**: Type-to-filter and Escape work correctly
3. Continue with US3 (T004–T007) for image previews

### Full Delivery

1. T001 + T002 + T003 in parallel → US3 (T004–T007) → Polish (T008–T011)
2. US1 and US2 are single-task stories; US3 is 4 tasks

---

## Notes

- No new dependencies — uses existing gtk4-rs `gdk_pixbuf::Pixbuf::scale_simple` and `PropagationPhase`
- Total: 11 tasks (1 setup, 1 US1, 1 US2, 4 US3, 4 polish)
- Key insight from research: `SearchEntry` intercepts Escape in bubble phase — must use capture phase
- `set_pixel_size(48)` is a no-op for paintable sources — only affects named icon sizes
