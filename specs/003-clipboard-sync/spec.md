# Feature Specification: Clipboard & Paste Buffer Synchronization

**Feature Branch**: `003-clipboard-sync`
**Created**: 2026-02-21
**Status**: Draft
**Input**: User description: "Синхронизация paste buffer (PRIMARY selection) и clipboard (CLIPBOARD selection) с настраиваемым направлением через конфиг"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Bidirectional Sync by Default (Priority: P1)

A user runs `clio watch` without any custom configuration. When they copy text with Ctrl+C (CLIPBOARD), the same text automatically becomes available via middle-click paste (PRIMARY). When they select text with the mouse (PRIMARY), it automatically becomes available via Ctrl+V (CLIPBOARD). Both changes are recorded in history.

**Why this priority**: The default "both" mode is the most common use case — users expect unified clipboard behavior without manual configuration.

**Independent Test**: Run `clio watch` with default config; copy text via Ctrl+C, verify it appears in PRIMARY via middle-click; select text with mouse, verify it appears in CLIPBOARD via Ctrl+V.

**Acceptance Scenarios**:

1. **Given** default configuration (sync mode "both"), **When** user copies text via Ctrl+C (CLIPBOARD changes), **Then** the same text becomes available in PRIMARY selection.
2. **Given** default configuration (sync mode "both"), **When** user selects text with mouse (PRIMARY changes), **Then** the same text becomes available in CLIPBOARD.
3. **Given** default configuration, **When** either selection changes, **Then** the new content is recorded in clipboard history.
4. **Given** the same content already exists in both selections, **When** no change is detected, **Then** no duplicate history entries are created and no unnecessary sync writes occur.

---

### User Story 2 - Configurable Sync Direction (Priority: P1)

A user edits their config file to set a specific sync direction. They can choose from four modes: sync from PRIMARY to CLIPBOARD only, sync from CLIPBOARD to PRIMARY only, sync both directions, or disable sync entirely.

**Why this priority**: Core feature — the configurable enum is the explicit requirement. Must work alongside the default mode.

**Independent Test**: Set each sync mode in config, run `clio watch`, verify only the configured direction is active.

**Acceptance Scenarios**:

1. **Given** sync mode set to "to-clipboard" (PRIMARY → CLIPBOARD), **When** user selects text (PRIMARY changes), **Then** CLIPBOARD is updated. **When** user copies via Ctrl+C (CLIPBOARD changes), **Then** PRIMARY is NOT updated.
2. **Given** sync mode set to "to-primary" (CLIPBOARD → PRIMARY), **When** user copies via Ctrl+C (CLIPBOARD changes), **Then** PRIMARY is updated. **When** user selects text (PRIMARY changes), **Then** CLIPBOARD is NOT updated.
3. **Given** sync mode set to "disabled", **When** either selection changes, **Then** the other selection is NOT updated. Changes are still recorded in history from whichever selection the system monitors.
4. **Given** sync mode set to "both", **When** either selection changes, **Then** the other is updated (same as Story 1).

---

### User Story 3 - Validate Sync Configuration (Priority: P2)

A user runs `clio config validate` or `clio config show` and the sync mode setting is correctly displayed and validated. Invalid values produce helpful error messages.

**Why this priority**: Supports discoverability and debugging of the new setting.

**Independent Test**: Run `clio config show` and verify the sync mode field appears; set an invalid value and run `clio config validate`.

**Acceptance Scenarios**:

1. **Given** default config (no sync mode set), **When** user runs `clio config show`, **Then** output shows the sync mode with its default value "both".
2. **Given** a config file with `sync_mode: disabled`, **When** user runs `clio config show`, **Then** output shows `sync_mode: disabled`.
3. **Given** a config file with an invalid sync mode value, **When** user runs `clio config validate`, **Then** an error message indicates the valid options.

---

### Edge Cases

- What happens when both PRIMARY and CLIPBOARD change simultaneously? The system processes changes in poll order; the most recent change wins and is synced to the other.
- What happens when the user copies an image via Ctrl+C? Image content is synced between selections following the same direction rules as text.
- What happens when PRIMARY selection is cleared (e.g., the source application closes)? Empty selections are not synced — the other selection retains its content.
- What happens if writing to a selection fails (e.g., due to an X11/Wayland error)? The error is logged, sync continues on next poll cycle.
- What happens on Wayland where PRIMARY selection behavior differs? The system uses whatever selection access the underlying clipboard library provides; behavior follows platform capabilities.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a configuration option for clipboard synchronization direction.
- **FR-002**: The sync direction option MUST accept exactly four values: "to-clipboard" (PRIMARY → CLIPBOARD), "to-primary" (CLIPBOARD → PRIMARY), "both" (bidirectional), and "disabled" (no sync).
- **FR-003**: The default sync direction MUST be "both" when not specified in the configuration.
- **FR-004**: The `clio watch` command MUST monitor both PRIMARY and CLIPBOARD selections when sync is enabled.
- **FR-005**: When a change is detected in a source selection, the system MUST write the content to the target selection according to the configured direction.
- **FR-006**: The system MUST NOT create infinite sync loops (writing to target must not re-trigger a sync back).
- **FR-007**: When sync mode is "disabled", the system MUST still record clipboard history from the CLIPBOARD selection (existing behavior unchanged).
- **FR-008**: The sync mode option MUST appear in `clio config show` output and be checked by `clio config validate`.
- **FR-009**: The `clio config init` generated default config MUST include the sync mode option with a comment explaining the four values.
- **FR-010**: Empty selections MUST NOT be synced — the system ignores empty content to avoid clearing the other selection.

### Key Entities

- **Sync Mode**: A four-valued setting controlling the direction of synchronization between PRIMARY selection and CLIPBOARD selection. Values: "to-clipboard", "to-primary", "both", "disabled". Default: "both".

### Assumptions

- The term "paste buffer" refers to the X11 PRIMARY selection (mouse selection, middle-click paste). The term "clipboard" refers to the X11 CLIPBOARD selection (Ctrl+C / Ctrl+V).
- Synchronization happens during `clio watch` polling — there is no separate daemon. Sync shares the existing polling interval.
- The loop-prevention mechanism uses content hashing: after writing to a target, the hash of the written content is tracked so the next poll cycle recognizes it as "already seen" and skips re-syncing.
- On Wayland, PRIMARY selection may not be fully supported by all compositors. The system does best-effort sync using whatever the clipboard library provides.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All four sync modes are selectable via configuration and produce the expected synchronization behavior during `clio watch`.
- **SC-002**: The default mode ("both") synchronizes changes between selections within one polling interval (< 1 second at default 500ms).
- **SC-003**: No infinite sync loops occur under any mode — the system converges to a stable state within 2 polling cycles after a single user action.
- **SC-004**: The sync mode setting is visible in `clio config show`, validated by `clio config validate`, and documented in `clio config init` output.
