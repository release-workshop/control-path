Status: done

## What to build

When a **local-mode** service catalog opts in (**`attributes:`** present), **`validate`** and **`compile`** reject **environment rules** and **segments** whose `when` expressions reference property names outside **base attributes** ∪ service **attribute schema** (top-level names only).

Property extraction covers bare identifiers and legacy `user.` / `context.` prefix sugar (normalized the same way as compile). Dot paths (e.g. `profile.tier`) are out of scope for v1 strict checks unless the top-level segment is declared.

**SaaS mode** service catalogs skip **environment rule** property checks (no local rules in Git).

## Acceptance criteria

- [x] Local catalog with **`attributes: { plan: string }`**: rule `plan == 'beta'` validates; `tier == 'x'` fails with a clear error
- [x] Segment `when` strings receive the same validation
- [x] Catalog without **`attributes:`** does not run property-name validation on rules
- [x] **`mode: saas`** catalog with **`attributes:`** validates schema shape but does not require local **`environments`** rule property checks
- [x] Tests cover at least one failing and one passing **`controlpath validate`** / **`compile`** path

## Blocked by

- `.scratch/evaluation-attribute-schema/issues/02-attribute-schema-parse-and-validate.md`
