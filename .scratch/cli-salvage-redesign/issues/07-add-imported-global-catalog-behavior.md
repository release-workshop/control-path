# Add import and global catalog behavior end-to-end

Status: done
Type: AFK

## What to build

Implement end-to-end support for explicitly imported/shared catalogs. Service catalogs should be able to consume global catalogs from local files or declared references, validate namespaces and collisions, and expose imported flags through SDK and compiler projections.

Consuming services must not define environment rules for imported flags. Rules for imported flags live in the source catalog only. There is no `overridable` field — that v1 concept is dropped.

## Contract (from issue 01)

See “Imports and shared catalogs” in `.scratch/cli-salvage-redesign/schema-decisions.md`. Fixtures:

- `schemas/examples/shared-platform.control-path.yaml` — source catalog with its own environment rules
- `schemas/examples/imported-global.control-path.yaml` — consuming service; rules for imported flags are a validation error

Import namespace is the explicit map key in `imports` (not inferred from path).

## Acceptance criteria

- [x] Catalog imports resolve deterministically from explicit `imports.<namespace>.path` declarations.
- [x] Imported catalogs require stable namespaces and duplicate namespaces are validation errors.
- [x] Local flag keys cannot conflict with import namespace prefixes.
- [x] Generated SDKs include imported flags under their import namespaces.
- [x] Validation rejects environment rules targeting imported flags in consuming service catalogs.
- [x] Compiler/AST projection includes imported flags from source catalogs where applicable.
- [x] Tests cover valid imports, namespace collisions, SDK generation with imports, and rejected imported-flag rules.

## Unblocks

- `.scratch/cli-salvage-redesign/issues/11-align-typescript-runtime-with-v2-semantics.md`

## Blocked by

- `.scratch/cli-salvage-redesign/issues/02-parse-validate-new-catalog-schema.md`
- `.scratch/cli-salvage-redesign/issues/04-generate-typescript-sdk-from-catalog-imports.md`
