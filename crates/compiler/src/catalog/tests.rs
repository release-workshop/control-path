/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use crate::catalog::{
    effective_catalog_id, load_and_validate_catalog, load_and_validate_workspace, parse_catalog,
    parse_workspace, validate_catalog, validate_catalog_value, CatalogMode,
    CatalogValidationContext, FlagLifecycle, ValidationMode, WorkspaceDocument,
};

const LOCAL_ONLY: &str = include_str!("../../../../schemas/examples/local-only.control-path.yaml");
const SAAS: &str = include_str!("../../../../schemas/examples/saas.control-path.yaml");
const SHARED_PLATFORM: &str =
    include_str!("../../../../schemas/examples/shared-platform.control-path.yaml");
const IMPORTED_GLOBAL: &str =
    include_str!("../../../../schemas/examples/imported-global.control-path.yaml");
const WORKSPACE: &str = include_str!("../../../../schemas/examples/control-path.workspace.yaml");

fn assert_valid_catalog(content: &str, file: &str, ctx: &CatalogValidationContext) {
    let (doc, result) = load_and_validate_catalog(content, file, ctx, ValidationMode::Compile)
        .expect("catalog should parse");
    assert!(result.is_ok(), "validation failed: {:?}", result.errors);
    let _ = doc;
}

#[test]
fn example_local_only_catalog_is_valid() {
    let workspace: WorkspaceDocument =
        parse_workspace(WORKSPACE, Some("control-path.workspace.yaml")).expect("workspace parses");
    let ctx = CatalogValidationContext {
        workspace: Some(workspace),
        ..Default::default()
    };
    assert_valid_catalog(LOCAL_ONLY, "local-only.control-path.yaml", &ctx);
    let doc = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
    assert_eq!(doc.mode, CatalogMode::Local);
    assert!(doc.environments.contains_key("production"));
    assert!(doc.segments.contains_key("beta_users"));
}

#[test]
fn example_saas_catalog_is_valid() {
    assert_valid_catalog(
        SAAS,
        "saas.control-path.yaml",
        &CatalogValidationContext::default(),
    );
    let doc = parse_catalog(SAAS, None).unwrap();
    assert_eq!(doc.mode, CatalogMode::Saas);
    assert_eq!(doc.catalog.namespace.as_deref(), Some("acme"));
    let deprecated = doc.flags.get("old_checkout_flow").expect("deprecated flag");
    assert_eq!(deprecated.lifecycle, FlagLifecycle::Deprecated);
}

#[test]
fn rejects_import_missing_path() {
    let content = r#"
catalog:
  id: svc
imports:
  platform: {}
flags:
  f:
    default: false
    kind: release
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
}

#[test]
fn rejects_invalid_import_namespace_key() {
    let content = r#"
catalog:
  id: svc
imports:
  BadNamespace:
    path: ../platform.yaml
flags:
  f:
    default: false
    kind: release
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
}

#[test]
fn example_shared_platform_catalog_is_valid() {
    assert_valid_catalog(
        SHARED_PLATFORM,
        "shared-platform.control-path.yaml",
        &CatalogValidationContext::default(),
    );
}

#[test]
fn example_imported_global_catalog_is_valid() {
    assert_valid_catalog(
        IMPORTED_GLOBAL,
        "imported-global.control-path.yaml",
        &CatalogValidationContext::default(),
    );
}

#[test]
fn example_workspace_parses_and_validates() {
    let (doc, result) =
        load_and_validate_workspace(WORKSPACE, "control-path.workspace.yaml").expect("parse");
    assert!(result.valid, "{:?}", result.errors);
    assert_eq!(doc.namespace, "acme");
}

#[test]
fn namespace_resolution_prefers_catalog_over_workspace() {
    let doc = parse_catalog(SAAS, None).unwrap();
    let workspace = parse_workspace(WORKSPACE, None).unwrap();
    let effective = effective_catalog_id(&doc.catalog, Some(&workspace));
    assert_eq!(effective.namespace.as_deref(), Some("acme"));
    assert_eq!(effective.as_str(), "acme.checkout-service");
}

#[test]
fn namespace_resolution_uses_workspace_when_catalog_omits_namespace() {
    let doc = parse_catalog(LOCAL_ONLY, None).unwrap();
    let workspace = parse_workspace(WORKSPACE, None).unwrap();
    let effective = effective_catalog_id(&doc.catalog, Some(&workspace));
    assert_eq!(effective.namespace.as_deref(), Some("acme"));
    assert_eq!(effective.as_str(), "acme.checkout-service");
}

#[test]
fn namespace_resolution_without_workspace_uses_bare_id() {
    let doc = parse_catalog(SHARED_PLATFORM, None).unwrap();
    let effective = effective_catalog_id(&doc.catalog, None);
    assert!(effective.namespace.is_none());
    assert_eq!(effective.as_str(), "platform");
}

#[test]
fn saas_mode_rejects_kill_switches_segments_and_artifacts() {
    for block in ["kill_switches", "segments", "artifacts"] {
        let content = format!(
            r#"
catalog:
  id: svc
mode: saas
saas:
  project: acme/svc
flags:
  f:
    default: false
    kind: release
{block}:
  x: {{}}
"#
        );
        let content = if block == "kill_switches" {
            content.replace(
                "{block}:\n  x: {}",
                "kill_switches:\n  production:\n    url: https://example.com/kill.json",
            )
        } else if block == "artifacts" {
            content.replace(
                "{block}:\n  x: {}",
                "artifacts:\n  production:\n    url: https://example.com/rules.ast",
            )
        } else {
            content.replace(
                "{block}:\n  x: {}",
                "segments:\n  beta:\n    when: \"true\"",
            )
        };
        let value = crate::catalog::parse_catalog_value(&content, Some("bad.yaml")).unwrap();
        let result = validate_catalog_value(
            "bad.yaml",
            &value,
            &CatalogValidationContext::default(),
            ValidationMode::Compile,
        );
        assert!(!result.valid, "expected {block} to be rejected");
        assert!(
            result.errors.iter().any(|e| e.message.contains(block)),
            "{block}: {:?}",
            result.errors
        );
    }
}

#[test]
fn saas_mode_rejects_environments_block() {
    let content = r#"
catalog:
  id: svc
mode: saas
saas:
  project: acme/svc
flags:
  f:
    default: false
    kind: release
environments:
  prod:
    rules: {}
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.message.contains("environments")));
}

#[test]
fn rejects_v1_array_flags() {
    let content = r#"
catalog:
  id: svc
flags:
  - name: old_flag
    type: boolean
    defaultValue: false
"#;
    let err = parse_catalog(content, Some("v1.yaml")).unwrap_err();
    assert!(err.to_string().contains("array"));
}

#[test]
fn load_and_validate_catalog_rejects_v1_flag_fields() {
    let content = r#"
catalog:
  id: svc
flags:
  old_flag:
    type: boolean
    defaultValue: false
    default: false
    kind: release
"#;
    let (doc, result) = load_and_validate_catalog(
        content,
        "v1.yaml",
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    )
    .expect("parse succeeds; check validation separately");
    assert!(
        !result.is_ok(),
        "expected validation failure, got {:?}",
        result.errors
    );
    assert!(
        result.errors.iter().any(|e| e.message.contains("type")),
        "{:?}",
        result.errors
    );
    assert_eq!(doc.catalog.id, "svc");
}

#[test]
fn rejects_v1_flag_fields() {
    let content = r#"
catalog:
  id: svc
flags:
  old_flag:
    type: boolean
    defaultValue: false
    default: false
    kind: release
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("v1.yaml")).unwrap();
    let result = validate_catalog_value(
        "v1.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.message.contains("type")));
}

#[test]
fn rejects_imported_flag_environment_rules() {
    let mut ctx = CatalogValidationContext::default();
    ctx.imported_flag_keys
        .insert("emergency_kill_switch".to_string());
    let content = r#"
catalog:
  id: svc
flags:
  local_flag:
    default: false
    kind: release
environments:
  prod:
    rules:
      emergency_kill_switch:
        - serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    let result = validate_catalog_value("bad.yaml", &value, &ctx, ValidationMode::Compile);
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.message.contains("imported flag")));
}

#[test]
fn rejects_kill_switch_rule_with_when() {
    let content = r#"
catalog:
  id: platform
flags:
  emergency_kill_switch:
    default: false
    kind: kill_switch
environments:
  production:
    rules:
      emergency_kill_switch:
        - when: "true"
          serve: false
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.message.contains("when")));
}

#[test]
fn rejects_kill_switch_rule_with_rollout() {
    let content = r#"
catalog:
  id: platform
flags:
  emergency_kill_switch:
    default: false
    kind: kill_switch
environments:
  production:
    rules:
      emergency_kill_switch:
        - rollout:
            percentage: 50
            serve: false
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.message.contains("rollout")));
}

#[test]
fn rejects_telemetry_in_flag_metadata() {
    let content = r#"
catalog:
  id: svc
flags:
  f:
    default: false
    kind: release
    metadata:
      lastEvaluated: "2026-01-01"
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.message.contains("telemetry")));
}

#[test]
fn warns_on_missing_owner() {
    let content = r#"
catalog:
  id: svc
flags:
  f:
    default: false
    kind: release
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("warn.yaml")).unwrap();
    let result = validate_catalog_value(
        "warn.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.valid);
    assert!(result.warnings.iter().any(|w| w.message.contains("owner")));
}

#[test]
fn warns_on_release_without_expires() {
    let content = r#"
catalog:
  id: svc
flags:
  f:
    default: false
    kind: release
    owner: team-a
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("warn.yaml")).unwrap();
    let result = validate_catalog_value(
        "warn.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.valid);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.message.contains("expires")));
}

#[test]
fn rejects_local_flag_colliding_with_import_namespace() {
    let content = r#"
catalog:
  id: svc
imports:
  platform:
    path: ../platform.yaml
flags:
  platform:
    default: false
    kind: release
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
    assert!(result
        .errors
        .iter()
        .any(|e| e.message.contains("import namespace")));
}

#[test]
fn shared_platform_kill_switch_serve_only_is_valid() {
    // Known-good fixture only: parse_catalog + validate_catalog skips v1 detection (see docs).
    let doc = parse_catalog(SHARED_PLATFORM, None).unwrap();
    let result = validate_catalog(
        "shared-platform.control-path.yaml",
        &doc,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.is_ok(), "{:?}", result.errors);
}
