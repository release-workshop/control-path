# Testing Strategy Review

Date: 2026-06-02
Scope reviewed: `crates/compiler`, `crates/cli`, `runtime/typescript`, `tests/e2e`, CI workflows, and contributor docs.

## Current testing landscape

### 1) Rust unit and module tests

Present in both Rust crates via `#[test]` / `#[tokio::test]`:

- `crates/compiler/src/**` (catalog validation, parser/compiler behavior, runtime evaluation, schema-facing behavior)
- `crates/cli/src/**` (command behavior, utilities, SDK generation, SaaS adapters)

Observed characteristics:

- Good breadth on validation and compatibility scenarios (especially in compiler catalog tests).
- Tests are mostly behavior-oriented and use real parsing/compilation flows.
- Two tests are currently `#[ignore]` due to a known MessagePack ordering issue (in compiler).

### 2) Rust CLI integration tests

Present in `crates/cli/tests/**`:

- Workflow and command suites (workflows, commands, error cases, imports, lifecycle, watch, SaaS, explain, debug UI, legacy prune).
- Shared helper infrastructure in `integration_test_helpers.rs`.

Observed characteristics:

- Strong end-to-end command coverage with temporary project fixtures.
- Many tests are marked `#[serial]`, and helper code uses a process-wide CWD mutex, indicating concurrency sensitivity.
- Some assertions degrade to weaker fallback checks when runtime evaluation is unavailable (for example, asserting YAML text instead of final behavior).

### 3) TypeScript runtime tests (Vitest)

Present in `runtime/typescript/src/*.test.ts`:

- Loader/evaluator/resolve tests
- Polling and coordinator tests (artifact + kill switch)
- Integration tests that compile a catalog through the Rust CLI and evaluate runtime behavior
- Performance-oriented test file and explicit coverage thresholds in `vitest.config.ts` (80% lines/functions/branches/statements)

Observed characteristics:

- Good boundary testing at runtime APIs and artifact lifecycle.
- Coverage thresholds are configured and enforced for this package in CI.

### 4) SDK generator E2E tests

Present in `tests/e2e/src/sdk-generator.e2e.test.ts` (Vitest, sequential suite):

- Generates SDK from catalog, compiles artifacts, compiles generated TS, executes evaluator flows.
- Covers simple/conditional/default/batch/context/overloads/error-handling/runtime integration scenarios.

Observed characteristics:

- This is genuine product-level E2E across CLI + generated SDK + runtime behavior.
- Currently run in **post-merge** workflow (`post-merge-e2e.yml`) and not as a required pre-merge gate in `main-ci.yml`.

### 5) Performance benchmarks

Present in `crates/compiler/benches/compilation.rs` with Criterion:

- Compilation and parsing throughput
- Artifact size scaling checks

Observed characteristics:

- Useful performance signals exist, but they are not integrated into merge gating.

### 6) Quality gates and static checks

Documented required gates (developer docs and AGENTS rules):

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo build --workspace`
- `cargo build --release --bin controlpath`
- Runtime-only: `npm run lint`, `npm run typecheck`, `npm test`

CI reality from `main-ci.yml` / `auto-merge-validation.yml`:

- Rust: clippy + `cargo test --workspace`
- Rust build of release CLI
- TypeScript runtime: lint + typecheck + tests with coverage
- Missing as explicit CI steps: `cargo fmt --all -- --check`, `cargo build --workspace`

## Assessment

Overall: the project has a **strong multi-layer test foundation** (unit/module + integration + package integration + E2E + performance benchmarks). The main strategy risk is not lack of tests, but **mismatch and blind spots in enforcement and reliability**.

## Gaps and risks

1. **Documented vs enforced gate drift**
   - Docs require formatting and full workspace builds, but primary CI workflows do not clearly enforce them as dedicated steps.
   - This can create "passes locally per docs, fails elsewhere" or the reverse.

2. **E2E timing risk (post-merge only)**
   - The strongest system test (`tests/e2e`) runs after merge.
   - Regressions can land on `main` before detection.

3. **Rust coverage policy is ambiguous**
   - Runtime TypeScript has enforced coverage thresholds.
   - Rust has coverage guidance (including tarpaulin references), but no consistent, clearly active root CI policy tied to merges.
   - A coverage workflow exists under `crates/cli/.github/workflows/coverage.yml`, which may be easy to miss compared with root workflows.

4. **Integration test flakiness pressure**
   - Serial execution and CWD locking are practical mitigations, but they also signal global-state coupling that can limit parallelism and speed.

5. **Ignored tests carry known defects**
   - `#[ignore]` tests tied to signature/messagepack ordering indicate unresolved correctness debt.

6. **Documentation staleness around test guarantees**
   - Some docs describe broad guarantees ("comprehensive", strict runtime targets, coverage goals) that should be continuously validated against actual CI behavior.

## Recommended changes (no code changes in this review)

### High priority

1. **Align CI with documented required gates**
   - In root CI workflows, add explicit steps for:
     - `cargo fmt --all -- --check`
     - `cargo build --workspace`
   - Keep the current clippy/test/build-release steps.

2. **Shift at least one representative E2E suite to pre-merge required checks**
   - Keep full post-merge E2E for broad verification.
   - Add a faster "smoke E2E" pre-merge slice (or gate full E2E for changed paths).

3. **Define one canonical Rust coverage strategy**
   - Decide tool and policy (e.g., `cargo llvm-cov` for CI reproducibility).
   - Document where coverage runs, which crates are in scope, and expected threshold/ratchet behavior.

### Medium priority

4. **Reduce serial-only dependence in CLI integration tests**
   - Continue using serial where unavoidable, but track and remove avoidable global-CWD/shared-state dependencies.
   - Favor per-test process isolation and explicit working directories over global state.

5. **Track and resolve ignored tests with owner + exit criteria**
   - For each ignored test, record:
     - owning team/person
     - root cause
     - target milestone
     - unignore condition

6. **Strengthen assertion quality in fallback paths**
   - Where runtime evaluation is optional, avoid relying only on string contains checks for behavior-critical paths.
   - Prefer deterministic artifact-level assertions when runtime execution is unavailable.

### Nice-to-have

7. **Promote benchmark guidance into explicit release checks**
   - Define when Criterion benchmarks are run (nightly/release-candidate).
   - Add simple regression thresholds for key scenarios (compile throughput, artifact size growth).

8. **Consolidate testing docs**
   - Keep one canonical strategy doc (this file can evolve into it).
   - Treat package-specific testing docs as implementation details linked from the canonical page.

## Suggested next action plan

1. Make CI/doc alignment the first change set (fastest risk reduction).
2. Introduce pre-merge E2E smoke gate as second change set.
3. Decide and standardize Rust coverage policy as third change set.

This sequence improves confidence quickly without large test rewrites.
