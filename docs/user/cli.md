# CLI Guide

This guide documents high-value `controlpath` workflows and command groups.

## Global flags

- `--json`: machine-readable output where supported
- `--non-interactive`: disable prompts
- `-v` / `-vv` / `--verbose`: more diagnostics
- `-q` / `--quiet`: errors only

## Workflow commands

- `setup`: bootstrap project scaffolding
- `new-flag`: add a flag and optionally enable it
- `flag enable`: enable a flag in one or more environments
- `deploy`: validate and compile for environments
- `dev`: developer-focused flow with smart defaults

## Core commands

- `validate`: validate catalog and rules
- `compile`: compile environment rules to artifact
- `generate-sdk`: regenerate TypeScript SDK

## Management commands

- `flag`: `add`, `list`, `show`, `remove`, `deprecate`, `report`
- `env`: `add`, `sync`, `list`, `remove`
- `kill-switch`: `set`, `clear`, `list`

## Debug and CI commands

- `explain`: inspect evaluation path for one flag/user/context
- `debug`: run interactive debug UI
- `watch`: watch and regenerate/recompile on changes
- `ci`: run CI validation/compile flow
- `completion`: shell completion output

## Common flows

Create and enable in one command:

```bash
controlpath new-flag checkout_revamp --enable-in staging
```

Enable in two environments:

```bash
controlpath flag enable checkout_revamp --env=staging,production --all
```

Explain with trace:

```bash
controlpath explain --flag checkout_revamp --user user.json --env staging --trace
```

CI with explicit envs:

```bash
controlpath ci --env=staging,production
```

## Migration naming notes

- Use `new-flag --enable-in` (not older `--enable`)
- Use `explain --user` and optional `--context`
- Use `kill-switch` (legacy `override` command is removed)
- For environment-scoped list/show, use deployment-oriented flags from command help

## Help

Use built-in help for exact options on your installed version:

```bash
controlpath --help
controlpath <command> --help
```
