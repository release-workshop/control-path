/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * Structured evaluation trace for boolean flags (shared by CLI explain and audit surfaces).
 *
 * Callers with MessagePack **compiled artifact** bytes should deserialize once (e.g. `rmp_serde::from_slice`)
 * and pass [`Artifact`] to [`explain_flag`].
 */

use std::collections::BTreeMap;

use serde_json::Value;

use crate::ast::Artifact;
use crate::catalog::model::{CatalogDocument, Rule as CatalogRuleRow};
use crate::catalog::SdkFlag;
use crate::runtime::{
    evaluate_flag, evaluate_rule, find_flag_index, rollout_bucket, user_id, EvaluationAttributes,
};

/// Which layer produced the final flag value (production order: kill switch → AST → default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplainLayer {
    KillSwitch,
    EnvironmentRule,
    CatalogDefault,
}

/// Per-rule trace entry when walking the compiled artifact.
#[derive(Debug, Clone, PartialEq)]
pub struct ExplainRuleTrace {
    /// 0-based index in the artifact rule list.
    pub rule_index: usize,
    pub matched: bool,
    pub evaluation_reason: String,
    pub catalog_reason: Option<String>,
    pub catalog_note: Option<String>,
    pub value: Option<Value>,
}

/// Full explain result for one flag evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct ExplainTrace {
    pub environment: String,
    pub flag: String,
    pub layer: ExplainLayer,
    pub value: bool,
    /// 0-based matched AST rule index, if evaluation reached the artifact.
    pub rule_index: Option<usize>,
    pub catalog_rule: Option<CatalogRuleRow>,
    pub imported: bool,
    pub deprecated: bool,
    pub rollout_bucket: Option<u32>,
    pub missing_id: bool,
    pub warnings: Vec<String>,
    pub rule_trace: Vec<ExplainRuleTrace>,
}

/// Local kill-switch file `flags` map (flag name → override value).
pub type KillSwitchOverrides = BTreeMap<String, bool>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplainError {
    FlagNotInArtifact { flag: String },
    NoRuleMatched { flag: String },
}

/// Inputs for [`explain_flag`]. Catalog metadata is used for `reason`, rollout display, and SaaS warnings.
///
/// [`ExplainTrace::rollout_bucket`] and [`ExplainTrace::missing_id`] are derived from the matched
/// catalog YAML row when present. A rollout rule can match in the artifact while catalog rows are missing
/// or out of sync (common with stale SaaS AST); in that case evaluation still uses the artifact but rollout
/// diagnostics are omitted until catalog metadata aligns with the artifact index.
#[derive(Debug, Clone)]
pub struct ExplainRequest<'a> {
    pub artifact: &'a Artifact,
    pub flag: &'a str,
    /// Environment name for catalog rule lookup (may differ from `artifact.environment` — warnings reflect that).
    pub environment: &'a str,
    pub catalog: &'a CatalogDocument,
    pub imports: &'a BTreeMap<String, CatalogDocument>,
    pub sdk_flag: &'a SdkFlag,
    pub attributes: &'a Value,
    pub kill_switch: Option<&'a KillSwitchOverrides>,
    /// When true, suppress warnings for missing local YAML rows (SaaS / remote AST).
    pub saas_mode: bool,
    /// When true, populate [`ExplainTrace::rule_trace`] for every artifact rule.
    pub include_rule_trace: bool,
}

/// Evaluate a flag and return a structured trace (kill switch → artifact rules → trailing default).
pub fn explain_flag(request: ExplainRequest<'_>) -> Result<ExplainTrace, ExplainError> {
    let flag_index = find_flag_index(request.artifact, request.flag).ok_or_else(|| {
        ExplainError::FlagNotInArtifact {
            flag: request.flag.to_string(),
        }
    })?;

    let mut warnings = Vec::new();

    let catalog_rules = catalog_rules_for_flag(
        request.catalog,
        request.imports,
        request.environment,
        request.flag,
    );

    let imported = request.sdk_flag.is_imported;
    let deprecated = request.sdk_flag.lifecycle == crate::catalog::FlagLifecycle::Deprecated;

    if let Some(kill_switch) = request.kill_switch {
        if let Some(&value) = kill_switch.get(request.flag) {
            return Ok(ExplainTrace {
                environment: request.artifact.environment.clone(),
                flag: request.flag.to_string(),
                layer: ExplainLayer::KillSwitch,
                value,
                rule_index: None,
                catalog_rule: None,
                imported,
                deprecated,
                rollout_bucket: None,
                missing_id: false,
                warnings,
                rule_trace: Vec::new(),
            });
        }
    }

    let attrs = EvaluationAttributes {
        attributes: request.attributes,
    };

    let (matched_rule_index, raw_value, rule_trace) = if request.include_rule_trace {
        let rule_trace = build_rule_trace(
            request.artifact,
            flag_index,
            &attrs,
            &catalog_rules,
            request.saas_mode,
        );
        let matched_rule_index = rule_trace.iter().position(|r| r.matched);
        let raw_value = matched_rule_index.and_then(|i| rule_trace[i].value.clone());
        (matched_rule_index, raw_value, rule_trace)
    } else {
        let (matched_rule_index, raw_value) = evaluate_flag(request.artifact, flag_index, &attrs);
        (matched_rule_index, raw_value, Vec::new())
    };

    let matched_rule_index = matched_rule_index.ok_or_else(|| ExplainError::NoRuleMatched {
        flag: request.flag.to_string(),
    })?;

    let value = payload_to_bool(&raw_value).unwrap_or(request.sdk_flag.default);
    let is_default_rule =
        is_compiled_catalog_default(request.artifact, flag_index, matched_rule_index);
    let layer = if is_default_rule {
        ExplainLayer::CatalogDefault
    } else {
        ExplainLayer::EnvironmentRule
    };

    let catalog_rule =
        catalog_rule_for_ast_index(&catalog_rules, matched_rule_index, is_default_rule);

    if let Some(msg) = catalog_metadata_warning(
        &catalog_rules,
        matched_rule_index,
        is_default_rule,
        request.saas_mode,
        request.environment,
    ) {
        warnings.push(msg);
    }

    let rollout_rule = catalog_rule.as_ref().is_some_and(|r| r.rollout.is_some());
    let missing_id = rollout_rule && user_id(request.attributes).is_none();

    Ok(ExplainTrace {
        environment: request.artifact.environment.clone(),
        flag: request.flag.to_string(),
        layer,
        value,
        rule_index: Some(matched_rule_index),
        catalog_rule,
        imported,
        deprecated,
        rollout_bucket: if rollout_rule {
            rollout_bucket(request.attributes)
        } else {
            None
        },
        missing_id,
        warnings,
        rule_trace,
    })
}

fn build_rule_trace(
    artifact: &Artifact,
    flag_index: usize,
    attrs: &EvaluationAttributes<'_>,
    catalog_rules: &[CatalogRuleRow],
    saas_mode: bool,
) -> Vec<ExplainRuleTrace> {
    let rules = match artifact.flags.get(flag_index) {
        Some(r) => r,
        None => return Vec::new(),
    };

    rules
        .iter()
        .enumerate()
        .map(|(index, rule)| {
            let eval = evaluate_rule(rule, artifact, attrs.attributes);
            let catalog_reason = catalog_rules.get(index).and_then(|r| r.reason.clone());
            let catalog_note = if catalog_reason.is_some() {
                None
            } else {
                trace_catalog_note(catalog_rules, index, flag_index, artifact, saas_mode)
                    .map(str::to_string)
            };
            ExplainRuleTrace {
                rule_index: index,
                matched: eval.matched,
                evaluation_reason: eval.reason,
                catalog_reason,
                catalog_note,
                value: eval.value,
            }
        })
        .collect()
}

fn catalog_rules_for_flag(
    catalog: &CatalogDocument,
    imports: &BTreeMap<String, CatalogDocument>,
    environment: &str,
    qualified_name: &str,
) -> Vec<CatalogRuleRow> {
    if let Some((namespace, flag_key)) = qualified_name.split_once('.') {
        return imports
            .get(namespace)
            .and_then(|doc| doc.environments.get(environment))
            .and_then(|env| env.rules.get(flag_key))
            .cloned()
            .unwrap_or_default();
    }

    catalog
        .environments
        .get(environment)
        .and_then(|env| env.rules.get(qualified_name))
        .cloned()
        .unwrap_or_default()
}

fn catalog_rule_for_ast_index(
    catalog_rules: &[CatalogRuleRow],
    ast_rule_index: usize,
    is_compiled_default: bool,
) -> Option<CatalogRuleRow> {
    if is_compiled_default {
        return None;
    }
    catalog_rules.get(ast_rule_index).cloned()
}

fn catalog_metadata_warning(
    catalog_rules: &[CatalogRuleRow],
    ast_rule_index: usize,
    is_compiled_default: bool,
    saas_mode: bool,
    environment: &str,
) -> Option<String> {
    if is_compiled_default || catalog_rules.get(ast_rule_index).is_some() {
        return None;
    }
    if saas_mode && catalog_rules.is_empty() {
        return None;
    }
    if catalog_rules.is_empty() {
        return Some(format!(
            "No environment rules for this flag in control-path.yaml for '{environment}'; when/reason/rollout metadata unavailable"
        ));
    }
    Some(format!(
        "No catalog metadata for AST rule {} (catalog lists {} rule(s) for this flag; recompile or check --env)",
        ast_rule_index + 1,
        catalog_rules.len()
    ))
}

fn trace_catalog_note(
    catalog_rules: &[CatalogRuleRow],
    ast_rule_index: usize,
    flag_index: usize,
    artifact: &Artifact,
    saas_mode: bool,
) -> Option<&'static str> {
    if is_compiled_catalog_default(artifact, flag_index, ast_rule_index) {
        return Some("compiled catalog default (no YAML row)");
    }
    if catalog_rules.get(ast_rule_index).is_some() {
        return None;
    }
    if saas_mode && catalog_rules.is_empty() {
        return Some("no local YAML rules (SaaS / remote AST)");
    }
    if catalog_rules.is_empty() {
        return Some("no local YAML rules for this flag/env");
    }
    Some("catalog metadata missing for this AST index")
}

/// Trailing AST rule appended at compile time from catalog `default`.
pub fn is_compiled_catalog_default(
    artifact: &Artifact,
    flag_index: usize,
    rule_index: usize,
) -> bool {
    artifact
        .flags
        .get(flag_index)
        .is_some_and(|rules| !rules.is_empty() && rule_index == rules.len() - 1)
}

fn payload_to_bool(value: &Option<Value>) -> Option<bool> {
    match value {
        Some(Value::Bool(b)) => Some(*b),
        Some(Value::String(s)) => match s.to_ascii_uppercase().as_str() {
            "ON" | "TRUE" => Some(true),
            "OFF" | "FALSE" => Some(false),
            _ => None,
        },
        Some(Value::Number(n)) => n.as_i64().map(|i| i != 0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Rule, ServePayload};
    use crate::catalog::{
        build_sdk_catalog, compile_catalog, compile_catalog_with_imports, parse_catalog,
    };
    use crate::runtime::{find_flag_index, rollout_bucket};
    use serde_json::json;

    const LOCAL_ONLY: &str =
        include_str!("../../../../schemas/examples/local-only.control-path.yaml");
    const SHARED_PLATFORM: &str =
        include_str!("../../../../schemas/examples/shared-platform.control-path.yaml");
    const IMPORTED_GLOBAL: &str =
        include_str!("../../../../schemas/examples/imported-global.control-path.yaml");

    fn imported_global_fixture() -> (CatalogDocument, BTreeMap<String, CatalogDocument>) {
        let catalog =
            parse_catalog(IMPORTED_GLOBAL, Some("imported-global.control-path.yaml")).unwrap();
        let platform =
            parse_catalog(SHARED_PLATFORM, Some("shared-platform.control-path.yaml")).unwrap();
        let mut imports = BTreeMap::new();
        imports.insert("platform".to_string(), platform);
        (catalog, imports)
    }

    fn sdk_flag<'a>(sdk: &'a crate::catalog::SdkCatalog, name: &str) -> &'a SdkFlag {
        sdk.flags
            .iter()
            .find(|f| f.qualified_name == name)
            .unwrap_or_else(|| panic!("sdk flag {name} not found"))
    }

    #[test]
    fn explain_kill_switch_overrides_artifact_rules() {
        let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
        let artifact = compile_catalog(&catalog, "production").unwrap();
        let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();
        let flag = sdk_flag(&sdk, "new_dashboard");

        let mut kill_switch = KillSwitchOverrides::new();
        kill_switch.insert("new_dashboard".to_string(), false);

        let user = json!({ "id": "user-1", "plan": "premium" });
        let trace = explain_flag(ExplainRequest {
            artifact: &artifact,
            flag: "new_dashboard",
            environment: "production",
            catalog: &catalog,
            imports: &BTreeMap::new(),
            sdk_flag: flag,
            attributes: &user,
            kill_switch: Some(&kill_switch),
            saas_mode: false,
            include_rule_trace: false,
        })
        .unwrap();

        assert_eq!(trace.layer, ExplainLayer::KillSwitch);
        assert!(!trace.value);
        assert!(trace.rule_index.is_none());
        assert!(trace.rule_trace.is_empty());
    }

    #[test]
    fn explain_matches_first_environment_rule_in_artifact() {
        let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
        let artifact = compile_catalog(&catalog, "staging").unwrap();
        let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();
        let flag = sdk_flag(&sdk, "new_dashboard");

        let user = json!({ "id": "user-1" });
        let trace = explain_flag(ExplainRequest {
            artifact: &artifact,
            flag: "new_dashboard",
            environment: "staging",
            catalog: &catalog,
            imports: &BTreeMap::new(),
            sdk_flag: flag,
            attributes: &user,
            kill_switch: None,
            saas_mode: false,
            include_rule_trace: false,
        })
        .unwrap();

        assert_eq!(trace.layer, ExplainLayer::EnvironmentRule);
        assert!(trace.value);
        assert_eq!(trace.rule_index, Some(0));
        assert!(trace.catalog_rule.is_some());
    }

    #[test]
    fn explain_falls_back_to_compiled_catalog_default() {
        let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
        let artifact = compile_catalog(&catalog, "staging").unwrap();
        let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();
        let flag = sdk_flag(&sdk, "premium_checkout");

        let user = json!({ "id": "user-1" });
        let trace = explain_flag(ExplainRequest {
            artifact: &artifact,
            flag: "premium_checkout",
            environment: "staging",
            catalog: &catalog,
            imports: &BTreeMap::new(),
            sdk_flag: flag,
            attributes: &user,
            kill_switch: None,
            saas_mode: false,
            include_rule_trace: false,
        })
        .unwrap();

        assert_eq!(trace.layer, ExplainLayer::CatalogDefault);
        assert!(!trace.value);
        assert_eq!(trace.rule_index, Some(0));
        assert!(trace.catalog_rule.is_none());
    }

    #[test]
    fn explain_rule_trace_includes_catalog_reason_from_yaml() {
        let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
        let artifact = compile_catalog(&catalog, "production").unwrap();
        let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();
        let flag = sdk_flag(&sdk, "new_dashboard");

        let user = json!({ "id": "user-1", "plan": "premium" });
        let trace = explain_flag(ExplainRequest {
            artifact: &artifact,
            flag: "new_dashboard",
            environment: "production",
            catalog: &catalog,
            imports: &BTreeMap::new(),
            sdk_flag: flag,
            attributes: &user,
            kill_switch: None,
            saas_mode: false,
            include_rule_trace: true,
        })
        .unwrap();

        let rollout_trace = trace
            .rule_trace
            .iter()
            .find(|r| r.catalog_reason.as_deref() == Some("Gradual rollout after beta validation"))
            .expect("rollout rule with catalog reason in trace");
        assert!(!rollout_trace.matched || trace.rule_index == Some(rollout_trace.rule_index));
        assert_eq!(
            rollout_trace.catalog_reason.as_deref(),
            Some("Gradual rollout after beta validation")
        );
    }

    #[test]
    fn explain_rollout_match_sets_bucket_and_missing_user_id_when_no_id() {
        const ROLLOUT_ONLY: &str = r#"
catalog:
  id: rollout-svc
mode: local
flags:
  feat:
    default: false
    kind: release
environments:
  production:
    rules:
      feat:
        - rollout:
            percentage: 100
            serve: true
"#;
        let catalog = parse_catalog(ROLLOUT_ONLY, Some("rollout.yaml")).unwrap();
        let artifact = compile_catalog(&catalog, "production").unwrap();
        let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();
        let flag = sdk_flag(&sdk, "feat");

        let with_id = json!({ "id": "rollout-user-1" });
        let trace = explain_flag(ExplainRequest {
            artifact: &artifact,
            flag: "feat",
            environment: "production",
            catalog: &catalog,
            imports: &BTreeMap::new(),
            sdk_flag: flag,
            attributes: &with_id,
            kill_switch: None,
            saas_mode: false,
            include_rule_trace: false,
        })
        .unwrap();

        assert_eq!(trace.layer, ExplainLayer::EnvironmentRule);
        assert_eq!(trace.rule_index, Some(0));
        assert!(trace.value);
        assert!(trace.catalog_rule.as_ref().unwrap().rollout.is_some());
        assert_eq!(trace.rollout_bucket, rollout_bucket(&with_id));
        assert!(!trace.missing_id);

        let without_id = json!({ "segment": "anon" });
        let trace = explain_flag(ExplainRequest {
            artifact: &artifact,
            flag: "feat",
            environment: "production",
            catalog: &catalog,
            imports: &BTreeMap::new(),
            sdk_flag: flag,
            attributes: &without_id,
            kill_switch: None,
            saas_mode: false,
            include_rule_trace: false,
        })
        .unwrap();

        assert!(trace.missing_id);
        assert!(trace.rollout_bucket.is_none());
    }

    #[test]
    fn explain_imported_flag_uses_namespace_catalog_rules() {
        let (catalog, imports) = imported_global_fixture();
        let artifact = compile_catalog_with_imports(&catalog, &imports, "production").unwrap();
        let sdk = build_sdk_catalog(&catalog, &imports).unwrap();
        let flag = sdk_flag(&sdk, "platform.emergency_kill_switch");

        let trace = explain_flag(ExplainRequest {
            artifact: &artifact,
            flag: "platform.emergency_kill_switch",
            environment: "production",
            catalog: &catalog,
            imports: &imports,
            sdk_flag: flag,
            attributes: &json!({ "id": "u1" }),
            kill_switch: None,
            saas_mode: false,
            include_rule_trace: false,
        })
        .unwrap();

        assert_eq!(trace.layer, ExplainLayer::EnvironmentRule);
        assert_eq!(
            trace
                .catalog_rule
                .as_ref()
                .and_then(|r| r.reason.as_deref()),
            Some("Default off; enable only during incidents via platform catalog")
        );
    }

    #[test]
    fn explain_imported_flag_matches_namespaced_runtime_attributes() {
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
        let sdk = build_sdk_catalog(&catalog, &imports).unwrap();
        let flag = sdk_flag(&sdk, "platform.emergency_kill_switch");

        let trace = explain_flag(ExplainRequest {
            artifact: &artifact,
            flag: "platform.emergency_kill_switch",
            environment: "production",
            catalog: &catalog,
            imports: &imports,
            sdk_flag: flag,
            attributes: &json!({ "id": "u1", "platform": { "org_tier": "gold" } }),
            kill_switch: None,
            saas_mode: false,
            include_rule_trace: false,
        })
        .unwrap();

        assert_eq!(trace.layer, ExplainLayer::EnvironmentRule);
        assert!(trace.value);

        let miss = explain_flag(ExplainRequest {
            artifact: &artifact,
            flag: "platform.emergency_kill_switch",
            environment: "production",
            catalog: &catalog,
            imports: &imports,
            sdk_flag: flag,
            attributes: &json!({ "id": "u1", "platform": { "org_tier": "silver" } }),
            kill_switch: None,
            saas_mode: false,
            include_rule_trace: false,
        })
        .unwrap();

        assert_eq!(miss.layer, ExplainLayer::CatalogDefault);
        assert!(!miss.value);
    }

    #[test]
    fn catalog_metadata_warning_when_ast_index_has_no_yaml_row() {
        let rules = vec![CatalogRuleRow {
            when: None,
            serve: Some(true),
            rollout: None,
            reason: None,
        }];
        let msg = catalog_metadata_warning(&rules, 1, false, false, "production").unwrap();
        assert!(msg.contains("AST rule 2"));
        assert!(msg.contains("1 rule"));
    }

    #[test]
    fn explain_surfaces_metadata_warning_for_stale_artifact_index() {
        const STALE_FIXTURE: &str = r#"
catalog:
  id: stale-svc
mode: local
flags:
  feature:
    default: false
    kind: release
environments:
  production:
    rules:
      feature:
        - when: "user.plan == 'never'"
          serve: true
"#;
        let catalog = parse_catalog(STALE_FIXTURE, Some("stale.yaml")).unwrap();
        let mut artifact = compile_catalog(&catalog, "production").unwrap();
        let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();
        let flag = sdk_flag(&sdk, "feature");

        let flag_index = find_flag_index(&artifact, "feature").unwrap();
        let orphan_index = artifact.flags[flag_index].len() - 1;
        artifact.flags[flag_index].insert(
            orphan_index,
            Rule::ServeWithoutWhen(ServePayload::Number(0)),
        );

        let trace = explain_flag(ExplainRequest {
            artifact: &artifact,
            flag: "feature",
            environment: "production",
            catalog: &catalog,
            imports: &BTreeMap::new(),
            sdk_flag: flag,
            attributes: &json!({ "id": "any" }),
            kill_switch: None,
            saas_mode: false,
            include_rule_trace: false,
        })
        .unwrap();

        assert_eq!(trace.rule_index, Some(orphan_index));
        assert!(trace
            .warnings
            .iter()
            .any(|w| w.contains("No catalog metadata for AST rule 2")));
    }

    #[test]
    fn explain_with_and_without_rule_trace_agree_on_outcome() {
        let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
        let artifact = compile_catalog(&catalog, "production").unwrap();
        let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();
        let flag = sdk_flag(&sdk, "new_dashboard");

        let base = ExplainRequest {
            artifact: &artifact,
            flag: "new_dashboard",
            environment: "production",
            catalog: &catalog,
            imports: &BTreeMap::new(),
            sdk_flag: flag,
            attributes: &json!({ "id": "equiv-user", "plan": "standard" }),
            kill_switch: None,
            saas_mode: false,
            include_rule_trace: false,
        };

        let without_trace = explain_flag(base.clone()).unwrap();
        let with_trace = explain_flag(ExplainRequest {
            include_rule_trace: true,
            ..base
        })
        .unwrap();

        assert_eq!(without_trace.layer, with_trace.layer);
        assert_eq!(without_trace.rule_index, with_trace.rule_index);
        assert_eq!(without_trace.value, with_trace.value);
        assert!(!with_trace.rule_trace.is_empty());
    }

    #[test]
    fn compiled_catalog_default_is_trailing_ast_rule() {
        let catalog = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
        let artifact = compile_catalog(&catalog, "staging").unwrap();
        let flag_index = find_flag_index(&artifact, "new_dashboard").unwrap();
        let last = artifact.flags[flag_index].len() - 1;

        assert!(!is_compiled_catalog_default(&artifact, flag_index, 0));
        assert!(is_compiled_catalog_default(&artifact, flag_index, last));
    }

    #[test]
    fn explain_saas_skips_metadata_warning_for_remote_ast_without_local_rules() {
        let local = parse_catalog(LOCAL_ONLY, Some("local-only.control-path.yaml")).unwrap();
        let artifact = compile_catalog(&local, "production").unwrap();
        let mut catalog = local.clone();
        catalog.mode = crate::catalog::CatalogMode::Saas;
        catalog.environments.clear();

        let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();
        let flag = sdk_flag(&sdk, "new_dashboard");
        let user = json!({ "id": "u1", "plan": "beta" });

        let saas_trace = explain_flag(ExplainRequest {
            artifact: &artifact,
            flag: "new_dashboard",
            environment: "production",
            catalog: &catalog,
            imports: &BTreeMap::new(),
            sdk_flag: flag,
            attributes: &user,
            kill_switch: None,
            saas_mode: true,
            include_rule_trace: false,
        })
        .unwrap();

        assert_eq!(saas_trace.rule_index, Some(0));
        assert_eq!(saas_trace.layer, ExplainLayer::EnvironmentRule);
        assert!(saas_trace.catalog_rule.is_none());
        assert!(!saas_trace
            .warnings
            .iter()
            .any(|w| w.contains("No catalog metadata")));

        let mut thin_catalog = local.clone();
        thin_catalog
            .environments
            .get_mut("production")
            .unwrap()
            .rules
            .remove("new_dashboard");

        let local_trace = explain_flag(ExplainRequest {
            artifact: &artifact,
            flag: "new_dashboard",
            environment: "production",
            catalog: &thin_catalog,
            imports: &BTreeMap::new(),
            sdk_flag: flag,
            attributes: &user,
            kill_switch: None,
            saas_mode: false,
            include_rule_trace: false,
        })
        .unwrap();

        assert!(local_trace
            .warnings
            .iter()
            .any(|w| w.contains("No environment rules for this flag")));
    }
}
