# Add SaaS-mode catalog sync boundary with fakes

Status: done
Type: AFK

## What to build

Add the SaaS-mode boundary for catalog sync and remote-compiled AST download, using a fake or abstract API client for now. SaaS mode should make the repo-owned flag catalog the source of truth for flags and metadata, while the SaaS owns environment rules and preserves telemetry/history.

This slice should not require a real SaaS backend, but it should lock down the CLI/domain behavior at the boundary.

## Contract (from issue 01)

See “SaaS mode” and “Catalog vs rules vs telemetry” in `.scratch/cli-salvage-redesign/schema-decisions.md`. Fixture: `schemas/examples/saas.control-path.yaml`.

- `mode: saas` requires `saas.project`; optional `saas.api_url` (sync API) and `saas.cdn_url` (SDK poll origin) for self-host
- Repo owns flags + declared metadata; SaaS owns environment rules, telemetry, and CDN-hosted kill switches
- No `environments`, `segments`, or `kill_switches` in Git for SaaS mode
- Multi-repo SaaS catalogs declare `catalog.namespace`; monorepos may omit it and rely on workspace namespace
- Removing a flag from Git retires it in SaaS (history preserved)

## Acceptance criteria

- [x] SaaS mode rejects local `environments`, `segments`, and `kill_switches` (validation from issue 02; CLI surfaces clear errors).
- [x] Catalog sync sends flag catalog changes through an API abstraction without syncing telemetry into Git.
- [x] Removing a flag from Git retires it through the API abstraction rather than hard-deleting history.
- [x] Downloaded remote-compiled AST artifacts are represented by a tested API boundary.
- [x] SaaS-mode CI/validate succeeds without local environment rules (catalog-only validation + sync boundary).
- [x] Tests cover local-rules rejection, catalog sync, removed-flag retirement, and signed/remote AST download behavior using fakes.

## Blocked by

- `.scratch/cli-salvage-redesign/issues/02-parse-validate-new-catalog-schema.md`
