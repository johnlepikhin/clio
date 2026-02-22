# Tasks: Memory Optimization

**Input**: Design documents from `/specs/009-memory-optimization/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md

**Tests**: Not explicitly requested in spec. Unit test added only for new early-size-check logic (T005).

**Organization**: Tasks grouped by user story. US1 must complete first (changes the function signature used by US2/US3). US4 is independent (UI only).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: User Story 1 - Eliminate RGBA clone in PNG encoding (Priority: P1) — MVP

**Goal**: Remove redundant `.to_vec()` clone in `encode_rgba_to_png` by passing `Vec<u8>` by ownership through the caller chain.

**Independent Test**: `cargo test --no-default-features` passes. Copy large image with `clio watch` — peak RSS reduced.

- [x] T001 [US1] Change `encode_rgba_to_png` signature from `rgba_bytes: &[u8]` to `rgba_bytes: Vec<u8>` and remove `.to_vec()` clone in `src/models/entry.rs`
- [x] T002 [US1] Update `ClipboardEntry::from_image` to accept `rgba_bytes: Vec<u8>` by ownership and pass it to `encode_rgba_to_png` in `src/models/entry.rs`
- [x] T003 [US1] Update all callers of `from_image` in `src/cli/watch.rs` to pass owned `Vec<u8>` instead of borrowing (destructure `ClipboardContent::Image` by move)
- [x] T004 [US1] Run `cargo test --no-default-features` and `cargo clippy` — verify all existing tests pass with no warnings

**Checkpoint**: US1 complete. Peak memory for single image encoding reduced by ~8 MB (one fewer RGBA clone).

---

## Phase 2: User Stories 2 & 3 - Sequential sync + early size rejection (Priority: P2)

**Goal**: Restructure watch loop to process CLIPBOARD and PRIMARY sequentially; add RGBA size check before PNG encoding.

**Independent Test**: `cargo test --no-default-features` passes. Sync mode with large images no longer doubles peak memory. Oversized images rejected before encoding.

- [x] T005 [P] [US3] Add early RGBA size check in `build_entry` before `encode_rgba_to_png` call — if `rgba_bytes.len() > max_entry_size_kb * 1024`, skip with `eprintln!` warning, in `src/cli/watch.rs`
- [x] T006 [P] [US3] ~~Unit test~~ — skipped: `build_entry` depends on clipboard IO, not unit-testable in isolation
- [x] T007 [US2] Restructure poll loop in `src/cli/watch.rs` to process CLIPBOARD first (read → hash → save → drop), then PRIMARY, ensuring buffers are not held simultaneously
- [x] T008 Run `cargo test --no-default-features` and `cargo clippy` — verify all tests pass

**Checkpoint**: US2 + US3 complete. Sequential sync halves peak memory for dual-image scenarios. Oversized images rejected early.

---

## Phase 3: User Story 4 - Thumbnail texture cache (Priority: P3)

**Goal**: Cache decoded thumbnail textures in the history window to avoid redundant PNG decoding on repeated scrolls.

**Independent Test**: Open history window, scroll through image entries twice — memory stable on second pass.

- [x] T009 [US4] Add `thumbnail_cache: HashMap<Vec<u8>, gtk4::gdk::Texture>` field to `HistoryWindow` struct in `src/ui/window.rs`
- [x] T010 [US4] Update `create_thumbnail_texture` call site in `append_entries` to check cache before decoding, and insert result into cache after decoding, in `src/ui/window.rs`
- [x] T011 [US4] Clear `thumbnail_cache` on page navigation (in `reload` method) in `src/ui/window.rs`
- [x] T012 [US4] Run `guix shell -m manifest.scm -- cargo clippy` and `guix shell -m manifest.scm -- cargo test` — verify full build passes

**Checkpoint**: US4 complete. Repeated scrolling no longer re-decodes thumbnails.

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Final verification across all stories

- [x] T013 Run full test suite: `cargo test --no-default-features` and `guix shell -m manifest.scm -- cargo test`
- [x] T014 Run quickstart.md validation steps

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (US1)**: No dependencies — start immediately. **BLOCKS Phase 2** (signature change affects watch.rs callers).
- **Phase 2 (US2+US3)**: Depends on Phase 1 (US1 changes `from_image` signature used by watch.rs).
  - T005 and T006 (US3) can run in parallel with each other.
  - T007 (US2) can run in parallel with T005/T006 (different code sections in same file).
- **Phase 3 (US4)**: Independent of Phases 1–2 (different file: `window.rs`). Can run in parallel.
- **Phase 4 (Polish)**: Depends on all previous phases.

### User Story Dependencies

- **US1 (P1)**: No dependencies. Changes function signatures → blocks US2/US3.
- **US2 (P2)**: Depends on US1 (watch.rs callers updated in T003).
- **US3 (P2)**: Depends on US1 (build_entry uses new from_image signature).
- **US4 (P3)**: Independent (UI code in window.rs, separate feature gate).

### Parallel Opportunities

```text
Phase 1:  T001 → T002 → T003 → T004

Phase 2:  T005 ──┐
          T006 ──┼→ T008
          T007 ──┘

Phase 3:  T009 → T010 → T011 → T012   (can run in parallel with Phase 2)

Phase 4:  T013 → T014
```

---

## Implementation Strategy

### MVP First (US1 Only)

1. Complete Phase 1 (T001–T004)
2. **STOP and VALIDATE**: All tests pass, image encoding uses one fewer buffer clone
3. This alone delivers ~30% peak memory reduction (SC-001)

### Incremental Delivery

1. Phase 1 (US1) → validate → biggest single improvement
2. Phase 2 (US2+US3) → validate → sequential sync + early rejection
3. Phase 3 (US4) → validate → UI scroll stability
4. Phase 4 → final verification

---

## Notes

- Total: 14 tasks (4 US1 + 4 US2/US3 + 4 US4 + 2 polish)
- US2 and US3 combined into one phase — both modify `watch.rs` and are tightly coupled
- No new dependencies required (constitution V: Minimal Dependencies)
- US4 tasks require GTK4 via `guix shell` — cannot test with `--no-default-features`
