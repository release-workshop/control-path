# SaaS runtime URL seam (disk-only environments)

Status: done
Type: AFK

## What to build

Separate **CDN URL construction** (compiler) from **environment discovery** (CLI adapter).

- Compiler: given `SaasProject`, effective **catalog identity**, CDN base, and a list of **environment** names → `BTreeMap` of **artifact URL** and **kill switch URL** per env (`build_saas_runtime_urls` contract unchanged).
- CLI adapter: `FilesystemAstCache` scans `.controlpath/*.ast` (disk-only; no Git env list) and passes env names into `build_sdk_catalog` (or equivalent) so `SdkCatalog` is populated in one step — not mutated after `build_sdk_catalog` returns.

Preserve ADR-0001 behaviour: no `.ast` → fail `generate-sdk` with actionable error; manual stray `.ast` files still embedded until deleted or sync prune.

## Acceptance criteria

- [x] `SdkCatalog` URL fields filled inside compiler SDK build when SaaS inputs provided, or via a single compiler helper called from one CLI place — not `apply_saas_runtime_urls` post-mutation pattern.
- [x] `fake.rs` and `integration_saas.rs` use the same adapter contract; no drift between hard-coded paths and `build_saas_runtime_urls`.
- [x] Unit tests in `cdn_tests.rs` unchanged or extended; CLI test proves multi-env sync → expected embedded URLs.

## Blocked by

- `.scratch/platform-spine/issues/03-catalog-orchestration-entry-points.md`

## Unblocks

- `.scratch/platform-spine/issues/08-thin-generated-sdk-deep-runtime.md`
