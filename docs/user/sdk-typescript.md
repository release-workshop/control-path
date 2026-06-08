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

### Evaluation attributes and generated types

Pass a single **evaluation attributes** object (for example `{ id: 'user-42', role: 'admin', plan: 'beta' }`). Rule `when` clauses read properties from this object. Stable `id` is required for consistent rollout bucketing.

- Field names, expression syntax, strict validation, and `explain --attributes`: [`rules.md`](rules.md#evaluation-attributes)
- Declaring fields in the catalog: [`configuration.md`](configuration.md#attribute-schema-attributes)

#### Legacy catalogs (no `attributes:`)

When the service catalog omits `attributes:`, generated `types.ts` exports:

```typescript
export interface Attributes extends BaseAttributes {
  [key: string]: unknown;
}
```

`BaseAttributes` (`id`, `email`, `role`, `environment`, `device`, `app_version`, …) comes from `@controlpath/runtime` and is not duplicated in generated files.

#### Opted-in catalogs (`attributes:` present)

When the service declares `attributes:` (including `{}`), `generate-sdk` emits a **closed** schema:

```typescript
import type { BaseAttributes } from '@controlpath/runtime';

/** Imported `platform` catalog fields. */
export interface PlatformAttributes {
  org_tier?: string;
}

/** Closed evaluation attributes for this service. */
export interface Attributes extends BaseAttributes {
  plan?: string;
  platform?: PlatformAttributes;
}

export type EvaluationAttributes = Attributes;
```

- Service-local fields sit at the top level beside **base attributes**.
- Each **import namespace** with an `attributes:` map becomes an optional nested object (`platform?: PlatformAttributes`).
- There is **no index signature** on opted-in `Attributes`.

#### Per-flag attribute types

Each generated flag method uses a **per-flag attribute type** derived from catalog ownership only — not from **environment rules** (those can change via **compiled artifact** or SaaS without regenerating the SDK):

| Flag | Typical generated parameter type |
| --- | --- |
| Local `newDashboard` | `BaseAttributes & { plan?: string; platform?: PlatformAttributes }` (full service schema) |
| Imported `platformOrgGoldFeature` | `BaseAttributes & { platform?: PlatformAttributes }` (that import namespace only) |

**Superset calling convention:** pass the full `Attributes` object (or any structural superset with the required fields present). TypeScript accepts wider objects; it rejects call sites that omit required namespace shape for an imported flag.

```typescript
// OK — full service object
await evaluator.newDashboard({ id: 'u1', plan: 'beta', platform: { org_tier: 'gold' } });

// OK — superset for imported flag (extra top-level fields allowed)
await evaluator.platformOrgGoldFeature({
  id: 'u1',
  plan: 'beta',
  platform: { org_tier: 'gold' },
});

// Type error — imported flag expects declared namespace fields
await evaluator.platformOrgGoldFeature({
  id: 'u1',
  platform: { not_declared: true },
});
```

Regenerate the SDK (`controlpath generate-sdk`) when the **flag catalog** or `attributes:` map changes. **Environment rules**-only updates do not require an SDK rebuild — publish a new artifact instead ([Deployment velocities](#deployment-velocities)).

Generated runtime supports:

- artifact polling and refresh (via embedded `ARTIFACT_URLS`)
- kill switch polling and refresh (via `KILL_SWITCH_URLS` and `KILL_SWITCH_PATHS`)
- evaluation order: kill switch -> artifact rules -> catalog default

## Runtime behavior notes

- `init({ artifact })` seeds runtime state from the **compiled artifact** only. Kill switch state loads on the first successful poll when a **kill switch URL** or **kill switch path** is configured for the artifact environment.
- Re-running `init()` without a new artifact keeps existing loaded state and restarts polling.
- Failed refresh (missing file, network error, invalid bytes, rejected artifact guardrails) keeps **last-good** state; check application logs for warnings.
- Artifact and kill switch polling run on separate timers with init jitter and per-tick interval jitter.

### Filesystem refresh (`path`)

Catalog `kill_switches.<env>.path` and `artifacts.<env>.path` are POSIX absolute refresh targets (mutually exclusive with `url` on the same entry). See [`configuration.md`](configuration.md#refresh-targets-local-mode).

| Target | Embedded in generated SDK | Poll behavior |
| --- | --- | --- |
| **Kill switch path** | `KILL_SWITCH_PATHS` | mtime + size check; hot-swap without restart |
| **Kill switch URL** | `KILL_SWITCH_URLS` | conditional GET with ETag when available |
| **Artifact URL** | `ARTIFACT_URLS` | conditional GET with ETag when available |
| **Artifact path** | (not yet embedded) | Same filesystem model as kill switch path — validate in catalog today; SDK `ARTIFACT_PATHS` polling ships separately |

For **kill switch path**, regenerate the SDK after changing `control-path.yaml`, then place or atomically replace the JSON at the configured path; running pods pick it up on the kill-switch poll interval.

For **rules-only** updates today, publish `.controlpath/<env>.ast` to each environment’s **artifact URL** (or re-run `init({ artifact })` when not using URL polling). When **artifact path** is embedded in the SDK, placement at `artifacts.<env>.path` will hot-swap rules the same way as URL polling.

## Deployment velocities

- Catalog changes: regenerate SDK and redeploy app.
- Rules-only changes: publish new **compiled artifact** to the environment’s **artifact URL** (or **artifact path** once SDK embedding is available).
- Incident toggles: update kill switch file at **kill switch URL** or **kill switch path** for faster propagation (no app restart when refresh targets are configured).

## See also

- [`configuration.md`](configuration.md)
- [`rules.md`](rules.md)
- [`kill-switches.md`](kill-switches.md)
- [`runtime/typescript/README.md`](../../runtime/typescript/README.md)
