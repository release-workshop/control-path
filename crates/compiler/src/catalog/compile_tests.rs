/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use crate::ast::{Artifact, Expression, RolloutValue, Rule, ServePayload};
use std::collections::BTreeMap;

use crate::catalog::{
    compile_catalog, compile_catalog_with_imports, load_validate_and_compile_catalog,
    parse_catalog, validate_and_compile_catalog, CatalogValidationContext, FlagKind, FlagLifecycle,
};
use crate::runtime::{evaluate_flag, EvaluationAttributes};
use serde::Deserialize;
use serde_json::json;

const LOCAL_ONLY: &str = include_str!("../../../../schemas/examples/local-only.control-path.yaml");
const SHARED_PLATFORM: &str =
    include_str!("../../../../schemas/examples/shared-platform.control-path.yaml");
const IMPORTED_GLOBAL: &str =
    include_str!("../../../../schemas/examples/imported-global.control-path.yaml");

const IMPORTED_NAMESPACED_EVAL_FIXTURE: &str =
    include_str!("../../../../schemas/test-fixtures/imported-namespaced-eval.fixture.json");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportedNamespacedEvalFixture {
    property_path: String,
    literal_value: String,
    flag_name: String,
    evaluations: Vec<ImportedNamespacedEvalCase>,
}

#[derive(Deserialize)]
struct ImportedNamespacedEvalCase {
    attributes: serde_json::Value,
    value: bool,
}

fn imported_namespaced_eval_fixture() -> ImportedNamespacedEvalFixture {
    serde_json::from_str(IMPORTED_NAMESPACED_EVAL_FIXTURE)
        .expect("imported-namespaced-eval.fixture.json must parse")
}

fn assert_compile_matches_imported_namespaced_fixture(artifact: &Artifact) {
    let fixture = imported_namespaced_eval_fixture();
    assert!(
        artifact.string_table.contains(&fixture.property_path),
        "string table: {:?}",
        artifact.string_table
    );
    assert!(
        artifact.string_table.contains(&fixture.literal_value),
        "string table: {:?}",
        artifact.string_table
    );
    let flag_index = artifact
        .flag_names
        .iter()
        .position(|&idx| str_at(artifact, idx) == fixture.flag_name)
        .unwrap_or_else(|| panic!("flag {} not found", fixture.flag_name));
    for case in fixture.evaluations {
        let attrs = EvaluationAttributes {
            attributes: &case.attributes,
        };
        let (idx, val) = evaluate_flag(artifact, flag_index, &attrs);
        if case.value {
            assert_eq!(idx, Some(0), "attributes {:?}", case.attributes);
            assert_eq!(val, Some(json!("ON")), "attributes {:?}", case.attributes);
        } else {
            assert_eq!(idx, Some(1), "attributes {:?}", case.attributes);
            assert_eq!(val, Some(json!("OFF")), "attributes {:?}", case.attributes);
        }
    }
}

fn str_at(artifact: &Artifact, index: u16) -> &str {
    &artifact.string_table[usize::from(index)]
}

fn when_expression_property_paths(expr: &Expression, artifact: &Artifact) -> Vec<String> {
    match expr {
        Expression::Property { prop_index } => {
            vec![str_at(artifact, *prop_index).to_string()]
        }
        Expression::BinaryOp { left, right, .. } => {
            let mut paths = when_expression_property_paths(left, artifact);
            paths.extend(when_expression_property_paths(right, artifact));
            paths
        }
        Expression::LogicalOp { left, right, .. } => {
            let mut paths = when_expression_property_paths(left, artifact);
            if let Some(right) = right {
                paths.extend(when_expression_property_paths(right, artifact));
            }
            paths
        }
        Expression::Func { args, .. } => args
            .iter()
            .flat_map(|arg| when_expression_property_paths(arg, artifact))
            .collect(),
        Expression::Literal { .. } => Vec::new(),
    }
}

fn flag_rules<'a>(artifact: &'a Artifact, flag_name: &str) -> &'a [Rule] {
    let flag_index = artifact
        .flag_names
        .iter()
        .position(|&name_idx| str_at(artifact, name_idx) == flag_name)
        .unwrap_or_else(|| panic!("flag {flag_name} not found in artifact"));
    &artifact.flags[flag_index]
}

#[test]
fn compiles_staging_explicit_serve_rule() {
    let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
    let artifact = compile_catalog(&catalog, "staging").unwrap();

    assert_eq!(artifact.environment, "staging");
    assert_eq!(artifact.flags.len(), 2);

    let new_dashboard = flag_rules(&artifact, "new_dashboard");
    assert_eq!(new_dashboard.len(), 2);
    match &new_dashboard[0] {
        Rule::ServeWithoutWhen(ServePayload::Number(idx)) => {
            assert_eq!(str_at(&artifact, *idx), "ON");
        }
        other => panic!("expected serve ON rule, got {other:?}"),
    }
    match &new_dashboard[1] {
        Rule::ServeWithoutWhen(ServePayload::Number(idx)) => {
            assert_eq!(str_at(&artifact, *idx), "OFF");
        }
        other => panic!("expected default OFF rule, got {other:?}"),
    }
}

#[test]
fn falls_back_to_catalog_default_when_no_env_rules() {
    let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
    let artifact = compile_catalog(&catalog, "staging").unwrap();

    let premium_checkout = flag_rules(&artifact, "premium_checkout");
    assert_eq!(premium_checkout.len(), 1);
    match &premium_checkout[0] {
        Rule::ServeWithoutWhen(ServePayload::Number(idx)) => {
            assert_eq!(str_at(&artifact, *idx), "OFF");
        }
        other => panic!("expected default OFF rule, got {other:?}"),
    }
}

#[test]
fn compiles_production_targeted_and_rollout_rules() {
    let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
    let artifact = compile_catalog(&catalog, "production").unwrap();

    let new_dashboard = flag_rules(&artifact, "new_dashboard");
    assert_eq!(new_dashboard.len(), 3);

    match &new_dashboard[0] {
        Rule::ServeWithWhen(expr, ServePayload::Number(idx)) => {
            assert_eq!(str_at(&artifact, *idx), "ON");
            assert!(matches!(expr, Expression::Func { .. }));
        }
        other => panic!("expected targeted serve rule, got {other:?}"),
    }

    match &new_dashboard[1] {
        Rule::RolloutWithoutWhen(payload) => {
            assert_eq!(payload.percentage, 10);
            match &payload.value_index {
                RolloutValue::Number(idx) => assert_eq!(str_at(&artifact, *idx), "ON"),
                other => panic!("expected rollout ON value, got {other:?}"),
            }
        }
        other => panic!("expected rollout rule, got {other:?}"),
    }

    match &new_dashboard[2] {
        Rule::ServeWithoutWhen(ServePayload::Number(idx)) => {
            assert_eq!(str_at(&artifact, *idx), "OFF");
        }
        other => panic!("expected default OFF rule, got {other:?}"),
    }
}

#[test]
fn rollout_reason_does_not_break_compilation() {
    let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
    let production = catalog
        .environments
        .get("production")
        .expect("production environment");
    let rules = production
        .rules
        .get("new_dashboard")
        .expect("new_dashboard rules");
    assert_eq!(
        rules[1].reason.as_deref(),
        Some("Gradual rollout after beta validation")
    );

    let artifact = compile_catalog(&catalog, "production").unwrap();
    let new_dashboard = flag_rules(&artifact, "new_dashboard");
    assert_eq!(new_dashboard.len(), 3);
    assert!(matches!(new_dashboard[1], Rule::RolloutWithoutWhen(_)));
}

#[test]
fn compiles_top_level_segments() {
    let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
    let artifact = compile_catalog(&catalog, "production").unwrap();

    let segments = artifact.segments.expect("segments should be present");
    assert_eq!(segments.len(), 1);
    assert!(artifact.string_table.contains(&"beta_users".to_string()));
    match &segments[0].1 {
        Expression::BinaryOp { .. } => {}
        other => panic!("expected segment predicate expression, got {other:?}"),
    }
}

#[test]
fn compiles_deprecated_flags() {
    let content = r#"
catalog:
  id: svc
mode: local
flags:
  active_flag:
    default: true
    kind: release
  legacy_flag:
    default: false
    kind: release
    lifecycle: deprecated
environments:
  production:
    rules:
      active_flag:
        - serve: true
      legacy_flag:
        - serve: true
"#;
    let catalog = parse_catalog(content, Some("catalog.yaml")).unwrap();
    let legacy = catalog.flags.get("legacy_flag").unwrap();
    assert_eq!(legacy.lifecycle, FlagLifecycle::Deprecated);

    let artifact = compile_catalog(&catalog, "production").unwrap();
    assert_eq!(artifact.flags.len(), 2);

    let legacy_rules = flag_rules(&artifact, "legacy_flag");
    assert_eq!(legacy_rules.len(), 2);
    match &legacy_rules[0] {
        Rule::ServeWithoutWhen(ServePayload::Number(idx)) => {
            assert_eq!(str_at(&artifact, *idx), "ON");
        }
        other => panic!("expected serve rule for deprecated flag, got {other:?}"),
    }
}

#[test]
fn compiles_kill_switch_serve_only_rule() {
    let catalog =
        parse_catalog(SHARED_PLATFORM, Some("shared-platform.control-path.yaml")).unwrap();
    let kill_switch = catalog.flags.get("emergency_kill_switch").unwrap();
    assert_eq!(kill_switch.kind, FlagKind::KillSwitch);

    let artifact = compile_catalog(&catalog, "production").unwrap();
    assert_eq!(artifact.flags.len(), 1);

    let rules = flag_rules(&artifact, "emergency_kill_switch");
    assert_eq!(rules.len(), 2);
    match &rules[0] {
        Rule::ServeWithoutWhen(ServePayload::Number(idx)) => {
            assert_eq!(str_at(&artifact, *idx), "OFF");
        }
        other => panic!("expected kill switch serve rule, got {other:?}"),
    }
}

#[test]
fn unknown_environment_compiles_defaults_only() {
    let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
    let artifact = compile_catalog(&catalog, "development").unwrap();

    assert_eq!(artifact.environment, "development");
    for flag_name in catalog.flags.keys() {
        let rules = flag_rules(&artifact, flag_name);
        assert_eq!(rules.len(), 1);
        match &rules[0] {
            Rule::ServeWithoutWhen(ServePayload::Number(idx)) => {
                assert_eq!(str_at(&artifact, *idx), "OFF");
            }
            other => panic!("expected default rule only for {flag_name}, got {other:?}"),
        }
    }
}

#[test]
fn rejects_saas_mode_catalog() {
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
"#;
    let catalog = parse_catalog(content, Some("saas.yaml")).unwrap();
    let err = compile_catalog(&catalog, "production").unwrap_err();
    assert!(err.to_string().contains("SaaS mode"));
}

#[test]
fn rejects_unknown_local_rule_key_at_compile_time() {
    let content = r#"
catalog:
  id: svc
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  production:
    rules:
      my_flga:
        - serve: true
"#;
    let catalog = parse_catalog(content, Some("bad.yaml")).unwrap();
    let err = compile_catalog(&catalog, "production").unwrap_err();
    assert!(err.to_string().contains("my_flga"));
    assert!(err.to_string().contains("not found in flag definitions"));
}

#[test]
fn ignores_empty_rules_on_unknown_local_rule_key() {
    let content = r#"
catalog:
  id: svc
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  production:
    rules:
      my_flga: []
"#;
    let catalog = parse_catalog(content, Some("catalog.yaml")).unwrap();
    let artifact = compile_catalog(&catalog, "production").unwrap();
    assert_eq!(flag_rules(&artifact, "my_flag").len(), 1);
}

#[test]
fn rejects_unknown_import_rule_key_at_compile_time() {
    let content = r#"
catalog:
  id: svc
mode: local
imports:
  platform:
    path: platform.yaml
flags:
  new_dashboard:
    default: false
    kind: release
environments:
  production:
    rules:
      new_dashboard:
        - serve: true
"#;
    let platform = r#"
catalog:
  id: platform
flags:
  emergency_kill_switch:
    default: false
    kind: kill_switch
environments:
  production:
    rules:
      emergency_killswitch:
        - serve: false
"#;
    let catalog = parse_catalog(content, Some("service.yaml")).unwrap();
    let imported = parse_catalog(platform, Some("platform.yaml")).unwrap();
    let mut imports = BTreeMap::new();
    imports.insert("platform".to_string(), imported);
    let err = compile_catalog_with_imports(&catalog, &imports, "production").unwrap_err();
    assert!(err.to_string().contains("platform.emergency_killswitch"));
    assert!(err.to_string().contains("not found in flag definitions"));
}

#[test]
fn rejects_rule_with_both_serve_and_rollout_at_compile_time() {
    let content = r#"
catalog:
  id: svc
mode: local
flags:
  f:
    default: false
    kind: release
environments:
  production:
    rules:
      f:
        - serve: true
          rollout:
            percentage: 10
            serve: true
"#;
    let catalog = parse_catalog(content, Some("bad.yaml")).unwrap();
    let err = compile_catalog(&catalog, "production").unwrap_err();
    assert!(err.to_string().contains("not both"));
}

#[test]
fn rejects_empty_rule_at_compile_time() {
    let content = r#"
catalog:
  id: svc
mode: local
flags:
  f:
    default: false
    kind: release
environments:
  production:
    rules:
      f:
        - when: "true"
"#;
    let catalog = parse_catalog(content, Some("bad.yaml")).unwrap();
    let err = compile_catalog(&catalog, "production").unwrap_err();
    assert!(err.to_string().contains("serve or rollout"));
}

#[test]
fn rejects_out_of_range_rollout_percentage_at_compile_time() {
    let content = r#"
catalog:
  id: svc
mode: local
flags:
  f:
    default: false
    kind: release
environments:
  production:
    rules:
      f:
        - rollout:
            percentage: 150
            serve: true
"#;
    let catalog = parse_catalog(content, Some("bad.yaml")).unwrap();
    let err = compile_catalog(&catalog, "production").unwrap_err();
    assert!(err.to_string().contains("between 0 and 100"));
}

#[test]
fn validate_and_compile_catalog_rejects_invalid_catalog() {
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
    let catalog = parse_catalog(content, Some("bad.yaml")).unwrap();
    let ctx = CatalogValidationContext::default();
    let err = validate_and_compile_catalog("bad.yaml", &catalog, &BTreeMap::new(), "prod", &ctx)
        .unwrap_err();
    assert!(err.to_string().contains("Validation error"));
}

fn imported_global_fixture() -> (
    crate::catalog::CatalogDocument,
    BTreeMap<String, crate::catalog::CatalogDocument>,
) {
    let catalog =
        parse_catalog(IMPORTED_GLOBAL, Some("imported-global.control-path.yaml")).unwrap();
    let platform =
        parse_catalog(SHARED_PLATFORM, Some("shared-platform.control-path.yaml")).unwrap();
    let mut imports = BTreeMap::new();
    imports.insert("platform".to_string(), platform);
    (catalog, imports)
}

#[test]
fn compiles_imported_flags_with_source_environment_rules() {
    let (catalog, imports) = imported_global_fixture();
    let artifact = compile_catalog_with_imports(&catalog, &imports, "production").unwrap();

    assert_eq!(artifact.flags.len(), 2);

    let imported = flag_rules(&artifact, "platform.emergency_kill_switch");
    assert_eq!(imported.len(), 2);
    match &imported[0] {
        Rule::ServeWithoutWhen(ServePayload::Number(idx)) => {
            assert_eq!(str_at(&artifact, *idx), "OFF");
        }
        other => panic!("expected imported kill switch serve rule, got {other:?}"),
    }

    let local = flag_rules(&artifact, "new_dashboard");
    assert_eq!(local.len(), 1);
    match &local[0] {
        Rule::ServeWithoutWhen(ServePayload::Number(idx)) => {
            assert_eq!(str_at(&artifact, *idx), "OFF");
        }
        other => panic!("expected local default rule, got {other:?}"),
    }
}

#[test]
fn compiles_local_and_imported_environment_rules_for_same_env() {
    let (catalog, imports) = imported_global_fixture();
    let artifact = compile_catalog_with_imports(&catalog, &imports, "staging").unwrap();

    let local = flag_rules(&artifact, "new_dashboard");
    assert_eq!(local.len(), 2);
    match &local[0] {
        Rule::ServeWithoutWhen(ServePayload::Number(idx)) => {
            assert_eq!(str_at(&artifact, *idx), "ON");
        }
        other => panic!("expected staging serve rule, got {other:?}"),
    }

    let imported = flag_rules(&artifact, "platform.emergency_kill_switch");
    assert_eq!(imported.len(), 1);
    match &imported[0] {
        Rule::ServeWithoutWhen(ServePayload::Number(idx)) => {
            assert_eq!(str_at(&artifact, *idx), "OFF");
        }
        other => panic!("expected imported default rule, got {other:?}"),
    }
}

#[test]
fn load_validate_and_compile_local_only_example() {
    let ctx = CatalogValidationContext::default();
    let artifact = load_validate_and_compile_catalog(
        LOCAL_ONLY,
        "local-only.control-path.yaml",
        &BTreeMap::new(),
        "staging",
        &ctx,
    )
    .unwrap();
    assert_eq!(artifact.environment, "staging");
    assert_eq!(flag_rules(&artifact, "new_dashboard").len(), 2);
}

#[test]
fn validate_and_compile_catalog_includes_resolved_imports() {
    let (catalog, imports) = imported_global_fixture();
    let artifact = validate_and_compile_catalog(
        "imported-global.control-path.yaml",
        &catalog,
        &imports,
        "production",
        &CatalogValidationContext::default(),
    )
    .unwrap();
    assert_eq!(
        flag_rules(&artifact, "platform.emergency_kill_switch").len(),
        2
    );
}

#[test]
fn rejects_segment_name_collision_between_service_and_import() {
    use crate::catalog::Segment;

    let (mut catalog, mut imports) = imported_global_fixture();
    catalog.segments.insert(
        "shared_segment".to_string(),
        Segment {
            when: "true".to_string(),
        },
    );
    let platform = imports.get_mut("platform").unwrap();
    platform.segments.insert(
        "shared_segment".to_string(),
        Segment {
            when: "false".to_string(),
        },
    );
    let err = compile_catalog_with_imports(&catalog, &imports, "staging").unwrap_err();
    assert!(err.to_string().contains("shared_segment"));
}

#[test]
fn rewrites_imported_rule_attribute_paths_to_import_namespace() {
    let platform = r#"
catalog:
  id: platform
attributes:
  org_tier: string
flags:
  emergency_kill_switch:
    default: false
    kind: kill_switch
environments:
  production:
    rules:
      emergency_kill_switch:
        - when: "org_tier == 'gold'"
          serve: true
"#;
    let service = r#"
catalog:
  id: checkout-service
mode: local
imports:
  platform:
    path: platform/control-path.yaml
flags:
  new_dashboard:
    default: false
    kind: release
"#;
    let mut imports = BTreeMap::new();
    imports.insert(
        "platform".to_string(),
        parse_catalog(platform, Some("platform/control-path.yaml")).unwrap(),
    );
    let catalog = parse_catalog(service, Some("control-path.yaml")).unwrap();
    let artifact = compile_catalog_with_imports(&catalog, &imports, "production").unwrap();

    assert!(
        artifact
            .string_table
            .contains(&"platform.org_tier".to_string()),
        "string table: {:?}",
        artifact.string_table
    );
    assert!(!artifact.string_table.contains(&"org_tier".to_string()));

    let rules = flag_rules(&artifact, "platform.emergency_kill_switch");
    match &rules[0] {
        Rule::ServeWithWhen(when, ServePayload::Number(_)) => {
            let paths = when_expression_property_paths(when, &artifact);
            assert_eq!(paths, vec!["platform.org_tier"]);
        }
        other => panic!("expected imported serve-with-when rule, got {other:?}"),
    }
}

#[test]
fn rewrites_imported_segment_attribute_paths_to_import_namespace() {
    let platform = r#"
catalog:
  id: platform
attributes:
  org_tier: string
segments:
  gold_orgs:
    when: "org_tier == 'gold'"
flags:
  emergency_kill_switch:
    default: false
    kind: kill_switch
"#;
    let service = r#"
catalog:
  id: checkout-service
mode: local
imports:
  platform:
    path: platform/control-path.yaml
flags:
  new_dashboard:
    default: false
    kind: release
"#;
    let mut imports = BTreeMap::new();
    imports.insert(
        "platform".to_string(),
        parse_catalog(platform, Some("platform/control-path.yaml")).unwrap(),
    );
    let catalog = parse_catalog(service, Some("control-path.yaml")).unwrap();
    let artifact = compile_catalog_with_imports(&catalog, &imports, "production").unwrap();

    assert!(
        artifact
            .string_table
            .contains(&"platform.org_tier".to_string()),
        "string table: {:?}",
        artifact.string_table
    );

    let segments = artifact
        .segments
        .as_ref()
        .expect("imported segments should be present");
    let gold_orgs = segments
        .iter()
        .find(|(name_idx, _)| str_at(&artifact, *name_idx) == "gold_orgs")
        .unwrap_or_else(|| panic!("gold_orgs segment not found"));
    let paths = when_expression_property_paths(&gold_orgs.1, &artifact);
    assert_eq!(paths, vec!["platform.org_tier"]);
}

#[test]
fn rewrites_legacy_prefixed_imported_rule_attributes_to_import_namespace() {
    let platform = r#"
catalog:
  id: platform
attributes:
  org_tier: string
flags:
  emergency_kill_switch:
    default: false
    kind: kill_switch
environments:
  production:
    rules:
      emergency_kill_switch:
        - when: "user.org_tier == 'gold'"
          serve: true
"#;
    let service = r#"
catalog:
  id: checkout-service
mode: local
imports:
  platform:
    path: platform/control-path.yaml
flags:
  new_dashboard:
    default: false
    kind: release
"#;
    let mut imports = BTreeMap::new();
    imports.insert(
        "platform".to_string(),
        parse_catalog(platform, Some("platform/control-path.yaml")).unwrap(),
    );
    let catalog = parse_catalog(service, Some("control-path.yaml")).unwrap();
    let artifact = compile_catalog_with_imports(&catalog, &imports, "production").unwrap();

    let rules = flag_rules(&artifact, "platform.emergency_kill_switch");
    match &rules[0] {
        Rule::ServeWithWhen(when, ServePayload::Number(_)) => {
            let paths = when_expression_property_paths(when, &artifact);
            assert_eq!(paths, vec!["platform.org_tier"]);
        }
        other => panic!("expected imported serve-with-when rule, got {other:?}"),
    }
}

#[test]
fn imported_namespaced_rule_evaluates_against_nested_runtime_attributes() {
    let platform = r#"
catalog:
  id: platform
attributes:
  org_tier: string
flags:
  emergency_kill_switch:
    default: false
    kind: kill_switch
environments:
  production:
    rules:
      emergency_kill_switch:
        - when: "org_tier == 'gold'"
          serve: true
"#;
    let service = r#"
catalog:
  id: checkout-service
mode: local
imports:
  platform:
    path: platform/control-path.yaml
flags:
  new_dashboard:
    default: false
    kind: release
"#;
    let mut imports = BTreeMap::new();
    imports.insert(
        "platform".to_string(),
        parse_catalog(platform, Some("platform/control-path.yaml")).unwrap(),
    );
    let catalog = parse_catalog(service, Some("control-path.yaml")).unwrap();
    let artifact = compile_catalog_with_imports(&catalog, &imports, "production").unwrap();
    assert_compile_matches_imported_namespaced_fixture(&artifact);
}

#[test]
fn local_rule_attribute_paths_stay_top_level_when_service_has_attributes() {
    let platform = r#"
catalog:
  id: platform
attributes:
  org_tier: string
flags:
  emergency_kill_switch:
    default: false
    kind: kill_switch
"#;
    let service = r#"
catalog:
  id: checkout-service
mode: local
attributes:
  plan: string
imports:
  platform:
    path: platform/control-path.yaml
flags:
  new_dashboard:
    default: false
    kind: release
environments:
  production:
    rules:
      new_dashboard:
        - when: "plan == 'beta'"
          serve: true
"#;
    let mut imports = BTreeMap::new();
    imports.insert(
        "platform".to_string(),
        parse_catalog(platform, Some("platform/control-path.yaml")).unwrap(),
    );
    let catalog = parse_catalog(service, Some("control-path.yaml")).unwrap();
    let artifact = compile_catalog_with_imports(&catalog, &imports, "production").unwrap();

    assert!(artifact.string_table.contains(&"plan".to_string()));
    assert!(!artifact
        .string_table
        .contains(&"checkout-service.plan".to_string()));

    let rules = flag_rules(&artifact, "new_dashboard");
    match &rules[0] {
        Rule::ServeWithWhen(when, ServePayload::Number(_)) => {
            let paths = when_expression_property_paths(when, &artifact);
            assert_eq!(paths, vec!["plan"]);
        }
        other => panic!("expected local serve-with-when rule, got {other:?}"),
    }

    let flag_index = artifact
        .flag_names
        .iter()
        .position(|&idx| str_at(&artifact, idx) == "new_dashboard")
        .expect("local flag");
    let attrs = EvaluationAttributes {
        attributes: &json!({ "id": "u1", "plan": "beta" }),
    };
    let (idx, val) = evaluate_flag(&artifact, flag_index, &attrs);
    assert_eq!(idx, Some(0));
    assert_eq!(val, Some(json!("ON")));
}

#[test]
fn rejects_segment_name_collision_between_imports() {
    use crate::catalog::Segment;

    let (catalog, mut imports) = imported_global_fixture();
    let platform = imports.remove("platform").unwrap();
    imports.insert("platform_a".to_string(), {
        let mut a = platform.clone();
        a.segments.insert(
            "shared_segment".to_string(),
            Segment {
                when: "true".to_string(),
            },
        );
        a
    });
    imports.insert("platform_b".to_string(), {
        let mut b = platform;
        b.segments.insert(
            "shared_segment".to_string(),
            Segment {
                when: "false".to_string(),
            },
        );
        b
    });
    let err = compile_catalog_with_imports(&catalog, &imports, "staging").unwrap_err();
    assert!(err.to_string().contains("platform_a"));
    assert!(err.to_string().contains("platform_b"));
}
