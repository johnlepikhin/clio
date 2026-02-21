# Research: Clipboard & Paste Buffer Synchronization

## R1: PRIMARY selection access via arboard

**Decision**: Use `arboard`'s `LinuxClipboardKind::Primary` with `GetExtLinux` / `SetExtLinux` traits to read/write the PRIMARY selection.

**Rationale**: `arboard` 3.6.1 (already a dependency) provides full support for PRIMARY selection on X11 and Wayland (compositor v2+). The traits are already imported in `src/clipboard/mod.rs` (`SetExtLinux` is used for `.wait()`). Adding `GetExtLinux` and `LinuxClipboardKind` to the import is all that's needed.

**Alternatives considered**:
- `x11-clipboard` crate — rejected, adds a dependency when arboard already covers the need (Principle V).
- Direct X11/Wayland protocol calls — rejected, unnecessarily complex (Principle VII).

## R2: Loop prevention strategy

**Decision**: Track separate `last_hash` values for PRIMARY and CLIPBOARD. After syncing content from source to target, immediately update the target's `last_hash` to the written content's hash. This way the next poll sees the target's content as "already seen" and doesn't re-trigger a reverse sync.

**Rationale**: Simple, no extra state or timers. The existing `last_hash` approach in `watch.rs` already demonstrates this pattern for CLIPBOARD. Extending it to two hashes is minimal change.

**Alternatives considered**:
- Timestamp-based suppression (ignore target changes within N ms of a write) — fragile, timing-dependent.
- Write-lock flag — adds complexity, race conditions possible.
- Content comparison between selections on each poll — works but wasteful; hash tracking is O(1).

## R3: SyncMode enum serialization

**Decision**: Use a serde-serializable enum with `#[serde(rename_all = "kebab-case")]` for YAML-friendly names: `to-clipboard`, `to-primary`, `both`, `disabled`.

**Rationale**: Kebab-case is idiomatic for YAML config files. `serde` derive handles serialization/deserialization with no extra code. Default is `both` via `impl Default`.

**Alternatives considered**:
- Snake_case (`to_clipboard`) — less readable in YAML.
- String field with manual validation — more error-prone, loses type safety.

## R4: Clipboard module changes

**Decision**: Add `read_primary()` and `write_primary_text()` / `write_primary_image()` functions to `src/clipboard/mod.rs`, mirroring the existing clipboard functions but using `LinuxClipboardKind::Primary`. Also add `read_clipboard_of(kind)` and `write_clipboard_text_of(kind)` generic variants to reduce duplication.

**Rationale**: The watch loop needs to read/write both selections independently. Parameterizing by `LinuxClipboardKind` avoids code duplication while keeping the API clear.

## R5: No new dependencies

**Decision**: No new crates required. All needed types (`LinuxClipboardKind`, `GetExtLinux`, `SetExtLinux`) are in `arboard` which is already a dependency.

**Rationale**: Constitution Principle V — minimal dependencies.
