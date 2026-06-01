//! Integration tests for flag lifecycle and rot reporting (issue 08).

mod integration_test_helpers;

use integration_test_helpers::TestProject;
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
fn flag_deprecate_sets_lifecycle_in_catalog() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: checkout
  namespace: acme
mode: local
flags:
  new_dashboard:
    default: false
    kind: release
environments:
  production:
    rules: {}
",
    );

    project.run_command_success(&["flag", "deprecate", "--name", "new_dashboard"]);

    let catalog = project.read_file("control-path.yaml");
    assert!(
        catalog.contains("lifecycle: deprecated"),
        "expected lifecycle: deprecated in catalog, got:\n{catalog}"
    );
}

#[test]
#[serial]
fn deprecated_flag_blocks_rule_changes_until_forced() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: checkout
  namespace: acme
mode: local
flags:
  new_dashboard:
    default: false
    kind: release
    lifecycle: deprecated
environments:
  production:
    rules:
      new_dashboard:
        - serve: false
",
    );

    let blocked = project.run_command(&[
        "flag",
        "enable",
        "new_dashboard",
        "--env",
        "production",
        "--all",
    ]);
    assert!(!blocked.status.success());
    let blocked_err = format!(
        "{}{}",
        String::from_utf8_lossy(&blocked.stdout),
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(
        blocked_err.contains("deprecated"),
        "expected deprecation block, got: {blocked_err}"
    );
    assert!(
        !blocked_err.contains("⚠ Warning"),
        "should not emit warning before hard error, got: {blocked_err}"
    );

    project.run_command_success(&[
        "flag",
        "enable",
        "new_dashboard",
        "--env",
        "production",
        "--all",
        "--force",
        "--no-compile",
    ]);

    let catalog = project.read_file("control-path.yaml");
    assert!(
        catalog.matches("serve: true").count() >= 1,
        "expected new serve: true rule after --force, got:\n{catalog}"
    );
}

fn minimal_saas_catalog(flags_yaml: &str) -> String {
    format!(
        r"catalog:
  namespace: acme
  id: checkout-service
mode: saas
saas:
  project: acme/checkout
flags:
{flags_yaml}"
    )
}

#[test]
#[serial]
fn flag_report_surfaces_saas_telemetry_without_writing_to_catalog() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &minimal_saas_catalog(
            "  stale_feature:\n    kind: release\n    default: false\n    owner: team-a\n",
        ),
    );
    project.write_file(
        ".controlpath/saas-fake-state.json",
        r#"{
  "projects": {},
  "remote_asts": {},
  "flag_telemetry": {
    "acme/checkout": {
      "stale_feature": {
        "last_evaluated": "2026-01-15",
        "evaluation_count": 0,
        "rot_suggestion": "unused"
      }
    }
  }
}"#,
    );

    let output = project.run_command(&["flag", "report"]);
    assert!(
        output.status.success(),
        "flag report failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("stale_feature") && combined.contains("unused"),
        "expected rot report output, got: {combined}"
    );

    let catalog = project.read_file("control-path.yaml");
    assert!(
        !catalog.contains("lastEvaluated")
            && !catalog.contains("last_evaluated")
            && !catalog.contains("evaluation_count")
            && !catalog.contains("rotSuggestion"),
        "telemetry must not be written to catalog:\n{catalog}"
    );
}

#[test]
#[serial]
fn saas_ci_warns_on_rot_suggestions_without_writing_telemetry() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &minimal_saas_catalog(
            "  stale_feature:\n    kind: release\n    default: false\n    owner: team-a\n",
        ),
    );
    project.write_file(
        ".controlpath/saas-fake-state.json",
        r#"{
  "projects": {},
  "remote_asts": {},
  "flag_telemetry": {
    "acme/checkout": {
      "stale_feature": {
        "last_evaluated": "2026-01-15",
        "evaluation_count": 0,
        "rot_suggestion": "unused"
      }
    }
  }
}"#,
    );

    let output = project.run_command(&["ci", "--no-sdk"]);
    assert!(
        output.status.success(),
        "ci failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("stale_feature") && combined.contains("unused"),
        "expected rot warning in ci output, got: {combined}"
    );

    let catalog = project.read_file("control-path.yaml");
    assert!(
        !catalog.contains("lastEvaluated") && !catalog.contains("rotSuggestion"),
        "telemetry must not be written to catalog:\n{catalog}"
    );
}

#[test]
#[serial]
fn removing_flag_from_git_retires_it_in_saas_history() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &minimal_saas_catalog(
            "  keep_me:\n    kind: release\n    default: true\n    owner: team-a\n  remove_me:\n    kind: release\n    default: false\n    owner: team-a\n",
        ),
    );
    project.run_command_success(&["ci", "--no-sdk"]);

    project.write_file(
        "control-path.yaml",
        &minimal_saas_catalog(
            "  keep_me:\n    kind: release\n    default: true\n    owner: team-a\n",
        ),
    );
    let output = project.run_command(&["ci", "--no-sdk"]);
    assert!(output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Retired 1 flag"),
        "expected retirement message, got: {combined}"
    );
}

#[test]
#[serial]
fn saas_ci_warns_on_deprecated_lifecycle() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &minimal_saas_catalog(
            "  old_flow:\n    kind: release\n    default: false\n    owner: team-a\n    lifecycle: deprecated\n",
        ),
    );

    let output = project.run_command(&["ci", "--no-sdk"]);
    assert!(output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("old_flow") && combined.contains("deprecated"),
        "expected deprecated lifecycle warning, got: {combined}"
    );
}

#[test]
#[serial]
fn flag_report_includes_imported_flags_and_telemetry() {
    let project = TestProject::new();
    write_import_fixture(&project);
    project.write_file(
        ".controlpath/saas-fake-state.json",
        r#"{
  "projects": {},
  "remote_asts": {},
  "flag_telemetry": {
    "acme/checkout": {
      "platform.emergency_kill_switch": {
        "last_evaluated": "2026-01-01",
        "evaluation_count": 0,
        "rot_suggestion": "unused"
      }
    }
  }
}"#,
    );

    // Local mode: imported flags appear; telemetry is ignored without SaaS mode.
    let local_output = project.run_command(&["flag", "report"]);
    assert!(local_output.status.success());
    let local_combined = format!(
        "{}{}",
        String::from_utf8_lossy(&local_output.stdout),
        String::from_utf8_lossy(&local_output.stderr)
    );
    assert!(
        local_combined.contains("platform.emergency_kill_switch"),
        "expected imported flag in local report, got: {local_combined}"
    );
    assert!(
        !local_combined.contains("unused"),
        "local report should not show SaaS telemetry, got: {local_combined}"
    );
}

#[test]
#[serial]
fn flag_report_json_output() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &minimal_saas_catalog(
            "  stale_feature:\n    kind: release\n    default: false\n    owner: team-a\n",
        ),
    );

    let output = project.run_command(&["--json", "flag", "report"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid json output");
    assert_eq!(
        parsed.get("command").and_then(|v| v.as_str()),
        Some("flag report")
    );
    let flags = parsed.get("flags").and_then(|v| v.as_array()).unwrap();
    assert!(flags
        .iter()
        .any(|f| f.get("flag_key").and_then(|k| k.as_str()) == Some("stale_feature")));
}
