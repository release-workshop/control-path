//! Integration tests for legacy surface removal (issue 09).

mod integration_test_helpers;

use integration_test_helpers::*;
use std::fs;

#[test]
fn legacy_definitions_only_project_rejected_by_compile() {
    let project = TestProject::new();
    fs::create_dir_all(project.path(".controlpath")).unwrap();
    project.write_file(
        "flags.definitions.yaml",
        r"flags:
  - name: my_flag
    type: boolean
    defaultValue: false
",
    );
    project.write_file(
        ".controlpath/production.deployment.yaml",
        r"environment: production
rules:
  my_flag:
    rules:
      - serve: true
",
    );

    let output = project.run_command(&["compile", "--env", "production"]);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("control-path.yaml"),
        "expected compile to require control-path.yaml, got: {combined}"
    );
}

#[test]
fn v1_array_flags_rejected_by_validate() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: test
mode: local
flags:
  - name: my_flag
    type: boolean
    defaultValue: false
",
    );

    let output = project.run_command(&["validate", "--all"]);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("v1 array \"flags\" is not supported; use map-keyed flags"),
        "expected v1 array flags rejection, got: {combined}"
    );
}

#[test]
fn multivariate_flag_field_rejected_by_validate() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: test
mode: local
flags:
  theme:
    default: false
    kind: release
    type: multivariate
    variations:
      - name: light
        value: light
",
    );

    let output = project.run_command(&["validate", "--all"]);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Unsupported v1 field 'type' on flag 'theme'"),
        "expected multivariate field rejection, got: {combined}"
    );
}
