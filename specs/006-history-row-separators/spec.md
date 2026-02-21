# Feature Specification: History Row Visual Separators

**Feature Branch**: `006-history-row-separators`
**Created**: 2026-02-21
**Status**: Draft
**Input**: User description: "Надо лучше визуально отделить отдельные записи в истории. Сейчас они все имеют сплошной белый фон и нет разделителей, поэтому тяжело понять где кончается одна запись и начинается другая."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Distinguish adjacent entries at a glance (Priority: P1)

A user opens the clipboard history window and immediately sees that each entry is visually separated from adjacent ones. The boundaries between entries are obvious without needing to read the content.

**Why this priority**: This is the core problem — users cannot tell where one entry ends and the next begins. Without this, the history list is hard to scan.

**Independent Test**: Can be fully tested by opening the history window with 5+ entries and confirming each entry is visually distinct from its neighbors.

**Acceptance Scenarios**:

1. **Given** a history window with 3+ text entries, **When** the user opens the window, **Then** each entry has a clear visual boundary separating it from adjacent entries.
2. **Given** a history window with mixed content (text and image entries), **When** the user scrolls through the list, **Then** every entry — regardless of type — is visually separated from its neighbors with the same consistent style.
3. **Given** a selected (highlighted) entry, **When** the user looks at the list, **Then** the separator style does not conflict with or obscure the selection highlight.

---

### User Story 2 - Comfortable reading during long scrolling sessions (Priority: P2)

A user scrolling through dozens of history entries finds the list easy to scan. The visual rhythm of separators helps the eye track individual entries and reduces cognitive effort.

**Why this priority**: Builds on P1 by ensuring the separators work well at scale (many entries, lazy-loaded pages), not just with a few items.

**Independent Test**: Can be tested by loading 50+ entries and scrolling through the full list, verifying that separators are consistent and do not introduce visual artifacts or layout jumps.

**Acceptance Scenarios**:

1. **Given** a history with 50+ entries, **When** the user scrolls through the entire list, **Then** separators are consistent in appearance and do not flicker or shift as new pages load.
2. **Given** search results showing a filtered subset of entries, **When** the user views the results, **Then** the same visual separators are applied consistently.

---

### Edge Cases

- What happens with a single entry in the list? The separator should still style the entry consistently (no dangling line below the last item or above the first item).
- What happens with an empty list (no history)? No separators or artifacts should appear.
- How do separators look in both light and dark GTK themes? They should remain visible and consistent regardless of system theme.

## Clarifications

### Session 2026-02-21

- Q: What visual separation style should be used? → A: Horizontal line separator (thin border-bottom) between entries.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST display a thin horizontal line between each pair of adjacent history entries in the list.
- **FR-002**: The visual separator MUST be consistent for all entry types (text, image, unknown).
- **FR-003**: The separator MUST NOT conflict with the GTK selection highlight on the focused/selected entry.
- **FR-004**: The separator style MUST work correctly with both light and dark system themes.
- **FR-005**: Separators MUST remain consistent as new pages are lazy-loaded during scrolling.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Each entry in the history list is visually distinguishable from adjacent entries without reading the entry content.
- **SC-002**: The visual separation is consistent across all entry types and list states (full list, filtered results, paginated loads).
- **SC-003**: The feature introduces no visible layout shifts, flicker, or rendering artifacts during scrolling or page loads.

## Assumptions

- The current background is the default GTK theme background; no custom background color is explicitly set by the application.
- Separators will be implemented via standard GTK/CSS mechanisms that respect theme colors automatically.
- The chosen visual style is a thin horizontal line (border-bottom) between rows, not zebra striping or card-style layout.
