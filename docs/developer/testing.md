# Testing in Control Path

Canonical guide for **how the repo is tested**, **what to run locally**, and **which CI workflows enforce each gate**. For a copy-paste pre-merge checklist, see [Testing and quality gates](./testing-and-quality-gates.md).

The 2026-06-02 [testing strategy review](./testing-strategy-review.md) remains as background; this page is kept aligned with shipped CI (see `cargo test --test ci_workflow_gates`).

## Test layers

| Layer | Location | What it exercises |
|-------|----------|-------------------|
| Compiler unit / module tests | `crates/compiler/src/**` (`#[test]`, `#[tokio::test]`) | Catalog validation, parse/compile pipeline, runtime evaluation, schema-facing behavior |
| CLI unit tests | `crates/cli/src/**` (`#[cfg(test)]`) | Command logic, utilities, SDK generation helpers |
| CLI integration tests | `crates/cli/tests/**` | End-to-end `controlpath` subprocesses on isolated temp projects (workflows, commands, SaaS, watch, explain, etc.) |
| TypeScript runtime (Vitest) | `runtime/typescript/src/*.test.ts` | Loader, evaluator, polling; some tests shell out to the release CLI |
| SDK generator E2E (Vitest) | `tests/e2e/src/**` | CLI → compile → `generate-sdk` → generated evaluator (product-level path) |
| Compiler benchmarks (Criterion) | `crates/compiler/benches/compilation.rs` | Compile/parsing throughput and artifact size scaling (signal only; not merge-blocking) |

### CLI integration vs unit tests

- **Integration** (`crates/cli/tests/`): parallel-safe. Each case uses `TestProject` and sets `Command::current_dir` on the CLI subprocess—no process-wide `set_current_dir` or `#[serial]` in integration suites.
- **Unit** (`crates/cli/src/`): some tests use `DirGuard` and `#[serial]` when they must change the process working directory. If `cargo test --workspace` flakes on CLI unit tests, confirm with `cargo test --workspace -- --test-threads=1` and fix isolation rather than relying on serial runs long term.

Workflow integration tests that call `assert_boolean_flag` need **`runtime/typescript/dist`** built and **Node.js** on PATH. CI always builds the runtime before workspace tests; locally, evaluation is skipped when `dist` is absent (AST checks still run). Setting `CI=true` without a build fails the same way as CI.

Package-level notes for writing CLI integration tests: [`crates/cli/TESTING.md`](../../crates/cli/TESTING.md) and [`crates/cli/tests/README.md`](../../crates/cli/tests/README.md).

## Local verification

### Rust (`crates/compiler`, `crates/cli`, shared schemas)

From repo root:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cd runtime/typescript && npm ci && npm run build   # CLI workflow tests evaluate flags via Node + dist/
cd ../..
cargo test --workspace          # fast loop; CI uses llvm-cov (below)
cargo build --workspace
cargo build --release --bin controlpath
```

### Rust coverage (`controlpath-compiler`, `controlpath-cli`)

Tool: [`cargo llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov). Scope: workspace crates under `crates/compiler` and `crates/cli`.

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --all-features
cargo llvm-cov --workspace --all-features --html   # open target/llvm-cov/html/index.html
```

CI uploads LCOV with `fail_ci_if_error: false`. Thresholds are **report-only through 2026-09-01**; see [quality gates](./testing-and-quality-gates.md#rust-coverage-controlpath-compiler-controlpath-cli) for the ratchet note. Legacy tarpaulin workflows under `crates/cli/.github/workflows/` were removed—root workflows are canonical.

### Runtime TypeScript only

From `runtime/typescript`:

```bash
npm run lint
npm run typecheck
npm test
```

Vitest enforces **80%** line/function/branch/statement coverage in `vitest.config.ts` (also in CI).

### SDK generator E2E

After `cargo build --release --bin controlpath` and `runtime/typescript` build:

```bash
cd tests/e2e && npm ci
cd tests/e2e && npm run test:smoke   # pre-merge gate (src/smoke/; vitest.smoke.config.ts)
cd tests/e2e && npm test             # full post-merge-equivalent suite
```

Smoke covers one representative catalog path through CLI → compile → `generate-sdk` → evaluator. The full suite adds conditional rules, batch evaluation, overloads, errors, and additional runtime cases.

### Compiler benchmarks (local)

```bash
cargo bench -p controlpath-compiler --bench compilation
```

Results land under `target/criterion/`. Use for local profiling or before large compiler changes—not as a substitute for unit tests.

## CI workflows and gates

**`main-ci.yml`** (PRs, merge queue, pushes to `main`) runs the **full** pre-merge matrix on every push (no path filters), including `cargo llvm-cov --workspace --all-features --lcov --output-path rust-lcov.info` and TypeScript coverage.

**`auto-merge-validation.yml`** (`validation/**`, maintainer `git pushmain`) is a **fast land path**: diffs against `main`, runs **package-affected** Rust tests (`cargo test -p …`), skips coverage upload, and does not run Rust land jobs on TypeScript-only changes. Full gates still apply on PRs; post-merge E2E on `main` catches regressions after land.

| Gate | Command / artifact | `main-ci.yml` | `auto-merge-validation.yml` |
|------|-------------------|---------------|----------------------------|
| Rust format | `cargo fmt --all -- --check` | `rust-tests` | `rust-land` / `docs-land` |
| Workspace build | `cargo build --workspace` | `rust-tests` | — (land uses targeted `cargo test`) |
| Clippy | `cargo clippy --workspace …` | `rust-tests` | `rust-land` (when Rust paths change) |
| Rust tests | `cargo test --workspace` or `llvm-cov` | `llvm-cov` + upload | `cargo test -p controlpath-compiler`, `-p controlpath-cli`, or `--workspace` in `rust-land` (by path) |
| Release CLI | `cargo build --release --bin controlpath` | `build-cli` | `rust-land` or `build-cli` |
| Runtime TS lint / typecheck | `npm run lint`, `npm run typecheck` | Lint and typecheck | `lint-and-typecheck` |
| Runtime TS tests | `npm test` (+ coverage on main-ci) | coverage in `typescript-tests` | `npm test` only |
| E2E smoke | `npm run test:smoke` | E2E smoke (pre-merge) | `pre-merge-e2e-smoke` |
| Docs-only land | `cargo fmt --all -- --check` | — | `docs-land` |

**Post-merge:** `post-merge-e2e.yml` runs after Main CI succeeds on `main` and executes the **full** `tests/e2e` suite (`npm test`). Failures require follow-up on `main` but do not block the merge that already landed.

**Release PRs** (`release-please--branches--main`): Main CI skips most jobs by design. Use `scripts/setup-e2e-ruleset.sh` for post-merge E2E on release branches; verify release PRs still merge before tightening rules on `main`.

**Branch protection:** workflows alone do not block GitHub merges until required status checks exist. After merging policy changes, run `scripts/setup-main-pre-merge-ruleset.sh` so PRs to `main` require **E2E smoke (pre-merge)** and other Main CI job names.

Workflow YAML is also checked by `cargo test --test ci_workflow_gates` from the repo root.

### Scheduled compiler benchmarks (non-blocking)

Workflow: **`.github/workflows/compiler-benchmarks.yml`** (weekly schedule + `workflow_dispatch`). It runs:

```bash
cargo bench -p controlpath-compiler --bench compilation
```

This workflow is **not** a required check and does not block merge. It does **not** cache `target/criterion/` between runs so scheduled deltas are indicative, not a strict regression gate. Maintainers should look at results when:

- Investigating compile-time or artifact-size regressions on large catalogs
- Preparing a release candidate or large compiler refactor
- A scheduled run fails repeatedly (environment or bench breakage)

Compare Criterion HTML under the workflow artifact or `target/criterion/` locally; large unexplained throughput drops or artifact-size growth warrant an issue, not an automatic revert.

## Known limitations

- **Watch / interactive CLI**: limited automated coverage; see [`crates/cli/TESTING.md`](../../crates/cli/TESTING.md).
- **MessagePack signatures**: compiler round-trip tests use canonical `serialize()` map encoding; do not use `rmp_serde::to_vec` for signed artifacts (see compiler tests).
- **Doc drift**: if you change CI steps or test layout, update this page, [quality gates](./testing-and-quality-gates.md), and run `ci_workflow_gates` tests.

## Related docs

- [Testing and quality gates](./testing-and-quality-gates.md) — contributor checklist
- [`DEVELOPING.md`](../../DEVELOPING.md) — onboarding map
- [`AGENTS.md`](../../AGENTS.md) — agent verification commands
- [Testing strategy review](./testing-strategy-review.md) — 2026-06-02 assessment (historical)
