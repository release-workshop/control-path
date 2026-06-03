//! Integration tests for catalog attribute schema validation.

mod integration_test_helpers;

use integration_test_helpers::*;
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

fn command_output(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn validate_passes_for_declared_scalar_attributes() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut catalog = project.read_file("control-path.yaml");
    catalog = catalog.replace(
        "flags:\n  new_dashboard:",
        "attributes:\n  plan: string\n  seats: number\nflags:\n  new_dashboard:",
    );
    project.write_file("control-path.yaml", &catalog);

    project.run_command_success(&["validate", "--all"]);
}

#[test]
fn validate_rejects_base_attribute_name_in_attribute_schema() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut catalog = project.read_file("control-path.yaml");
    catalog = catalog.replace(
        "flags:\n  new_dashboard:",
        "attributes:\n  role: string\nflags:\n  new_dashboard:",
    );
    project.write_file("control-path.yaml", &catalog);

    let output = project.run_command_failure(&["validate", "--all"]);
    let combined = command_output(&output);
    assert!(combined.contains("base attribute"));
}

#[test]
fn validate_rejects_attribute_schema_key_colliding_with_import_namespace() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut catalog = project.read_file("control-path.yaml");
    catalog = catalog.replace(
        "flags:\n  new_dashboard:",
        "attributes:\n  platform: string\nflags:\n  new_dashboard:",
    );
    project.write_file("control-path.yaml", &catalog);

    let output = project.run_command_failure(&["validate", "--all"]);
    let combined = command_output(&output);
    assert!(
        combined.contains("import namespace"),
        "expected import namespace collision error in output: {combined}"
    );
}

#[test]
fn validate_rejects_unknown_attribute_type() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut catalog = project.read_file("control-path.yaml");
    catalog = catalog.replace(
        "flags:\n  new_dashboard:",
        "attributes:\n  plan: object\nflags:\n  new_dashboard:",
    );
    project.write_file("control-path.yaml", &catalog);

    let output = project.run_command_failure(&["validate", "--all"]);
    let combined = command_output(&output);
    assert!(
        combined.contains("object") && combined.contains("boolean"),
        "expected schema enum error in output: {combined}"
    );
}

#[test]
fn validate_rejects_invalid_attribute_key() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut catalog = project.read_file("control-path.yaml");
    catalog = catalog.replace(
        "flags:\n  new_dashboard:",
        "attributes:\n  BadKey: string\nflags:\n  new_dashboard:",
    );
    project.write_file("control-path.yaml", &catalog);

    let output = project.run_command_failure(&["validate", "--all"]);
    let combined = command_output(&output);
    assert!(
        combined.contains("BadKey") || combined.contains("pattern"),
        "expected invalid key error in output: {combined}"
    );
}

#[test]
fn validate_passes_without_attributes_unchanged() {
    let project = TestProject::new();
    write_import_fixture(&project);
    project.run_command_success(&["validate", "--all"]);
}

#[test]
fn imported_catalog_can_declare_its_own_attributes() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut platform = project.read_file("platform/control-path.yaml");
    platform = platform.replace(
        "flags:\n  emergency_kill_switch:",
        "attributes:\n  org_tier: string\nflags:\n  emergency_kill_switch:",
    );
    project.write_file("platform/control-path.yaml", &platform);

    project.run_command_success(&["validate", "--all"]);
}

#[test]
fn validate_passes_rule_with_declared_attribute() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut catalog = project.read_file("control-path.yaml");
    catalog = catalog.replace(
        "flags:\n  new_dashboard:",
        "attributes:\n  plan: string\nflags:\n  new_dashboard:",
    );
    catalog = catalog.replace(
        "staging:\n    rules:\n      new_dashboard:\n        - serve: true",
        "staging:\n    rules:\n      new_dashboard:\n        - when: \"plan == 'beta'\"\n          serve: true",
    );
    project.write_file("control-path.yaml", &catalog);

    project.run_command_success(&["validate", "--all"]);
}

#[test]
fn validate_rejects_rule_with_unknown_attribute() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut catalog = project.read_file("control-path.yaml");
    catalog = catalog.replace(
        "flags:\n  new_dashboard:",
        "attributes:\n  plan: string\nflags:\n  new_dashboard:",
    );
    catalog = catalog.replace(
        "staging:\n    rules:\n      new_dashboard:\n        - serve: true",
        "staging:\n    rules:\n      new_dashboard:\n        - when: \"tier == 'x'\"\n          serve: true",
    );
    project.write_file("control-path.yaml", &catalog);

    let output = project.run_command_failure(&["validate", "--all"]);
    let combined = command_output(&output);
    assert!(
        combined.contains("tier") && combined.contains("Unknown evaluation attribute"),
        "expected unknown attribute error in output: {combined}"
    );
}

#[test]
fn compile_passes_rule_with_declared_attribute() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut catalog = project.read_file("control-path.yaml");
    catalog = catalog.replace(
        "flags:\n  new_dashboard:",
        "attributes:\n  plan: string\nflags:\n  new_dashboard:",
    );
    catalog = catalog.replace(
        "staging:\n    rules:\n      new_dashboard:\n        - serve: true",
        "staging:\n    rules:\n      new_dashboard:\n        - when: \"plan == 'beta'\"\n          serve: true",
    );
    project.write_file("control-path.yaml", &catalog);

    project.run_command_success(&["compile", "--env", "staging"]);
}

#[test]
fn compile_rejects_rule_with_unknown_attribute() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut catalog = project.read_file("control-path.yaml");
    catalog = catalog.replace(
        "flags:\n  new_dashboard:",
        "attributes:\n  plan: string\nflags:\n  new_dashboard:",
    );
    catalog = catalog.replace(
        "staging:\n    rules:\n      new_dashboard:\n        - serve: true",
        "staging:\n    rules:\n      new_dashboard:\n        - when: \"tier == 'x'\"\n          serve: true",
    );
    project.write_file("control-path.yaml", &catalog);

    let output = project.run_command_failure(&["compile", "--env", "staging"]);
    let combined = command_output(&output);
    assert!(
        combined.contains("tier") && combined.contains("Unknown evaluation attribute"),
        "expected unknown attribute error in output: {combined}"
    );
}

#[test]
fn validate_rejects_segment_with_unknown_attribute() {
    let project = TestProject::new();
    write_import_fixture(&project);

    let mut catalog = project.read_file("control-path.yaml");
    catalog = catalog.replace(
        "flags:\n  new_dashboard:",
        "attributes:\n  plan: string\nsegments:\n  beta_users:\n    when: \"tier == 'x'\"\nflags:\n  new_dashboard:",
    );
    project.write_file("control-path.yaml", &catalog);

    let output = project.run_command_failure(&["validate", "--all"]);
    let combined = command_output(&output);
    assert!(
        combined.contains("tier") && combined.contains("Unknown evaluation attribute"),
        "expected unknown attribute error in output: {combined}"
    );
}
