Status: ready-for-agent

# Document filesystem refresh and fix deploy messaging

## Parent

- `.scratch/local-filesystem-refresh/PRD.md`
- `docs/adr/0003-local-filesystem-refresh-targets.md`

## What to build

Document the filesystem refresh workflow for self-hosted users and align CLI UX with hot-swap semantics.

User-facing docs should explain:

- **artifact path** and **kill switch path** as refresh-only targets (mirror **artifact URL** / **kill switch URL** split from ADR 0001)
- POSIX absolute paths only in v1; relative paths via `init({ artifact })` for local-only workflows without configured refresh
- Volume-mount / sidecar pattern: compile locally, place bytes at configured path, runtime polls and hot-swaps
- Atomic replace guidance (write-then-rename) for publishers
- Last-good behavior on missing or invalid files
- Mixed transports per environment (e.g. staging **artifact path**, production **artifact URL**)

Add a schema example catalog demonstrating path-based targets.

Fix `controlpath deploy` success output: remove “restart your application to load new flags”; describe replacing bytes at **artifact URL**, **artifact path**, **kill switch URL**, or **kill switch path** and runtime poll pickup instead.

## Acceptance criteria

- [ ] `docs/user/configuration.md` documents `path` XOR `url` on `artifacts` and `kill_switches`
- [ ] `docs/user/kill-switches.md` and `docs/user/sdk-typescript.md` cover **kill switch path** and **artifact path** refresh
- [ ] `docs/user/troubleshooting.md` includes filesystem refresh failure modes (missing file, invalid bytes, wrong environment)
- [ ] Schema example (e.g. under `schemas/examples/`) shows volume-mount style paths
- [ ] `controlpath deploy` success message reflects hot-swap / poll semantics, not restart
- [ ] CLI integration test or snapshot updated if deploy output is asserted

## Blocked by

- `.scratch/local-filesystem-refresh/issues/02-artifact-path-refresh.md`
