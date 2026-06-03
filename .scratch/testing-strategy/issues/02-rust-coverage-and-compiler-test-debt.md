Status: done

# Rust coverage and compiler test debt

## Parent

Derived from [Testing strategy review](../../../docs/developer/testing-strategy-review.md).

## What to build

Establish **one canonical Rust coverage story** for the workspace (compiler + CLI), wire it into root CI, and clear **compiler test debt** from ignored MessagePack signature round-trip tests.

End-to-end outcome: contributors know which tool to run locally, CI produces consistent coverage artifacts on merge paths, orphan or misleading coverage workflow docs are consolidated, and signature round-trip tests run in the default suite without `#[ignore]`.

Compiler test debt was **mislabeled** as “map field ordering”: production already uses `serialize()` (MessagePack **map** for JS/TS). The failing tests used `rmp_serde::to_vec` (**tuple** encoding). Fix: round-trip via `serialize()`; regression tests document canonical map encoding, optional key order for external writers, and that `to_vec` must not be used for signed artifacts.

Record the coverage policy choice (tool, scope, report-only vs threshold ratchet) in developer documentation as part of this slice—no separate policy ticket required.

## Acceptance criteria

- [x] Developer docs state the chosen coverage tool, scoped crates, how to run locally, and what CI enforces (including whether thresholds are blocking).
- [x] Root CI runs Rust coverage on the same paths that gate merges (or documents an intentional report-only phase with a follow-up ratchet date in the doc).
- [x] Redundant or orphaned coverage workflow under `crates/cli/.github/workflows/` is removed, redirected, or clearly superseded by root workflows.
- [x] MessagePack signature round-trip tests in the compiler are fixed and no longer `#[ignore]`; regression coverage documents canonical `serialize()` map encoding vs `rmp_serde::to_vec` tuple encoding (not map-key-order bugs in production).

## Notes

`docs/developer/testing-and-quality-gates.md` is the shared canonical gate doc; E2E smoke and other pre-merge rows from issue 01 live there too—issue 04 may further consolidate package-specific testing docs.

## Blocked by

None — can start immediately.
