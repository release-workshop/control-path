/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use crate::catalog::parse::parse_catalog_value;
use crate::catalog::validate::{validate_catalog_value, CatalogValidationContext, ValidationMode};

fn parse_fixture(content: &str) -> serde_json::Value {
    parse_catalog_value(content, Some("fixture.yaml")).unwrap()
}

fn import_rule_context() -> CatalogValidationContext {
    let mut ctx = CatalogValidationContext::default();
    ctx.imported_flag_keys
        .insert("emergency_kill_switch".to_string());
    ctx
}

const IMPORT_RULE_VIOLATION: &str = r#"
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

const SEMANTIC_VIOLATION_SAAS: &str = r#"
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

const SCHEMA_VIOLATION: &str = r#"
catalog:
  id: svc
flags:
  f: {}
"#;

#[test]
fn compile_mode_rejects_schema_semantic_and_import_failures() {
    let schema_value = parse_fixture(SCHEMA_VIOLATION);
    let schema_result = validate_catalog_value(
        "fixture.yaml",
        &schema_value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!schema_result.is_ok());

    let semantic_value = parse_fixture(SEMANTIC_VIOLATION_SAAS);
    let semantic_result = validate_catalog_value(
        "fixture.yaml",
        &semantic_value,
        &CatalogValidationContext::default(),
        ValidationMode::Compile,
    );
    assert!(!semantic_result.is_ok());
    assert!(semantic_result
        .errors
        .iter()
        .any(|e| e.message.contains("environments")));

    let import_value = parse_fixture(IMPORT_RULE_VIOLATION);
    let import_result = validate_catalog_value(
        "fixture.yaml",
        &import_value,
        &import_rule_context(),
        ValidationMode::Compile,
    );
    assert!(!import_result.is_ok());
    assert!(import_result
        .errors
        .iter()
        .any(|e| e.message.contains("imported flag")));
}

#[test]
fn sdk_generate_mode_rejects_schema_semantic_and_import_failures() {
    let schema_value = parse_fixture(SCHEMA_VIOLATION);
    let schema_result = validate_catalog_value(
        "fixture.yaml",
        &schema_value,
        &CatalogValidationContext::default(),
        ValidationMode::SdkGenerate,
    );
    assert!(!schema_result.is_ok());

    let semantic_value = parse_fixture(SEMANTIC_VIOLATION_SAAS);
    let semantic_result = validate_catalog_value(
        "fixture.yaml",
        &semantic_value,
        &CatalogValidationContext::default(),
        ValidationMode::SdkGenerate,
    );
    assert!(!semantic_result.is_ok());

    let import_value = parse_fixture(IMPORT_RULE_VIOLATION);
    let import_result = validate_catalog_value(
        "fixture.yaml",
        &import_value,
        &import_rule_context(),
        ValidationMode::SdkGenerate,
    );
    assert!(!import_result.is_ok());
    assert!(import_result
        .errors
        .iter()
        .any(|e| e.message.contains("imported flag")));
}

#[test]
fn bootstrap_sync_mode_allows_saas_environments_block() {
    let semantic_value = parse_fixture(SEMANTIC_VIOLATION_SAAS);
    let result = validate_catalog_value(
        "fixture.yaml",
        &semantic_value,
        &CatalogValidationContext::default(),
        ValidationMode::BootstrapSync,
    );
    assert!(
        result.is_ok(),
        "bootstrap sync permits transitional environments in SaaS mode"
    );
}

#[test]
fn authoring_mode_rejects_schema_and_semantic_but_not_import_rules() {
    let schema_value = parse_fixture(SCHEMA_VIOLATION);
    let schema_result = validate_catalog_value(
        "fixture.yaml",
        &schema_value,
        &CatalogValidationContext::default(),
        ValidationMode::Authoring,
    );
    assert!(!schema_result.is_ok());

    let semantic_value = parse_fixture(SEMANTIC_VIOLATION_SAAS);
    let semantic_result = validate_catalog_value(
        "fixture.yaml",
        &semantic_value,
        &CatalogValidationContext::default(),
        ValidationMode::Authoring,
    );
    assert!(!semantic_result.is_ok());

    let import_value = parse_fixture(IMPORT_RULE_VIOLATION);
    let import_result = validate_catalog_value(
        "fixture.yaml",
        &import_value,
        &import_rule_context(),
        ValidationMode::Authoring,
    );
    assert!(
        import_result.is_ok(),
        "authoring validates the document alone; import cross-catalog rules run in SdkGenerate/Compile"
    );
}
