Status: done

# Canonical testing documentation (+ optional benchmark signal)

## Parent

Derived from [Testing strategy review](../../../docs/developer/testing-strategy-review.md).

## What to build

Make **one canonical testing guide** the hub for how Control Path is tested and gated, linked from contributor onboarding docs. Demote overlapping package-level testing write-ups to pointers so guarantees do not drift from CI reality.

Optionally add a **scheduled** (non-blocking) compiler benchmark workflow using existing Criterion benches, with short guidance on when maintainers should look at results.

End-to-end outcome: a new contributor can read a single developer doc for test layers, local commands, CI jobs, E2E smoke vs post-merge, Rust/TS coverage, and known limitations—without conflicting stories in `crates/cli/TESTING.md` and scattered workflow comments.

## Acceptance criteria

- [x] A canonical page under `docs/developer/` (evolving from or replacing the strategy review) describes test layers, local verification commands, and which CI workflows enforce each gate.
- [x] `DEVELOPING.md` and `docs/developer/testing-and-quality-gates.md` link to the canonical page; duplicate claims in `crates/cli/TESTING.md` / `crates/cli/tests/README.md` are trimmed to pointers or updated to match.
- [x] Documented gates match shipped behavior from issues `01` and `02` (pre-merge CI, E2E smoke, Rust coverage).
- [x] (Optional) A scheduled workflow runs `crates/compiler` Criterion benchmarks and docs explain interpretation; merge is not blocked unless explicitly decided and documented.

## Blocked by

- `.scratch/testing-strategy/issues/01-strengthen-pre-merge-verification.md`
- `.scratch/testing-strategy/issues/02-rust-coverage-and-compiler-test-debt.md`
