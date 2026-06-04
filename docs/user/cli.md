# CLI Guide

This is the command reference for `controlpath` boolean-flag workflows.

## Global flags

- `--json`: machine-readable output where supported
- `--non-interactive`: disable prompts
- `-v` / `-vv` / `--verbose`: increase diagnostics
- `-q` / `--quiet`: reduce output to errors

## Safety-first release flow

Use this as the default rollout loop for release safety:

```bash
controlpath new-flag checkout_revamp --type boolean --default false
controlpath flag enable checkout_revamp --env staging --rule "role == 'admin'"
controlpath explain --flag checkout_revamp --attributes attributes.json --env staging --trace
controlpath ci --env staging
controlpath deploy --env staging
```

When promoting, prefer a `rollout` rule in `control-path.yaml` for percentage changes (see [`rules.md`](rules.md)). For a CLI-only slice, you can use a `when` expression with `HASHED_PARTITION`:

```bash
controlpath flag enable checkout_revamp --env production --rule "HASHED_PARTITION(id, 100) < 5"
controlpath ci --env production
controlpath deploy --env production
```

Important boolean DX notes:

- `new-flag --default` now rejects invalid values (use `true` / `false`, also accepts `ON` / `OFF`).
- `flag enable --value` now rejects invalid values (use `true` / `false`, also accepts `ON` / `OFF`).
- If `flag enable` is run without `--rule`, it configures a catch-all rule for the target environment.
- `flag enable` appends a `serve` rule; it does not replace the full rule list. Use YAML for `rollout` rules.
- Expression syntax does not support `%`; use `rollout` or `HASHED_PARTITION` (see [`rules.md`](rules.md)).
- Prefer explicit `--env` in production workflows.

## Top-level commands

### `setup`

Bootstrap a local project with catalog scaffolding, initial compile, and SDK generation.

Common options:

- `--lang <lang>`
- `--skip-install`
- `--no-examples`

### `init`

Scaffold workspace/service catalog files for monorepo or multi-repo setups.

Common options:

- `--monorepo` or `--no-monorepo`
- `--namespace <name>`
- `--service-id <id>`

### `new-flag`

Create a new boolean flag and optionally seed environment rules.

Common options:

- positional `name`
- `--type boolean`
- `--default <true|false>`
- `--description <text>`
- `--enable-in <env[,env2]>`
- `--skip-sdk`
- `--best-effort`

### `deploy`

Validate and compile artifacts for one or more environments.

Common options:

- `--env <env[,env2]>`
- `--dry-run`

### `dev`

Development loop command with smart defaults and watch behavior.

Common options:

- `--lang <lang>`

### `watch`

Watch catalog changes and regenerate SDK and/or compile artifacts.

Common options:

- `--lang <lang>`
- `--definitions`
- `--deployments`

### `validate`

Validate catalog/environment configuration. Semantic warnings (missing `owner`, release without `expires`, entitlement with `default: true`, etc.) are printed to stderr; validation still succeeds unless schema or semantic **errors** are present. Strict CI may treat stderr warnings as failures.

Common options:

- `--env <env>`
- `--all`

### `compile`

Compile environment rules into `.controlpath/<env>.ast`.

Common options:

- `--env <env>`
- `--output <path>`

### `generate-sdk`

Regenerate SDK from catalog definitions.

Common options:

- `--lang <lang>`
- `--output <dir>`

### `ci`

Run CI workflow checks (validate + compile, optional SDK regen).

Common options:

- `--env <env>` (repeatable / comma-separated)
- `--no-sdk`

### `explain`

Show evaluation details for one flag and evaluation context.

Common options:

- `--flag <name>` (required)
- `--attributes <path-or-json>` — evaluation attributes (optional; defaults to `{}`)
- `--env <env>` or `--ast <path>`
- `--trace`

### `debug`

Launch the interactive debug UI.

Common options:

- `--port <number>`
- `--env <env>` or `--ast <path>`
- `--open`

### `completion`

Generate shell completion scripts.

Usage:

```bash
controlpath completion bash
controlpath completion zsh
controlpath completion fish
```

## `flag` subcommands

### `flag add`

Add a boolean flag to the catalog.

Common options:

- `--name <name>`
- `--type boolean`
- `--default <true|false>`
- `--description <text>`
- `--lang <lang>`
- `--sync`
- `--no-interactive`

### `flag list`

List flag definitions or environment-scoped rule status.

Common options:

- `--definitions`
- `--deployment <env>`
- `--format table|json|yaml`

### `flag show`

Show one flag in detail, including environment rule coverage.

Common options:

- `--name <name>`
- `--deployment <env>`
- `--format table|json|yaml`

### `flag remove`

Remove a flag globally or clear it from one environment.

Common options:

- `--name <name>`
- `--env <env>`

### `flag enable`

Enable/configure a flag in one or more environments.

Common options:

- positional `name`
- `--env <env[,env2]>`
- `--rule "<expression>"`
- `--all`
- `--value <true|false>`
- `--interactive`
- `--no-compile`
- `--best-effort`
- `--force`

### `flag deprecate`

Mark a flag as deprecated; future rule changes require `flag enable --force`.

Common options:

- `--name <name>`

### `flag report`

Show lifecycle/rot reporting (SaaS telemetry is read-only when available).

## `env` subcommands

### `env add`

Add a new environment in catalog workflows.

Common options:

- `--name <env>`
- `--interactive`

### `env sync`

Sync and validate environment rule surfaces against catalog flags.

Common options:

- `--env <env>`
- `--dry-run`

### `env list`

List configured environments.

Common options:

- `--format table|json|yaml`

### `env remove`

Remove an environment from catalog rules.

Common options:

- `--name <env>`

## `kill-switch` subcommands

Kill-switch commands are for incident overrides and require explicit `--env`.

### `kill-switch set`

Set a kill switch value for a `kind: kill_switch` flag.

Usage:

```bash
controlpath kill-switch set emergency_stop true --env production
```

### `kill-switch clear`

Clear a kill switch override.

Usage:

```bash
controlpath kill-switch clear emergency_stop --env production
```

### `kill-switch list`

List active kill switches in an environment.

Usage:

```bash
controlpath kill-switch list --env production
```

## Help

Use built-in help for exact options on your installed version:

```bash
controlpath --help
controlpath <command> --help
controlpath <command> <subcommand> --help
```
