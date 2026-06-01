# Compile local boolean rules to the existing AST artifact

Status: done
Type: AFK

## What to build

Compile local-mode boolean environment rules from the v2 catalog schema into the existing AST artifact contract. Local rules should be ordered, support optional targeting and boolean percentage rollout, and fall back to the catalog default when no rule matches.

This slice proves that the redesigned schema can still produce the runtime artifact applications already consume.

## Contract (from issue 01)

Local rule shape is top-level `environments.<env>.rules.<flag>` with ordered rule arrays. See `schemas/examples/local-only.control-path.yaml` and the “Environment rules (local mode)” section of `.scratch/cli-salvage-redesign/schema-decisions.md`.

- `when`, `serve`, `rollout` (boolean-only, embedded `serve`), optional `reason`
- Reusable `segments` for local mode
- First match wins; no match → catalog `default`
- SaaS-mode catalogs have no local environments to compile (handled in issue 06)

## Acceptance criteria

- [x] Local mode projects compile `.controlpath/<env>.ast` artifacts from the v2 typed catalog (not v1 array/per-flag environments). **Compiler API done** (`compile_catalog`, `validate_and_compile_catalog`, `load_validate_and_compile_catalog`); CLI wiring is issue 05.
- [x] Missing environment rules fall back to the flag catalog default.
- [x] Ordered boolean rules support `when`, `rollout`, `serve`, and optional `reason` (reason is source metadata only; not in AST).
- [x] Top-level `segments` compile into the deployment/AST projection.
- [x] Percentage rollout remains boolean-only and does not introduce variant allocation semantics.
- [x] Compiler tests verify artifact behavior for default fallback, explicit serve rules, targeted rules, rollout rules, segment references, and deprecated flags.

## Blocked by

- `.scratch/cli-salvage-redesign/issues/02-parse-validate-new-catalog-schema.md`

## Public API

| Function | Use when |
|---|---|
| `load_validate_and_compile_catalog` | Untrusted YAML (parse + validate + compile) |
| `validate_and_compile_catalog` | Typed catalog not yet validated in this pipeline |
| `compile_catalog` | Catalog already validated; also rejects empty rules and out-of-range rollout % at compile time |
