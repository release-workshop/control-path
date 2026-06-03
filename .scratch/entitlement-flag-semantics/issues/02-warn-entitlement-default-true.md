Status: ready-for-agent

# Warn when entitlement catalog default is true

## Parent

- `.scratch/entitlement-flag-semantics/PRD.md`
- `docs/adr/0004-entitlement-flag-semantics.md`

## What to build

Deliver fail-closed authoring governance for **entitlement** flags: `controlpath validate` warns when a flag with `kind: entitlement` has `default: true`.

End-to-end behavior:

1. **`controlpath validate`** emits a **warning** (not an error) when `flags.<name>.default` is `true` and `kind` is `entitlement`.
2. Omitting **`expires`** on an entitlement emits **no** warning (unlike **`kind: release`**, which may warn when `expires` is missing).
3. Warning style matches existing catalog metadata warnings (path, message suitable for strict CI treating warnings as errors).

No runtime or SDK changes — validation-only slice.

## Acceptance criteria

- [ ] Compiler semantic warnings include entitlement + `default: true`
- [ ] Compiler unit tests: warning emitted for `default: true`; no warning for `default: false`; no `expires`-missing warning on entitlements
- [ ] Brief note in user docs that entitlements should use `default: false` for deny-by-default access
- [ ] `cargo test`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo fmt --all -- --check` pass

## Blocked by

None — can start immediately
