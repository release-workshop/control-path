Status: ready-for-agent

## What to build

When the service catalog opts in (**`attributes:`** present), **`generate-sdk`** emits:

1. Closed **`Attributes`** extending **`BaseAttributes`** — service fields at top level, each import namespace as a nested optional object typed from that imported catalog’s **`attributes:`** (no index signature).
2. **`export type EvaluationAttributes = Attributes`** (or equivalent) for **`setAttributes`**.
3. **Per-flag attribute types** from catalog ownership (Git-stable only):
   - Local flags → **`BaseAttributes`** + full service schema
   - Imported flags → **`BaseAttributes`** + `{ [namespace]?: NamespaceAttributes }` for that flag’s import only
4. Generated flag methods accept their **per-flag attribute type** or a structural superset (standard TypeScript optional-field widening).

Legacy catalogs without **`attributes:`** unchanged (issue 01).

## Acceptance criteria

- [ ] Opt-in service with `attributes: { plan: string }` and a `platform` import yields nested `platform?: { … }` on **`Attributes`**
- [ ] Generated local flag method parameter type includes service schema fields; imported flag method requires the import namespace object type
- [ ] No `[key: string]: unknown` on opt-in **`Attributes`**
- [ ] SDK e2e test: TypeScript compiler rejects an object missing required namespace shape for an imported flag call (where types are structurally narrower)
- [ ] **`cargo test`** / SDK generator tests pass

## Blocked by

- `.scratch/evaluation-attribute-schema/issues/01-base-attributes-in-runtime.md`
- `.scratch/evaluation-attribute-schema/issues/02-attribute-schema-parse-and-validate.md`
- `.scratch/evaluation-attribute-schema/issues/04-imported-namespace-compile-rewrite.md`
