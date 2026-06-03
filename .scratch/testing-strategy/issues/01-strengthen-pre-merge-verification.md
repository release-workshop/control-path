Status: done

# Strengthen pre-merge verification (CI + E2E smoke)

## Parent

Derived from [Testing strategy review](../../../docs/developer/testing-strategy-review.md).

## What to build

Bring **pre-merge** CI in line with the gates documented for Rust contributors, and add a **fast, required** SDK-generator smoke path before merge while keeping the existing post-merge E2E workflow for full verification.

End-to-end outcome: a PR or validation-branch push cannot merge unless formatting and workspace builds pass alongside existing clippy/tests, and a representative CLI → compile → generate-sdk → evaluator path is exercised before code lands on `main`. Contributor docs describe the same commands CI runs.

## Acceptance criteria

- [x] Root merge workflows (`main-ci`, `auto-merge-validation`) run `cargo fmt --all -- --check` and `cargo build --workspace` as explicit failing steps alongside clippy, workspace tests (CI: `cargo llvm-cov --workspace --all-features`), and release CLI build.
- [x] A pre-merge job runs a **smoke subset** of `tests/e2e` on PRs / merge queue / validation branches; failures block merge when branch protection is applied.
- [x] Full `post-merge-e2e` remains for post-merge verification; smoke vs full scope is documented.
- [x] `docs/developer/testing-and-quality-gates.md` lists pre-merge gates; `DEVELOPING.md` and `CONTRIBUTING.md` link to it.

## Shipping (repo admin)

- [ ] Run `scripts/setup-main-pre-merge-ruleset.sh` (or equivalent) so PRs to `main` require `E2E smoke (pre-merge)` and other Main CI contexts.
- [ ] Confirm `release-please--branches--main` PRs still merge (Main CI skips those jobs; use `scripts/setup-e2e-ruleset.sh` for release PRs).

## Blocked by

None — can start immediately.
