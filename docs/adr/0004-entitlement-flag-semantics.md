# Entitlement flag semantics

Status: accepted

Control Path already exposes `kind: entitlement` in the flag catalog, but without documented semantics or kind-specific validation it is indistinguishable from `kind: release`. Teams use long-lived flags to gate product capabilities (plan tier, role, org attributes) — sometimes described informally as “enablement” flags or as an RBAC alternative. Industry usage ([LaunchDarkly entitlement flags](https://docs.launchdarkly.com/guides/flags/entitlements/), [Featureflow](https://www.featureflow.com/blog/feature-flags-vs-entitlements)) treats **entitlements** as durable access gates, separate from rollout flags and incident kill switches.

## Decision

1. **Naming:** **`kind: entitlement`** is the canonical catalog marker for long-lived access gates. Do not introduce a parallel “enablement” kind or term.

2. **Access evaluation:** Whether a principal may use a capability is decided by **environment rules** on **evaluation attributes** the application passes at SDK call time (e.g. plan, org tier, `role` from a token). How attributes are populated (JWT, session, billing lookup) is **out of scope** for Control Path.

3. **Fail-closed authoring:** Missing attributes make `when` expressions false (existing rule walk). Authors should use `default: false`. `validate` **warns** when `kind: entitlement` has `default: true` (strict CI may treat warnings as errors). Do not warn or error when `expires` is absent.

4. **Rule shapes:** **`kind: entitlement`** may use `when` and plain `serve` in **environment rules**. **`rollout` is forbidden** (same class of restriction as `kill_switch`, inverted: entitlements allow `when`, kill switches do not).

5. **Composition at the call site:**
   - **Release + entitlement:** Gradually shipping UI or behavior for a paid capability uses a separate **`kind: release`** flag; the application ANDs both evaluations. Remove the release flag after rollout; keep the entitlement.
   - **Incident + entitlement:** Disabling a paid feature during an incident uses a companion **`kind: kill_switch`** flag (e.g. `premium_checkout_kill`), toggled via the **kill switch file** or SaaS dashboard. The application ANDs entitlement and kill switch. Do not use CLI `kill-switch set` on the entitlement flag name (CLI requires `kind: kill_switch`).

6. **Shared catalogs:** Plan- or platform-wide entitlements live in a **shared catalog** imported by each service. **Environment rules** for imported entitlement flags are authored only in the source catalog (existing import rule — no per-service rule copies).

7. **`expires` metadata:** Optional on entitlements for trials or SKU sunsets. Semantics differ from **`kind: release`**, where missing `expires` may warn (rollout cleanup). Omitting `expires` on an entitlement is normal.

8. **v1 product scope:** Ship **governance + documentation** only — compiler validation for decisions 3–4, user docs, and glossary entries in `CONTEXT.md`. No billing sync, no new runtime evaluation path, no generated SDK helpers for AND-composition.

## Considered options

- **“Enablement” as a separate kind or rename:** Rejected; not standard industry terminology and duplicates `entitlement`.
- **Entitlements as org-only; RBAC always separate:** Rejected for Control Path’s model; role claims are passed as **evaluation attributes** and may appear in the same entitlement flag’s `when` rules. Server-side authz beyond flag evaluation remains the application’s responsibility.
- **`rollout` on entitlements:** Rejected; percentage access is a product bug, not access control. Use **`kind: release`** for gradual ship.
- **Kill switch file overrides on entitlement flag names:** Rejected for ops tooling consistency; companion **`kind: kill_switch`** preserves commercial rules in the **compiled artifact** while incidents use the existing kill-switch path.
- **Per-service duplicate entitlement catalogs:** Rejected; causes rule drift across services. Use **shared catalog** imports.
- **Hard error on `default: true` for entitlements:** Rejected; warn-only matches existing governance style; deny-list rule patterns may intentionally use `default: true`.
- **Billing integration as source of truth:** Out of scope; applications supply plan/org fields as attributes.

## Consequences

- Compiler gains entitlement-specific validation mirroring kill-switch rollout checks (forbid `rollout`; warn on `default: true`).
- User documentation should describe entitlement vs release vs kill_switch composition patterns and shared-catalog placement.
- Applications must AND multiple flag evaluations where release, entitlement, and incident layers apply — no single combined flag kind.
- SaaS and local modes inherit the same catalog semantics; **environment rules** for shared entitlements follow existing mode ownership (local in source YAML vs remote SaaS project).
