# Feature Specification: History Preview & Lazy Loading

**Feature Branch**: `004-history-preview`
**Created**: 2026-02-21
**Status**: Draft
**Input**: User description: "Конфигом задавать размер предпросмотра текста в истории (по умолчанию первые 4KB). Изображения грузить как есть. При старте команды clio history подгружать только верхние 50 записей (дать возможность задавать в конфиге). Остальные только при скроллинге. Если текст многострочный - показывать его с разделением на строки (пусть строчка записи в списке растянется горизонтально - это не страшно)"

## Clarifications

### Session 2026-02-21

- Q: Should truncated text entries show a visual indicator that text was cut off? → A: Add `…` (ellipsis) at the end of truncated text.
- Q: How should text filtering interact with lazy loading? → A: Filtering searches the entire database (not just loaded entries), with results also loaded in pages.

## User Scenarios & Testing

### User Story 1 - Text Preview in History Window (Priority: P1)

When a user opens the clipboard history window, each text entry displays a preview limited to a configurable size (default: first 4 KB of text). This lets users quickly scan history without loading megabytes of content into the list. Multiline text entries are displayed preserving line breaks — the row stretches vertically to show all lines within the preview limit.

**Why this priority**: Core visual improvement — every user sees the history list, and preview size directly affects usability and performance.

**Independent Test**: Open `clio history` with several text entries of varying lengths (short, multiline, and very long). Verify that short entries display fully, multiline entries show line breaks, and entries exceeding the preview limit are truncated.

**Acceptance Scenarios**:

1. **Given** a text entry shorter than the preview limit, **When** the history window opens, **Then** the full text is displayed in the list row.
2. **Given** a text entry longer than the preview limit (e.g., 10 KB), **When** the history window opens, **Then** only the first 4 KB (default) of text is displayed followed by `…` (ellipsis).
3. **Given** a multiline text entry within the preview limit, **When** the history window opens, **Then** the entry is displayed preserving line breaks — the row height expands to accommodate multiple lines.
4. **Given** the user has set `preview_text_bytes: 8192` in config, **When** the history window opens, **Then** text previews show up to 8 KB.

---

### User Story 2 - Lazy Loading of History Entries (Priority: P1)

When the user opens `clio history`, only the top N entries are loaded initially (default: 50, configurable via `history_page_size`). As the user scrolls down past the loaded entries, additional entries are loaded on demand. This improves startup time and reduces memory usage for large histories.

**Why this priority**: Equally critical — without lazy loading, a history of thousands of entries causes slow startup and high memory consumption.

**Independent Test**: Populate the database with 200+ entries, open `clio history`. Verify only 50 entries are initially loaded. Scroll to the bottom and verify additional entries appear.

**Acceptance Scenarios**:

1. **Given** a database with 200 entries, **When** the user opens `clio history`, **Then** only the first 50 entries are displayed (most recent first).
2. **Given** the initial 50 entries are displayed, **When** the user scrolls past the last visible entry, **Then** the next batch of entries is loaded and appended to the list.
3. **Given** the user has set `history_page_size: 100` in config, **When** the history window opens, **Then** the first 100 entries are loaded initially.
4. **Given** fewer entries exist than the page size (e.g., 30 entries, page size 50), **When** the history window opens, **Then** all 30 entries are displayed and no further loading attempts are made on scroll.
5. **Given** 200 entries in the database with only 50 loaded, **When** the user types a filter query matching an entry not yet loaded, **Then** the system searches the full database and displays matching results (in pages).

---

### User Story 3 - Image Entries Displayed As-Is (Priority: P2)

Image entries in the history list are loaded and displayed at their full content — no truncation or preview limit is applied. Users see thumbnail representations of their copied images directly in the history list.

**Why this priority**: Complements the text preview feature. Images are already stored as compressed PNG, so displaying them as-is is the natural behavior.

**Independent Test**: Copy an image to clipboard, let `clio watch` save it, open history. Verify the image entry shows a visual thumbnail.

**Acceptance Scenarios**:

1. **Given** an image entry in the history, **When** the history window opens, **Then** the image is displayed as a thumbnail in the list row without truncation.
2. **Given** a mix of text and image entries, **When** the history window opens, **Then** text entries show text previews and image entries show image thumbnails.

---

### Edge Cases

- What happens when a text entry is exactly at the preview limit boundary? It is displayed without ellipsis (no truncation occurred).
- What happens when the user scrolls very fast? Loading keeps up without showing blank entries or crashing.
- What happens when the database has zero entries? The history window displays an empty state, no errors.
- What happens when text contains only whitespace or control characters? It is displayed as-is within the preview limit.
- What happens when the user sets config values to 0 or negative? The application rejects invalid values during config validation.
- What happens when truncation falls in the middle of a multi-byte UTF-8 character? The truncation point is adjusted to the nearest valid character boundary.

## Requirements

### Functional Requirements

- **FR-001**: System MUST limit text preview in history list rows to a configurable number of bytes (default: 4096 bytes). Truncated entries MUST display `…` (ellipsis) at the end to indicate continuation.
- **FR-002**: System MUST display multiline text entries preserving line breaks — each line of the preview appears on its own visual line within the list row.
- **FR-003**: System MUST load only the first N entries when opening the history window, where N is configurable (default: 50).
- **FR-004**: System MUST load additional entries on demand when the user scrolls past the currently loaded entries.
- **FR-010**: When the user types a filter query, the system MUST search across all entries in the database (not only loaded ones). Filtered results MUST also be loaded in pages of `history_page_size`.
- **FR-005**: System MUST display image entries at their full stored content (no truncation).
- **FR-006**: System MUST expose `preview_text_bytes` configuration field to control the text preview size.
- **FR-007**: System MUST expose `history_page_size` configuration field to control the initial and incremental loading batch size.
- **FR-008**: System MUST validate that `preview_text_bytes` and `history_page_size` are greater than 0.
- **FR-009**: System MUST truncate text at a valid UTF-8 character boundary when the byte limit falls within a multi-byte character.

### Key Entities

- **Configuration fields**: `preview_text_bytes` (size of text preview in bytes, default 4096) and `history_page_size` (number of entries per page/batch, default 50).
- **History list row**: Visual representation of a clipboard entry — either a truncated text preview with preserved line breaks, or an image thumbnail.

## Success Criteria

### Measurable Outcomes

- **SC-001**: History window opens and displays entries within 1 second for databases with up to 10,000 entries.
- **SC-002**: Text entries longer than the preview limit display only the configured preview portion.
- **SC-003**: Multiline text entries display each line separately in the list row.
- **SC-004**: Scrolling past loaded entries triggers loading of additional entries without noticeable lag (under 500 ms).
- **SC-005**: Both configuration fields are visible in `clio config show` and editable in the config file.

## Assumptions

- Text preview truncation is byte-based (not character-based), adjusted to the nearest valid UTF-8 character boundary.
- The page size applies both to the initial load and to each subsequent incremental batch loaded on scroll.
- The history window already orders entries by most recent first — this feature does not change ordering.
- Image thumbnails use a reasonable display size determined by the UI — exact thumbnail dimensions are an implementation detail.
