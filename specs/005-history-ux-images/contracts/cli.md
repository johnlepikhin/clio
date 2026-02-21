# CLI Contract: History UX & Image Previews

## Config Changes

### New field in config.yaml

```yaml
# Maximum image preview size in pixels (longest side, default 320).
image_preview_max_px: 320
```

### `clio config show` (updated output)

The new field appears in YAML output alongside existing fields.

### `clio config validate` (updated behavior)

| Scenario | stdout / stderr | Exit code |
|----------|-----------------|-----------|
| Valid image_preview_max_px | `Configuration is valid.` | 0 |
| image_preview_max_px = 0 | `image_preview_max_px must be greater than 0` | 1 |
| image_preview_max_px < 0 | `image_preview_max_px must be greater than 0` | 1 |

### `clio config init` (updated template)

The default config template includes the new field with an explanatory comment.

## `clio history` (updated behavior)

### Focus

- Window opens with focus on the entry list (not the search field).
- Typing characters immediately filters entries; typed text appears in the search field.
- The search field remains clickable and editable.

### Escape key

- Pressing Escape closes the window from any widget (list or search field).
- A single Escape press closes the window even when the search field has text.

### Image previews

- Image entries display at up to `image_preview_max_px` pixels on their longest side.
- Images smaller than the configured maximum are shown at original size.
- Images larger are proportionally scaled down.
- Default maximum: 320 pixels.
