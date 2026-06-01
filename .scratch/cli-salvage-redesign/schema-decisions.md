# Schema v2 decision notes

Decision record for issue 01. Implements the boolean-only `control-path.yaml` contract AFK agents should follow without reopening product semantics.

## Artifacts

| File | Purpose |
|---|---|
| `schemas/control-path.schema.v2.json` | JSON Schema for service `control-path.yaml` |
| `schemas/control-path.workspace.schema.v1.json` | JSON Schema for monorepo `control-path.workspace.yaml` |
| `schemas/examples/*.yaml` | Representative examples |

v1 schemas remain until issue 09 prunes legacy surfaces.

## Salvage from v1

| v1 concept | v2 disposition |
|---|---|
| `mode: local \| saas` | Kept |
| Ordered rules: `when`, `serve`, `rollout` | Kept — boolean rollout embeds `serve` |
| `segments` | Kept — local mode only |
| Flag key pattern `^[a-z][a-z0-9_]*$` | Kept |
| Per-flag `kind`, metadata | Kept — tightened enums and field shapes |
| `flags` as array with `name` | **Dropped** — map-keyed flags |
| Per-flag nested `environments` | **Dropped** — top-level `environments.<env>.rules.<flag>` |
| `type`, `variations`, multivariate rollout | **Dropped** — boolean-only |
| `context` schema extensions | **Dropped** — not used by compiler/SDK |
| `defaultValue`, `ON`/`OFF` strings | **Dropped** — native booleans only |

## Catalog vs rules vs telemetry

Three ownership boundaries:

1. **Flag catalog (Git)** — what flags exist, defaults, kind, lifecycle, declared metadata.
2. **Environment rules** — local mode: Git `environments`; SaaS mode: remote SaaS project.
3. **Observed telemetry (SaaS only)** — `lastEvaluated`, evaluation counts, rot suggestions. Never written to Git.

Declared metadata fields: `owner`, `ticket`, `expires`, `tags`, `description`, `lifecycle`, plus free-form `metadata`. Validation warns on missing recommended fields; strict CI policy is optional.

## Catalog identity

```yaml
catalog:
  id: checkout-service        # required
  namespace: acme             # optional — multi-repo: declare here; monorepo: omit and use workspace file
```

**Namespace resolution (first match wins):**

1. `catalog.namespace` in `control-path.yaml`
2. `namespace` from `control-path.workspace.yaml` (walk-up from service directory)
3. Neither — effective id is `catalog.id` alone

**Multi-repo:** each repo declares `catalog.namespace` explicitly.

**Monorepo:** root `control-path.workspace.yaml` supplies namespace; service files use short `catalog.id` only.

- No Backstage or external registry link in schema.

## Monorepo workspace file

`control-path.workspace.yaml` is a **scaffold manifest**, not a runtime merge layer. Created at monorepo root by `controlpath init`.

```yaml
namespace: acme

scaffold:
  imports:
    platform:
      path: ../../platform/control-path.yaml
  defaults:
    owner: platform-team
  mode: saas
  saas:
    project: acme/{{service-id}}
```

**Runtime use:** namespace fallback only (walk-up from service directory).

**Init use:** `scaffold` block is copied into new `control-path.yaml` files — not merged on every compile.

### `controlpath init` behaviour

1. Prompts: **monorepo setup?** (yes / no).
2. **Monorepo + run from repo root:** creates `control-path.workspace.yaml` (namespace + optional scaffold).
3. **Monorepo + run from service folder:** walks up to find workspace file; scaffolds `control-path.yaml` in cwd using workspace `scaffold` and namespace.
4. **Multi-repo (not monorepo):** creates `control-path.yaml` only; prompts for `catalog.namespace` — no workspace file.

Walk-up discovery applies in both init (find workspace for scaffold) and runtime (resolve namespace).

## Imports and shared catalogs

```yaml
imports:
  platform:
    path: ../../platform/control-path.yaml
```

- Namespace is the import map key — explicit, not inferred from path.
- Shared flags (monorepo root or org repo) are ordinary imported catalogs — no special "root catalog" type.
- **Consuming services must not define environment rules for imported flags.** Rules live in the source catalog only. Prevents incident-time confusion when a service shadows a global kill switch.
- Issue 07 enforces this rule (no `overridable` field).

## Flag model

Required: `default` (boolean), `kind` (`release` | `kill_switch` | `entitlement`).

Optional: `lifecycle` (`active` | `deprecated`, default `active`), `description`, `owner`, `ticket`, `expires`, `tags`, `metadata`.

No `type` field — all flags are boolean.

## Environment rules (local mode)

```yaml
environments:
  staging:
    description: Optional metadata
    rules:
      new_dashboard:
        - when: "..."
          serve: true
        - rollout:
            percentage: 10
            serve: true
          reason: Optional audit note
```

- Entire `environments` block optional in local mode.
- `environments` and `segments` invalid when `mode: saas`.
- Rules keyed by **local flag names only**.
- First match wins; no match → catalog `default`.

## SaaS mode

```yaml
mode: saas
saas:
  project: acme/checkout
  api_url: https://api.controlpath.dev   # optional — catalog sync API (self-host)
  cdn_url: https://cdn.mycompany.com     # optional — SDK runtime poll origin (self-host)
```

- Repo catalog is source of truth for flags and declared metadata.
- SaaS owns environment rules and preserves telemetry/history.
- Removing a flag from Git retires it in SaaS (history preserved, not hard-deleted).
- `lifecycle: deprecated` blocks new **local** rule changes unless forced (CLI `flag enable --force`). SaaS environment rules are not edited through the CLI.

## Validation beyond JSON Schema

JSON Schema cannot express all rules. The compiler validator (issue 02) must also enforce:

- `environments` / `segments` rejected when `mode: saas`
- `kill_switches` rejected when `mode: saas` (CDN URLs come from platform)
- Duplicate import namespaces
- Local flag keys must not use import namespace prefixes
- Environment rules must not reference imported flag keys
- `kind: kill_switch` rules must not use `when` or `rollout` (serve only)
- No telemetry fields in catalog `metadata`
- Warn: missing `owner`; `kind: release` without `expires`
- Resolve catalog namespace: `catalog.namespace` → workspace walk-up → none

## Examples

See `schemas/examples/`:

- `local-only.control-path.yaml` — local mode with segments, rollout, metadata
- `saas.control-path.yaml` — multi-repo SaaS catalog with `catalog.namespace`
- `shared-platform.control-path.yaml` — shared catalog with its own rules
- `imported-global.control-path.yaml` — service importing platform catalog
- `control-path.workspace.yaml` — monorepo scaffold manifest with namespace + init boilerplate

## Kill switch files

Separate runtime artifact from `control-path.yaml`. v1 called this "override file"; v2 product language is **kill switch file**.

### URL configuration (`kill_switches`)

**Local mode** — committed in Git:

```yaml
kill_switches:
  production:
    url: https://flags.example.com/production/kill-switches.json
```

SDK reads URL from generated constants. Ops manually uploads the deploy build output to this URL.

**SaaS mode** — no `kill_switches` block in Git. The platform CDN serves kill switch values. During incidents, users toggle flags in the **SaaS dashboard** (direct write); the SDK polls the CDN. No CLI or deploy step required in an incident.

**CDN path contract** (implemented in `crates/compiler/src/catalog/cdn.rs`, documented on the fake SaaS client):

- Base: `saas.cdn_url` when set, else `https://cdn.controlpath.dev` (`saas.api_url` is sync-only)
- Kill switch: `{base}/v2/runtime/projects/{saas.project}/catalogs/{effective_catalog_id}/environments/{env}/kill-switches.json`
- Compiled artifact: `{base}/v2/runtime/projects/{saas.project}/catalogs/{effective_catalog_id}/environments/{env}/rules.ast`
- Embedded at `generate-sdk` only for environments with `.controlpath/<env>.ast` after sync (see ADR 0001).

### Evaluation order

**Kill switch file → AST → catalog default.** Listed flags skip rule evaluation.

### Rule constraints for `kind: kill_switch`

In `environments.*.rules`, kill switch flags may only use plain serve rules:

```yaml
emergency_kill_switch:
  - serve: false
```

No `when`, no `rollout`. Release/entitlement flags keep full rule shape.

### Kill switch file format (follow-up schema)

Add `schemas/kill-switch-file.schema.v2.json`. Boolean map only; optional audit fields. Any flag may appear; `kind: kill_switch` guides scaffold content.

### Deploy build output

`deploy` / `ci` **always** writes `.controlpath/<env>.kill-switches.json` alongside the AST.

**Local mode incident workflow:**

1. `controlpath kill-switch set …` updates local state.
2. `controlpath deploy` regenerates `.controlpath/<env>.kill-switches.json`.
3. Ops manually copies that file to the URL in `kill_switches.<env>.url`.

**SaaS mode incident workflow:**

1. User toggles flag in the **SaaS dashboard**.
2. Platform writes directly to CDN.
3. SDK polls and applies — no CLI, no deploy, no Git change.

Live boolean values are never stored in Git. Example output shape: `schemas/examples/production.kill-switches.json`.

## Compiled artifacts (runtime)

See `docs/adr/0001-compiled-artifact-runtime-delivery.md`. Product terms: **compiled artifact**, **artifact URL** (`CONTEXT.md`). Evaluation order: kill switch file → **compiled artifact** → catalog default.

### URL configuration (`artifacts`)

**Local mode** — committed in Git (mirror `kill_switches`):

```yaml
artifacts:
  production:
    url: https://flags.example.com/production/rules.ast
```

`deploy` / `ci` still writes `.controlpath/<env>.ast`; ops upload to `artifacts.<env>.url` when rules are remote-hosted.

**SaaS mode** — no `artifacts` block in Git (reject in validator, same as `kill_switches`). Platform CDN URLs use the contract above; embedded at `generate-sdk` for every `.controlpath/<env>.ast` on disk. `generate-sdk` fails if no `*.ast` exists. SaaS sync prunes `.controlpath/<env>.ast` for environments no longer returned by the platform **on download only** — manually copied `*.ast` files are embedded until deleted or the next sync.

### SDK polling

- Poll when `artifacts.<loaded-env>.url` is configured (or SaaS-embedded equivalent), after first load from file or URL.
- Independent jittered poll loop from kill switches; **kill switches poll faster** than compiled artifacts.
- Conditional fetch (ETag / 304): unchanged remote → no replace, no signature re-verify.
- Signature verification (when configured): only on new bytes.
- Failed poll or failed verify → keep last good compiled artifact.
- Successful poll → hot-swap artifact and rebuild flag index maps (no process restart).

### Deploy velocities (one YAML, two shipping paths)

| Change | Ship |
|---|---|
| **Flag catalog** (flags, defaults, kinds, imports) | Regenerate SDK + redeploy app |
| **Environment rules** only | Replace compiled artifact at artifact URL (poll) |
| **Kill switch** (incident) | Kill switch file at kill switch URL (faster poll) |

Forward-compatible rollout: older SDK may load a newer artifact; extra flag names in the artifact not present in the SDK are ignored. Reject poll when `env` mismatches or **zero** flag-name overlap with the SDK (wrong object). Optional future: embed catalog identity in the artifact binary (ADR candidate).
