# Research: History UX & Image Previews

## R1: ListView Focus on Window Open

**Decision**: Call `list_view.grab_focus()` after `window.present()`.

**Rationale**: GTK4 gives initial focus to the first focusable widget in tree order — currently `SearchEntry`. Calling `grab_focus()` after `present()` moves focus to `ListView`. The existing `search_entry.set_key_capture_widget(Some(&list_view))` then becomes active: keystrokes typed while `ListView` is focused are forwarded to `SearchEntry` in the bubble phase, populating the filter field.

**Alternatives considered**:
- Reorder widgets (put list before search): Breaks visual layout, search field must be at top.
- `set_focus_child`: Less idiomatic than `grab_focus()`.

## R2: Escape Key from SearchEntry

**Decision**: Move the Escape `EventControllerKey` to the **capture** propagation phase on the window, so it fires before `SearchEntry` can intercept and consume the Escape event.

**Rationale**: `SearchEntry` has built-in Escape handling — first press clears the search text and stops propagation, so a bubble-phase controller on the window never sees it. By using `PropagationPhase::Capture`, the window-level controller intercepts Escape before it reaches `SearchEntry`.

**Alternatives considered**:
- Connect to `SearchEntry::stop-search` signal: Would require extra wiring and still wouldn't close the window on first Escape press if text is empty.
- Two Escape presses (clear then close): Poor UX, user expects single Escape to dismiss.

## R3: Image Scaling with Pixbuf

**Decision**: Use `gdk_pixbuf::Pixbuf::scale_simple(dst_w, dst_h, InterpType::Bilinear)` inside `create_thumbnail_texture` to produce a correctly-sized pixbuf before converting to `Texture`.

**Rationale**: `Pixbuf::scale_simple` is available in the core `gdk-pixbuf` API (no extra feature flags). `InterpType::Bilinear` gives good quality/speed balance for thumbnail previews. The scaling formula: `scale = min(max_px / max(src_w, src_h), 1.0)`, applied to both dimensions preserves aspect ratio. If both dimensions are at or below max_px, the original pixbuf is used unchanged.

**Alternatives considered**:
- `Image::set_pixel_size()`: Only affects icon-name sources, has no effect on paintable/texture sources. Confirmed by GTK4 docs and GNOME Discourse (changed behavior in GTK 4.19.2).
- `GtkPicture` instead of `GtkImage`: Would work but requires restructuring the row layout; `GtkImage` with a pre-scaled texture is simpler.

## R4: Entry Row Pixel Size

**Decision**: Remove `thumbnail.set_pixel_size(48)` from `entry_row.rs`. The `GtkImage` widget will render the texture at its natural (scaled) size.

**Rationale**: `set_pixel_size` is a no-op for paintable sources. Currently images render at original blob dimensions regardless of this setting. After implementing R3, the texture will already have the correct dimensions, so no widget-level size override is needed.

## R5: Config Field Naming

**Decision**: Name the config field `image_preview_max_px` (type `i32`, default `320`).

**Rationale**: Consistent with existing naming pattern (`window_width`, `window_height` are `i32`). The `_px` suffix clarifies units. Using `i32` matches the `Pixbuf::scale_simple` parameter type directly.
