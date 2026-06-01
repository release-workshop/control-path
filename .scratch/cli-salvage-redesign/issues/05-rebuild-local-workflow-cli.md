# Rebuild local workflow CLI around typed operations

Status: done
Type: AFK

## What to build

Rebuild the workflow-first local CLI on top of typed catalog operations. The command surface should support initializing a project, managing flags and environments, generating the SDK, deploying local AST artifacts, running CI checks, and the local dev loop — all against the v2 schema.

This slice should replace the prototype command flow with a coherent local/free path while preserving the existing AST artifact contract.

## Contract (from issue 01)

Init/scaffold behaviour is defined in `.scratch/cli-salvage-redesign/schema-decisions.md` (“Monorepo workspace file”, “`controlpath init` behaviour”):

- Monorepo root → `control-path.workspace.yaml` (namespace + optional `scaffold` boilerplate)
- Service folder → walk up for workspace; scaffold `control-path.yaml` from workspace `scaffold` (copy, not merge)
- Multi-repo → `control-path.yaml` only; prompt for `catalog.namespace`

Local incident workflow uses **kill switch files** (v2 product term; replaces “override file”):

- `deploy` / `ci` write `.controlpath/<env>.kill-switches.json` alongside `.controlpath/<env>.ast`
- Git declares URLs in `kill_switches.<env>.url` (local mode only)
- Example output shape: `schemas/examples/production.kill-switches.json`

v2 environments are top-level `environments.<name>` — `env` commands mutate that block, not per-flag nested envs or `.controlpath/*.deployment.yaml` files.

## Acceptance criteria

### Core workflow commands

- [x] The CLI supports `init`, `flag add`, `flag enable`, `flag deprecate`, `flag remove`, `sdk generate`, `deploy`, and `ci` for local mode against the v2 catalog shape.
- [x] `init` prompts for monorepo vs multi-repo setup; at monorepo root creates `control-path.workspace.yaml`; in a service folder walks up for workspace and scaffolds `control-path.yaml` from workspace boilerplate.
- [x] Standalone `validate` and `compile` work against the v2 typed catalog (also invoked by `deploy` and `ci`).

### Environment and dev loop

- [x] `env add`, `env sync`, `env list`, and `env remove` operate on top-level v2 `environments` (not v1 per-flag envs or `.controlpath/*.deployment.yaml`).
- [x] `dev` and `watch` use v2 `control-path.yaml` only (no `flags.definitions.yaml` fallback); changes trigger recompile and/or SDK regeneration as today.

### Kill switches and typed operations

- [x] Command handlers use typed operations and centralized output/error behavior rather than direct raw config mutation or v1 `extract_definitions`/`extract_deployment` shims.
- [x] `flag deprecate` sets `lifecycle: deprecated`; deprecated flags warn and block new rule changes unless explicitly forced.
- [x] `deploy` writes AST artifacts and kill-switch JSON through the typed compiler projection.
- [x] `kill-switch` commands (rename from `override`) update local kill-switch state and regenerate deploy artifacts (no live boolean values in Git).

### Tests

- [x] Integration tests cover the full local workflow from init through env management, SDK generation, AST deploy, kill-switch artifact output; v2 watch regen (`integration_watch::test_watch_v2_regenerates_sdk_on_catalog_change`) plus unit coverage for v2 watch/dev paths.

### Imported flags (issue 07 stance)

- [x] Imported flags are typed in generated SDK; Evaluator methods are local-only until AST merge (issue 07). Documented in generated `index.ts` header.

## Blocked by

- `.scratch/cli-salvage-redesign/issues/02-parse-validate-new-catalog-schema.md`
- `.scratch/cli-salvage-redesign/issues/03-compile-local-boolean-rules-to-ast.md`
- `.scratch/cli-salvage-redesign/issues/04-generate-typescript-sdk-from-catalog-imports.md`

## Unblocks

- `.scratch/cli-salvage-redesign/issues/08-add-lifecycle-rot-reporting-surface.md`
- `.scratch/cli-salvage-redesign/issues/09-prune-legacy-multivariate-surfaces.md`
- `.scratch/cli-salvage-redesign/issues/10-restore-minimal-explain.md`
