# Native v2 compile path (remove legacy JSON shim)

Status: done
Type: AFK

## What to build

Implement **compiled artifact** production directly from `CatalogDocument` + resolved **imports** + **environment** — without `catalog_to_definitions` / `catalog_to_deployment` and without calling legacy `compiler::compile` on v1-shaped JSON.

Deliver in **one PR**: delete the shim, update `lib.rs` exports so catalog compile is the documented path, keep behaviour and golden AST tests equivalent for existing fixtures.

`reason` remains catalog-only (not in AST); document for explain (issue 07).

## Acceptance criteria

- [x] `compile_catalog_with_imports` produces `Artifact` via native lowering (rules, segments, import qualification, kill_switch constraints, trailing default serve).
- [x] Legacy `compiler::compile` path removed or only used by deleted code; no `defaultValue` / v1 field names in catalog compile module.
- [x] Existing catalog compile unit tests and CLI compile integration tests pass without behaviour regression.
- [x] `cargo test --workspace`, clippy, and release build pass.

## Blocked by

- `.scratch/platform-spine/issues/01-validation-modes-replace-skip-validation.md`

## Unblocks

- `.scratch/platform-spine/issues/07-shared-explain-evaluation-trace.md`
