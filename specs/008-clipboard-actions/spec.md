# Feature Specification: Clipboard Actions

**Feature Branch**: `008-clipboard-actions`
**Created**: 2026-02-22
**Status**: Draft
**Input**: User description: "Custom user-defined actions in config triggered by conditions (WM_CLASS, regex) performing actions (external command, custom TTL)."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Set custom TTL by source app (Priority: P1)

A user wants clipboard entries from a password manager (e.g., KeePassXC) to auto-expire after 30 seconds, while all other entries use the default retention policy.

The user adds a rule to `~/.config/clio/config.yaml`:

```yaml
actions:
  - name: "Expire passwords quickly"
    conditions:
      source_app: "KeePassXC"
    actions:
      ttl: "30s"
```

When KeePassXC copies a password to the clipboard, Clio detects `source_app = "KeePassXC"`, matches the rule, and stores the entry with a 30-second TTL. The entry is pruned on the next prune cycle after 30 seconds.

**Why this priority**: Security-sensitive use case — passwords in clipboard history are a real attack surface. This delivers immediate, tangible value.

**Independent Test**: Copy text from an app matching `source_app`, verify the entry is stored with the custom TTL, and verify it is pruned after the TTL expires.

**Acceptance Scenarios**:

1. **Given** a rule with `source_app: "KeePassXC"` and `ttl: "30s"`, **When** KeePassXC copies text, **Then** the entry is stored with a 30-second TTL.
2. **Given** a rule with `source_app: "KeePassXC"` and `ttl: "30s"`, **When** Firefox copies text, **Then** the entry uses the default retention (no custom TTL).
3. **Given** a rule with `ttl: "30s"` on a matching entry, **When** 30+ seconds elapse and prune runs, **Then** the entry is deleted.

---

### User Story 2 - Set custom TTL by text pattern (Priority: P1)

A user wants entries that look like secrets (API keys, tokens) to auto-expire quickly, regardless of source app.

```yaml
actions:
  - name: "Expire API keys"
    conditions:
      content_regex: "^(sk-|ghp_|AKIA)[A-Za-z0-9]+"
    actions:
      ttl: "1m"
```

When any app copies text matching the regex, Clio stores the entry with a 1-minute TTL.

**Why this priority**: Equally important as P1 — regex-based matching covers cases where source app is unknown or unavailable (Wayland).

**Independent Test**: Copy text matching the regex pattern, verify the entry gets the custom TTL.

**Acceptance Scenarios**:

1. **Given** a rule with `content_regex: "^sk-"` and `ttl: "1m"`, **When** text "sk-abc123..." is copied, **Then** the entry is stored with a 1-minute TTL.
2. **Given** the same rule, **When** text "Hello world" is copied, **Then** the entry uses the default retention.
3. **Given** a rule with an invalid regex, **When** config is loaded, **Then** a validation error is reported and the rule is skipped (other rules still work).

---

### User Story 3 - Filter clipboard through external command (Priority: P2)

A user wants to automatically strip tracking parameters from URLs copied from the browser.

```yaml
actions:
  - name: "Strip tracking params"
    conditions:
      content_regex: "^https?://.*[?&](utm_|fbclid|gclid)"
    actions:
      command: ["sed", "s/[?&]\\(utm_[^&]*\\|fbclid=[^&]*\\|gclid=[^&]*\\)//g"]
```

When a URL with tracking parameters is copied, Clio pipes the clipboard text through the command and replaces the stored content with the command's stdout.

**Why this priority**: Powerful but more complex. Requires careful security considerations (external process execution). Builds on the conditions infrastructure from P1.

**Independent Test**: Copy a URL with tracking params, verify the stored entry has the cleaned URL.

**Acceptance Scenarios**:

1. **Given** a rule with a command action and matching regex, **When** matching text is copied, **Then** the text is piped through the command and the output is stored.
2. **Given** a command that exits with non-zero, **When** matching text is copied, **Then** the original text is stored unchanged (fail-safe).
3. **Given** a command that takes longer than 5 seconds, **When** matching text is copied, **Then** the command is killed, and the original text is stored.
4. **Given** an image entry (not text), **When** any rule with `command` matches, **Then** the command action is skipped (commands only apply to text).

---

### User Story 4 - Combine conditions and multiple actions (Priority: P2)

A user wants rules with multiple conditions (AND logic) and multiple actions.

```yaml
actions:
  - name: "Short-lived browser secrets"
    conditions:
      source_app: "Firefox"
      content_regex: "^(password|secret):"
    actions:
      ttl: "15s"
      command: ["tr", "-d", "\n"]
```

Both conditions must match. Both actions apply: the text is first processed by the command, then stored with the custom TTL.

**Why this priority**: Natural extension of P1/P2. Multiple conditions and actions compose cleanly once the basic infrastructure exists.

**Independent Test**: Copy matching text from the matching app, verify both the command transformation and TTL are applied.

**Acceptance Scenarios**:

1. **Given** a rule with `source_app` AND `content_regex`, **When** both match, **Then** all actions fire.
2. **Given** a rule with `source_app` AND `content_regex`, **When** only one matches, **Then** no actions fire.

---

### Edge Cases

- What happens when multiple rules match the same clipboard entry? **All matching rules apply in config order.** If multiple rules set `ttl`, the last one wins. If multiple rules set `command`, they are chained (output of one is input to the next).
- What happens when `source_app` is `None` (Wayland, detection failure)? **Rules with `source_app` condition never match.** Rules with only `content_regex` still apply.
- What happens when clipboard content is an image? **`content_regex` conditions never match images.** `source_app` conditions can match. Only `ttl` action applies to images (not `command`).
- What happens with an empty `actions` list in config? **No rules apply; default behavior unchanged.**
- What happens when `content_regex` and `source_app` are both absent in a rule? **Validation error — at least one condition is required.**

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST support user-defined rules in the config file under an `actions` key, as a list of rule objects.
- **FR-002**: Each rule MUST have a `name` (string, for logging/identification), a `conditions` object, and an `actions` object.
- **FR-003**: Conditions MUST support `source_app` (exact string match, case-sensitive) and `content_regex` (regex pattern match against clipboard text content).
- **FR-004**: When multiple conditions are specified in one rule, ALL conditions must match (AND logic).
- **FR-005**: Actions MUST support `ttl` (duration string, parsed via `humantime_serde` format, e.g., `"30s"`, `"5m"`, `"1h"`).
- **FR-006**: Actions MUST support `command` (list of strings: command + arguments). The clipboard text is piped via stdin; stdout replaces the stored text.
- **FR-007**: When a `command` fails (non-zero exit, timeout, missing binary), the original clipboard text MUST be stored unchanged.
- **FR-008**: Command execution MUST have a configurable timeout (default: 5 seconds).
- **FR-009**: Multiple matching rules MUST apply in definition order. For `ttl`, the last matching rule's value wins. For `command`, commands chain sequentially.
- **FR-010**: Rules with `content_regex` MUST NOT match image entries. Rules with only `source_app` CAN match image entries, but only `ttl` action applies.
- **FR-011**: Config validation MUST reject rules with invalid regex patterns, empty conditions, or missing required fields.
- **FR-012**: At least one condition (`source_app` or `content_regex`) MUST be present in each rule.
- **FR-013**: Per-entry TTL MUST be stored in the database and respected by the pruning logic alongside global `max_age`.

### Key Entities

- **Rule**: A named condition-action pair defined in config. Contains a name, conditions, and actions.
- **Condition**: A predicate evaluated against a clipboard entry. Types: `source_app` (exact match), `content_regex` (regex match).
- **Action**: An operation applied to a matching clipboard entry. Types: `ttl` (custom expiration), `command` (external text filter).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Entries matching a `ttl` rule are automatically pruned after the specified duration.
- **SC-002**: Entries matching a `command` rule have their text content transformed by the external command before storage.
- **SC-003**: Failed commands do not prevent entry storage — original content is preserved.
- **SC-004**: Rules with invalid configuration (bad regex, missing conditions) are reported as validation errors at config load time.
- **SC-005**: The action matching and execution adds no more than 50ms to clipboard processing time (excluding external command time).
- **SC-006**: Existing configurations without `actions` continue to work with no changes (full backward compatibility).
