# TypeScript SDK Integration

Control Path's generated SDK provides typed methods per flag and delegates runtime behavior
to `@controlpath/runtime`.

## Install runtime package

```bash
npm install @controlpath/runtime
```

## Generate SDK

From project root:

```bash
controlpath generate-sdk
```

The generated output location depends on CLI configuration and mode.

## Initialize evaluator

Typical flow:

1. Load artifact for target environment.
2. Initialize evaluator runtime.
3. Call generated methods with **evaluation attributes** (one object per request).

### Evaluation attributes

Pass a single attributes object (for example `{ id: 'user-42', role: 'admin', plan: 'beta' }`). Rule `when` clauses read properties from this object. Stable `id` is required for consistent rollout bucketing.

- Field names, expression syntax, and `explain` usage: [`rules.md`](rules.md#evaluation-attributes)
- Generated SDK type: `Attributes` in `types.ts` extends `BaseAttributes` from `@controlpath/runtime` (index signature for extra fields until catalog `attributes:` opts in to closed typing)

Generated runtime supports:

- artifact polling and refresh
- kill switch polling and refresh
- evaluation order: kill switch -> artifact rules -> catalog default

## Runtime behavior notes

- `init({ artifact })` seeds runtime state.
- Re-running `init()` without a new artifact keeps existing loaded state and restarts polling.
- Failed refresh keeps prior good state.
- Artifact and kill switch polling run on separate timers.

## Deployment velocities

- Catalog changes: regenerate SDK and redeploy app.
- Rules-only changes: publish new artifact to artifact URL.
- Incident toggles: update kill switch file for faster propagation.

## See also

- [`configuration.md`](configuration.md)
- [`rules.md`](rules.md)
- [`kill-switches.md`](kill-switches.md)
- [`runtime/typescript/README.md`](../../runtime/typescript/README.md)
