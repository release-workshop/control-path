# Platform spine — catalog CLI/compiler deepening

Parent initiative from architecture review (2026-06). Deepens shallow seams between Git **flag catalog**, **compiled artifact**, SDK generation, and runtime — without changing ADR-0001 deploy velocities.

## Decisions (locked)

| # | Topic | Decision |
|---|--------|----------|
| 1 | YAML authoring | Full re-serialization on save is acceptable (no comment preservation). |
| 2 | SaaS embedded environments | Disk-only: environments for CDN URLs come from `.controlpath/<env>.ast` at `generate-sdk` time, not a Git-declared list. |
| 3 | `explain` in SaaS mode | Supported for local dev when sync cache exists: **environment rules** come from the **compiled artifact**; **flag catalog** metadata (`reason`, `lifecycle`) from Git + **imports**. Same “no `.ast` → actionable error” as `generate-sdk`. |
| 4 | `skip_validation` / FastPath | Remove. All compile and SDK paths run full schema + semantic validation (including imports). |
| 5 | Legacy compile shim | Delete v1 JSON adapter in a single PR once native path is ready. |
| 6 | Generated SDK / runtime | Breaking change: bump `@controlpath/runtime` (and generated SDK contract); no external users yet. |

## Sequence

1. Validation modes → 2. Catalog store → 3. Load entry points → 4. Generate-sdk unify → 5. SaaS URL seam → 6. Native compile → 7. Explain trace → 8. Thin SDK / deep runtime

## References

- `CONTEXT.md`, `docs/adr/0001-compiled-artifact-runtime-delivery.md`
- `.scratch/cli-salvage-redesign/schema-decisions.md`
- Prior salvage issues (done): `.scratch/cli-salvage-redesign/issues/`
