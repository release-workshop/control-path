Status: ready-for-agent

# Kill switch path refresh

## Parent

- `.scratch/local-filesystem-refresh/PRD.md`
- `docs/adr/0003-local-filesystem-refresh-targets.md`

## What to build

Deliver the first tracer bullet for **kill switch path**: a service can declare `kill_switches.<env>.path` (POSIX absolute, mutually exclusive with `url`) in local mode, regenerate the SDK, and have the runtime refresh the **kill switch file** from that filesystem location on the existing kill-switch poll interval — without HTTP hosting and without restart.

Implement shared file-refresh infrastructure (mtime + size check, interval poll with jitter, last-good on missing file / I/O error / invalid bytes) in the TypeScript runtime so the artifact slice can reuse it.

End-to-end behavior:

1. **Catalog validation** rejects `path` when `mode: saas`, rejects `path` and `url` together on the same target, rejects relative and non-POSIX paths.
2. **`controlpath validate`** and **`generate-sdk`** accept a catalog with `kill_switches.production.path: /mnt/flags/production.kill-switches.json`.
3. Generated SDK embeds **kill switch path** map (alongside existing **kill switch URL** map).
4. After `init({ artifact })`, when a **kill switch path** exists for the loaded environment, the runtime polls that path on the kill-switch timer. Unchanged `mtime`/size skips read. Successful read hot-swaps in-memory kill switch state. Failed read keeps last-good and logs a warning.
5. **Refresh-only** — no bundled kill switch at init; first successful poll loads state (same as **kill switch URL** today).

Publishers should replace files atomically (write-then-rename); invalid bytes after read keep last-good (no stable-size gate).

## Acceptance criteria

- [ ] Schema and compiler validation: `kill_switches.<env>.path` XOR `url`; POSIX absolute only; invalid in SaaS mode
- [ ] `SdkCatalog` and SDK generator embed kill switch paths for local mode
- [ ] Runtime file refresh coordinator with mtime/size skip and last-good failure semantics
- [ ] `GeneratedEvaluatorRuntime` starts kill-switch file poll when **kill switch path** configured for loaded environment
- [ ] Runtime unit tests: unchanged file skipped, updated file hot-swapped, missing file keeps last-good, invalid JSON keeps last-good
- [ ] CLI integration test: temp project with **kill switch path**, write file, verify evaluation reflects kill switch override without restart
- [ ] `cargo test`, runtime `npm test`, and relevant clippy/fmt gates pass

## Blocked by

None — can start immediately
