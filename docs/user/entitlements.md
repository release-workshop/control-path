# Entitlement flags

**Entitlement** flags (`kind: entitlement`) are long-lived **access gates**: they answer whether a principal may use a product capability, based on **evaluation attributes** your application passes at SDK call time (plan tier, `role`, and other fields from your **attribute schema**).

They are not rollout flags and not incident toggles. Industry practice treats entitlements as durable commercial or policy gates, separate from **`kind: release`** (gradual ship) and companion **`kind: kill_switch`** flags (incidents). See [`docs/adr/0004-entitlement-flag-semantics.md`](../adr/0004-entitlement-flag-semantics.md) and the **Entitlement** glossary entry in [`CONTEXT.md`](../../CONTEXT.md).

Do not use informal “enablement” terminology — **`kind: entitlement`** is the canonical catalog marker.

## What entitlements decide

Control Path evaluates entitlements like any boolean flag: **kill switch file** → **compiled artifact** rules → catalog **`default`**. The application supplies attributes (JWT claims, session, billing lookup, etc.); **how** those attributes are populated is out of scope for Control Path.

Typical attribute sources in rules:

| Source | Examples in `when` |
| --- | --- |
| **Base attributes** | `role`, `id`, `environment` ([`schemas/base-attributes.json`](../../schemas/base-attributes.json)) |
| **Service attribute schema** | `plan`, `org_tier` declared under `attributes:` in the catalog |
| **Imported catalog fields** | Bare names in the **source** catalog; runtime JSON nests under the import namespace (see [`rules.md`](rules.md#bare-names-in-rules-vs-namespaced-runtime-json)) |

**Permission** (RBAC) is not a separate flag kind. Role claims appear as `role` (or other base fields) inside entitlement `when` rules alongside commercial attributes — e.g. org purchased Pro **and** the user’s role may use export.

## Fail-closed authoring

- Prefer **`default: false`** so unmatched cases **deny** access.
- Missing attributes make `when` expressions **false** (standard rule walk).
- `controlpath validate` **warns** when `kind: entitlement` has **`default: true`** (strict CI may treat warnings as errors). Some deny-list patterns intentionally use `default: true`; treat the warning as a review signal, not a hard block.

Omitting **`expires`** on an entitlement is normal — no warning. When set, `expires` marks a planned trial or SKU sunset in **declared metadata**, not rollout cleanup. **`kind: release`** may warn when `expires` is missing; entitlements do not.

Declare commercial fields under **`attributes:`** when you use them in `when` expressions so `validate` and `compile` enforce property names ([`configuration.md` — Attribute schema](configuration.md#attribute-schema-attributes)).

## Environment rules for entitlements

| Allowed | Forbidden |
| --- | --- |
| `when` + `serve` | `rollout` (use a separate **`kind: release`** flag) |
| Plain `serve` (catch-all or default-off tail rule) | Percentage “access” via rollout |

Example (plan + role, with attribute schema):

```yaml
attributes:
  plan: string

flags:
  premium_checkout:
    kind: entitlement
    default: false
    owner: team-checkout
    description: Premium checkout experience
    expires: 2027-12-31   # optional trial / SKU sunset

environments:
  production:
    rules:
      premium_checkout:
        - when: "plan == 'pro' AND role IN ['admin', 'billing']"
          serve: true
        - serve: false
          reason: Deny when plan or role does not match
```

The explicit `serve: false` tail is optional when `default: false` (compile also appends a catalog-default rule) but helps `explain` and reviewers see deny intent. Use bare names (`plan`, `role`) in new rules. Rule ordering and expressions: [`rules.md`](rules.md).

## Composition at the call site

Use **separate flags** and **AND** evaluations in application code. Control Path does not merge release, entitlement, and kill-switch layers into one flag.

### Gradual ship: entitlement + release

Shipping UI or behavior for a capability users may already be entitled to:

1. **`kind: entitlement`** — who may use the capability (often in a **shared catalog**; rules in the source file only).
2. **`kind: release`** — rollout (`rollout` or beta `when` rules) in the **service** catalog for the experience layer.

The schema examples split these across files on purpose. In [`imported-global.control-path.yaml`](../../schemas/examples/imported-global.control-path.yaml), `checkout-service` imports `platform` and defines local `premium_export_ui`; entitlement rules live in [`shared-platform.control-path.yaml`](../../schemas/examples/shared-platform.control-path.yaml). Generated SDK methods use import prefixes:

```typescript
// checkout-service with imports.platform → shared-platform catalog
const entitled = await sdk.platformPremiumExport(attributes);
const uiOn = await sdk.premiumExportUi(attributes);
const enabled = entitled && uiOn;
```

Remove the **release** flag and its rules after rollout finishes; keep the **entitlement** for the life of the plan.

### Incidents: entitlement + kill switch

Disabling a paid feature during an incident without editing commercial rules:

1. Keep **`kind: entitlement`** rules as the access source of truth in the **compiled artifact**.
2. Add a companion **`kind: kill_switch`** (e.g. `platform.premium_export_kill` when the entitlement is imported).
3. Toggle the kill switch via the **kill switch file**, SaaS dashboard, or `controlpath kill-switch set` — **not** on the entitlement flag name (`set` requires `kind: kill_switch`).

```typescript
const entitled = await sdk.platformPremiumExport(attributes);
const notKilled = !(await sdk.platformPremiumExportKill(attributes));
const enabled = entitled && notKilled;
```

Details: [`kill-switches.md`](kill-switches.md).

## Shared catalogs for plan-wide entitlements

Capabilities that span checkout, analytics, billing, and other services belong in a **shared catalog** imported by each service:

```yaml
# service control-path.yaml
imports:
  platform:
    path: ../../platform/control-path.yaml
```

- SDK and `explain` use qualified names (`platform.premium_export`).
- **Environment rules** for imported entitlement flags are authored **only** in the **source** catalog. Putting rules for `platform.*` flags in a consumer catalog fails validation.

### Schema examples (illustrative siblings)

The repo examples are **teaching catalogs**, not one wired product feature:

| Catalog | Illustrates |
| --- | --- |
| [`shared-platform.control-path.yaml`](../../schemas/examples/shared-platform.control-path.yaml) | Platform entitlement (`premium_export`, role-only rules) + companion kill switch; rules authored in the source file |
| [`imported-global.control-path.yaml`](../../schemas/examples/imported-global.control-path.yaml) | Consumer import + local `premium_export_ui` **release** rollout |
| [`local-only.control-path.yaml`](../../schemas/examples/local-only.control-path.yaml) | Single-service entitlement (`premium_checkout`) with `attributes.plan` and plan + role rules |

`premium_export` (platform) and `premium_checkout` (checkout-service) are **different flag names** showing different authoring shapes — do not assume they compose unless your app imports the platform catalog and ANDs the flags you define.

## Validate examples

Examples are covered by compiler tests (`example_*_catalog_is_valid`). To validate locally, run `controlpath validate` from a directory that contains `control-path.yaml` (copy or symlink an example catalog). Import examples need the platform file at the path declared in `imports` (see [`configuration.md`](configuration.md#imports-and-shared-catalogs)).

## Related docs

- Catalog structure and flag kinds: [`configuration.md`](configuration.md)
- Rule syntax and attributes: [`rules.md`](rules.md)
- Kill switches and CLI: [`kill-switches.md`](kill-switches.md), [`cli.md`](cli.md)
- Quickstart workflow: [`quickstart.md`](quickstart.md)
- TypeScript SDK: [`sdk-typescript.md`](sdk-typescript.md)
