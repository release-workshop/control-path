Status: done

# Reject rollout on entitlement environment rules

## Parent

- `.scratch/entitlement-flag-semantics/PRD.md`
- `docs/adr/0004-entitlement-flag-semantics.md`

## What to build

Deliver the first tracer bullet for **entitlement** rule governance: a catalog author cannot use percentage **`rollout`** rules on flags with `kind: entitlement`.

End-to-end behavior:

1. **`controlpath validate`** and **`compile`** emit a **validation error** when any **environment rule** for an entitlement flag includes `rollout` (entitlements may still use `when` and plain `serve` — unlike **kill_switch** flags, which forbid `when`).
2. Error messages and paths mirror the existing **kill_switch** rollout constraint style (actionable suggestion in the error).
3. Valid entitlement catalogs with `when` + `serve` continue to validate and compile unchanged.

Mirror the existing **kill_switch** rule constraint pattern in the compiler validator; do not add runtime evaluation changes.

## Acceptance criteria

- [x] Compiler rejects `rollout` on **environment rules** for flags with `kind: entitlement`
- [x] Compiler unit tests: invalid catalog (entitlement + rollout) fails; valid catalog (entitlement + `when` + `serve`) passes
- [x] `configuration.md` or `rules.md` rule-types table notes that **`kind: entitlement`** forbids `rollout` ( **`when`** allowed)
- [x] `cargo test`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo fmt --all -- --check` pass

## Blocked by

None — can start immediately
