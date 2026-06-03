# Evaluation attribute schema and namespaced typing

Status: accepted

Services pass a single **evaluation attributes** object at flag evaluation time. Today the generated SDK exposes a loose `Attributes` type (platform base fields plus `[key: string]: unknown`). Catalogs cannot declare service-specific fields, and compile cannot reject typos in rule property references.

## Decision

1. **Catalog config:** Optional top-level `attributes:` map on each flag catalog (service and shared). Scalar types only in v1: `string`, `number`, `boolean`. Omitting `attributes:` preserves legacy behavior (loose SDK type, no property-name validation on rules). Declaring `attributes:` (including `{}`) opts in to strict mode for that catalog scope.

2. **Base vs catalog fields:** **Base attributes** (`id`, `email`, `role`, `environment`, `device`, `app_version`, …) are platform-owned, exported from `@controlpath/runtime` as `BaseAttributes`. Catalog `attributes:` must not redeclare base names (validation error). Generated SDK extends `BaseAttributes`; the generator must not duplicate the base field list.

3. **Imports and namespacing:** Each imported catalog may declare its own `attributes:`. At runtime, imported fields live under that catalog’s **import namespace** (e.g. `{ platform: { org_tier: 'gold' } }`). Shared-catalog authors write bare names in rules (`org_tier`); compile rewrites property paths in merged artifacts to `namespace.field`. Service-local rules use bare top-level names (`plan`).

4. **Strict property validation:** When opted in, unknown property names in rule expressions fail validation (errors, not warnings). **Local mode:** validate **environment rules** and **segments** in the service repo against base ∪ service schema; validate imported catalogs in their source files against base ∪ that catalog’s schema. **SaaS mode:** service-side validation covers catalog shape, flags, and imports only — remote **environment rules** are validated where they are authored, not in the service repo.

5. **SDK typing (Git-stable only):** Types must not derive from **environment rules** (rules can change via **compiled artifact** or SaaS without `generate-sdk`). When opted in, emit a closed `Attributes` type (no index signature): service fields at top level plus nested namespace objects from imports. **Per-flag attribute types** come from catalog ownership: local flags accept base ∪ full service schema; imported flags accept base ∪ that flag’s namespace object. Callers may pass a structural superset, never a narrower object.

6. **CLI / explain:** Evaluation input is `--attributes` (JSON file or inline), matching the runtime object shape.

7. **Init:** `controlpath init` does not scaffold `attributes:` by default.

## Considered options

- **Flat merged attribute schema across imports:** Rejected; collisions and weak typing for wrong-namespace mistakes.
- **Qualified property names in shared-catalog rule strings (`platform.org_tier`):** Rejected for author DX; compile rewrite preferred.
- **Rule-derived per-flag TypeScript minimums:** Rejected; **environment rules** change without SDK rebuild (local artifact deploy and SaaS).
- **Warn-only on unknown rule properties:** Rejected; opted-in catalogs use errors.
- **Separate runtime `user` and `context` bags:** Rejected; single **evaluation attributes** object (see prior CLI merge).

## Consequences

- v2 JSON Schema, catalog model, validator, expression property extractor, compiler string-table rewrite, SDK generator, and runtime package all gain coordinated work (see `.scratch/evaluation-attribute-schema/`).
- `@controlpath/runtime` `Attributes` may remain a loose superset for internal evaluation; generated service SDKs use closed types when opted in.
- Platform/shared-catalog repos should adopt `attributes:` before services opt in and reference imported fields under namespace keys.
- Future v2 extensions (nested object schema, required-field markers, enum unions) are out of scope for v1 and need a follow-up ADR if added.
