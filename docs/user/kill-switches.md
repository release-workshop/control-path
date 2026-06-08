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
- For **entitled** capabilities, use a **companion** `kind: kill_switch` flag (not the entitlement name). The application ANDs entitlement evaluation and kill-switch evaluation. Do not run `kill-switch set` on an entitlement flag — the CLI requires `kind: kill_switch`. See [`entitlements.md`](entitlements.md).

## Publishing and runtime refresh (local mode)

`controlpath deploy` writes `.controlpath/<env>.kill-switches.json`. For pods to pick up changes without restart, configure a refresh target per environment in `control-path.yaml`:

```yaml
kill_switches:
  production:
    path: /mnt/flags/production.kill-switches.json
```

Or use HTTP hosting:

```yaml
kill_switches:
  production:
    url: https://flags.example.com/production/kill-switches.json
```

After `controlpath generate-sdk`, the TypeScript SDK embeds `KILL_SWITCH_URLS` and/or `KILL_SWITCH_PATHS`. At runtime:

1. `await evaluator.init({ artifact: '...' })` loads the **compiled artifact** only (no bundled kill switch file).
2. The runtime polls the **kill switch URL** or **kill switch path** for the artifact’s environment on a staggered interval (faster than artifact polling).
3. The first successful read applies overrides; later polls hot-swap when mtime/size change.

**Volume-mount / sidecar pattern:** compile and deploy in CI, copy or atomically replace the JSON at the configured `path` on the host or shared volume. Sidecars can write the same path; the SDK does not watch inotify — it polls on an interval.

**Atomic replace:** write to a temporary file in the same directory, then `rename` into place so polls do not read partial JSON.

## Related docs

- [`cli.md`](cli.md)
- [`configuration.md`](configuration.md)
- [`rules.md`](rules.md)
- [`troubleshooting.md`](troubleshooting.md)
