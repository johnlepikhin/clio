# CLI Contract: Config Subcommands

## `clio config show`

**Description**: Print the effective configuration as YAML to stdout.

```
clio [--config PATH] config show
```

| Scenario             | stdout                        | stderr | Exit code |
|----------------------|-------------------------------|--------|-----------|
| Config file exists   | Merged config as YAML         | —      | 0         |
| No config file       | Default config as YAML        | —      | 0         |
| Config file invalid  | —                             | Error  | 1         |

---

## `clio config init`

**Description**: Create a default configuration file with comments.

```
clio [--config PATH] config init [--force]
```

| Scenario                     | stdout                              | stderr | Exit code |
|------------------------------|--------------------------------------|--------|-----------|
| No existing file             | `Config written to <path>`           | —      | 0         |
| File exists, no `--force`    | —                                    | Error  | 1         |
| File exists, `--force`       | `Config written to <path>`           | —      | 0         |
| Write permission denied      | —                                    | Error  | 1         |

**Side effect**: Creates file at config path. Creates parent directories if needed.

---

## `clio config validate`

**Description**: Validate the configuration file.

```
clio [--config PATH] config validate
```

| Scenario              | stdout                        | stderr            | Exit code |
|-----------------------|-------------------------------|-------------------|-----------|
| Valid config          | `Configuration is valid.`     | —                 | 0         |
| No config file        | `No config file found at <path>. Using defaults.` `Configuration is valid.` | — | 0 |
| Invalid YAML syntax  | —                             | Parse error       | 1         |
| Invalid field values  | —                             | Validation errors | 1         |

---

## `clio config path`

**Description**: Print the resolved config file path.

```
clio [--config PATH] config path
```

| Scenario             | stdout                        | stderr | Exit code |
|----------------------|-------------------------------|--------|-----------|
| Default path         | Absolute XDG config path      | —      | 0         |
| Custom `--config`    | The custom path               | —      | 0         |
