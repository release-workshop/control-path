# Configuration

Control Path uses a service catalog file, `control-path.yaml`, as the Git source of truth for boolean flag definitions and (in local mode) environment rules. Monorepos may also have `control-path.workspace.yaml` at the repository root.

Environment rule syntax and expressions: [`rules.md`](rules.md).

## What the catalog contains

| Section | Purpose |
| --- | --- |
| `catalog` | Stable catalog identity (`id`, optional `namespace`) |
| `flags` | Boolean flag definitions (defaults, kind, metadata) |
| `mode` | `local` (rules in Git) or `saas` (rules on platform) |
| `environments` | Local-mode ordered rules per environment |
| `segments` | Local-mode reusable `when` predicates |
| `imports` | Shared catalogs by namespace |
| `attributes` | Optional evaluation attribute schema (scalar types) |
| `artifacts` / `kill_switches` | Local-mode per-environment poll URLs |
| `saas` | SaaS project identity when `mode: saas` |

Tooling also writes under `.controlpath/` (compiled `.ast` files, CLI config). Treat that directory as **generated output**, not hand-edited configuration.

## Mental model

Three change speeds:

- **Flag catalog** (new flags, defaults, kinds, imports): regenerate SDK and redeploy the application.
- **Environment rules** only: publish a new **compiled artifact** for that environment (no SDK rebuild if the catalog unchanged).
- **Kill switches**: update the kill switch file; fastest runtime propagation ([`kill-switches.md`](kill-switches.md)).

## Catalog identity

```yaml
catalog:
  id: checkout-service
  # namespace: acme   # optional; see Monorepos below
```

- **`catalog.id`** (required): stable id for sync, imports, and telemetry.
- **`catalog.namespace`** (optional): prefix for effective id `namespace.id`. In monorepos, namespace often comes from the workspace file instead of each service file.

## Mode: local vs SaaS

```yaml
mode: local   # default
```

| `mode` | Rules in Git | Typical workflow |
| --- | --- | --- |
| `local` | `environments`, `segments`, `artifacts`, `kill_switches` allowed | Edit YAML → `validate` → `deploy` / upload artifact URLs |
| `saas` | `environments` / `segments` / local URLs **not** allowed | Edit flags in Git → platform owns rules → `controlpath sync` for `.controlpath/*.ast` |

SaaS example: [`schemas/examples/saas.control-path.yaml`](../../schemas/examples/saas.control-path.yaml).

```yaml
mode: saas
saas:
  project: acme/checkout
```

## Flag definitions (`flags`)

Each key is a flag name (`^[a-z][a-z0-9_]*$`). Minimal shape:

```yaml
flags:
  new_dashboard:
    kind: release
    default: false
```

### Essentials

| Field | Required | Description |
| --- | --- | --- |
| `kind` | yes | `release`, `kill_switch`, or `entitlement` |
| `default` | yes | Boolean when no rule matches (`true` / `false`) |

### Governance and metadata (recommended)

Validation can **warn** when recommended fields are missing; CI may enforce stricter checks.

| Field | Notes |
| --- | --- |
| `owner` | Owning team or contact |
| `description` | Human-readable intent |
| `ticket` | Tracking ticket (e.g. `JIRA-456`) |
| `expires` | Intended cleanup date (`YYYY-MM-DD`); especially important for `kind: release` |
| `lifecycle` | `active` (default) or `deprecated` — deprecated flags block rule changes unless `flag enable --force` |
| `tags` | Optional classification strings |
| `metadata` | Free-form object; must not contain SaaS telemetry |

**Kill switch flags** (`kind: kill_switch`): environment rules may only use plain `serve` (no `when` / `rollout`). Incidents use the kill switch file or SaaS dashboard toggles.

**Entitlement flags** (`kind: entitlement`): environment rules may use `when` and plain `serve` (no `rollout`). Use a separate `kind: release` flag for gradual rollout. Prefer `default: false` so access is deny-by-default when rules do not match; `controlpath validate` warns when `default` is `true`. Authoring guide: [`entitlements.md`](entitlements.md).

Full machine-readable schema: [`schemas/control-path.schema.v2.json`](../../schemas/control-path.schema.v2.json).

## Attribute schema (`attributes`)

Optional top-level map declaring service-specific **evaluation attributes** and their scalar types. Omitting `attributes:` preserves legacy behavior (loose generated SDK typing, no property-name validation on rules). Declaring `attributes:` (including an empty `{}`) opts in to strict mode for that catalog scope: `controlpath validate` and `compile` check map shape and reject unknown property names in **local-mode** environment rule and segment `when` expressions (base attributes plus declared service fields; top-level names only).

```yaml
attributes:
  plan: string
  seats: number
  beta: boolean
```

An empty map opts in without declaring service fields yet:

```yaml
attributes: {}
```

- Keys must be valid identifiers (`^[a-z][a-z0-9_]*$`).
- Values must be `string`, `number`, or `boolean` (nested object types are not supported in v1).
- Keys must not collide with **base attributes** — platform-owned fields listed in [`schemas/base-attributes.json`](../../schemas/base-attributes.json) (`id`, `email`, `role`, `environment`, `device`, `app_version`). `@controlpath/runtime` exports the same set as `BaseAttributes`.
- Each imported catalog may declare its own `attributes:` map; fields are namespaced at runtime under that import’s namespace (see [`sdk-typescript.md`](sdk-typescript.md)).

## Local mode: environments and compile

Rules are documented in [`rules.md`](rules.md). Summary:

- Declared under `environments.<name>.rules`
- Ordered, first-match wins; fallback to flag `default`
- Compile output: `.controlpath/<env>.ast`

```bash
controlpath validate
controlpath compile --env production
# or
controlpath deploy --env production
```

Snippet (staging always on, production with segment + rollout):

```yaml
environments:
  staging:
    rules:
      new_dashboard:
        - serve: true
  production:
    rules:
      new_dashboard:
        - when: "segment('beta_users')"
          serve: true
        - rollout:
            percentage: 10
            serve: true
```

Complete example: [`schemas/examples/local-only.control-path.yaml`](../../schemas/examples/local-only.control-path.yaml).

## Remote URLs (local mode)

When pods should poll for updated rules or kill switches, declare URLs per environment:

```yaml
artifacts:
  production:
    url: https://flags.example.com/production/rules.ast
kill_switches:
  production:
    url: https://flags.example.com/production/kill-switches.json
```

Omit these sections for purely local workflows without remote polling. Invalid when `mode: saas` (platform CDN serves URLs after sync).

## Imports and shared catalogs

Import shared flag catalogs by namespace:

```yaml
imports:
  platform:
    path: ../../platform/control-path.yaml
```

- SDK and `explain` use qualified names such as `platform.emergency_kill_switch`.
- The **consumer** catalog must **not** define `environments.*.rules` for imported flags—rules belong in the source catalog only.

Example: [`schemas/examples/imported-global.control-path.yaml`](../../schemas/examples/imported-global.control-path.yaml).

## Monorepos and workspace file

At the repo root, `control-path.workspace.yaml` supplies namespace fallback and scaffold defaults for `controlpath init` in service directories. It is **not** merged at compile time; values are copied when scaffolding new service catalogs.

```yaml
namespace: acme

scaffold:
  imports:
    platform:
      path: ../../platform/control-path.yaml
  mode: saas
  saas:
    project: acme/{{service-id}}
```

Example: [`schemas/examples/control-path.workspace.yaml`](../../schemas/examples/control-path.workspace.yaml).

**Service catalog in a monorepo:**

- Often omits `catalog.namespace` on the service file; runtime resolves namespace from the workspace file via walk-up.
- File-level `catalog.namespace` overrides workspace when both are set.

**Commands:**

```bash
controlpath init --monorepo          # workspace file at repo root
controlpath init --service-id checkout-service   # service catalog in a package directory
```

## Example catalogs

Entitlement authoring (composition, shared catalogs, fail-closed defaults): [`entitlements.md`](entitlements.md).

| File | Shows |
| --- | --- |
| [`local-only.control-path.yaml`](../../schemas/examples/local-only.control-path.yaml) | Local rules, segments, rollout, service-local entitlement (`premium_checkout`), `attributes`, artifact/kill-switch URLs |
| [`imported-global.control-path.yaml`](../../schemas/examples/imported-global.control-path.yaml) | `imports`, consumer rule boundaries, stacked `release` + platform entitlements |
| [`saas.control-path.yaml`](../../schemas/examples/saas.control-path.yaml) | SaaS mode catalog without local environments |
| [`shared-platform.control-path.yaml`](../../schemas/examples/shared-platform.control-path.yaml) | Shared platform catalog: entitlements, kill switches, import source rules |
| [`control-path.workspace.yaml`](../../schemas/examples/control-path.workspace.yaml) | Monorepo workspace scaffold |

## Validation

After editing `control-path.yaml`:

```bash
controlpath validate
controlpath validate --all
```

Common failures: invalid YAML, unknown fields, expression parse errors, rules for imported flags in the wrong catalog, SaaS-forbidden sections in local-only fields (and vice versa). See [`troubleshooting.md`](troubleshooting.md).
