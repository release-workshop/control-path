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
Ordered targeting rules under `environments.<name>.rules`, keyed by local flag name only (not imported flags). Each rule may have optional `when`, optional boolean `rollout` (`percentage` + `serve`), required `serve`, and optional `reason`. Flags with `kind: kill_switch` may only use plain `serve` rules (no `when` or `rollout`). No match falls back to the catalog `default`. Changing **environment rules** alone is published by replacing the **compiled artifact** at the **artifact URL** (and does not require an SDK rebuild when the **flag catalog** is unchanged).
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
Git-authored fields on flags expressing intent: required `kind`; optional `owner`, `ticket`, `expires`, `tags`, `description`, `lifecycle` (defaults to `active`), and free-form `metadata`. Validation warns on missing recommended fields; strict enforcement is optional in CI.
_Avoid_: Telemetry, observed data

**Flag kind**:
Why a flag exists: `release`, `kill_switch`, or `entitlement`. Required on every flag.
_Avoid_: Type, status, experiment

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
The runtime binary encoding **environment rules** for one **environment**, produced by compile (local mode) or the SaaS project. Carries merged rules for the service catalog and its **imports**. Evaluated after the **kill switch file** and before the catalog default. When signature verification is configured, a polled replacement is verified only when new bytes are received — an unchanged remote copy does not re-run verification. In local mode, rule changes in Git are published by compile/deploy and upload to the **artifact URL**. In SaaS mode, rule changes in the **SaaS project** are compiled and published to the platform CDN by the platform — the SDK learns them only via poll, not Git. A successful poll hot-swaps the in-memory artifact and rebuilds flag index maps without restart. During rollout, an older SDK may load a newer **compiled artifact** that includes extra flags — names not in the generated SDK are ignored so **environment rules** can advance without every pod on the newest SDK. A poll is rejected (last good artifact kept) when the environment does not match or when no flag name in the **compiled artifact** appears in the generated SDK, indicating a likely wrong object. In SaaS mode, **environment rules** are owned remotely while the **flag catalog** stays in Git, which limits cross-service catalog mix-ups at the **artifact URL**.
_Avoid_: AST, deployment artifact, rules file

**Artifact URL**:
Where the SDK polls for the **compiled artifact**. In local mode, committed per **environment** in `artifacts.<env>.url` when rules are hosted remotely (e.g. object storage). In SaaS mode, URLs are derived from the platform CDN contract (`saas.project`, catalog identity, environment) and embedded when the SDK is generated — not declared in Git. When an **artifact URL** is configured for the loaded environment, the SDK keeps refreshing from that URL after the first load (which may use a bundled file path). A failed refresh keeps the last successfully loaded **compiled artifact**. An unchanged remote copy (not modified since the last fetch) does not replace or re-verify the in-memory artifact. Refresh uses the same staggered polling approach as **kill switch URL** (init spread + interval jitter), on an independent timer with a longer default interval — kill switches change faster in incidents than **environment rules** do in deploys.
_Avoid_: rules URL, deployment URL

**Kill switch file**:
A runtime JSON file listing boolean values for flags. Skips rule evaluation for listed flags. Evaluation order: kill switch file → compiled artifact → catalog default. In local mode, deploy writes a build artifact and ops uploads to a self-hosted URL. In SaaS mode, the platform CDN serves values; incidents are handled via dashboard toggles (direct write), not CLI deploy.
_Avoid_: Override file (legacy v1 name)

**Explain trace**:
Structured output from `controlpath explain` describing which layer matched (kill switch file, environment rule in the **compiled artifact**, or catalog default) and optional per-rule walk. Rule `when` / rollout semantics come from the artifact; declared `reason` and flag metadata come from the **flag catalog** (and **imports** for qualified names). In SaaS mode, `explain` uses a synced `.controlpath/<env>.ast` after `sync` — it does not require local **environment rules** in Git, but still needs the **flag catalog** for names, defaults, and metadata. Without a cached artifact, explain fails like `generate-sdk`.

**Kill switch URL**:
Where the SDK polls for the **kill switch file**. In local mode, committed per environment in `kill_switches.<env>.url`. In SaaS mode, URLs are derived from the platform CDN contract and embedded when the SDK is generated — not declared in Git. Polled more frequently than the **artifact URL** because incident toggles must propagate faster than **environment rules** deploys.
_Avoid_: overrideUrl (legacy SDK-only config)

## Example dialogue

**Dev:** We're adding global kill switches shared across services — where do they live?

**Expert:** In a **shared catalog** file — maybe at the monorepo root — imported by each service. Run `controlpath init` at the repo root to create the **workspace file**; run it in a service folder to scaffold that service's catalog using workspace boilerplate.

**Dev:** Production is on fire — do we change `control-path.yaml`?

**Expert:** No. In **SaaS mode**, toggle it in the **dashboard** — that writes directly to the platform CDN and the SDK picks it up. In **local mode**, use the CLI, deploy the build output, then upload to your bucket.

**Dev:** We changed targeting rules in staging — do pods need a restart?

**Expert:** No — upload the new **compiled artifact** to the bucket (or wait for SaaS CDN). The SDK hot-swaps rules on poll. You only rebuild the SDK when the **flag catalog** changes — new flags, defaults, kinds, or imports.

**Dev:** We merged definitions and rules into one YAML file — do we still ship two things?

**Expert:** One file in Git, two deployment speeds: **environment rules** → **artifact URL** only; **flag catalog** → `generate-sdk` plus app deploy. Same pattern as the old two-file setup.
