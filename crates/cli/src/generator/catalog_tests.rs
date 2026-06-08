//! Unit tests for the TypeScript SDK generator (v2 catalog)

use crate::generator::typescript::TypeScriptGenerator;
use crate::generator::Generator;
use controlpath_compiler::{build_sdk_catalog, parse_catalog, SdkCatalog};
use std::collections::BTreeMap;
use std::fs;
use tempfile::TempDir;

const LOCAL_ONLY: &str = include_str!("../../../../schemas/examples/local-only.control-path.yaml");
const SHARED_PLATFORM: &str =
    include_str!("../../../../schemas/examples/shared-platform.control-path.yaml");
const IMPORTED_GLOBAL: &str =
    include_str!("../../../../schemas/examples/imported-global.control-path.yaml");

fn sdk_from_yaml(content: &str, path: &str) -> SdkCatalog {
    let catalog = parse_catalog(content, Some(path)).unwrap();
    build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap()
}

#[test]
fn generated_sdk_delegates_orchestration_to_runtime() {
    let sdk = sdk_from_yaml(LOCAL_ONLY, "local-only.control-path.yaml");
    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("private readonly runtime = new GeneratedEvaluatorRuntime"));
    assert!(index_content.contains("SDK_QUALIFIED_FLAG_NAMES"));
    assert!(index_content.contains("sdkQualifiedFlagNames: SDK_QUALIFIED_FLAG_NAMES"));
    assert!(index_content.contains("this.runtime.evaluateBooleanFlag"));
    assert!(!index_content.contains("const runtime = new GeneratedEvaluatorRuntime"));
    assert!(!index_content.contains("private async refreshKillSwitch"));
    assert!(!index_content.contains("loadFromFile"));
}

#[test]
fn generates_boolean_sdk_from_v2_local_catalog() {
    let sdk = sdk_from_yaml(LOCAL_ONLY, "local-only.control-path.yaml");
    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    generator.generate(&sdk, temp_dir.path()).unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();
    assert!(types_content.contains("import type { BaseAttributes } from '@controlpath/runtime'"));
    assert!(types_content.contains("export interface Attributes extends BaseAttributes"));
    assert!(!types_content.contains("/** User ID */"));
    assert!(types_content.contains("'newDashboard'"));
    assert!(types_content.contains("newDashboard: boolean"));
    assert!(!types_content.contains("Variation"));
    assert!(!types_content.contains("multivariate"));

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("async newDashboard()"));
    assert!(index_content.contains("'new_dashboard'"));
    assert!(!index_content.contains("type: 'multivariate'"));
}

#[test]
fn generates_imported_flags_under_import_namespace() {
    let catalog =
        parse_catalog(IMPORTED_GLOBAL, Some("imported-global.control-path.yaml")).unwrap();
    let platform =
        parse_catalog(SHARED_PLATFORM, Some("shared-platform.control-path.yaml")).unwrap();
    let mut imports = BTreeMap::new();
    imports.insert("platform".to_string(), platform);
    let sdk = build_sdk_catalog(&catalog, &imports).unwrap();

    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();
    assert!(types_content.contains("'platformEmergencyKillSwitch'"));

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("async platformEmergencyKillSwitch()"));
    assert!(index_content.contains("'platform.emergency_kill_switch'"));
}

#[test]
fn generates_urls_with_json_escaped_quotes() {
    let catalog = parse_catalog(
        r#"
catalog:
  id: svc
mode: local
flags:
  feature:
    default: false
    kind: release
kill_switches:
  production:
    url: https://flags.example.com/o'reilly/kill-switches.json
artifacts:
  production:
    url: https://flags.example.com/o'reilly/rules.ast
"#,
        Some("svc.yaml"),
    )
    .unwrap();
    let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();
    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let index = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index.contains("\"https://flags.example.com/o'reilly/kill-switches.json\""));
    assert!(index.contains("\"https://flags.example.com/o'reilly/rules.ast\""));
}

#[test]
fn generates_kill_switch_paths_from_catalog() {
    let catalog = parse_catalog(
        r#"
catalog:
  id: svc
flags:
  f:
    default: false
    kind: kill_switch
kill_switches:
  production:
    path: /mnt/flags/production.kill-switches.json
"#,
        Some("svc.yaml"),
    )
    .unwrap();
    let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();
    assert_eq!(
        sdk.kill_switch_paths.get("production").map(String::as_str),
        Some("/mnt/flags/production.kill-switches.json")
    );

    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("KILL_SWITCH_PATHS"));
    assert!(index_content.contains("killSwitchPaths: KILL_SWITCH_PATHS"));
    assert!(index_content.contains("\"/mnt/flags/production.kill-switches.json\""));
}

#[test]
fn generates_kill_switch_urls_from_catalog() {
    let sdk = sdk_from_yaml(LOCAL_ONLY, "local-only.control-path.yaml");
    assert!(sdk.kill_switch_urls.contains_key("production"));

    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("KILL_SWITCH_URLS"));
    assert!(index_content.contains("production"));
    assert!(index_content.contains("kill-switches.json"));
    assert!(index_content.contains("GeneratedEvaluatorRuntime"));
    assert!(index_content.contains("evaluateBooleanFlag"));
    assert!(!index_content.contains("KillSwitchRefreshCoordinator"));
    assert!(!index_content.contains("refreshKillSwitch()"));
}

#[test]
fn generates_artifact_urls_from_catalog() {
    let sdk = sdk_from_yaml(LOCAL_ONLY, "local-only.control-path.yaml");
    assert!(sdk.artifact_urls.contains_key("production"));

    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("ARTIFACT_URLS"));
    assert!(index_content.contains("rules.ast"));
    assert!(index_content.contains("DEFAULT_GENERATED_ARTIFACT_POLL_MS"));
    assert!(!index_content.contains("ArtifactRefreshCoordinator"));
    assert!(!index_content.contains("assertArtifactAccepted"));
}

#[test]
fn build_sdk_catalog_rejects_duplicate_sdk_method_name_in_generator_path() {
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
}

#[test]
fn environment_rules_do_not_affect_generated_sdk() {
    let with_env = sdk_from_yaml(LOCAL_ONLY, "local-only.control-path.yaml");

    let mut without_env = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
    without_env.environments.clear();
    without_env.segments.clear();
    without_env.kill_switches.clear();
    without_env.artifacts.clear();
    let without_env = build_sdk_catalog(&without_env, &BTreeMap::new()).unwrap();

    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    generator.generate(&with_env, temp_dir.path()).unwrap();
    let with_env_types = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();

    generator.generate(&without_env, temp_dir.path()).unwrap();
    let without_env_types = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();

    assert_eq!(with_env_types, without_env_types);
}

#[test]
fn marks_deprecated_flags_in_generated_sdk() {
    let sdk = sdk_from_yaml(
        r#"
catalog:
  id: svc
flags:
  old_flow:
    default: false
    kind: release
    lifecycle: deprecated
"#,
        "svc.yaml",
    );

    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("@deprecated"));
    assert!(index_content.contains("async oldFlow()"));
}

#[test]
fn generates_closed_attributes_with_import_namespace_when_opted_in() {
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
    assert!(sdk.attribute_schema.is_some());

    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();
    assert!(types_content.contains("plan?: string"));
    assert!(types_content.contains("platform?: PlatformAttributes"));
    assert!(types_content.contains("org_tier?: string"));
    assert!(types_content.contains("export interface PlatformAttributes"));
    assert!(types_content.contains("export type EvaluationAttributes = Attributes"));
    assert!(!types_content.contains("[key: string]: unknown"));

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("async newDashboard(attributes: Attributes)"));
    assert!(index_content.contains("platform?: PlatformAttributes"));
    assert!(
        index_content.contains("async platformOrgGoldFeature(attributes: BaseAttributes & { platform?: PlatformAttributes })")
    );
}

#[test]
fn generates_imported_flags_with_attributes_when_import_has_no_schema() {
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
    assert!(sdk
        .attribute_schema
        .as_ref()
        .is_some_and(|s| s.import_namespaces.is_empty()));

    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();
    assert!(types_content.contains("plan?: string"));
    assert!(!types_content.contains("PlatformAttributes"));
    assert!(!types_content.contains("platform?:"));

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("async platformOrgGoldFeature(attributes: Attributes)"));
    assert!(!index_content.contains("BaseAttributes"));
}

#[test]
fn generates_mixed_per_flag_types_when_only_some_imports_opt_in() {
    let catalog = parse_catalog(
        r#"
catalog:
  id: svc
attributes:
  plan: string
imports:
  platform:
    path: platform.yaml
  billing:
    path: billing.yaml
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
    let billing = parse_catalog(
        r#"
catalog:
  id: billing
flags:
  invoice_v2:
    default: false
    kind: release
"#,
        Some("billing.yaml"),
    )
    .unwrap();
    let mut imports = BTreeMap::new();
    imports.insert("platform".to_string(), platform);
    imports.insert("billing".to_string(), billing);
    let sdk = build_sdk_catalog(&catalog, &imports).unwrap();
    assert_eq!(sdk.flags.len(), 3);
    assert_eq!(
        sdk.attribute_schema
            .as_ref()
            .map(|s| s.import_namespaces.len()),
        Some(1)
    );

    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();
    assert!(types_content.contains("export interface PlatformAttributes"));
    assert!(types_content.contains("platform?: PlatformAttributes"));
    assert!(!types_content.contains("BillingAttributes"));
    assert!(!types_content.contains("billing?:"));

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("async newDashboard(attributes: Attributes)"));
    assert!(index_content.contains(
        "async platformOrgGoldFeature(attributes: BaseAttributes & { platform?: PlatformAttributes })"
    ));
    assert!(index_content.contains("async billingInvoiceV2(attributes: Attributes)"));
    assert!(index_content.contains("import type { BaseAttributes }"));
}

#[test]
fn legacy_catalog_without_attributes_keeps_loose_attributes_type() {
    let sdk = sdk_from_yaml(
        r#"
catalog:
  id: checkout-service
mode: local
flags:
  new_dashboard:
    kind: release
    default: false
"#,
        "legacy-no-attributes.yaml",
    );
    assert!(sdk.attribute_schema.is_none());

    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();
    assert!(types_content.contains("[key: string]: unknown"));
    assert!(!types_content.contains("export type EvaluationAttributes"));
}

#[test]
fn generates_catalog_without_environments() {
    let sdk = sdk_from_yaml(
        r#"
catalog:
  id: svc
flags:
  feature:
    default: true
    kind: release
"#,
        "svc.yaml",
    );

    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    assert!(generator.generate(&sdk, temp_dir.path()).is_ok());
}
