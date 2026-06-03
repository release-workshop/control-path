/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use std::collections::BTreeMap;

use crate::catalog::{
    build_sdk_catalog, parse_catalog, FlagKind, FlagLifecycle, SdkCatalog, SdkFlag,
};

const LOCAL_ONLY: &str = include_str!("../../../../schemas/examples/local-only.control-path.yaml");
const SHARED_PLATFORM: &str =
    include_str!("../../../../schemas/examples/shared-platform.control-path.yaml");
const IMPORTED_GLOBAL: &str =
    include_str!("../../../../schemas/examples/imported-global.control-path.yaml");

fn local_flag<'a>(sdk: &'a SdkCatalog, qualified_name: &str) -> &'a SdkFlag {
    sdk.flags
        .iter()
        .find(|f| f.qualified_name == qualified_name)
        .unwrap_or_else(|| panic!("flag {qualified_name} not found in {sdk:?}"))
}

#[test]
fn build_sdk_catalog_from_local_only_flags() {
    let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
    let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();

    assert_eq!(sdk.flags.len(), 2);
    let dashboard = local_flag(&sdk, "new_dashboard");
    assert!(!dashboard.default);
    assert_eq!(dashboard.kind, FlagKind::Release);
    assert_eq!(dashboard.lifecycle, FlagLifecycle::Active);
    assert_eq!(dashboard.sdk_method_name, "newDashboard");
}

#[test]
fn build_sdk_catalog_ignores_environments() {
    let mut with_env = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
    with_env.environments.clear();

    let sdk_with_env = build_sdk_catalog(
        &parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap(),
        &BTreeMap::new(),
    )
    .unwrap();
    let sdk_without_env = build_sdk_catalog(&with_env, &BTreeMap::new()).unwrap();

    assert_eq!(sdk_with_env, sdk_without_env);
}

#[test]
fn build_sdk_catalog_includes_imported_flags_under_namespace() {
    let catalog =
        parse_catalog(IMPORTED_GLOBAL, Some("imported-global.control-path.yaml")).unwrap();
    let platform =
        parse_catalog(SHARED_PLATFORM, Some("shared-platform.control-path.yaml")).unwrap();
    let mut imports = BTreeMap::new();
    imports.insert("platform".to_string(), platform);

    let sdk = build_sdk_catalog(&catalog, &imports).unwrap();

    assert_eq!(sdk.flags.len(), 2);
    let local = local_flag(&sdk, "new_dashboard");
    assert_eq!(local.sdk_method_name, "newDashboard");

    let imported = local_flag(&sdk, "platform.emergency_kill_switch");
    assert!(!imported.default);
    assert_eq!(imported.kind, FlagKind::KillSwitch);
    assert_eq!(imported.sdk_method_name, "platformEmergencyKillSwitch");
}

#[test]
fn build_sdk_catalog_includes_artifact_urls() {
    let content = r#"
catalog:
  id: svc
flags:
  f:
    default: false
    kind: release
artifacts:
  production:
    url: https://flags.example.com/production/rules.ast
"#;
    let catalog = parse_catalog(content, Some("svc.yaml")).unwrap();
    let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();

    assert_eq!(
        sdk.artifact_urls.get("production").map(String::as_str),
        Some("https://flags.example.com/production/rules.ast")
    );
}

#[test]
fn build_sdk_catalog_marks_deprecated_flags() {
    let content = r#"
catalog:
  id: svc
flags:
  old_flow:
    default: false
    kind: release
    lifecycle: deprecated
"#;
    let catalog = parse_catalog(content, Some("svc.yaml")).unwrap();
    let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();

    let flag = local_flag(&sdk, "old_flow");
    assert_eq!(flag.lifecycle, FlagLifecycle::Deprecated);
}

#[test]
fn build_sdk_catalog_keeps_local_and_imported_flags_distinct() {
    let catalog = parse_catalog(
        r#"
catalog:
  id: svc
flags:
  emergency_kill_switch:
    default: false
    kind: release
"#,
        Some("svc.yaml"),
    )
    .unwrap();

    let imported = parse_catalog(
        r#"
catalog:
  id: platform
flags:
  emergency_kill_switch:
    default: true
    kind: kill_switch
"#,
        Some("platform.yaml"),
    )
    .unwrap();

    let mut imports = BTreeMap::new();
    imports.insert("platform".to_string(), imported);

    let sdk = build_sdk_catalog(&catalog, &imports).unwrap();
    assert_eq!(sdk.flags.len(), 2);
    assert!(sdk
        .flags
        .iter()
        .any(|f| f.qualified_name == "emergency_kill_switch"));
    assert!(sdk
        .flags
        .iter()
        .any(|f| f.qualified_name == "platform.emergency_kill_switch"));
    assert!(!local_flag(&sdk, "emergency_kill_switch").is_imported);
    assert!(local_flag(&sdk, "platform.emergency_kill_switch").is_imported);
}

#[test]
fn build_sdk_catalog_rejects_duplicate_qualified_name() {
    let catalog = parse_catalog(
        r#"
catalog:
  id: svc
flags:
  platform.emergency_kill_switch:
    default: false
    kind: release
"#,
        Some("svc.yaml"),
    )
    .unwrap();

    let imported = parse_catalog(
        r#"
catalog:
  id: platform
flags:
  emergency_kill_switch:
    default: true
    kind: kill_switch
"#,
        Some("platform.yaml"),
    )
    .unwrap();

    let mut imports = BTreeMap::new();
    imports.insert("platform".to_string(), imported);

    let err = build_sdk_catalog(&catalog, &imports).unwrap_err();
    assert!(err.contains("duplicate SDK flag"));
    assert!(err.contains("platform.emergency_kill_switch"));
}

#[test]
fn build_sdk_catalog_includes_attribute_schema_when_service_opts_in() {
    let catalog = parse_catalog(
        r#"
catalog:
  id: svc
attributes:
  plan: string
imports:
  platform:
    path: platform.yaml
flags:
  new_dashboard:
    default: false
    kind: release
"#,
        Some("svc.yaml"),
    )
    .unwrap();
    let platform = parse_catalog(
        r#"
catalog:
  id: platform
attributes:
  org_tier: string
flags:
  org_gold_feature:
    default: false
    kind: release
"#,
        Some("platform.yaml"),
    )
    .unwrap();
    let mut imports = BTreeMap::new();
    imports.insert("platform".to_string(), platform);

    let sdk = build_sdk_catalog(&catalog, &imports).unwrap();
    let schema = sdk.attribute_schema.as_ref().expect("opted in");
    assert_eq!(
        schema.service_fields.get("plan").copied(),
        Some(crate::catalog::AttributeScalarType::String)
    );
    assert_eq!(schema.import_namespaces.len(), 1);
    assert_eq!(schema.import_namespaces[0].namespace, "platform");
    assert_eq!(
        schema.import_namespaces[0].fields.get("org_tier").copied(),
        Some(crate::catalog::AttributeScalarType::String)
    );

    let imported = local_flag(&sdk, "platform.org_gold_feature");
    assert_eq!(imported.import_namespace.as_deref(), Some("platform"));
}

#[test]
fn build_sdk_catalog_rejects_duplicate_sdk_method_name() {
    let catalog = parse_catalog(
        r#"
catalog:
  id: svc
flags:
  platform_emergency_kill_switch:
    default: false
    kind: release
"#,
        Some("svc.yaml"),
    )
    .unwrap();

    let imported = parse_catalog(
        r#"
catalog:
  id: platform
flags:
  emergency_kill_switch:
    default: true
    kind: kill_switch
"#,
        Some("platform.yaml"),
    )
    .unwrap();

    let mut imports = BTreeMap::new();
    imports.insert("platform".to_string(), imported);

    let err = build_sdk_catalog(&catalog, &imports).unwrap_err();
    assert!(err.contains("duplicate SDK method"));
    assert!(err.contains("platformEmergencyKillSwitch"));
}
