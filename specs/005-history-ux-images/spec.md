# Feature Specification: History UX & Image Previews

**Feature Branch**: `005-history-ux-images`
**Created**: 2026-02-21
**Status**: Draft
**Input**: User description: "Focus on list at startup, type-to-filter with visible search field, escape closes from anywhere, configurable larger image previews"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Type-to-Filter with List Focus (Priority: P1)

When the history window opens, the entry list is focused. The user immediately starts typing characters to filter entries. The typed text appears in the search field at the top. The user can also click into the search field to edit the filter text manually. Filtering is instant as each character is typed.

**Why this priority**: Core UX improvement. Currently users must manually focus the search field before typing. Direct type-to-filter from the list is the most natural interaction pattern for a clipboard manager and reduces friction on every use.

**Independent Test**: Open `clio history`, start typing without clicking anything. Verify filter text appears in the search field and entries are filtered. Click into the search field, edit the text, verify filtering updates.

**Acceptance Scenarios**:

1. **Given** history window just opened, **When** user types "hello", **Then** the search field shows "hello" and entries are filtered to those containing "hello"
2. **Given** history window with list focused, **When** user types characters, **Then** the search field updates in real time and filtering happens instantly
3. **Given** filter text "hel" is active, **When** user clicks into the search field and edits text to "hello world", **Then** entries are re-filtered to match "hello world"
4. **Given** filter text is active, **When** user clears the search field, **Then** all entries are shown again (paginated)

---

### User Story 2 - Escape Closes Window from Anywhere (Priority: P1)

Pressing Escape closes the history window regardless of which widget has focus — whether the entry list or the search field.

**Why this priority**: Essential usability. Users expect Escape to dismiss the window at all times, not only when the list is focused. Without this, the window feels broken when the search field has focus.

**Independent Test**: Open `clio history`, focus the search field, press Escape. Verify the window closes. Repeat with focus on the entry list.

**Acceptance Scenarios**:

1. **Given** history window open with list focused, **When** user presses Escape, **Then** the window closes
2. **Given** history window open with search field focused, **When** user presses Escape, **Then** the window closes

---

### User Story 3 - Larger Image Previews (Priority: P2)

Image entries in history display larger preview thumbnails instead of small icons. The maximum preview size is configurable (default: 320 pixels on the longest side). Images smaller than or equal to the configured size are shown at original size. Images larger are scaled down proportionally so neither dimension exceeds the configured maximum.

**Why this priority**: Significant visual improvement but secondary to interaction fixes. Users need to recognize images in their clipboard history without opening them, especially screenshots and design assets.

**Independent Test**: Copy a large image (e.g., 1920x1080) and a small image (e.g., 100x80) to clipboard. Open history. Verify the large image is scaled down to fit within 320x320 and the small image is shown at original size. Change config to a different max size, reopen history, verify the new size is applied.

**Acceptance Scenarios**:

1. **Given** an image entry of 1920x1080 and default config (max 320), **When** history is opened, **Then** the image preview is displayed at 320x180 (scaled proportionally)
2. **Given** an image entry of 100x80, **When** history is opened, **Then** the image is displayed at its original 100x80 size
3. **Given** an image entry of 640x640, **When** history is opened, **Then** the image preview is displayed at 320x320
4. **Given** config sets max preview size to 200, **When** history is opened, **Then** all images larger than 200px on any side are scaled to fit within 200x200

---

### Edge Cases

- What happens when the user types characters that match no entries? The list shows empty, search field displays the typed text.
- What happens when the user pastes text into the search field? Filtering applies to the pasted text.
- What happens with very wide or very tall images (extreme aspect ratios like 10:1)? They scale proportionally — the longest side is capped at the configured maximum.
- What happens with a 1x1 pixel image? It is displayed at original size (1x1).
- What happens when the configured max preview size is very large (e.g., 2000)? Images smaller than 2000px on both sides show at original size; larger images scale down.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: History window MUST open with focus on the entry list, not the search field
- **FR-002**: When the entry list is focused, typing characters MUST forward keystrokes to the search field and trigger filtering
- **FR-003**: The search field MUST display the currently typed filter text at all times
- **FR-004**: The search field MUST remain editable — users can click into it and modify the filter text directly
- **FR-005**: Filtering MUST happen instantly as each character is typed
- **FR-006**: Pressing Escape MUST close the history window regardless of which widget currently has focus
- **FR-007**: A configurable setting MUST control the maximum image preview size (in pixels, applied to the longest side)
- **FR-008**: The default maximum image preview size MUST be 320 pixels
- **FR-009**: Images with both dimensions at or below the configured maximum MUST be displayed at their original size
- **FR-010**: Images with any dimension exceeding the configured maximum MUST be scaled down proportionally so the longest side equals the maximum
- **FR-011**: The configured max image preview size MUST be validated as greater than 0

### Key Entities

- **Configuration**: New field for maximum image preview size (integer, pixels, default 320)
- **Image Preview**: Scaled representation of a clipboard image entry, sized according to the configured maximum

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can begin filtering entries within 0 seconds of opening the history window (no extra click or tab required)
- **SC-002**: Escape key closes the window 100% of the time regardless of focus state
- **SC-003**: Image previews are displayed at the correct size (within 1 pixel tolerance) for all tested image dimensions
- **SC-004**: All existing history window functionality (select entry, delete entry, scroll-to-load, text filtering) continues to work without regression

## Assumptions

- The search field widget remains visible at the top of the window and continues to serve as both display and input for filter text
- The existing keyboard navigation (Enter to select, Delete to remove) continues to work from the list when no filter text is being typed
- Image scaling is done at display time for preview purposes only — the original image data in the database is not modified
- The max image preview size config field uses the same configuration file as other settings
