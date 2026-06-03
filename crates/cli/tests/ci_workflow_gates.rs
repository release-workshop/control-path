//! Ensures root merge CI workflows enforce the same gates documented for contributors.

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

const PRE_MERGE_RUST_GATES: &[&str] = &[
    "cargo fmt --all -- --check",
    "cargo build --workspace",
    "cargo clippy",
    "cargo llvm-cov --workspace --all-features --lcov --output-path rust-lcov.info",
    "cargo build --release --bin controlpath",
];

#[test]
fn main_ci_enforces_documented_pre_merge_rust_gates() {
    let workflow = read_workflow("main-ci.yml");
    for gate in PRE_MERGE_RUST_GATES {
        assert_workflow_contains(&workflow, "main-ci.yml", gate);
    }
    assert_workflow_contains(&workflow, "main-ci.yml", "npm run test:smoke");
    assert_workflow_contains(&workflow, "main-ci.yml", "controlpath-cli-release");
}

#[test]
fn auto_merge_validation_enforces_documented_pre_merge_rust_gates() {
    let workflow = read_workflow("auto-merge-validation.yml");
    for gate in PRE_MERGE_RUST_GATES {
        assert_workflow_contains(&workflow, "auto-merge-validation.yml", gate);
    }
    assert_workflow_contains(&workflow, "auto-merge-validation.yml", "npm run test:smoke");
    assert_workflow_contains(
        &workflow,
        "auto-merge-validation.yml",
        "controlpath-cli-release",
    );
}

#[test]
fn auto_merge_validation_runs_rust_gates_on_typescript_changes() {
    let workflow = read_workflow("auto-merge-validation.yml");
    let rust_job = workflow
        .split("rust-tests:")
        .nth(1)
        .and_then(|s| s.split("\n  build-cli:").next())
        .expect("rust-tests job block");
    assert!(
        rust_job.contains("needs.changes.outputs.typescript == 'true'"),
        "typescript-only validation pushes must still run Rust pre-merge gates"
    );
}

#[test]
fn auto_merge_validation_wires_workflows_filter_and_blocks_empty_merge() {
    let workflow = read_workflow("auto-merge-validation.yml");
    assert_workflow_contains(
        &workflow,
        "auto-merge-validation.yml",
        "needs.changes.outputs.workflows == 'true'",
    );
    assert_workflow_contains(
        &workflow,
        "auto-merge-validation.yml",
        "needs.changes.outputs.e2e == 'true'",
    );
    assert_workflow_contains(
        &workflow,
        "auto-merge-validation.yml",
        "needs.changes.outputs.typescript == 'true'",
    );
    assert!(
        !workflow.contains("outputs.schemas"),
        "schemas/** is covered by the rust path filter; do not reference a dead schemas output"
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
