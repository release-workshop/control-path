# Kill Switches

Kill switches let you force boolean values during incidents without changing rollout rules.

## CLI surfaces

- `controlpath kill-switch set <flag> <value> --env <env>`
- `controlpath kill-switch clear <flag> --env <env>`
- `controlpath kill-switch list --env <env>`

## Command semantics

- `--env` is required for all kill-switch commands.
- `set` and `clear` are local-mode only.
- `set` requires the target flag to exist and have `kind: kill_switch`.

## Typical usage

Set a kill switch:

```bash
controlpath kill-switch set emergency_stop true --env production
```

Inspect active kill switches:

```bash
controlpath kill-switch list --env production
```

Clear a kill switch:

```bash
controlpath kill-switch clear emergency_stop --env production
```

## Operational guidance

- Keep kill-switch values explicit and time-bounded.
- Prefer `kind: kill_switch` flags for incident response paths.
- Remove stale kill switches after root-cause remediation.

## Related docs

- [`cli.md`](cli.md)
- [`configuration.md`](configuration.md)
- [`rules.md`](rules.md)
- [`troubleshooting.md`](troubleshooting.md)
