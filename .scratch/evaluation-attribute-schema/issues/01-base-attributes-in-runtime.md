Status: done

## What to build

Introduce **`BaseAttributes`** in `@controlpath/runtime` as the single source of truth for platform-owned evaluation fields (`id`, `email`, `role`, `environment`, `device`, `app_version`, …).

Update **`controlpath generate-sdk`** so catalogs **without** **`attributes:`** still work: generated `types.ts` imports **`BaseAttributes`** from the runtime and exports `interface Attributes extends BaseAttributes { [key: string]: unknown }` (today’s loose behavior, but without duplicating base field literals in the Tera template).

Verify the runtime evaluator still accepts attribute objects at the boundary (structural superset).

## Acceptance criteria

- [x] `@controlpath/runtime` exports `BaseAttributes`; existing `Attributes` remains compatible for non-opt-in consumers
- [x] Generated SDK (no catalog **`attributes:`**) extends **`BaseAttributes`** and keeps the index signature
- [x] Generator template no longer hardcodes duplicate base field definitions
- [x] Runtime TypeScript tests pass (`npm run lint`, `typecheck`, `test` in `runtime/typescript/`)
- [x] Existing SDK generator e2e / integration tests still pass for legacy catalogs

## Blocked by

None — can start immediately
