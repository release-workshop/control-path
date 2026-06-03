Status: ready-for-agent

# Entitlement authoring guide and schema examples

## Parent

- `.scratch/entitlement-flag-semantics/PRD.md`
- `docs/adr/0004-entitlement-flag-semantics.md`

## What to build

Document how teams author and operate **`kind: entitlement`** flags end-to-end, aligned with ADR 0004 and `CONTEXT.md`.

User-facing guide should cover:

- **Entitlement** as long-lived access gates (not “enablement”); rules on **evaluation attributes** the app passes (`plan`, `role`, **attribute schema** fields); attribute sourcing out of scope
- Fail-closed authoring: prefer `default: false`; missing attributes make `when` false
- Rule shapes: `when` + `serve` allowed; **`rollout` forbidden**
- Composition at the call site: AND **`kind: release`** for gradual ship; AND companion **`kind: kill_switch`** for incidents (do not `kill-switch set` the entitlement name)
- Plan-wide entitlements in a **shared catalog** with **environment rules** only in the source catalog
- Optional **`expires`** for trials/SKU sunset (no warn when absent); contrast with **`kind: release`** cleanup semantics

Add schema example catalog(s) demonstrating shared-catalog entitlement, plan/`role` rules, companion kill switch, and optional stacked release flag. Examples must pass **`controlpath validate`**.

Cross-link from `configuration.md`, `rules.md`, `kill-switches.md`, and `quickstart.md`.

## Acceptance criteria

- [ ] New or expanded user doc (e.g. `docs/user/entitlements.md`) covers the topics above
- [ ] Cross-links added from existing user docs
- [ ] Schema example under `schemas/examples/` demonstrates entitlement patterns and passes `controlpath validate`
- [ ] No contradiction with `CONTEXT.md` glossary or ADR 0004

## Blocked by

- `.scratch/entitlement-flag-semantics/issues/01-reject-entitlement-rollout.md`
- `.scratch/entitlement-flag-semantics/issues/02-warn-entitlement-default-true.md`
