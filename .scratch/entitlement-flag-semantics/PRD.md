# Entitlement flag semantics

Long-lived **`kind: entitlement`** flags gate product capabilities via **environment rules** on caller-supplied **evaluation attributes** (plan, `role`, etc.). Distinct from **`kind: release`** (rollout) and companion **`kind: kill_switch`** flags (incidents).

## Source decisions

- `docs/adr/0004-entitlement-flag-semantics.md`
- `CONTEXT.md` — **Entitlement**, **Permission**, **Environment rules**, **Declared metadata**

## Scope (v1)

- Compiler governance: forbid `rollout` on entitlement rules; warn on `default: true`
- User documentation and schema examples (composition, shared catalog, fail-closed authoring)
- Glossary already updated in `CONTEXT.md`

## Out of scope

- Billing sync or commercial source of truth in Control Path
- New runtime evaluation path or SDK AND-composition helpers
- Renaming `kind: entitlement` or introducing “enablement” terminology

## Issues

1. `issues/01-reject-entitlement-rollout.md`
2. `issues/02-warn-entitlement-default-true.md`
3. `issues/03-entitlement-authoring-guide.md`
