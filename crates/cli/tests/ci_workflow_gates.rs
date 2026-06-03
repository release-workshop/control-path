//! Ensures root merge CI workflows enforce the gates documented for contributors.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn read_workflow(name: &str) -> String {
    let path = repo_root().join(".github/workflows").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn read_e2e_package_json() -> String {
    let path = repo_root().join("tests/e2e/package.json");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn assert_workflow_contains(workflow: &str, label: &str, needle: &str) {
    assert!(
        workflow.contains(needle),
        "{label} must run `{needle}` as an explicit CI step"
    );
}

fn assert_workflow_lacks(workflow: &str, label: &str, needle: &str) {
    assert!(
        !workflow.contains(needle),
        "{label} must not reference `{needle}`"
    );
}

/// Full pre-merge gates for PRs, merge queue, and pushes to main.
const MAIN_CI_PRE_MERGE_RUST_GATES: &[&str] = &[
    "cargo fmt --all -- --check",
    "cargo build --workspace",
    "cargo clippy",
    "cargo llvm-cov --workspace --all-features --lcov --output-path rust-lcov.info",
    "cargo build --release --bin controlpath",
];

/// Pre-merge validation branch gates (integration tests run on main via main-ci.yml).
const VALIDATION_LAND_RUST_GATES: &[&str] = &[
    "cargo fmt --all -- --check",
    "cargo clippy --workspace --all-targets --all-features -- -D warnings",
    "cargo test -p controlpath-compiler",
    "cargo test -p controlpath-cli -p controlpath-compiler",
    "cargo test --workspace",
];

const VALIDATION_LAND_TYPESCRIPT_GATES: &[&str] = &["npm run lint", "npm run typecheck"];

#[test]
fn main_ci_enforces_documented_pre_merge_rust_gates() {
    let workflow = read_workflow("main-ci.yml");
    for gate in MAIN_CI_PRE_MERGE_RUST_GATES {
        assert_workflow_contains(&workflow, "main-ci.yml", gate);
    }
    assert_workflow_contains(&workflow, "main-ci.yml", "npm run test:smoke");
    assert_workflow_contains(&workflow, "main-ci.yml", "controlpath-cli-release");
    assert_workflow_contains(
        &workflow,
        "main-ci.yml",
        "Build TypeScript runtime (CLI integration evaluation)",
    );
}

#[test]
fn auto_merge_validation_uses_pre_merge_gates_only() {
    let workflow = read_workflow("auto-merge-validation.yml");
    for gate in VALIDATION_LAND_RUST_GATES {
        assert_workflow_contains(&workflow, "auto-merge-validation.yml", gate);
    }
    for gate in VALIDATION_LAND_TYPESCRIPT_GATES {
        assert_workflow_contains(&workflow, "auto-merge-validation.yml", gate);
    }
    for needle in [
        "cargo llvm-cov",
        "codecov/codecov-action",
        "npm run test:smoke",
        "run: npm test",
        "controlpath-cli-release",
        "upload-artifact@v4",
        "pre-merge-e2e-smoke:",
        "typescript-tests:",
        "build-cli:",
    ] {
        assert_workflow_lacks(&workflow, "auto-merge-validation.yml", needle);
    }
    assert_workflow_contains(&workflow, "auto-merge-validation.yml", "base: main");
    assert_workflow_contains(
        &workflow,
        "auto-merge-validation.yml",
        "Build TypeScript runtime (CLI integration tests)",
    );
}

#[test]
fn main_ci_runs_deferred_integration_gates() {
    let workflow = read_workflow("main-ci.yml");
    assert_workflow_contains(&workflow, "main-ci.yml", "npm run test:smoke");
    assert_workflow_contains(&workflow, "main-ci.yml", "npm test -- --coverage");
    assert_workflow_contains(&workflow, "main-ci.yml", "controlpath-cli-release");
}

#[test]
fn auto_merge_validation_does_not_run_rust_checks_on_typescript_only() {
    let workflow = read_workflow("auto-merge-validation.yml");
    let rust_checks = workflow
        .split("rust-checks:")
        .nth(1)
        .and_then(|s| s.split("\n  lint-and-typecheck:").next())
        .expect("rust-checks job block");
    assert!(
        !rust_checks.contains("needs.changes.outputs.typescript == 'true'"),
        "typescript-only validation pushes must not run rust-checks"
    );
}

#[test]
fn auto_merge_validation_wires_package_path_filters() {
    let workflow = read_workflow("auto-merge-validation.yml");
    for needle in [
        "crates/compiler/**",
        "crates/cli/**",
        "Cargo.toml",
        "needs.changes.outputs.compiler",
        "needs.changes.outputs.cli",
        "needs.changes.outputs.workspace",
        "docs-format:",
    ] {
        assert!(
            workflow.contains(needle),
            "auto-merge-validation.yml must reference `{needle}`"
        );
    }
    assert!(
        !workflow.contains("outputs.schemas"),
        "schemas/** is under the compiler filter; do not reference a dead schemas output"
    );
}

#[test]
fn e2e_smoke_uses_dedicated_vitest_config() {
    let package = read_e2e_package_json();
    assert!(
        package.contains("vitest.smoke.config.ts"),
        "test:smoke must use vitest.smoke.config.ts, not a title filter"
    );
    let smoke_config = fs::read_to_string(repo_root().join("tests/e2e/vitest.smoke.config.ts"))
        .expect("vitest.smoke.config.ts");
    assert!(
        smoke_config.contains("src/smoke"),
        "smoke config must only include src/smoke tests"
    );
    assert!(
        !smoke_config.contains("'**/smoke/**'"),
        "smoke config must not inherit the full-suite exclude on src/smoke"
    );
}

fn read_developer_doc(name: &str) -> String {
    let path = repo_root().join("docs/developer").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn canonical_testing_doc_lists_layers_and_pre_merge_gates() {
    let doc = read_developer_doc("testing.md");
    for gate in MAIN_CI_PRE_MERGE_RUST_GATES {
        assert!(
            doc.contains(gate),
            "docs/developer/testing.md must document main-ci gate `{gate}`"
        );
    }
    assert!(
        doc.contains("cargo test -p controlpath-compiler"),
        "canonical testing doc must describe validation affected Rust tests"
    );
    assert!(
        doc.contains("npm run test:smoke"),
        "canonical testing doc must describe E2E smoke"
    );
    assert!(doc.contains("main-ci.yml"), "must map gates to Main CI");
    assert!(
        doc.contains("auto-merge-validation.yml"),
        "must map gates to auto-merge validation"
    );
    assert!(
        doc.contains("post-merge-e2e.yml"),
        "must describe post-merge E2E"
    );
    assert!(
        doc.contains("crates/compiler"),
        "must describe compiler tests"
    );
    assert!(
        doc.contains("crates/cli/tests"),
        "must describe CLI integration tests"
    );
    assert!(
        doc.contains("runtime/typescript"),
        "must describe TypeScript runtime tests"
    );
}

const HUB_PATH: &str = "docs/developer/testing.md";
const HUB_CI_GATES_ANCHOR: &str = "#ci-workflows-and-gates";

fn assert_repo_entry_points_link_to_testing_hub(content: &str, label: &str) {
    assert!(
        content.contains(HUB_PATH),
        "{label} must link to `{HUB_PATH}` (not merely mention testing.md)"
    );
}

#[test]
fn contributor_docs_link_to_canonical_testing_page() {
    for (content, label) in [
        (read_repo_file("DEVELOPING.md"), "DEVELOPING.md"),
        (read_repo_file("README.md"), "README.md"),
        (read_repo_file("CONTRIBUTING.md"), "CONTRIBUTING.md"),
        (read_repo_file("AGENTS.md"), "AGENTS.md"),
    ] {
        assert_repo_entry_points_link_to_testing_hub(&content, label);
    }
    let gates = read_developer_doc("testing-and-quality-gates.md");
    assert!(
        gates.contains("./testing.md"),
        "testing-and-quality-gates.md must link to the hub (relative path)"
    );
}

#[test]
fn checklist_links_to_hub_ci_workflows_section() {
    let gates = read_developer_doc("testing-and-quality-gates.md");
    assert!(
        gates.contains(&format!("./testing.md{HUB_CI_GATES_ANCHOR}")),
        "checklist must link to the hub CI workflows section anchor"
    );
    let hub = read_developer_doc("testing.md");
    assert!(
        hub.contains("## CI workflows and gates"),
        "hub must keep the heading that generates the ci-workflows-and-gates anchor"
    );
}

#[test]
fn cli_package_testing_docs_point_at_canonical_hub() {
    let cli_testing = read_repo_file("crates/cli/TESTING.md");
    let cli_tests_readme = read_repo_file("crates/cli/tests/README.md");
    let hub = "docs/developer/testing.md";
    assert!(
        cli_testing.contains(hub),
        "crates/cli/TESTING.md must point to the canonical hub"
    );
    assert!(
        cli_tests_readme.contains(hub),
        "crates/cli/tests/README.md must point to the canonical hub"
    );
}

#[test]
fn compiler_benchmark_workflow_is_scheduled_and_documented() {
    let workflow = read_workflow("compiler-benchmarks.yml");
    assert!(
        workflow.contains("schedule:"),
        "compiler-benchmarks.yml must run on a schedule"
    );
    assert_workflow_contains(
        &workflow,
        "compiler-benchmarks.yml",
        "cargo bench -p controlpath-compiler --bench compilation",
    );
    let doc = read_developer_doc("testing.md");
    assert!(
        doc.contains("compiler-benchmarks.yml"),
        "docs/developer/testing.md must document the benchmark workflow"
    );
    let cache_step = workflow
        .split("Cache Cargo registry")
        .nth(1)
        .and_then(|s| s.split("Run Criterion").next())
        .expect("cache step block");
    assert!(
        !cache_step.contains("target/"),
        "benchmark workflow must not cache target/ (avoids stale Criterion baselines)"
    );
}

#[test]
fn post_merge_e2e_workflow_still_runs_full_suite() {
    let workflow = read_workflow("post-merge-e2e.yml");
    assert_workflow_contains(&workflow, "post-merge-e2e.yml", "npm test --");
    assert!(
        !workflow.contains("npm run test:smoke"),
        "post-merge E2E should run the full suite, not the smoke script"
    );
    assert_workflow_contains(
        &workflow,
        "post-merge-e2e.yml",
        "workflow_run.conclusion == 'success'",
    );
}
