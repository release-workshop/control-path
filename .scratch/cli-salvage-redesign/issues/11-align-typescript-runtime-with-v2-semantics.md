# Align TypeScript runtime with v2 boolean and kill switch semantics

Status: done
Type: AFK

## What to build

Update `runtime/typescript/` and the generated SDK integration so application runtimes match the v2 product model: boolean-only evaluation, kill switch files (not “override files”), and the correct evaluation order.

Issue 04 generates the typed SDK; this slice makes the runtime the generated SDK actually calls into.

## Contract (from issue 01)

See “Kill switch files” and evaluation order in `.scratch/cli-salvage-redesign/schema-decisions.md` and `CONTEXT.md`.

- **Evaluation order:** kill switch file → AST → catalog default (listed kill-switch flags skip rule evaluation)
- **Kill switch file shape:** boolean map — see `schemas/examples/production.kill-switches.json` (formal schema deferred to issue 09 or follow-up)
- **Kill switch URL (local mode):** from `kill_switches.<env>.url` in catalog, embedded in generated SDK constants — not ad hoc `overrideUrl` init config
- **Boolean-only:** no multivariate rules, variations, or variant selection in the public runtime API
- **Imported flags:** evaluate using merged projections from issue 07 (method names/namespaces from generated SDK)

## Acceptance criteria

- [x] Replace override-file loader/API with kill switch file loader (`loadKillSwitchFromFile`, `loadKillSwitchFromURL`, etc.) accepting the v2 boolean-map format.
- [x] Generated SDK (from issue 04) wires kill switch polling using per-environment URLs from the catalog; removes `overrideUrl`-style manual init where catalog declares URLs.
- [x] Evaluator applies kill switch → AST → catalog default; listed flags skip AST rule evaluation.
- [x] Multivariate/variation rule types and selection logic are removed from the runtime public surface.
- [x] Generated SDK template (`index.ts.tera`) delegates to the updated runtime; boolean-only method signatures only.
- [x] Tests cover kill switch override, AST rule match, default fallback, imported-namespace flags, and 304/polling behavior for remote kill switch files.

## Deliverables

- Updated `runtime/typescript/` modules, types, and tests
- Generated SDK template changes if required for kill switch URL wiring
- Runtime README aligned with v2 terminology

## Blocked by

- `.scratch/cli-salvage-redesign/issues/03-compile-local-boolean-rules-to-ast.md`
- `.scratch/cli-salvage-redesign/issues/04-generate-typescript-sdk-from-catalog-imports.md`
- `.scratch/cli-salvage-redesign/issues/07-add-imported-global-catalog-behavior.md`

## Unblocks

- `.scratch/cli-salvage-redesign/issues/09-prune-legacy-multivariate-surfaces.md` (runtime legacy cleanup)
- `.scratch/cli-salvage-redesign/issues/10-restore-minimal-explain.md` (shared evaluation semantics)
