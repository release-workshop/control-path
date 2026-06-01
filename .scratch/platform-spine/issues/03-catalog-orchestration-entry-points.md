# Catalog orchestration: three load entry points

Status: ready-for-agent
Type: AFK

## What to build

Refactor CLI catalog orchestration so callers use a small set of entry points instead of six overlapping loaders:

1. **SDK generate** — validated **flag catalog** + **imports** → `SdkCatalog` (including SaaS URL embedding hook from issue 05).
2. **Compile** — validated bundle → **compiled artifact** bytes per **environment** (local mode).
3. **Explain / audit** — validated `CatalogBundle` (catalog + imports + SDK projection as needed).

Deduplicate checked vs unchecked import resolution into one implementation. Align `saas/sync` with the same load path (no parallel validate + workspace walk-up).

Optional: accept `&CatalogStore` from issue 02 when the caller already has an in-memory document (watch mode / post-edit regen).

## Acceptance criteria

- [ ] Public CLI catalog API is three entry points (names may differ); old `load_sdk_catalog_unchecked*` variants removed or made `pub(crate)` with no external callers.
- [ ] `saas/sync` uses shared import resolution and validation context building.
- [ ] Table-driven unit tests: local vs SaaS mode, with/without imports, expected errors for missing import path.
- [ ] `compile_catalog_envs` behaviour unchanged for valid fixtures; tests from salvage era still pass.

## Blocked by

- `.scratch/platform-spine/issues/01-validation-modes-replace-skip-validation.md`
- `.scratch/platform-spine/issues/02-catalog-document-store.md`

## Unblocks

- `.scratch/platform-spine/issues/04-unify-generate-sdk-command.md`
- `.scratch/platform-spine/issues/05-saas-runtime-url-seam.md`
- `.scratch/platform-spine/issues/07-shared-explain-evaluation-trace.md`
