//! Integration tests for SaaS-mode catalog sync boundary.

mod integration_test_helpers;

use integration_test_helpers::TestProject;
use std::fs;
use std::path::PathBuf;

fn saas_fixture() -> String {
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/examples");
    fs::read_to_string(fixture_root.join("saas.control-path.yaml")).unwrap()
}

#[test]
fn saas_validate_succeeds_without_local_environments() {
    let project = TestProject::new();
    project.write_file("control-path.yaml", &saas_fixture());
    project.run_command_success(&["validate"]);
}

#[test]
fn saas_ci_succeeds_without_local_environments() {
    let project = TestProject::new();
    project.write_file("control-path.yaml", &saas_fixture());
    project.run_command_success(&["ci", "--no-sdk"]);
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
fn saas_ci_retires_removed_flags_across_runs() {
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
    assert!(
        output.status.success(),
        "second CI run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
fn saas_ci_syncs_catalog_on_first_run() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &minimal_saas_catalog(
            "  feature_a:\n    kind: release\n    default: false\n    owner: team-a\n",
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
        combined.contains("Synced 1 flag"),
        "expected sync message, got: {combined}"
    );
}

fn saas_catalog_with_extra_block(extra: &str) -> String {
    format!(
        r"catalog:
  namespace: acme
  id: checkout-service
mode: saas
saas:
  project: acme/checkout
flags:
  feature_a:
    kind: release
    default: false
    owner: team-a
{extra}"
    )
}

#[test]
fn saas_mode_rejects_local_segments_via_validate() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &saas_catalog_with_extra_block("segments:\n  beta:\n    when: \"true\"\n"),
    );

    let output = project.run_command_failure(&["validate"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("segments") && combined.contains("saas"),
        "expected SaaS local-rules error, got: {combined}"
    );
}

#[test]
fn saas_mode_rejects_local_artifacts_via_validate() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &saas_catalog_with_extra_block(
            "artifacts:\n  production:\n    url: https://example.com/rules.ast\n",
        ),
    );

    let output = project.run_command_failure(&["validate"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("artifacts") && combined.contains("saas"),
        "expected SaaS local-rules error, got: {combined}"
    );
}

#[test]
fn saas_mode_rejects_local_kill_switches_via_validate() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &saas_catalog_with_extra_block(
            "kill_switches:\n  production:\n    url: https://example.com/kill.json\n",
        ),
    );

    let output = project.run_command_failure(&["validate"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("kill_switches") && combined.contains("saas"),
        "expected SaaS local-rules error, got: {combined}"
    );
}

#[test]
fn saas_validate_rejects_require_ast_signature_without_public_key() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        r"catalog:
  namespace: acme
  id: checkout-service
mode: saas
saas:
  project: acme/checkout
  require_ast_signature: true
flags:
  feature_a:
    kind: release
    default: false
    owner: team-a
",
    );

    let output = project.run_command_failure(&["validate"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("saas.ast_public_key is required"),
        "expected ast_public_key requirement error, got: {combined}"
    );
}

#[test]
fn saas_ci_rejects_unsigned_remote_ast_when_signature_required() {
    use base64::Engine;
    use controlpath_compiler::ast::Artifact;
    use controlpath_compiler::serialize;
    use ed25519_dalek::SigningKey;

    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
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

    let artifact = Artifact {
        version: "1.0".to_string(),
        environment: "production".to_string(),
        string_table: vec!["feature_a".to_string()],
        flags: vec![vec![]],
        flag_names: vec![0],
        segments: None,
        signature: None,
    };
    let bytes = serialize(&artifact).unwrap();
    let state = serde_json::json!({
        "projects": {},
        "remote_asts": {
            "production": bytes
        }
    });
    project.write_file(
        ".controlpath/saas-fake-state.json",
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    let output = project.run_command(&["ci", "--no-sdk"]);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Signature required but not present"),
        "expected signature error, got: {combined}"
    );
}

#[test]
fn saas_ci_downloads_signed_remote_ast() {
    use base64::Engine;
    use controlpath_compiler::ast::Artifact;
    use controlpath_compiler::serialize;
    use ed25519_dalek::{Signer, SigningKey};

    let signing_key = SigningKey::from_bytes(&[8u8; 32]);
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
        string_table: vec!["feature_a".to_string()],
        flags: vec![vec![]],
        flag_names: vec![0],
        segments: None,
        signature: None,
    };
    let message = serialize(&artifact).unwrap();
    artifact.signature = Some(signing_key.sign(&message).to_bytes().to_vec());
    let bytes = serialize(&artifact).unwrap();
    let state = serde_json::json!({
        "projects": {},
        "remote_asts": {
            "production": bytes
        }
    });
    project.write_file(
        ".controlpath/saas-fake-state.json",
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    project.run_command_success(&["ci", "--no-sdk"]);
    assert!(project.path(".controlpath/production.ast").exists());
}

#[test]
fn saas_ci_rejects_unresolved_import_path() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        r"catalog:
  namespace: acme
  id: checkout-service
mode: saas
saas:
  project: acme/checkout
imports:
  platform:
    path: missing/control-path.yaml
flags:
  feature_a:
    kind: release
    default: false
    owner: team-a
",
    );

    project.run_command_failure(&["validate"]);
    project.run_command_failure(&["ci", "--no-sdk"]);
}

#[test]
fn saas_generate_sdk_embeds_cdn_urls_for_sync_cached_environments() {
    use controlpath_compiler::ast::Artifact;
    use controlpath_compiler::CatalogIdentity;
    use controlpath_compiler::{effective_catalog_id, serialize};

    use integration_test_helpers::expected_saas_runtime_url_maps;

    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &minimal_saas_catalog(
            "  feature_a:\n    kind: release\n    default: false\n    owner: team-a\n",
        ),
    );

    let artifact = |env: &str| {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: env.to_string(),
            string_table: vec!["feature_a".to_string()],
            flags: vec![vec![]],
            flag_names: vec![0],
            segments: None,
            signature: None,
        };
        serialize(&artifact).unwrap()
    };

    let state = serde_json::json!({
        "projects": {},
        "remote_asts": {
            "production": artifact("production"),
            "staging": artifact("staging")
        }
    });
    project.write_file(
        ".controlpath/saas-fake-state.json",
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    project.run_command_success(&["ci", "--no-sdk"]);

    project.run_command_success(&["generate-sdk", "--output", "generated"]);

    let index = fs::read_to_string(project.path("generated/index.ts")).unwrap();
    let catalog_id = effective_catalog_id(
        &CatalogIdentity {
            id: "checkout-service".to_string(),
            namespace: Some("acme".to_string()),
            scope: Default::default(),
        },
        None,
    );
    let url_maps = expected_saas_runtime_url_maps(
        &project.path("."),
        "https://cdn.controlpath.dev",
        "acme/checkout",
        &catalog_id,
    );

    for env in ["production", "staging"] {
        assert!(index.contains(
            url_maps
                .artifact_urls
                .get(env)
                .expect("artifact URL for synced env")
        ));
        assert!(index.contains(
            url_maps
                .kill_switch_urls
                .get(env)
                .expect("kill switch URL for synced env")
        ));
    }
    assert!(index.contains("ARTIFACT_URLS"));
    assert!(index.contains("KILL_SWITCH_URLS"));
}

fn saas_catalog_with_cdn_url(flags_yaml: &str, cdn_url: &str) -> String {
    format!(
        r"catalog:
  namespace: acme
  id: checkout-service
mode: saas
saas:
  project: acme/checkout
  cdn_url: {cdn_url}
flags:
{flags_yaml}"
    )
}

#[test]
fn saas_generate_sdk_uses_custom_cdn_url() {
    use controlpath_compiler::ast::Artifact;
    use controlpath_compiler::CatalogIdentity;
    use controlpath_compiler::{effective_catalog_id, serialize};

    use integration_test_helpers::expected_saas_runtime_url_maps;

    let cdn = "https://cdn.mycompany.com";
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &saas_catalog_with_cdn_url(
            "  feature_a:\n    kind: release\n    default: false\n    owner: team-a\n",
            cdn,
        ),
    );

    let artifact = Artifact {
        version: "1.0".to_string(),
        environment: "production".to_string(),
        string_table: vec!["feature_a".to_string()],
        flags: vec![vec![]],
        flag_names: vec![0],
        segments: None,
        signature: None,
    };
    let state = serde_json::json!({
        "projects": {},
        "remote_asts": { "production": serialize(&artifact).unwrap() }
    });
    project.write_file(
        ".controlpath/saas-fake-state.json",
        &serde_json::to_string_pretty(&state).unwrap(),
    );
    project.run_command_success(&["ci", "--no-sdk"]);

    project.run_command_success(&["generate-sdk", "--output", "generated"]);
    let index = fs::read_to_string(project.path("generated/index.ts")).unwrap();

    let catalog_id = effective_catalog_id(
        &CatalogIdentity {
            id: "checkout-service".to_string(),
            namespace: Some("acme".to_string()),
            scope: Default::default(),
        },
        None,
    );
    let url_maps =
        expected_saas_runtime_url_maps(&project.path("."), cdn, "acme/checkout", &catalog_id);
    assert!(index.contains(
        url_maps
            .artifact_urls
            .get("production")
            .expect("production artifact URL")
    ));
    assert!(index.contains(
        url_maps
            .kill_switch_urls
            .get("production")
            .expect("production kill switch URL")
    ));
    assert!(!index.contains("https://cdn.controlpath.dev"));
}

#[test]
fn saas_generate_sdk_fails_without_sync_cache() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &minimal_saas_catalog(
            "  feature_a:\n    kind: release\n    default: false\n    owner: team-a\n",
        ),
    );

    let output = project.run_command_failure(&["generate-sdk", "--output", "generated"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("no compiled artifacts in .controlpath"),
        "expected sync-cache error, got: {combined}"
    );
}

#[test]
fn saas_sync_prunes_stale_ast_before_generate_sdk() {
    use controlpath_compiler::ast::Artifact;
    use controlpath_compiler::serialize;

    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &minimal_saas_catalog(
            "  feature_a:\n    kind: release\n    default: false\n    owner: team-a\n",
        ),
    );

    fs::create_dir_all(project.path(".controlpath")).unwrap();
    fs::write(project.path(".controlpath/staging.ast"), b"stale").unwrap();

    let artifact = Artifact {
        version: "1.0".to_string(),
        environment: "production".to_string(),
        string_table: vec!["feature_a".to_string()],
        flags: vec![vec![]],
        flag_names: vec![0],
        segments: None,
        signature: None,
    };
    let state = serde_json::json!({
        "projects": {},
        "remote_asts": { "production": serialize(&artifact).unwrap() }
    });
    project.write_file(
        ".controlpath/saas-fake-state.json",
        &serde_json::to_string_pretty(&state).unwrap(),
    );

    project.run_command_success(&["ci", "--no-sdk"]);
    assert!(!project.path(".controlpath/staging.ast").exists());

    project.run_command_success(&["generate-sdk", "--output", "generated"]);
    let index = fs::read_to_string(project.path("generated/index.ts")).unwrap();
    assert!(index.contains("production"));
    assert!(!index.contains("staging"));
}

fn saas_catalog_with_bootstrap_environments() -> String {
    minimal_saas_catalog("  feature_a:\n    kind: release\n    default: false\n    owner: team-a\n")
        + "environments:\n  staging:\n    rules:\n      feature_a:\n        - serve: true\n"
}

#[test]
fn saas_ci_imports_bootstrap_rules_on_first_sync() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &saas_catalog_with_bootstrap_environments(),
    );

    project.run_command_success(&["ci", "--no-sdk"]);
    assert!(project.path(".controlpath/staging.ast").exists());
}

#[test]
fn saas_ci_ignores_bootstrap_rules_on_subsequent_sync() {
    let project = TestProject::new();
    project.write_file(
        "control-path.yaml",
        &saas_catalog_with_bootstrap_environments(),
    );
    project.run_command_success(&["ci", "--no-sdk"]);

    let mut catalog = saas_catalog_with_bootstrap_environments();
    catalog = catalog.replace("serve: true", "serve: false");
    project.write_file("control-path.yaml", &catalog);
    project.run_command_success(&["ci", "--no-sdk"]);

    let state_path = project.path(".controlpath/saas-fake-state.json");
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(state_path).unwrap()).unwrap();
    let rules = &state["projects"]["acme/checkout"]["environment_rules"]["staging"]["feature_a"][0]
        ["serve"];
    assert_eq!(rules, true);
}

#[test]
fn saas_mode_rejects_local_environments_via_validate() {
    let project = TestProject::new();
    let mut catalog = saas_fixture();
    catalog.push_str(
        "
environments:
  production:
    rules:
      new_dashboard:
        - serve: true
",
    );
    project.write_file("control-path.yaml", &catalog);

    let output = project.run_command_failure(&["validate"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("environments") && combined.contains("saas"),
        "expected SaaS local-rules error, got: {combined}"
    );
}
