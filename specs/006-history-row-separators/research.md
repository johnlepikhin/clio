# Research: History Row Visual Separators

## GTK4 ListView Built-in Separators

**Decision**: Use `ListView::set_show_separators(true)` — GTK4's built-in separator support.

**Rationale**: GTK4 ListView has a native `show-separators` property that renders theme-aware horizontal dividers between rows. It uses the `@borders` theme color, works with both light and dark themes, and requires zero additional CSS or widget configuration. This is the simplest possible implementation.

**Alternatives considered**:

1. **Custom CssProvider with border-bottom on rows** — Would require creating a CssProvider, loading CSS, and attaching it to the display. Achieves the same visual result but with more code and maintenance burden.

2. **Gtk::Separator widget between rows** — Not feasible with ListView. ListView renders items from a ListModel; there is no mechanism to insert non-data separator widgets between rows.

3. **Programmatic border via widget margins/padding** — Hacky, theme-unaware, not idiomatic GTK.

## GTK4 CSS Node Reference

When `show-separators` is `true`, GTK4 adds the `.separators` CSS class to the `listview` node. The CSS hierarchy:

```
listview.separators > row
```

The default theme provides a thin 1px border using `@borders` color variable. No custom CSS needed.

## gtk4-rs API

The property is available via the `ListViewExt` trait (part of `gtk4::prelude::*`):

```rust
list_view.set_show_separators(true);
```

No feature flags or additional imports required with gtk4-rs 0.9.
