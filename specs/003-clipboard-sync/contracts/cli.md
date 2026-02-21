# CLI Contract: Clipboard Sync

## Config changes

### `sync_mode` field in config.yaml

```yaml
# Synchronization between PRIMARY selection (mouse) and CLIPBOARD (Ctrl+C/V).
# Values: both (default), to-clipboard, to-primary, disabled
sync_mode: both
```

### `clio config show` (updated output)

The `sync_mode` field appears in the YAML output alongside existing fields.

### `clio config validate` (updated behavior)

| Scenario                    | stdout / stderr          | Exit code |
|-----------------------------|--------------------------|-----------|
| Valid sync_mode value       | `Configuration is valid.`| 0         |
| Invalid sync_mode value     | Parse error (serde)      | 1         |

Invalid values are caught at YAML parse time by serde enum deserialization — no custom validation needed.

### `clio config init` (updated template)

The default config template includes the `sync_mode` field with a comment explaining all four values.

## `clio watch` (updated behavior)

| Sync Mode      | Monitors          | Syncs to         | Records history from |
|----------------|--------------------|------------------|----------------------|
| `both`         | CLIPBOARD + PRIMARY| Both directions  | Both                 |
| `to-clipboard` | PRIMARY            | CLIPBOARD        | Both                 |
| `to-primary`   | CLIPBOARD          | PRIMARY          | Both                 |
| `disabled`     | CLIPBOARD only     | —                | CLIPBOARD only       |

**Loop prevention**: After writing to a target selection, the target's content hash is immediately updated so the next poll cycle does not re-trigger a reverse sync.

**stderr output** on startup:
```
watching clipboard (interval: 500ms, sync: both)...
```
