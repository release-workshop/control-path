# Prune legacy and multivariate surfaces

Status: done
Type: AFK

## What to build

Remove obsolete public surfaces that conflict with the redesigned product model. The codebase should no longer carry the old split-file authoring model, duplicate workflow command paths, or multivariate/variant/experiment concepts in CLI, schema, SDK, docs, or tests.

This slice makes the product boundary crisp: boolean feature control, not experimentation and not dynamic configuration.

## Contract (from issue 01)

v2 is the public schema; v1 remains only until this slice removes it. See “Salvage from v1” in `.scratch/cli-salvage-redesign/schema-decisions.md` for the full drop list.

Artifacts to retire or replace:

- `schemas/control-path.schema.v1.json` → v2 is canonical
- `schemas/flag-definitions.schema.v1.json`, `schemas/flag-deployment.schema.v1.json` → absorbed into v2
- Compiler embed in `crates/compiler/src/schemas.rs` (currently v1)
- CLI paths for `flags.definitions.yaml`, `.controlpath/*.deployment.yaml`, v1 `extract_definitions`/`extract_deployment` shims
- Runtime `override-loader`, `OverrideFile` types, and multivariate evaluator paths (v2 replacements land in issue 11)
- “Override file” product language → **kill switch file** in CLI, runtime, and docs (`docs/override-*.md`)

Follow-up (optional in this slice or separate): add `schemas/kill-switch-file.schema.v2.json` per schema-decisions.

## Acceptance criteria

- [x] Legacy `flags.definitions.yaml` and `.deployment.yaml` authoring support is removed from the public CLI path.
- [x] v1 unified/split schemas are removed or internal-only; v2 schema is the public contract.
- [x] Multivariate, variation, and experiment concepts are removed from the public schema, CLI, SDK, docs, and tests.
- [x] Duplicate command paths from the prototype workflow are removed or redirected to the coherent nested command vocabulary (`init`, typed `flag`/`env` subcommands).
- [x] Stale docs and tests are updated to the boolean-only v2 catalog model and examples in `schemas/examples/`.
- [x] The remaining compatibility surface is intentional and covered by tests.

## Blocked by

- `.scratch/cli-salvage-redesign/issues/05-rebuild-local-workflow-cli.md`
- `.scratch/cli-salvage-redesign/issues/07-add-imported-global-catalog-behavior.md`
- `.scratch/cli-salvage-redesign/issues/11-align-typescript-runtime-with-v2-semantics.md`
