Status: ready-for-agent

# Artifact path refresh

## Parent

- `.scratch/local-filesystem-refresh/PRD.md`
- `docs/adr/0003-local-filesystem-refresh-targets.md`

## What to build

Deliver the second tracer bullet for **artifact path**: a service can declare `artifacts.<env>.path` (POSIX absolute, mutually exclusive with `url`) in local mode, regenerate the SDK, and have the runtime refresh the **compiled artifact** from that filesystem location on the existing artifact poll interval — without HTTP hosting and without restart.

Reuse file-refresh infrastructure from the kill switch slice. **Refresh-only**: cold start still loads via `init({ artifact })` (bundled path, mount path, or URL); configured **artifact path** may differ from the init source (bundled `.ast` in image, live rules on volume).

End-to-end behavior:

1. **Catalog validation** for `artifacts.<env>.path` with same rules as kill switch paths (POSIX absolute, XOR `url`, invalid in SaaS mode).
2. **`generate-sdk`** embeds **artifact path** map alongside **artifact URL** map.
3. After init, when **artifact path** exists for the loaded environment, runtime polls on the artifact timer with mtime/size skip. Successful read runs env/overlap guardrails before hot-swap. Rejected guardrail or invalid bytes keep last-good.
4. **Init guardrails** (environment match, flag-name overlap with generated SDK) run at `init({ artifact })` when either **artifact URL** or **artifact path** is configured for that environment — extend existing `shouldValidateArtifactAtInit` logic accordingly.
5. Independent artifact and kill-switch poll loops unchanged.

## Acceptance criteria

- [ ] Schema and compiler validation: `artifacts.<env>.path` XOR `url`; POSIX absolute only; invalid in SaaS mode
- [ ] `SdkCatalog` and SDK generator embed artifact paths for local mode
- [ ] Runtime artifact file refresh reusing shared file-poll helpers; same guardrails as URL refresh before hot-swap
- [ ] Init guardrails trigger when **artifact URL** or **artifact path** configured for environment
- [ ] Runtime unit tests: hot-swap on file change, guardrail rejection keeps last-good, init guardrails with path configured, bundled init + different refresh path
- [ ] CLI integration test: compile artifact, init from bundled path, write updated artifact to configured **artifact path**, verify flag evaluation updates on poll without restart
- [ ] `cargo test`, runtime `npm test`, and relevant clippy/fmt gates pass

## Blocked by

- `.scratch/local-filesystem-refresh/issues/01-kill-switch-path-refresh.md`
