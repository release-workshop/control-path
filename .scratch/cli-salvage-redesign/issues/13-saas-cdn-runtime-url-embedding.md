# SaaS CDN embedding for artifact and kill switch URLs at SDK generation

Status: done
Type: AFK

## What to build

When `mode: saas`, the generated SDK must embed **artifact URL** and **kill switch URL** entries without Git-declared `artifacts` or `kill_switches` blocks. URLs come from a documented platform CDN path contract (`saas.project`, effective catalog identity, environment) for each environment with a `.controlpath/<env>.ast` on disk at `generate-sdk` time (typically after `controlpath ci` / SaaS sync; see `cdn.rs` for manual-file and prune-on-download caveats).

At runtime, services poll those embedded URLs with the same semantics as issue 12 (independent timers, kill switches faster, ETag, signatures on new bytes only, migration guardrails). Align the fake SaaS client / CLI sync boundary so tests prove URL generation and that polling targets match what sync downloaded.

## Acceptance criteria

- [x] Stable CDN URL builder (compiler or CLI) documented alongside the fake SaaS client; unit tests for path shape per project/env/catalog identity.
- [x] `generate-sdk` in SaaS mode embeds `ARTIFACT_URLS` and completes `KILL_SWITCH_URLS` per on-disk `.controlpath/<env>.ast` (not only local-mode catalog fields).
- [x] Integration or generator tests: after `saas sync` with multiple env ASTs, generated SDK contains expected URL constants; runtime poll tests use mocked fetch (no live CDN).
- [x] `schema-decisions.md` / ADR 0001 cross-links remain accurate for SaaS vs local URL sources.

## Blocked by

- `.scratch/cli-salvage-redesign/issues/12-artifact-urls-local-mode-runtime-poll.md`
- `.scratch/cli-salvage-redesign/issues/06-add-saas-catalog-sync-boundary.md`

## Unblocks

- None required for explain; improves SaaS production parity with local remote hosting.
