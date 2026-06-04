/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use crate::catalog::{
    effective_catalog_id, load_and_validate_catalog, load_and_validate_workspace, parse_catalog,
    parse_workspace, validate_catalog, validate_catalog_value, AttributeScalarType,
    CatalogDocument, CatalogMode, CatalogValidationContext, FlagLifecycle, ValidationMode,
    WorkspaceDocument,
};
use std::collections::BTreeMap;

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

const ENTITLEMENT_RULE_WITH_ROLLOUT: &str = r#"
catalog:
  id: billing
flags:
  premium_export:
    default: false
    kind: entitlement
environments:
  production:
    rules:
      premium_export:
        - rollout:
            percentage: 50
            serve: true
"#;

#[test]
fn rejects_entitlement_rule_with_rollout() {
    let value =
        crate::catalog::parse_catalog_value(ENTITLEMENT_RULE_WITH_ROLLOUT, Some("bad.yaml"))
            .unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
    let rollout_err = result
        .errors
        .iter()
        .find(|e| e.message.contains("rollout"))
        .expect("rollout error");
    assert!(rollout_err.message.contains("premium_export"));
    assert!(rollout_err.message.contains("Entitlement"));
    assert_eq!(
        rollout_err.path.as_deref(),
        Some("environments.production.rules.premium_export[0].rollout")
    );
    assert!(rollout_err
        .suggestion
        .as_deref()
        .is_some_and(|s| s.contains("when")));
}

#[test]
fn rejects_entitlement_rule_with_rollout_at_authoring() {
    let value =
        crate::catalog::parse_catalog_value(ENTITLEMENT_RULE_WITH_ROLLOUT, Some("bad.yaml"))
            .unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Authoring,
    );
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.message.contains("rollout") && e.message.contains("Entitlement")));
}

#[test]
fn entitlement_rule_with_when_and_serve_is_valid() {
    let content = r#"
catalog:
  id: billing
flags:
  premium_export:
    default: false
    kind: entitlement
environments:
  production:
    rules:
      premium_export:
        - when: "plan == 'enterprise'"
          serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("ok.yaml")).unwrap();
    let result = validate_catalog_value(
        "ok.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.valid, "{:?}", result.errors);
}

#[test]
fn entitlement_rule_with_plain_serve_is_valid() {
    let content = r#"
catalog:
  id: billing
flags:
  premium_export:
    default: false
    kind: entitlement
environments:
  production:
    rules:
      premium_export:
        - serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("ok.yaml")).unwrap();
    let result = validate_catalog_value(
        "ok.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.valid, "{:?}", result.errors);
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
fn warns_on_entitlement_with_default_true() {
    let content = r#"
catalog:
  id: svc
flags:
  premium_feature:
    default: true
    kind: entitlement
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
    let default_warnings: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| w.path.as_deref() == Some("flags.premium_feature.default"))
        .collect();
    assert_eq!(default_warnings.len(), 1);
    assert!(default_warnings[0].message.contains("entitlement"));
    assert!(default_warnings[0].message.contains("default"));
}

#[test]
fn release_default_true_emits_no_entitlement_default_warning() {
    let content = r#"
catalog:
  id: svc
flags:
  rollout_flag:
    default: true
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
    assert!(!result
        .warnings
        .iter()
        .any(|w| w.path.as_deref() == Some("flags.rollout_flag.default")));
}

#[test]
fn entitlement_default_false_emits_no_default_warning() {
    let content = r#"
catalog:
  id: svc
flags:
  premium_feature:
    default: false
    kind: entitlement
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
    assert!(!result
        .warnings
        .iter()
        .any(|w| w.path.as_deref() == Some("flags.premium_feature.default")));
}

#[test]
fn entitlement_without_expires_emits_no_expires_warning() {
    let content = r#"
catalog:
  id: svc
flags:
  premium_feature:
    default: false
    kind: entitlement
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
    assert!(!result
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
fn accepts_valid_attribute_schema() {
    let content = r#"
catalog:
  id: svc
attributes:
  plan: string
  seats: number
  beta: boolean
flags:
  f:
    default: false
    kind: release
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("ok.yaml")).unwrap();
    let result = validate_catalog_value(
        "ok.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.is_ok(), "{:?}", result.errors);
    let doc = parse_catalog(content, Some("ok.yaml")).unwrap();
    assert!(doc.attribute_schema_opted_in());
    let fields = doc.attribute_schema_fields().expect("opted in");
    assert_eq!(fields.get("plan"), Some(&AttributeScalarType::String));
    assert_eq!(fields.get("seats"), Some(&AttributeScalarType::Number));
    assert_eq!(fields.get("beta"), Some(&AttributeScalarType::Boolean));
}

#[test]
fn accepts_valid_attribute_schema_in_sdk_generate_and_compile_modes() {
    let content = r#"
catalog:
  id: svc
attributes:
  plan: string
flags:
  f:
    default: false
    kind: release
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("ok.yaml")).unwrap();
    for mode in [ValidationMode::SdkGenerate, ValidationMode::Compile] {
        let result = validate_catalog_value(
            "ok.yaml",
            &value,
            &CatalogValidationContext::default(),
            mode,
        );
        assert!(result.is_ok(), "{mode:?}: {:?}", result.errors);
    }
}

#[test]
fn catalog_without_attributes_is_not_opted_in() {
    let doc = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
    assert!(!doc.attribute_schema_opted_in());
    assert!(doc.attribute_schema_fields().is_none());
    let value =
        crate::catalog::parse_catalog_value(LOCAL_ONLY, Some("local-only.control-path.yaml"))
            .unwrap();
    for mode in [ValidationMode::SdkGenerate, ValidationMode::Compile] {
        let result = validate_catalog_value(
            "local-only.control-path.yaml",
            &value,
            &CatalogValidationContext::default(),
            mode,
        );
        assert!(result.is_ok(), "{mode:?}: {:?}", result.errors);
    }
}

#[test]
fn accepts_empty_attribute_schema() {
    let content = r#"
catalog:
  id: svc
attributes: {}
flags:
  f:
    default: false
    kind: release
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("ok.yaml")).unwrap();
    let result = validate_catalog_value(
        "ok.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::SdkGenerate,
    );
    assert!(result.is_ok(), "{:?}", result.errors);
    let doc = parse_catalog(content, Some("ok.yaml")).unwrap();
    assert!(doc.attribute_schema_opted_in());
    assert_eq!(doc.attribute_schema_fields(), Some(&BTreeMap::new()));
}

#[test]
fn empty_attribute_schema_opt_in_survives_json_round_trip() {
    let content = r#"
catalog:
  id: svc
attributes: {}
flags:
  f:
    default: false
    kind: release
"#;
    let doc = parse_catalog(content, Some("ok.yaml")).unwrap();
    assert!(doc.attribute_schema_opted_in());
    let result = validate_catalog(
        "ok.yaml",
        &doc,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.is_ok(), "{:?}", result.errors);

    let value = serde_json::to_value(&doc).unwrap();
    assert!(
        value.get("attributes").is_some(),
        "serialized catalog must retain attributes key"
    );
    let restored: CatalogDocument = serde_json::from_value(value).unwrap();
    assert!(restored.attribute_schema_opted_in());
    assert_eq!(restored.attribute_schema_fields(), Some(&BTreeMap::new()));
}

#[test]
fn load_and_validate_catalog_shell_preserves_attributes_opt_in_on_invalid_values() {
    let content = r#"
catalog:
  id: svc
attributes:
  plan: object
flags:
  f:
    default: false
    kind: release
"#;
    let (doc, result) = load_and_validate_catalog(
        content,
        "bad.yaml",
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    )
    .expect("parse");
    assert!(!result.is_ok());
    assert!(
        doc.attribute_schema_opted_in(),
        "recovery shell must preserve opt-in when attributes key was present"
    );
    assert_eq!(
        doc.attribute_schema_fields(),
        Some(&BTreeMap::new()),
        "invalid attribute values collapse to an empty opted-in map in the shell"
    );
}

#[test]
fn load_and_validate_catalog_shell_does_not_opt_in_when_attributes_not_object() {
    let content = r#"
catalog:
  id: svc
attributes: null
flags:
  f:
    default: false
    kind: release
"#;
    let (doc, result) = load_and_validate_catalog(
        content,
        "bad.yaml",
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    )
    .expect("parse");
    assert!(!result.is_ok());
    assert!(
        !doc.attribute_schema_opted_in(),
        "non-object attributes must not imply opt-in on the recovery shell"
    );
}

#[test]
fn rejects_attribute_schema_base_name_collision() {
    let content = r#"
catalog:
  id: svc
attributes:
  role: string
flags:
  f:
    default: false
    kind: release
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    for mode in [ValidationMode::SdkGenerate, ValidationMode::Compile] {
        let result = validate_catalog_value(
            "bad.yaml",
            &value,
            &CatalogValidationContext::default(),
            mode,
        );
        assert!(!result.valid, "expected rejection in {mode:?}");
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("base attribute")),
            "{mode:?}: {:?}",
            result.errors
        );
    }
}

#[test]
fn rejects_attribute_schema_key_colliding_with_import_namespace() {
    let content = r#"
catalog:
  id: svc
imports:
  platform:
    path: ../platform.yaml
attributes:
  platform: string
flags:
  f:
    default: false
    kind: release
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    for mode in [ValidationMode::SdkGenerate, ValidationMode::Compile] {
        let result = validate_catalog_value(
            "bad.yaml",
            &value,
            &CatalogValidationContext::default(),
            mode,
        );
        assert!(!result.valid, "expected rejection in {mode:?}");
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("import namespace")),
            "{mode:?}: {:?}",
            result.errors
        );
    }
}

#[test]
fn rejects_attribute_schema_unknown_type() {
    let content = r#"
catalog:
  id: svc
attributes:
  plan: object
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
    assert!(
        result.errors.iter().any(|e| {
            e.path
                .as_deref()
                .is_some_and(|p| p.contains("attributes") && p.contains("plan"))
        }),
        "{:?}",
        result.errors
    );
    assert!(
        result.errors.iter().any(|e| {
            e.message.contains("object")
                || e.message.contains("enum")
                || e.message.contains("valid")
                || e.message.contains("boolean")
        }),
        "{:?}",
        result.errors
    );
}

#[test]
fn rejects_attribute_schema_invalid_key() {
    let content = r#"
catalog:
  id: svc
attributes:
  BadKey: string
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
    assert!(
        result.errors.iter().any(|e| {
            e.message.contains("BadKey")
                || e.message.contains("additional")
                || e.message.contains("pattern")
        }),
        "{:?}",
        result.errors
    );
}

#[test]
fn imported_catalog_validates_its_own_attribute_schema() {
    let content = r#"
catalog:
  id: platform
attributes:
  org_tier: string
flags:
  emergency_kill_switch:
    default: false
    kind: kill_switch
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("platform.yaml")).unwrap();
    let result = validate_catalog_value(
        "platform.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Authoring,
    );
    assert!(result.is_ok(), "{:?}", result.errors);
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

#[test]
fn local_rule_with_declared_attribute_validates() {
    let content = r#"
catalog:
  id: svc
attributes:
  plan: string
flags:
  f:
    default: false
    kind: release
environments:
  production:
    rules:
      f:
        - when: "plan == 'beta'"
          serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("ok.yaml")).unwrap();
    let result = validate_catalog_value(
        "ok.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.is_ok(), "{:?}", result.errors);
}

#[test]
fn local_rule_with_unknown_attribute_fails_validation() {
    let content = r#"
catalog:
  id: svc
attributes:
  plan: string
flags:
  f:
    default: false
    kind: release
environments:
  production:
    rules:
      f:
        - when: "tier == 'x'"
          serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
    assert!(
        result.errors.iter().any(|e| {
            e.message.contains("tier")
                && e.message.contains("Unknown evaluation attribute")
                && e.path
                    .as_deref()
                    .is_some_and(|p| p.contains("environments.production.rules.f"))
        }),
        "{:?}",
        result.errors
    );
}

#[test]
fn segment_when_receives_attribute_property_validation() {
    let content = r#"
catalog:
  id: svc
attributes:
  plan: string
flags:
  f:
    default: false
    kind: release
segments:
  beta_users:
    when: "tier == 'x'"
environments:
  production:
    rules:
      f:
        - serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
    assert!(
        result.errors.iter().any(|e| {
            e.path
                .as_deref()
                .is_some_and(|p| p == "segments.beta_users.when")
                && e.message.contains("tier")
        }),
        "{:?}",
        result.errors
    );
}

#[test]
fn catalog_without_attributes_skips_rule_property_validation() {
    let content = r#"
catalog:
  id: svc
flags:
  f:
    default: false
    kind: release
environments:
  production:
    rules:
      f:
        - when: "tier == 'x'"
          serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("ok.yaml")).unwrap();
    let result = validate_catalog_value(
        "ok.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.is_ok(), "{:?}", result.errors);
}

#[test]
fn saas_catalog_with_attributes_skips_local_rule_property_checks() {
    let content = r#"
catalog:
  id: svc
mode: saas
saas:
  project: acme/svc
attributes:
  plan: string
flags:
  f:
    default: false
    kind: release
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("ok.yaml")).unwrap();
    let result = validate_catalog_value(
        "ok.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.is_ok(), "{:?}", result.errors);
}

#[test]
fn legacy_user_prefix_normalizes_for_attribute_validation() {
    let content = r#"
catalog:
  id: svc
attributes:
  plan: string
flags:
  f:
    default: false
    kind: release
environments:
  production:
    rules:
      f:
        - when: "user.plan == 'beta'"
          serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("ok.yaml")).unwrap();
    let result = validate_catalog_value(
        "ok.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.is_ok(), "{:?}", result.errors);
}

#[test]
fn dot_path_validates_top_level_segment_only() {
    let content = r#"
catalog:
  id: svc
attributes:
  profile: string
flags:
  f:
    default: false
    kind: release
environments:
  production:
    rules:
      f:
        - when: "profile.tier == 'gold'"
          serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("ok.yaml")).unwrap();
    let result = validate_catalog_value(
        "ok.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.is_ok(), "{:?}", result.errors);
}

#[test]
fn empty_attributes_schema_rejects_unknown_rule_property() {
    let content = r#"
catalog:
  id: svc
attributes: {}
flags:
  f:
    default: false
    kind: release
environments:
  production:
    rules:
      f:
        - when: "tier == 'x'"
          serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
    assert!(
        result.errors.iter().any(|e| e.message.contains("tier")),
        "{:?}",
        result.errors
    );
}

#[test]
fn opted_in_rule_allows_base_attribute() {
    let content = r#"
catalog:
  id: svc
attributes:
  plan: string
flags:
  f:
    default: false
    kind: release
environments:
  production:
    rules:
      f:
        - when: "environment == 'production'"
          serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("ok.yaml")).unwrap();
    let result = validate_catalog_value(
        "ok.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.is_ok(), "{:?}", result.errors);
}

#[test]
fn context_prefix_normalizes_for_attribute_validation() {
    let content = r#"
catalog:
  id: svc
attributes:
  plan: string
flags:
  f:
    default: false
    kind: release
environments:
  production:
    rules:
      f:
        - when: "context.plan == 'beta'"
          serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("ok.yaml")).unwrap();
    let result = validate_catalog_value(
        "ok.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(result.is_ok(), "{:?}", result.errors);
}

#[test]
fn unparseable_when_expression_fails_validation_on_opted_in_catalog() {
    let content = r#"
catalog:
  id: svc
attributes:
  plan: string
flags:
  f:
    default: false
    kind: release
environments:
  production:
    rules:
      f:
        - when: "tier === 'x'"
          serve: true
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("bad.yaml")).unwrap();
    let result = validate_catalog_value(
        "bad.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!result.valid);
    assert!(
        result.errors.iter().any(|e| {
            e.path
                .as_deref()
                .is_some_and(|p| p.contains("environments.production.rules.f"))
                && e.message.contains("Invalid when expression")
        }),
        "{:?}",
        result.errors
    );
}

#[test]
fn imported_catalog_validates_its_own_rule_property_references() {
    let content = r#"
catalog:
  id: platform
attributes:
  org_tier: string
segments:
  gold_orgs:
    when: "tier == 'gold'"
flags:
  emergency_kill_switch:
    default: false
    kind: kill_switch
environments:
  production:
    rules:
      emergency_kill_switch:
        - serve: false
"#;
    let value = crate::catalog::parse_catalog_value(content, Some("platform.yaml")).unwrap();
    let result = validate_catalog_value(
        "platform.yaml",
        &value,
        &CatalogValidationContext::default(),
        ValidationMode::Authoring,
    );
    assert!(!result.valid);
    assert!(
        result.errors.iter().any(|e| {
            e.path.as_deref() == Some("segments.gold_orgs.when") && e.message.contains("tier")
        }),
        "{:?}",
        result.errors
    );
}
