# Control Path

Feature flag management for local development and SaaS-hosted targeting. The committed config file defines a boolean flag catalog; environment rules are authored locally or owned by the SaaS depending on mode.

## Language

**Flag catalog**:
The set of boolean flag definitions declared in a `control-path.yaml` file — keys, defaults (`true`/`false`), kind, lifecycle, and declared metadata. Distinct from environment rules or runtime telemetry. Changing the **flag catalog** requires regenerating and redeploying the application SDK; it is not applied by replacing the **compiled artifact** alone.
_Avoid_: Flag list, definitions file

**Catalog identity**:
Stable identity for a flag catalog: `catalog.id` (required) plus optional `catalog.namespace`. Effective id is `{namespace}.{id}` when namespace resolves, otherwise `id` alone. Used for SaaS sync, imports, and telemetry.
_Avoid_: Project name, package name (unless explicitly chosen as the id)

**Catalog namespace**:
Domain-boundary prefix qualifying `catalog.id`. Multi-repo: declare as `catalog.namespace`. Monorepo: omit on services; supplied by `control-path.workspace.yaml` via walk-up. File value takes precedence over workspace.
_Avoid_: Import namespace

**Workspace file**:
Monorepo-only scaffold manifest (`control-path.workspace.yaml`) at repo root. Provides namespace fallback at runtime and `scaffold` boilerplate for `controlpath init`. Not merged at compile time — copied into new service files at init only.
_Avoid_: Root catalog, global config file

**Import namespace**:
The key under `imports` that qualifies flags from a shared catalog in SDK and rules (e.g. `platform.emergency_kill_switch`). Required for every import. Consuming services must not define environment rules for imported flags — rules live in the source catalog only, to avoid incident-time confusion.
_Avoid_: Catalog namespace, override

**Segment**:
A named, reusable targeting predicate declared under `segments`. Rules reference segments in `when` expressions. Local mode only.
_Avoid_: Audience, cohort (unless used consistently elsewhere)

**Shared catalog**:
A flag catalog consumed via `imports` — whether it lives at the monorepo root, in another repo, or (later) remotely. There is no special "root catalog" type; a root-level catalog is just another imported catalog. Declared as a namespace-keyed map with a required `path`.
_Avoid_: Root catalog, global config file

**Environment**:
A named deployment target (e.g. `staging`, `production`). In local mode, declared under `environments` with a required `rules` map and optional metadata such as `description`. Invalid when `mode: saas`.
_Avoid_: Deployment file

**Environment rules**:
Ordered targeting rules under `environments.<name>.rules`, keyed by local flag name only (not imported flags). Each rule may have optional `when`, optional boolean `rollout` (`percentage` + `serve`), required `serve`, and optional `reason`. Flags with `kind: kill_switch` may only use plain `serve` rules (no `when` or `rollout`). Flags with `kind: entitlement` may use `when` and plain `serve` but not `rollout`. No match falls back to the catalog `default`. Changing **environment rules** alone is published by replacing the **compiled artifact** at the **artifact URL** or **artifact path** (and does not require an SDK rebuild when the **flag catalog** is unchanged).
_Avoid_: Deployment, targeting config

**Evaluation attributes**:
The single object passed at runtime (and to `explain`) whose fields rule `when` expressions read — **base attributes**, service-local **attribute schema** fields, and **namespaced attributes** for each **import namespace**. Not a nested **user** inside a separate **context** bag: `user.` and `context.` prefixes in rule strings are optional authoring sugar, compiled to top-level keys on this object. The generated SDK types a closed shape when the service opts into **attribute schema**; each flag method uses a **per-flag attribute type** from catalog ownership (not from **environment rules**).
_Avoid_: User object, context object (as parallel runtime bags), evaluation context (unqualified)

**Attribute schema**:
Optional `attributes:` map declaring service-specific evaluation attribute names and scalar types only in v1: `string`, `number`, `boolean`. Scoped like flags: the service catalog owns top-level fields; each **import namespace** owns fields declared in that imported catalog’s **attribute schema** (not flattened into the service file). Omitting `attributes:` on a catalog leaves legacy behavior for that scope (including `controlpath init`, which does not scaffold **attribute schema** by default). Opting in on the service catalog enables strict property validation and a closed generated SDK type. In **local mode**, `validate` / `compile` / `generate-sdk` reject unknown property names in **environment rules** and **segments** under that catalog. In **SaaS mode**, the service validates **attribute schema**, flags, and imports only — remote **environment rules** are validated where they are authored, not from the service repo. Distinct from the runtime **evaluation attributes** object passed at call sites.
_Avoid_: Context schema, user schema, property definitions (unqualified)

**Per-flag attribute type**:
Generated flag method parameter type derived from Git-stable catalog data only — not from **environment rules** (those can change via **compiled artifact** / SaaS without an SDK rebuild). Local flags: **base attributes** plus the full service **attribute schema**. Imported flags: **base attributes** plus that flag’s **import namespace** object only. Callers may pass a wider **evaluation attributes** object (structural superset), never a narrower one.
_Avoid_: Rule-derived minimum types, environment-rule typing, SaaS-synced TypeScript

**Namespaced attributes**:
Evaluation attribute fields from an imported catalog are grouped under that catalog’s **import namespace** in the runtime object and generated TypeScript (e.g. service passes `{ platform: { org_tier: 'gold' } }`; rules in the shared catalog author bare `org_tier`, compiled into the merged artifact as `platform.org_tier`). Service-local **attribute schema** fields stay at the top level beside **base attributes**. Aligns attribute typing with **import namespace** the same way flag names are qualified in the SDK.
_Avoid_: Flat import fields, global attribute bag, qualified property names in shared-catalog rule strings

**Base attributes**:
Platform-owned evaluation attribute fields (`id`, `email`, `role`, `environment`, `device`, `app_version`, …), exported from `@controlpath/runtime` as `BaseAttributes`. Not redeclared in catalog **attribute schema** — doing so fails validation. The generated SDK extends **base attributes** with service and **namespaced attributes** types; the runtime package owns the base field list so it is not duplicated in the generator template.
_Avoid_: Default context, standard user object, built-in context, duplicated base fields in generated types

**Declared metadata**:
Git-authored fields on flags expressing intent: required `kind`; optional `owner`, `ticket`, `expires`, `tags`, `description`, `lifecycle` (defaults to `active`), and free-form `metadata`. Validation warns on missing recommended fields; strict enforcement is optional in CI. For **`kind: release`**, missing `expires` may warn (rollout cleanup). For **`kind: entitlement`**, `expires` is optional with no warn or error when absent — when set, it marks a planned offering or trial sunset, not rollout cleanup.
_Avoid_: Telemetry, observed data

**Flag kind**:
Why a flag exists: `release`, `kill_switch`, or `entitlement`. Required on every flag.
_Avoid_: Type, status, experiment

**Entitlement**:
Long-lived access gate declared in the **flag catalog** with `kind: entitlement`. Whether a principal may use a capability is decided only at evaluation time: **environment rules** on the **evaluation attributes** the application passes in (e.g. plan or org tier, `role` from a token, other **attribute schema** fields). Same evaluation stack as other flags (kill switch file → **compiled artifact** → catalog **default**). Missing attributes make `when` expressions false (standard rule walk); authors should use `default: false` so unmatched cases deny access. `validate` warns when `default: true` on an **entitlement** (suspicious; strict CI may treat warnings as errors). **Environment rules** may use `when` and plain `serve` but not `rollout`. Compose with a separate **`kind: release`** flag when gradually shipping UI or behavior for an already-entitled capability — the application ANDs both evaluations; remove the **release** flag after rollout, keep the **entitlement**. Optional **`expires`** may mark a trial or SKU sunset in **declared metadata**; omitting `expires` is normal and must not warn or error. Plan- or platform-wide entitlements belong in a **shared catalog** imported by each service; **environment rules** for those flags live only in the source catalog (not per consuming service). Incidents on an entitled capability use a companion **`kind: kill_switch`** flag (not the entitlement name in the **kill switch file** via CLI); the application ANDs both evaluations. Distinct from **kill_switch** as a **flag kind** (incident layer) vs **entitlement** (access layer). How attributes are obtained (JWT, session, service call) is out of scope for Control Path.
_Avoid_: Enablement flag, enablement, feature enablement

**Permission** (RBAC):
Role or permission claims used inside **environment rules** — typically via `role` on **base attributes** when the identity token carries roles. Not a separate **flag kind**; not populated by Control Path. May appear in the same **entitlement** flag’s rules alongside commercial attributes (org purchased the feature and user’s role allows use).
_Avoid_: Entitlement (as a synonym for role), enablement

**Flag lifecycle**:
Repo-owned deprecation signal: `active` (default) or `deprecated`. Removal from the catalog retires the flag in SaaS history.
_Avoid_: Status, archived

**Observed telemetry**:
Runtime evaluation signals from the SaaS (`lastEvaluated`, evaluation counts, unused-flag detection). Never written back into Git.
_Avoid_: Metadata (unqualified)

**SaaS project**:
The remote Control Path project that owns environment rules when `mode: saas`. Declared non-secret identity in `saas.project`; credentials and cached artifacts live outside Git. SDK generation in SaaS mode embeds **artifact URL** and **kill switch URL** entries for each **environment** the service has already received a compiled artifact for via sync (not every environment the platform might support).
_Avoid_: Deployment, environment config

**Compiled artifact**:
The runtime binary encoding **environment rules** for one **environment**, produced by compile (local mode) or the SaaS project. Carries merged rules for the service catalog and its **imports**. Evaluated after the **kill switch file** and before the catalog default. When signature verification is configured, a polled replacement is verified only when new bytes are received — an unchanged remote copy does not re-run verification. In local mode, rule changes in Git are published by compile/deploy and placement at the **artifact URL** or **artifact path**. In SaaS mode, rule changes in the **SaaS project** are compiled and published to the platform CDN by the platform — the SDK learns them only via poll, not Git. A successful poll hot-swaps the in-memory artifact and rebuilds flag index maps without restart. During rollout, an older SDK may load a newer **compiled artifact** that includes extra flags — names not in the generated SDK are ignored so **environment rules** can advance without every pod on the newest SDK. A poll is rejected (last good artifact kept) when the environment does not match or when no flag name in the **compiled artifact** appears in the generated SDK, indicating a likely wrong object. In SaaS mode, **environment rules** are owned remotely while the **flag catalog** stays in Git, which limits cross-service catalog mix-ups at the **artifact URL**.
_Avoid_: AST, deployment artifact, rules file

**Artifact URL**:
Where the SDK polls for the **compiled artifact** over HTTP(S). In local mode, committed per **environment** as `artifacts.<env>.url` when rules are hosted remotely (e.g. object storage). Mutually exclusive with **artifact path** on the same target. In SaaS mode, URLs are derived from the platform CDN contract (`saas.project`, catalog identity, environment) and embedded when the SDK is generated — not declared in Git. When an **artifact URL** is configured for the loaded environment, the SDK keeps refreshing from that URL after the first load (which may use a bundled file path). Init guardrails (environment match, flag-name overlap with the generated SDK) run at `init({ artifact })` when either an **artifact URL** or **artifact path** is configured for that environment; the same checks apply before a refresh hot-swaps new bytes. A failed refresh keeps the last successfully loaded **compiled artifact**. An unchanged remote copy (not modified since the last fetch) does not replace or re-verify the in-memory artifact. Refresh uses the same staggered polling approach as **kill switch URL** (init spread + interval jitter), on an independent timer with a longer default interval — kill switches change faster in incidents than **environment rules** do in deploys.
_Avoid_: rules URL, deployment URL

**Artifact path**:
Where the SDK refreshes the **compiled artifact** from the local filesystem on a staggered poll interval. In local mode, committed per **environment** as `artifacts.<env>.path` — a POSIX absolute filesystem path (must start with `/`; e.g. a Docker volume mount or sidecar-written file). Relative paths and native Windows paths are invalid in v1; use `init({ artifact })` with a relative path for local-only workflows without configured refresh. Mutually exclusive with **artifact URL** on the same target. Invalid when `mode: saas`. **Refresh-only** — the same split as **artifact URL**: cold start loads via `init({ artifact })` (bundled path, mount path, or URL); the configured **artifact path** is where subsequent refreshes read from and may differ from the init source. When an **artifact path** is configured for the loaded environment, the SDK keeps re-reading that file on the same staggered interval as **artifact URL** polling, skipping reload when `mtime` and size are unchanged since the last successful read. Init guardrails (environment match, flag-name overlap with the generated SDK) run at `init({ artifact })` when either an **artifact URL** or **artifact path** is configured for that environment; the same checks apply before a refresh hot-swaps new bytes. Publishers should replace the file atomically (write-then-rename); a read of invalid bytes keeps last-good state. A failed refresh (missing file, I/O error, parse error, or rejected guardrail) keeps the last successfully loaded **compiled artifact**; evaluation continues on last-good state until a later refresh succeeds.
_Avoid_: rules path, bundled artifact path (unqualified — use **artifact path** only for configured refresh targets, not one-off `init` paths)

**Kill switch file**:
A runtime JSON file listing boolean values for flags. Skips rule evaluation for listed flags. Evaluation order: kill switch file → compiled artifact → catalog default. In local mode, deploy writes a build artifact; ops place it at a self-hosted **kill switch URL** or **kill switch path**. In SaaS mode, the platform CDN serves values; incidents are handled via dashboard toggles (direct write), not CLI deploy.
_Avoid_: Override file (legacy v1 name)

**Explain trace**:
Structured output from `controlpath explain` describing which layer matched (kill switch file, environment rule in the **compiled artifact**, or catalog default) and optional per-rule walk. Rule `when` / rollout semantics come from the artifact; declared `reason` and flag metadata come from the **flag catalog** (and **imports** for qualified names). In SaaS mode, `explain` uses a synced `.controlpath/<env>.ast` after `sync` — it does not require local **environment rules** in Git, but still needs the **flag catalog** for names, defaults, and metadata. Without a cached artifact, explain fails like `generate-sdk`.

**Kill switch URL**:
Where the SDK polls for the **kill switch file** over HTTP(S). In local mode, committed per environment as `kill_switches.<env>.url`. Mutually exclusive with **kill switch path** on the same target. In SaaS mode, URLs are derived from the platform CDN contract and embedded when the SDK is generated — not declared in Git. Polled more frequently than the **artifact URL** because incident toggles must propagate faster than **environment rules** deploys.
_Avoid_: overrideUrl (legacy SDK-only config)

**Kill switch path**:
Where the SDK refreshes the **kill switch file** from the local filesystem on a staggered poll interval. In local mode, committed per environment as `kill_switches.<env>.path` — a POSIX absolute filesystem path (must start with `/`). Relative paths and native Windows paths are invalid in v1. Mutually exclusive with **kill switch URL** on the same target. Invalid when `mode: saas`. **Refresh-only** — same as **kill switch URL**: no bundled kill switch at init; the first successful refresh loads state, then polling continues. Refreshed on the same faster staggered interval as **kill switch URL** polling, skipping reload when `mtime` and size are unchanged since the last successful read. Publishers should replace the file atomically (write-then-rename); a read of invalid bytes keeps last-good state. A failed refresh (missing file, I/O error, or invalid bytes) keeps the last successfully loaded **kill switch file**; evaluation continues on last-good state until a later refresh succeeds.
_Avoid_: override path, local override file (unqualified)

## Example dialogue

**Dev:** We're adding global kill switches shared across services — where do they live?

**Expert:** In a **shared catalog** file — maybe at the monorepo root — imported by each service. Run `controlpath init` at the repo root to create the **workspace file**; run it in a service folder to scaffold that service's catalog using workspace boilerplate.

**Dev:** Production is on fire — do we change `control-path.yaml`?

**Expert:** No. In **SaaS mode**, toggle it in the **dashboard** — that writes directly to the platform CDN and the SDK picks it up. In **local mode**, use the CLI, deploy the build output, then upload to your bucket.

**Dev:** We changed targeting rules in staging — do pods need a restart?

**Expert:** No — upload the new **compiled artifact** to the bucket (or wait for SaaS CDN). The SDK hot-swaps rules on poll. You only rebuild the SDK when the **flag catalog** changes — new flags, defaults, kinds, or imports.

**Dev:** We mount rules as a file in the container — do we need an **artifact URL**?

**Expert:** No. Set `artifacts.production.path` to the mount (POSIX absolute, e.g. `/mnt/flags/production.ast`). `init({ artifact })` loads your bundled or mount copy at startup; the SDK polls that path on interval and hot-swaps when the file changes. Write-then-rename when publishing. Same last-good semantics as URL polling if the file is missing or invalid.

**Dev:** We merged definitions and rules into one YAML file — do we still ship two things?

**Expert:** One file in Git, two deployment speeds: **environment rules** → **artifact URL** only; **flag catalog** → `generate-sdk` plus app deploy. Same pattern as the old two-file setup.

**Dev:** We're rolling out a new export UI for a Pro-only feature — one flag or two?

**Expert:** Two. **`kind: entitlement`** for whether the org and user may use export (plan + `role` in rules). **`kind: release`** for the UI rollout (`rollout` or beta rules). The app checks both. Delete the **release** flag when rollout finishes; the **entitlement** stays for the life of the plan.

**Dev:** Pro plan features span checkout, analytics, and billing — where do we define them?

**Expert:** In a **shared catalog** at the repo root (or another imported path), one flag per capability under an **import namespace** like `platform`. **Environment rules** for those flags live only in that source file — each service imports and evaluates the same rules, not copies per service.

**Dev:** Premium checkout is on fire — do we flip the entitlement?

**Expert:** No. Add a companion **`kind: kill_switch`** (e.g. `platform.premium_checkout_kill`), toggle it in the **kill switch file** or dashboard. The app ANDs entitlement and kill switch. **Entitlement** rules stay the commercial source of truth.
