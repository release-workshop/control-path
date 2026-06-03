//! Infrastructure tests for integration-test helpers (workspace root, parallelism).

mod integration_test_helpers;

use integration_test_helpers::{typescript_runtime_built, workspace_root, TestProject};

#[test]
fn workspace_root_resolves_from_cargo_manifest_not_process_cwd() {
    let root = workspace_root();
    assert!(
        root.join("Cargo.toml").is_file(),
        "workspace root should contain Cargo.toml, got {}",
        root.display()
    );
    assert!(
        root.join("runtime/typescript").is_dir(),
        "workspace root should contain runtime/typescript, got {}",
        root.display()
    );
}

#[test]
fn integration_tests_use_per_project_current_dir_not_process_cwd() {
    let a = TestProject::new();
    let b = TestProject::new();
    a.write_file("marker-a.txt", "a");
    b.write_file("marker-b.txt", "b");
    assert!(a.file_exists("marker-a.txt"));
    assert!(b.file_exists("marker-b.txt"));
    assert!(!a.file_exists("marker-b.txt"));
    assert!(!b.file_exists("marker-a.txt"));
}

#[test]
fn typescript_runtime_detection_matches_dist_layout() {
    if typescript_runtime_built() {
        assert!(workspace_root()
            .join("runtime/typescript/dist/ast-loader.js")
            .is_file());
    }
}

/// When `runtime/typescript` is built locally, `assert_boolean_flag` must observe real evaluation.
/// Skips quietly without `dist` (workflow tests still run AST-only locally; CI enforces `dist` elsewhere).
#[test]
fn assert_boolean_flag_evaluates_enable_all_when_runtime_built() {
    if !typescript_runtime_built() {
        return;
    }

    let project = TestProject::with_definitions(
        r"catalog:
  id: test-service
mode: local
flags:
  probe_flag:
    default: false
    kind: release
environments:
  production:
    rules: {}
",
    );

    project.run_command_success(&[
        "flag",
        "enable",
        "probe_flag",
        "--env",
        "production",
        "--all",
    ]);
    project.run_command_success(&["compile", "--env", "production"]);
    project.assert_boolean_flag("probe_flag", "production", r#"{"id": "eval_probe"}"#, true);
}
