# User Quickstart

This guide gets a project from zero to first flag evaluation using current CLI workflows.

## Prerequisites

- `controlpath` CLI available in `PATH`
- Node.js project (for generated TypeScript SDK usage)

## 1) Bootstrap a project

```bash
controlpath setup
```

This scaffolds a `control-path.yaml`, generates initial artifacts, and prepares SDK output.

## 2) Add a flag

```bash
controlpath new-flag my_feature --type boolean --default false
```

To seed rules in environments during creation:

```bash
controlpath new-flag my_feature --enable-in staging
```

## 3) Enable in an environment

```bash
controlpath flag enable my_feature --env staging --all
```

Or add a targeting rule:

```bash
controlpath flag enable my_feature --env staging --rule "role == 'admin'"
```

## 4) Validate and compile

```bash
controlpath deploy --env staging
```

Equivalent lower-level flow:

```bash
controlpath validate
controlpath compile --env staging
```

## 5) Generate or refresh SDK

```bash
controlpath generate-sdk
```

## 6) Evaluate in app code

Import your generated evaluator and call typed flag methods with a user/context object.
See `docs/user/sdk-typescript.md` for full integration details.

## Next

- Command reference: `docs/user/cli.md`
- Catalog structure: `docs/user/configuration.md`
- Runtime integration: `docs/user/sdk-typescript.md`
