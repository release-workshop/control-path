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
controlpath explain --flag <flag> --attributes attributes.json --env <env> --trace
```

Confirm evaluation attributes include fields referenced by rules (see [`rules.md`](rules.md#evaluation-attributes)).

## Runtime not updating after publish

Refresh targets (**artifact URL**, **artifact path**, **kill switch URL**, **kill switch path**) are polled on staggered intervals — updates are not instant.

**HTTP (`url`) targets:**

- Confirm the URL in `control-path.yaml` matches the environment loaded by `init({ artifact })`.
- Verify the remote object was updated and is reachable from the pod (TLS, auth, CDN delay).
- For artifacts, remember conditional GET: an unchanged object returns 304 and keeps last-good state.

**Filesystem (`path`) targets:**

- Path must be POSIX absolute (`/mnt/...`). Relative paths belong in `init({ artifact })`, not in catalog `path` fields.
- Confirm the file exists at the configured path inside the container mount namespace (not only on the build host).
- After `deploy`, copy or atomically replace (write-then-rename) `.controlpath/<env>.ast` or `.controlpath/<env>.kill-switches.json` to the configured path.
- Unchanged mtime and size skip a read — touch or replace the file when testing.
- Invalid JSON (kill switch) or corrupt artifact bytes keep **last-good** state; check logs for refresh warnings instead of expecting a hard failure.

**Wrong environment:**

- Poll targets are keyed by the **compiled artifact** environment (`artifact.env`). Mismatched env in the file vs catalog target produces guardrail rejection (artifacts) or no override (kill switches).

**Kill switch never applies:**

- Kill switch files are refresh-only — no override until the first successful poll. Ensure `KILL_SWITCH_PATHS` / `KILL_SWITCH_URLS` includes the loaded environment after `generate-sdk`.

## Last-resort reset

Regenerate and recompile:

```bash
controlpath generate-sdk
controlpath deploy --env <env>
```
