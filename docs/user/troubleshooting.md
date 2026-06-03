# Troubleshooting

## `control-path.yaml` not found

Run commands from the project root and confirm the file exists:

```bash
ls control-path.yaml
```

## Validation fails

Run:

```bash
controlpath validate
```

Common causes:

- invalid YAML structure
- unknown fields for current schema
- invalid rule expression syntax
- environment or flag references that do not exist

## Compile fails

Run:

```bash
controlpath compile --env <env>
```

Check:

- the target environment exists
- all expressions parse
- flags used by rules are declared

## Explain output is unexpected

Use trace mode:

```bash
controlpath explain --flag <flag> --user user.json --env <env> --trace
```

Confirm evaluation attributes include fields referenced by rules (see [`rules.md`](rules.md#evaluation-attributes)).

## Runtime not updating after remote change

- confirm correct artifact URL / kill switch URL for environment
- verify remote object updated and accessible
- remember polling is interval-based; updates are not instant
- check app logs for refresh warnings

## Last-resort reset

Regenerate and recompile:

```bash
controlpath generate-sdk
controlpath deploy --env <env>
```
