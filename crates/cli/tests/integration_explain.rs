//! Integration tests for minimal `explain` (kill switch → AST → catalog default).

mod integration_test_helpers;

use integration_test_helpers::*;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;

fn combined_output(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn write_import_fixture(project: &TestProject) {
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/examples");

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
fn explain_kill_switch_skips_ast_rules() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: svc
mode: local
flags:
  my_flag:
    default: false
    kind: kill_switch
environments:
  production:
    rules:
      my_flag:
        - serve: true
",
    );

    project.run_command_success(&["compile", "--env", "production"]);
    project.run_command_success(&[
        "kill-switch",
        "set",
        "my_flag",
        "false",
        "--env",
        "production",
    ]);

    let output = project.run_command(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        r#"{"id":"user-1"}"#,
        "--env",
        "production",
    ]);
    assert!(output.status.success(), "{}", combined_output(&output));
    let combined = combined_output(&output);
    assert!(
        combined.contains("kill switch"),
        "expected kill switch layer, got: {combined}"
    );
    assert!(
        combined.contains("Value: false"),
        "expected kill switch value false, got: {combined}"
    );
}

#[test]
#[serial]
fn explain_targeted_environment_rule() {
    let project = TestProject::with_definitions(
        r#"catalog:
  id: svc
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  production:
    rules:
      my_flag:
        - when: 'user.role == "admin"'
          serve: true
          reason: Admins only
        - serve: false
"#,
    );

    project.run_command_success(&["compile", "--env", "production"]);

    let admin = project.run_command(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        r#"{"id":"a1","role":"admin"}"#,
        "--env",
        "production",
    ]);
    assert!(admin.status.success());
    let admin_out = combined_output(&admin);
    assert!(admin_out.contains("environment rule"));
    assert!(admin_out.contains("Admins only"));
    assert!(admin_out.contains("Value: true"));

    let other = project.run_command(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        r#"{"id":"u2","role":"member"}"#,
        "--env",
        "production",
    ]);
    assert!(other.status.success());
    let other_out = combined_output(&other);
    assert!(
        other_out.contains("environment rule"),
        "catch-all serve:false is an environment rule, not catalog default: {other_out}"
    );
    assert!(other_out.contains("Value: false"));
    assert!(
        !other_out.contains("catalog default"),
        "non-admin should not be labeled catalog default: {other_out}"
    );
}

#[test]
#[serial]
fn explain_catalog_default_when_no_environment_rules() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: svc
mode: local
flags:
  my_flag:
    default: true
    kind: release
",
    );

    project.run_command_success(&["compile", "--env", "production"]);
    let output = project.run_command(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        r#"{"id":"u1"}"#,
        "--env",
        "production",
    ]);
    assert!(output.status.success(), "{}", combined_output(&output));
    let combined = combined_output(&output);
    assert!(
        combined.contains("catalog default"),
        "expected trailing compiled default layer, got: {combined}"
    );
    assert!(combined.contains("Value: true"));
}

#[test]
#[serial]
fn explain_rollout_skipped_serve_match_does_not_warn_missing_id() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: svc
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  production:
    rules:
      my_flag:
        - rollout:
            percentage: 0
            serve: true
        - serve: true
",
    );

    project.run_command_success(&["compile", "--env", "production"]);
    let output = project.run_command(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        r#"{}"#,
        "--env",
        "production",
    ]);
    assert!(output.status.success(), "{}", combined_output(&output));
    let combined = combined_output(&output);
    assert!(
        combined.contains("environment rule"),
        "expected serve rule match: {combined}"
    );
    assert!(
        !combined.contains("Missing user.id"),
        "rollout was skipped; matched serve rule should not warn: {combined}"
    );
}

#[test]
#[serial]
fn explain_rollout_rule_reports_bucket_and_missing_identity() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: svc
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  production:
    rules:
      my_flag:
        - rollout:
            percentage: 100
            serve: true
          reason: Full rollout
        - serve: false
",
    );

    project.run_command_success(&["compile", "--env", "production"]);

    let with_id = project.run_command(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        r#"{"id":"stable-user"}"#,
        "--env",
        "production",
    ]);
    assert!(with_id.status.success());
    let with_id_out = combined_output(&with_id);
    assert!(with_id_out.contains("Rollout"));
    assert!(with_id_out.contains("Rollout bucket"));

    let no_id = project.run_command(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        r#"{}"#,
        "--env",
        "production",
    ]);
    assert!(no_id.status.success());
    let no_id_out = combined_output(&no_id);
    assert!(
        no_id_out.contains("Missing user.id"),
        "expected identity diagnostic, got: {no_id_out}"
    );
}

#[test]
#[serial]
fn explain_imported_namespace_flag() {
    let project = TestProject::new();
    write_import_fixture(&project);
    project.run_command_success(&["compile", "--env", "staging"]);

    let output = project.run_command(&[
        "explain",
        "--flag",
        "platform.emergency_kill_switch",
        "--user",
        r#"{"id":"u1"}"#,
        "--env",
        "staging",
    ]);
    assert!(output.status.success(), "{}", combined_output(&output));
    let combined = combined_output(&output);
    assert!(combined.contains("imported"));
    assert!(combined.contains("Value: false"));
}

#[test]
#[serial]
fn explain_deprecated_flag_warns() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: svc
mode: local
flags:
  legacy_flag:
    default: false
    kind: release
    lifecycle: deprecated
environments:
  production:
    rules:
      legacy_flag:
        - serve: true
",
    );

    project.run_command_success(&["compile", "--env", "production"]);
    let output = project.run_command(&[
        "explain",
        "--flag",
        "legacy_flag",
        "--user",
        r#"{"id":"u1"}"#,
        "--env",
        "production",
    ]);
    assert!(output.status.success());
    assert!(combined_output(&output).contains("deprecated"));
}

#[test]
#[serial]
fn explain_works_with_downloaded_saas_ast() {
    use base64::Engine;
    use controlpath_compiler::ast::{Artifact, Rule, ServePayload};
    use controlpath_compiler::serialize;
    use ed25519_dalek::{Signer, SigningKey};

    let signing_key = SigningKey::from_bytes(&[9u8; 32]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key());

    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &format!(
            r"catalog:
  namespace: acme
  id: checkout-service
mode: saas
saas:
  project: acme/checkout
  require_ast_signature: true
  ast_public_key: {encoded}
flags:
  feature_a:
    kind: release
    default: false
    owner: team-a
"
        ),
    );

    let mut artifact = Artifact {
        version: "1.0".to_string(),
        environment: "production".to_string(),
        string_table: vec!["ON".to_string(), "OFF".to_string(), "feature_a".to_string()],
        flags: vec![vec![
            Rule::ServeWithoutWhen(ServePayload::Number(0)),
            Rule::ServeWithoutWhen(ServePayload::Number(1)),
        ]],
        flag_names: vec![2],
        segments: None,
        signature: None,
    };
    let message = serialize(&artifact).unwrap();
    artifact.signature = Some(signing_key.sign(&message).to_bytes().to_vec());
    let bytes = serialize(&artifact).unwrap();
    let state = serde_json::json!({
        "projects": {},
        "remote_asts": { "production": bytes }
    });
    project.write_file(
        ".controlpath/saas-fake-state.json",
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    project.run_command_success(&["ci", "--no-sdk"]);
    let output = project.run_command(&[
        "explain",
        "--flag",
        "feature_a",
        "--user",
        r#"{"id":"saas-user"}"#,
        "--env",
        "production",
    ]);
    assert!(output.status.success(), "{}", combined_output(&output));
    let combined = combined_output(&output);
    assert!(combined.contains("Value: true"));
    assert!(
        combined.contains("environment rule"),
        "first compiled rule is not the trailing default: {combined}"
    );
    assert!(
        !combined.contains("catalog default"),
        "SaaS AST match on rule 1 must not be mislabeled as catalog default: {combined}"
    );
}

#[test]
#[serial]
fn explain_json_output_includes_layer_and_value() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: svc
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  production:
    rules:
      my_flag:
        - serve: true
",
    );

    project.run_command_success(&["compile", "--env", "production"]);
    let output = project.run_command(&[
        "--json",
        "explain",
        "--flag",
        "my_flag",
        "--user",
        r#"{"id":"u1"}"#,
        "--env",
        "production",
    ]);
    assert!(output.status.success(), "{}", combined_output(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("expected JSON stdout, got {stdout}: {e}"));
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["command"], "explain");
    assert_eq!(parsed["layer"], "environment rule");
    assert_eq!(parsed["value"], true);
}
