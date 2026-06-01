# CLI Usage Guide

This guide reflects the current `controlpath` CLI behavior implemented in `crates/cli/src/main.rs`.

## Mental Model

Control Path is built around three artifacts:

1. **Configuration**: `control-path.yaml`
2. **Generated SDK**: default `node_modules/@controlpath/generated` (or custom output)
3. **Compiled AST artifacts**: `.controlpath/<env>.ast`

Most workflows (`setup`, `new-flag`, `enable`, `deploy`, `dev`, `ci`) compose lower-level commands for you.

## Global Flags

These are available on all commands:

- `--json`: machine-readable output where supported
- `--non-interactive`: disable interactive prompts
- `-v`, `-vv`, `--verbose`: increase verbosity
- `-q`, `--quiet`: reduce output to errors only

## Quick Start

```bash
# 1) Bootstrap a project
controlpath setup

# 2) Add a flag
controlpath new-flag my_feature

# 3) Enable it in staging
controlpath flag enable my_feature --env staging --all

# 4) Validate + compile for deployment
controlpath deploy --env staging
```

## Command Groups

### Workflow Commands

- `setup`: bootstrap project scaffolding
- `new-flag`: add a flag and optionally enable it immediately
- `enable`: enable a flag in one or more environments
- `deploy`: validate + compile for target environments
- `dev`: development mode with smart defaults

### Core Commands

- `validate`: validate definitions/deployments
- `compile`: compile deployment data to AST artifacts
- `generate-sdk`: regenerate SDK from definitions

### Management Commands

- `flag`: `add`, `list`, `show`, `remove`, `deprecate`, `report`
- `env`: `add`, `sync`, `list`, `remove`
- `override`: `set`, `clear`, `list`, `history`

### Debug / Dev / CI

- `explain`: explain evaluation for a flag/user/context
- `debug`: launch interactive debug UI
- `watch`: watch files and auto-regenerate/recompile
- `ci`: CI pipeline command
- `completion`: generate shell completions

## High-Value Usage Patterns

### Add and Enable in One Flow

```bash
controlpath new-flag my_feature --type boolean --default false --enable-in staging
```

`--enable-in` can be repeated or comma-separated:

```bash
controlpath new-flag my_feature --enable-in staging --enable-in production
controlpath new-flag my_feature --enable-in=staging,production
```

### Enable Flag with Rule

```bash
controlpath flag enable my_feature --env staging --rule "role == 'admin'"
```

Deprecated flags block new environment rule changes unless `--force` is set (local mode only; SaaS environment rules are owned by the platform dashboard, not the CLI).

```bash
controlpath flag deprecate --name my_feature
controlpath flag enable my_feature --env staging --all --force   # override deprecation locally
```

### Flag lifecycle and rot report

`flag report` merges declared Git lifecycle metadata with read-only SaaS telemetry (evaluation counts, rot suggestions). Imported flags appear with qualified names (for example `platform.emergency_kill_switch`). Telemetry is never written back to `control-path.yaml`.

```bash
controlpath flag report
controlpath --json flag report
```

In SaaS mode, `ci` also emits warnings for deprecated flags and rot suggestions after catalog sync.

Multiple environments:

```bash
controlpath flag enable my_feature --env staging --env production --all
controlpath flag enable my_feature --env=staging,production --all
```

### Explain Evaluation

Use `--user` (not `--attributes`) and optional `--context`:

```bash
controlpath explain --flag my_feature --user user.json --env staging
controlpath explain --flag my_feature --user '{"id":"123"}' --context context.json --env staging --trace
```

### CI Pipeline

```bash
controlpath ci
controlpath ci --env production --env staging
controlpath ci --env=production,staging --no-sdk
controlpath ci --no-validate
```

## Option Name Notes (Common Migrations)

If you used older docs/examples, these are the important updates:

- `new-flag --enable` -> `new-flag --enable-in`
- `explain --attributes` -> `explain --user` (plus optional `--context`)
- `flag list --env` -> `flag list --deployment`
- `flag show --env` -> `flag show --deployment`
- `controlpath test`, `controlpath disable`, `controlpath status` are not top-level commands in current CLI

## Next Steps

- See [CLI Usage Examples](./cli-usage-examples.md)
- See [Rust CLI Documentation](./rust-cli.md)
- Run `controlpath --help` or `controlpath <command> --help`
