# Thin generated SDK and deep TypeScript runtime (breaking)

Status: ready-for-agent
Type: AFK

## What to build

Shrink the generated TypeScript SDK to data + thin `init` wiring; move poll orchestration, artifact hot-swap, init validation, and shared evaluation helpers into `@controlpath/runtime`.

Bump **runtime package version** (breaking change; no external consumers yet). Update Tera template and any generated export names accordingly. Generated SDK depends on the new runtime version.

Consolidate duplicate poll logic between artifact and **kill switch URL** loaders where safe while keeping **independent timers** per ADR-0001.

## Acceptance criteria

- [ ] Generated `index.ts` is mostly constants (`FLAGS`, `ARTIFACT_URLS`, `KILL_SWITCH_URLS`, intervals) plus delegation to runtime.
- [ ] `runtime/typescript`: `npm run lint`, `npm run typecheck`, `npm test` pass; version bumped in `package.json`.
- [ ] CLI generator tests / `integration_saas` reflect new generated shape; ETag / 304 artifact poll behaviour covered in runtime tests.
- [ ] ADR-0001 consequences still satisfied (separate poll intervals, migration guardrails, signature on new bytes only).

## Blocked by

- `.scratch/platform-spine/issues/04-unify-generate-sdk-command.md`
- `.scratch/platform-spine/issues/05-saas-runtime-url-seam.md`

## Unblocks

None — terminal slice for this initiative.
