# Testing and Quality Gates

Copy-paste checklist before marking work done. For test layers, CI job mapping, E2E smoke vs post-merge, coverage policy, and known limitations, see the canonical hub: **[Testing in Control Path](testing.md)** (`docs/developer/testing.md`).

## Rust changes (`crates/compiler`, `crates/cli`, shared schemas)

Run from repo root:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cd runtime/typescript && npm ci && npm run build   # CLI workflow tests evaluate flags via Node + dist/
cd ../..
cargo test --workspace          # fast local loop; CI runs the same tests via llvm-cov below
cargo build --workspace
cargo build --release --bin controlpath
```

**CLI integration tests** (`crates/cli/tests/`) are parallel-safe (isolated temp dirs; CLI invoked with per-project `current_dir`). **CLI unit tests** in `src/` may still need serial execution when they use `DirGuard` to change the process cwd.

If parallel **unit** tests flake due to working-directory pollution:

```bash
cargo test --workspace -- --test-threads=1
```

`assert_boolean_flag` needs `runtime/typescript/dist/ast-loader.js` and **Node.js** on PATH. In CI, missing `dist` fails the test; locally, evaluation is skipped only when `dist` is absent (AST checks still run).

Pre-merge CI builds `runtime/typescript` in the `rust-tests` job before `cargo llvm-cov`.

## Rust coverage (`controlpath-compiler`, `controlpath-cli`)

**Tool:** [`cargo llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) (LLVM source-based coverage; no OpenSSL dependency unlike tarpaulin).

**Scope:** workspace crates `crates/compiler` and `crates/cli` (`cargo llvm-cov --workspace`).

**Local:**

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --all-features
cargo llvm-cov --workspace --all-features --html   # open target/llvm-cov/html/index.html
```

**CI:** `main-ci` and `auto-merge-validation` run the same workspace command, emit `rust-lcov.info`, and upload to Codecov with `fail_ci_if_error: false`.

**Thresholds:** report-only through **2026-09-01**. After that date, revisit blocking line/branch thresholds in CI (ratchet from the report baseline; TypeScript runtime already enforces 80% in `runtime/typescript/vitest.config.ts`).

The legacy tarpaulin workflow under `crates/cli/.github/workflows/` was removed; root workflows are canonical.

## Runtime TypeScript-only changes

Run from `runtime/typescript`:

```bash
npm run lint
npm run typecheck
npm test
```

## SDK generator E2E (`tests/e2e`)

Pre-merge CI runs a **smoke** slice; the full suite runs **post-merge** on `main`.

Local commands (from repo root after `cargo build --release --bin controlpath`):

```bash
cd runtime/typescript && npm ci && npm run build
cd tests/e2e && npm ci
cd tests/e2e && npm run test:smoke   # pre-merge gate (src/smoke/ only; vitest.smoke.config.ts)
cd tests/e2e && npm test             # full post-merge-equivalent suite
```

Smoke covers CLI → compile → `generate-sdk` → generated evaluator for one representative catalog path. Full E2E adds conditional rules, batch evaluation, overloads, error handling, and additional runtime integration cases.

## Documentation and command updates

When behavior changes:

- update user docs under `docs/user/`
- update developer docs under `docs/developer/`
- ensure `README.md` and `DEVELOPING.md` still point to correct pages

## CI expectations

**Pre-merge** (`main-ci` on PRs / merge queue / pushes to `main`, and `auto-merge-validation` on `validation/**` branches) must pass.

Gate-to-workflow mapping (commands, job names, post-merge E2E): **[Testing in Control Path — CI workflows and gates](./testing.md#ci-workflows-and-gates)**.

`main-ci` runs all gates on every push (no path filters). `auto-merge-validation` is the fast, package-affected land path for `validation/**` (see [hub — CI workflows](./testing.md#ci-workflows-and-gates)); TypeScript-only pushes skip Rust land jobs there.

**Post-merge** (`post-merge-e2e` after `Main CI` succeeds on `main`): runs `npm test` in `tests/e2e` (full suite). Failures require follow-up on `main` but do not block the merge that already landed.

Local success on the commands above should match CI; workflow contracts are also checked by `cargo test --test ci_workflow_gates`.

**Branch protection (required to ship issue 01):** workflows alone do not block GitHub merges until required status checks exist. Run `scripts/setup-main-pre-merge-ruleset.sh` after merge (idempotent: creates or updates the ruleset) so PRs to `main` require `E2E smoke (pre-merge)` and other Main CI job names. **Release PRs** (`release-please--branches--main`): Main CI skips jobs by design — use `scripts/setup-e2e-ruleset.sh` for post-merge E2E only; verify release PRs still merge before tightening rules on `main`.
