//! Integration tests for catalog import resolution and validation.

mod integration_test_helpers;

use integration_test_helpers::*;
use serial_test::serial;
use std::fs;

fn write_import_fixture(project: &TestProject) {
    let fixture_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/examples");

    let platform_dir = project.path("platform");
    fs::create_dir_all(&platform_dir).unwrap();
    fs::copy(
        fixture_root.join("shared-platform.control-path.yaml"),
        platform_dir.join("control-path.yaml"),
    )
    .unwrap();

    let mut imported =
        fs::read_to_string(fixture_root.join("imported-global.control-path.yaml")).unwrap();
    imported = imported.replace(
        "path: ../../platform/control-path.yaml",
        "path: platform/control-path.yaml",
    );
    project.write_file("control-path.yaml", &imported);
}

#[test]
#[serial]
fn validate_and_compile_succeed_with_resolved_imports() {
    let project = TestProject::new();
    write_import_fixture(&project);

    project.run_command_success(&["validate", "--all"]);
    project.run_command_success(&["compile", "--env", "production"]);

    assert!(project.ast_exists("production"));
    let ast_bytes = fs::read(project.path(".controlpath/production.ast")).unwrap();
    let ast_text = String::from_utf8_lossy(&ast_bytes);
    assert!(ast_text.contains("platform.emergency_kill_switch"));
}

#[test]
#[serial]
fn validate_rejects_environment_rules_for_imported_flags() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut catalog = project.read_file("control-path.yaml");
    catalog = catalog.replace(
        "      new_dashboard:\n        - serve: true",
        "      new_dashboard:\n        - serve: true\n      emergency_kill_switch:\n        - serve: true",
    );
    project.write_file("control-path.yaml", &catalog);

    let output = project.run_command_failure(&["validate", "--all"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("imported flag"));
}

#[test]
#[serial]
fn validate_rejects_local_flag_colliding_with_import_namespace() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut catalog = project.read_file("control-path.yaml");
    catalog = catalog.replace(
        "flags:\n  new_dashboard:",
        "flags:\n  platform:\n    kind: release\n    default: false\n    owner: team-web\n  new_dashboard:",
    );
    project.write_file("control-path.yaml", &catalog);

    let output = project.run_command_failure(&["validate", "--all"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("import namespace"));
}

#[test]
#[serial]
fn generate_sdk_includes_imported_flags() {
    let project = TestProject::new();
    write_import_fixture(&project);

    project.run_command_success(&["generate-sdk", "--output", "generated"]);

    let index = project.read_file("generated/index.ts");
    assert!(index.contains("platformEmergencyKillSwitch"));
    assert!(index.contains("platform.emergency_kill_switch"));
}
