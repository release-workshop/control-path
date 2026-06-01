# CLI Usage for Kill Switches

This guide explains how to use the Control Path CLI to manage kill switch files.

## Overview

Kill switches are boolean runtime overrides stored per environment in `.controlpath/<env>.kill-switches.json`. The CLI writes these files locally; upload them to your preferred storage (CDN, S3, web server, etc.) and point the SDK at the URL.

**Workflow:**
1. Edit kill switch files locally using `controlpath kill-switch` (alias: `override`)
2. Upload to your storage (manual step)
3. SDK loads kill switches from the URL you configure

## Commands

### Set Kill Switch

```bash
controlpath kill-switch set <flag> <value> --env <env> [options]
```

**Examples:**

```bash
# Disable a flag in production
controlpath kill-switch set new_dashboard false \
  --env production

# Re-enable after fix
controlpath kill-switch set new_dashboard true \
  --env production

# Alias (legacy command name)
controlpath override set new_dashboard false --env production
```

**Options:**
- `--env <name>`: Environment (default: `defaultEnv` from `.controlpath/config.yaml`, else first catalog environment, else `production`)
- `--reason <text>`: Accepted for compatibility; **not persisted** in kill switch files
- `--operator <name>`: Accepted for compatibility; **not persisted**
- `--file <path>`: **Deprecated and ignored** — files are always written to `.controlpath/<env>.kill-switches.json`

**Boolean values:** `true`/`false`, `ON`/`OFF`, `1`/`0`, `yes`/`no` (case-insensitive).

### Clear Kill Switch

```bash
controlpath kill-switch clear <flag> --env <env>
```

Removes the flag from the kill switch file; evaluation falls back to compiled AST rules.

### List Kill Switches

```bash
controlpath kill-switch list --env <env>
```

### Show Kill Switch State

```bash
controlpath kill-switch history <flag> --env <env>
controlpath kill-switch history --env <env>
```

## Requirements

- A v2 `control-path.yaml` catalog must exist in the project root.
- Kill switch commands do not support multivariate flags or custom override file paths.

## File Format

Kill switch files use JSON schema version `2.0`:

```json
{
  "version": "2.0",
  "flags": {
    "new_dashboard": false
  }
}
```

## See Also

- [Storage Setup Guide](./override-setup.md)
- [SDK Configuration](./override-sdk-config.md)
- [Examples](./override-examples.md)
