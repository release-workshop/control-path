# Generate TypeScript SDK from catalog and imports

Status: done
Type: AFK

## What to build

Generate the TypeScript SDK from the boolean flag catalog and any explicitly imported catalogs. SDK shape should depend only on flag catalog data, not environment rules or SaaS targeting state.

Imported flags are exposed under their declared import namespace (the import map key, e.g. `platform.emergency_kill_switch`) so application code has one typed view of all flags it may evaluate.

## Contract (from issue 01)

See “Imports and shared catalogs” in `.scratch/cli-salvage-redesign/schema-decisions.md`. Fixtures: `schemas/examples/imported-global.control-path.yaml`, `schemas/examples/shared-platform.control-path.yaml`.

- SDK generation reads catalog + imports only — not `environments`, `segments`, or `kill_switches`
- Boolean-only: no `type`, variations, or multivariate API surface
- Effective catalog id `{namespace}.{id}` is for sync/telemetry attribution, not SDK method naming

## Acceptance criteria

- [x] SDK generation reads typed v2 catalog data rather than raw JSON or v1 definitions extraction.
- [x] Generated SDK includes local flags and imported flags under stable import namespaces.
- [x] Environment rules do not affect generated SDK types or public methods.
- [x] Boolean-only output removes variant/multivariate public API concepts.
- [x] Tests cover local-only catalogs, imported catalogs, namespace collisions, deprecated flag annotations, and catalog-only generation without environments.

## Blocked by

- `.scratch/cli-salvage-redesign/issues/02-parse-validate-new-catalog-schema.md`

## Unblocks

- `.scratch/cli-salvage-redesign/issues/05-rebuild-local-workflow-cli.md`
- `.scratch/cli-salvage-redesign/issues/07-add-imported-global-catalog-behavior.md`
- `.scratch/cli-salvage-redesign/issues/11-align-typescript-runtime-with-v2-semantics.md`
