//! Minimal `explain` command: boolean evaluation via shared compiler runtime semantics.

use crate::error::{CliError, CliResult};
use crate::utils::{catalog, kill_switch, runtime};
use controlpath_compiler::ast::Artifact;
use controlpath_compiler::catalog::model::{
    CatalogDocument, CatalogMode, FlagLifecycle, Rule as CatalogRule,
};
use controlpath_compiler::{
    evaluate_flag, evaluate_rule, find_flag_index, rollout_bucket, user_id, EvaluationAttributes,
};
use rmp_serde::from_slice;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Options {
    pub flag: String,
    pub user: Option<String>,
    pub context: Option<String>,
    pub env: Option<String>,
    pub trace: bool,
    pub ast: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MatchedLayer {
    KillSwitch,
    EnvironmentRule,
    CatalogDefault,
}

#[derive(Debug, Clone)]
struct ExplainOutcome {
    environment: String,
    layer: MatchedLayer,
    value: bool,
    rule_index: Option<usize>,
    catalog_rule: Option<CatalogRule>,
    imported: bool,
    deprecated: bool,
    rollout_bucket: Option<u32>,
    missing_user_id: bool,
    warnings: Vec<String>,
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(outcome) => {
            if runtime::is_json_output() {
                print_json(options, &outcome);
            } else {
                print_human(options, &outcome);
            }
            0
        }
        Err(e) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    json!({
                        "status": "error",
                        "command": "explain",
                        "flag": options.flag,
                        "warnings": [],
                        "errors": [e.to_string()]
                    })
                );
            } else {
                eprintln!("✗ Explanation failed");
                eprintln!("  Error: {e}");
            }
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<ExplainOutcome> {
    let base_dir = std::env::current_dir()
        .map_err(|e| CliError::Message(format!("Failed to resolve working directory: {e}")))?;

    let environment = resolve_environment(options)?;
    let ast_path = determine_ast_path(options, &environment)?;
    let artifact = load_artifact(&ast_path)?;
    let mut warnings = env_ast_warnings(options, &environment, &artifact);

    let flag_index = find_flag_index(&artifact, &options.flag).ok_or_else(|| {
        CliError::Message(format!(
            "Flag '{}' not found in artifact {}",
            options.flag,
            ast_path.display()
        ))
    })?;

    let bundle = catalog::load_for_explain(&base_dir)?;
    let sdk_flag = bundle
        .sdk
        .flags
        .iter()
        .find(|f| f.qualified_name == options.flag)
        .ok_or_else(|| {
            CliError::Message(format!(
                "Flag '{}' not found in control-path.yaml catalog",
                options.flag
            ))
        })?;

    let catalog_rules = catalog_rules_for_flag(
        &bundle.catalog,
        &bundle.imports,
        &environment,
        &options.flag,
    );

    let user_json = if let Some(user_input) = &options.user {
        parse_json_or_file(user_input, "--user")?
    } else {
        Value::Object(serde_json::Map::new())
    };
    let context_json = if let Some(context_input) = &options.context {
        Some(parse_json_or_file(context_input, "--context")?)
    } else {
        None
    };

    let kill_path = kill_switch::kill_switch_path(&environment);
    let kill_file = kill_switch::read_kill_switch_file(&kill_path)?;
    if let Some(kill_value) = kill_switch_value(&kill_file, &options.flag) {
        return Ok(ExplainOutcome {
            environment: artifact.environment.clone(),
            layer: MatchedLayer::KillSwitch,
            value: kill_value,
            rule_index: None,
            catalog_rule: None,
            imported: sdk_flag.is_imported,
            deprecated: sdk_flag.lifecycle == FlagLifecycle::Deprecated,
            rollout_bucket: None,
            missing_user_id: false,
            warnings,
        });
    }

    let attrs = EvaluationAttributes {
        user: &user_json,
        context: context_json.as_ref(),
    };

    if options.trace && !runtime::is_json_output() {
        print_trace_header(options, &artifact, &user_json);
        trace_rules(
            &artifact,
            flag_index,
            &attrs,
            &catalog_rules,
            bundle.catalog.mode == CatalogMode::Saas,
        );
    }

    let (matched_rule_index, raw_value) = evaluate_flag(&artifact, flag_index, &attrs);
    let matched_rule_index = matched_rule_index.ok_or_else(|| {
        CliError::Message(format!(
            "No rules matched for flag '{}' (artifact may be corrupt)",
            options.flag
        ))
    })?;

    let value = payload_to_bool(&raw_value).unwrap_or(sdk_flag.default);
    let is_default_rule = is_compiled_catalog_default(&artifact, flag_index, matched_rule_index);
    let layer = if is_default_rule {
        MatchedLayer::CatalogDefault
    } else {
        MatchedLayer::EnvironmentRule
    };

    let catalog_rule =
        catalog_rule_for_ast_index(&catalog_rules, matched_rule_index, is_default_rule);

    if let Some(msg) = catalog_metadata_warning(
        &catalog_rules,
        matched_rule_index,
        is_default_rule,
        bundle.catalog.mode == CatalogMode::Saas,
        &environment,
    ) {
        warnings.push(msg);
    }

    let rollout_rule = catalog_rule.as_ref().is_some_and(|r| r.rollout.is_some());
    let missing_user_id = rollout_rule && user_id(&user_json).is_none();

    Ok(ExplainOutcome {
        environment: artifact.environment.clone(),
        layer,
        value,
        rule_index: Some(matched_rule_index),
        catalog_rule,
        imported: sdk_flag.is_imported,
        deprecated: sdk_flag.lifecycle == FlagLifecycle::Deprecated,
        rollout_bucket: if rollout_rule {
            rollout_bucket(&user_json)
        } else {
            None
        },
        missing_user_id,
        warnings,
    })
}

/// Map AST rule index → catalog YAML row.
///
/// Correct when the artifact was built from the same catalog and `--env` as explain.
/// Trailing AST rules are compiled defaults (no YAML row). SaaS / stale artifacts may
/// have no row — see [`catalog_metadata_warning`] and trace `Catalog:` notes.
fn catalog_rule_for_ast_index(
    catalog_rules: &[CatalogRule],
    ast_rule_index: usize,
    is_compiled_default: bool,
) -> Option<CatalogRule> {
    if is_compiled_default {
        return None;
    }
    catalog_rules.get(ast_rule_index).cloned()
}

/// Warn when a non-default AST rule has no matching catalog row (stale artifact / wrong env).
/// SaaS mode with no local `environments.rules` is expected — no warning.
fn catalog_metadata_warning(
    catalog_rules: &[CatalogRule],
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

fn env_ast_warnings(options: &Options, environment: &str, artifact: &Artifact) -> Vec<String> {
    let mut warnings = Vec::new();

    if let (Some(env), Some(ast)) = (&options.env, &options.ast) {
        let stem = Path::new(ast)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if !stem.is_empty() && stem != env.as_str() {
            warnings.push(format!(
                "--env '{env}' does not match --ast path stem '{stem}'; kill-switch path and catalog metadata use --env, evaluation uses {}",
                ast
            ));
        }
    }

    if artifact.environment != environment {
        warnings.push(format!(
            "AST artifact env is '{}' but explain resolved env is '{}' (check --env and --ast)",
            artifact.environment, environment
        ));
    }

    warnings
}

fn trace_catalog_note(
    catalog_rules: &[CatalogRule],
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

fn resolve_environment(options: &Options) -> CliResult<String> {
    if let Some(env) = &options.env {
        return kill_switch::resolve_kill_switch_env(Some(env));
    }
    if let Some(ast) = &options.ast {
        let stem = Path::new(ast)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("production");
        return kill_switch::resolve_kill_switch_env(Some(stem));
    }
    kill_switch::resolve_kill_switch_env(None)
}

/// The compiler appends a trailing serve rule from catalog `default` (see `compile_catalog`).
fn is_compiled_catalog_default(artifact: &Artifact, flag_index: usize, rule_index: usize) -> bool {
    artifact
        .flags
        .get(flag_index)
        .is_some_and(|rules| !rules.is_empty() && rule_index == rules.len() - 1)
}

fn determine_ast_path(options: &Options, environment: &str) -> CliResult<PathBuf> {
    if let Some(ast) = &options.ast {
        return Ok(PathBuf::from(ast));
    }
    if options.env.is_some() || options.ast.is_none() {
        let path = PathBuf::from(format!(".controlpath/{environment}.ast"));
        if path.exists() {
            return Ok(path);
        }
    }

    let ast_dir = PathBuf::from(".controlpath");
    let entries = fs::read_dir(&ast_dir).map_err(|e| {
        CliError::Message(format!(
            "Either --ast <file> or --env <env> is required, and AST auto-detection failed to read {}: {e}",
            ast_dir.display()
        ))
    })?;

    let mut ast_files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("ast") {
            ast_files.push(path);
        }
    }
    ast_files.sort();

    match ast_files.len() {
        1 => Ok(ast_files.remove(0)),
        0 => Err(CliError::Message(
            "Either --ast <file> or --env <env> must be provided (no .ast files found in .controlpath/)"
                .to_string(),
        )),
        _ => Err(CliError::Message(
            "Either --ast <file> or --env <env> must be provided (multiple .ast files found in .controlpath/)"
                .to_string(),
        )),
    }
}

fn load_artifact(path: &Path) -> CliResult<Artifact> {
    let ast_bytes =
        fs::read(path).map_err(|e| CliError::Message(format!("Failed to read AST file: {e}")))?;
    from_slice(&ast_bytes).map_err(|e| CliError::Message(format!("Failed to deserialize AST: {e}")))
}

fn kill_switch_value(kill_file: &Value, flag: &str) -> Option<bool> {
    kill_file.get("flags")?.get(flag)?.as_bool()
}

fn catalog_rules_for_flag(
    catalog: &CatalogDocument,
    imports: &BTreeMap<String, CatalogDocument>,
    environment: &str,
    qualified_name: &str,
) -> Vec<CatalogRule> {
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

fn layer_label(layer: &MatchedLayer) -> &'static str {
    match layer {
        MatchedLayer::KillSwitch => "kill switch file",
        MatchedLayer::EnvironmentRule => "environment rule",
        MatchedLayer::CatalogDefault => "catalog default",
    }
}

fn print_human(options: &Options, outcome: &ExplainOutcome) {
    println!("Flag: {}", options.flag);
    println!("Environment: {}", outcome.environment);
    if outcome.imported {
        println!("Source: imported catalog projection");
    }
    if outcome.deprecated {
        println!("Warning: flag lifecycle is deprecated");
    }
    for warning in &outcome.warnings {
        println!("Warning: {warning}");
    }
    println!();
    println!("Result:");
    println!("  Layer: {}", layer_label(&outcome.layer));
    println!("  Value: {}", outcome.value);
    if let Some(idx) = outcome.rule_index {
        if outcome.layer == MatchedLayer::EnvironmentRule {
            println!("  Rule: {} (1-based in environment)", idx + 1);
        } else if outcome.layer == MatchedLayer::CatalogDefault {
            println!(
                "  Rule: catalog default (compiled trailing rule {})",
                idx + 1
            );
        }
    }
    if let Some(rule) = &outcome.catalog_rule {
        if let Some(when) = &rule.when {
            println!("  When: {when}");
        }
        if let Some(rollout) = &rule.rollout {
            println!(
                "  Rollout: {}% → serve {}",
                rollout.percentage, rollout.serve
            );
            if let Some(bucket) = outcome.rollout_bucket {
                println!("  Rollout bucket: {bucket} (0–99, user id hash)");
            }
        }
        if let Some(reason) = &rule.reason {
            println!("  Reason: {reason}");
        }
    }
    if outcome.missing_user_id {
        println!();
        println!("  ⚠ Missing user.id — rollout rules need a stable identity for bucketing");
    }
    println!();
}

fn print_json(options: &Options, outcome: &ExplainOutcome) {
    let mut body = json!({
        "status": "ok",
        "command": "explain",
        "flag": options.flag,
        "environment": outcome.environment,
        "layer": layer_label(&outcome.layer),
        "value": outcome.value,
        "imported": outcome.imported,
        "deprecated": outcome.deprecated,
        "matchedRule": outcome.rule_index.map(|i| i + 1),
        "warnings": [],
        "errors": []
    });
    let warnings = body["warnings"].as_array_mut().unwrap();
    if outcome.deprecated {
        warnings.push(json!("flag lifecycle is deprecated"));
    }
    for warning in &outcome.warnings {
        warnings.push(json!(warning));
    }
    if outcome.missing_user_id {
        warnings.push(json!("missing user.id for rollout bucketing"));
    }
    if let Some(rule) = &outcome.catalog_rule {
        body["catalogRule"] = json!({
            "when": rule.when,
            "serve": rule.serve,
            "rollout": rule.rollout,
            "reason": rule.reason,
        });
    }
    if let Some(bucket) = outcome.rollout_bucket {
        body["rolloutBucket"] = json!(bucket);
    }
    println!("{body}");
}

fn print_trace_header(options: &Options, artifact: &Artifact, user: &Value) {
    println!("Flag: {}", options.flag);
    println!("Environment: {}", artifact.environment);
    if let Some(id) = user.get("id") {
        println!("User ID: {id}");
    }
    println!();
    println!("Rule trace:");
}

fn trace_rules(
    artifact: &Artifact,
    flag_index: usize,
    attrs: &EvaluationAttributes<'_>,
    catalog_rules: &[CatalogRule],
    saas_mode: bool,
) {
    let context_owned = attrs.context.cloned();
    let rules = &artifact.flags[flag_index];
    for (index, rule) in rules.iter().enumerate() {
        let eval = evaluate_rule(rule, artifact, attrs.user, &context_owned);
        let catalog_hint = catalog_rules.get(index).and_then(|r| r.reason.as_deref());
        println!(
            "  Rule {}: {}",
            index + 1,
            if eval.matched { "matched" } else { "skipped" }
        );
        println!("    {}", eval.reason);
        if let Some(reason) = catalog_hint {
            println!("    Catalog reason: {reason}");
        } else if let Some(note) =
            trace_catalog_note(catalog_rules, index, flag_index, artifact, saas_mode)
        {
            println!("    Catalog: {note}");
        }
        if let Some(ref val) = eval.value {
            println!("    Value: {val}");
        }
    }
    println!();
}

fn parse_json_or_file(input: &str, input_name: &str) -> CliResult<Value> {
    if let Ok(parsed) = serde_json::from_str::<Value>(input) {
        return Ok(parsed);
    }

    let content = fs::read_to_string(input).map_err(|file_err| {
        CliError::Message(format!(
            "Failed to parse {input_name} as inline JSON and failed to read file '{input}': {file_err}"
        ))
    })?;

    serde_json::from_str(&content).map_err(|parse_err| {
        CliError::Message(format!(
            "Failed to parse {input_name} JSON from file '{input}': {parse_err}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determine_ast_path_prefers_explicit_ast() {
        let options = Options {
            flag: "f".to_string(),
            user: None,
            context: None,
            env: None,
            trace: false,
            ast: Some("custom.ast".to_string()),
        };
        assert_eq!(
            determine_ast_path(&options, "production").unwrap(),
            PathBuf::from("custom.ast")
        );
    }

    #[test]
    fn kill_switch_value_reads_boolean_map() {
        let file = json!({ "version": "2.0", "flags": { "my_flag": true } });
        assert_eq!(kill_switch_value(&file, "my_flag"), Some(true));
        assert_eq!(kill_switch_value(&file, "other"), None);
    }

    #[test]
    fn payload_to_bool_maps_on_off() {
        assert_eq!(payload_to_bool(&Some(json!("ON"))), Some(true));
        assert_eq!(payload_to_bool(&Some(json!("OFF"))), Some(false));
    }

    #[test]
    fn compiled_catalog_default_is_trailing_ast_rule() {
        use controlpath_compiler::ast::{Rule, ServePayload};

        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "production".to_string(),
            string_table: vec!["ON".to_string(), "OFF".to_string()],
            flags: vec![vec![
                Rule::ServeWithoutWhen(ServePayload::Number(0)),
                Rule::ServeWithoutWhen(ServePayload::Number(1)),
            ]],
            flag_names: vec![0],
            segments: None,
            signature: None,
        };
        assert!(!is_compiled_catalog_default(&artifact, 0, 0));
        assert!(is_compiled_catalog_default(&artifact, 0, 1));
    }

    #[test]
    fn catalog_rules_for_imported_flag() {
        let mut imports = BTreeMap::new();
        let platform = CatalogDocument {
            catalog: controlpath_compiler::catalog::model::CatalogIdentity {
                id: "platform".to_string(),
                namespace: None,
            },
            mode: controlpath_compiler::catalog::model::CatalogMode::Local,
            saas: None,
            imports: BTreeMap::new(),
            flags: BTreeMap::new(),
            environments: [(
                "production".to_string(),
                controlpath_compiler::catalog::model::Environment {
                    description: None,
                    rules: [(
                        "emergency_kill_switch".to_string(),
                        vec![CatalogRule {
                            when: None,
                            serve: Some(false),
                            rollout: None,
                            reason: Some("incident default".to_string()),
                        }],
                    )]
                    .into_iter()
                    .collect(),
                },
            )]
            .into_iter()
            .collect(),
            segments: BTreeMap::new(),
            kill_switches: BTreeMap::new(),
            artifacts: BTreeMap::new(),
        };
        imports.insert("platform".to_string(), platform);

        let service = CatalogDocument {
            catalog: controlpath_compiler::catalog::model::CatalogIdentity {
                id: "svc".to_string(),
                namespace: None,
            },
            mode: controlpath_compiler::catalog::model::CatalogMode::Local,
            saas: None,
            imports: BTreeMap::new(),
            flags: BTreeMap::new(),
            environments: BTreeMap::new(),
            segments: BTreeMap::new(),
            kill_switches: BTreeMap::new(),
            artifacts: BTreeMap::new(),
        };

        let rules = catalog_rules_for_flag(
            &service,
            &imports,
            "production",
            "platform.emergency_kill_switch",
        );
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].reason.as_deref(), Some("incident default"));
    }

    #[test]
    fn catalog_metadata_warning_when_rules_out_of_sync() {
        let rules = vec![CatalogRule {
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
    fn catalog_metadata_warning_skipped_for_saas_without_local_rules() {
        assert!(catalog_metadata_warning(&[], 0, false, true, "production").is_none());
    }

    #[test]
    fn env_ast_warnings_when_env_and_ast_stem_differ() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "production".to_string(),
            string_table: vec![],
            flags: vec![vec![]],
            flag_names: vec![0],
            segments: None,
            signature: None,
        };
        let options = Options {
            flag: "f".into(),
            user: None,
            context: None,
            env: Some("staging".into()),
            trace: false,
            ast: Some(".controlpath/production.ast".into()),
        };
        let warnings = env_ast_warnings(&options, "staging", &artifact);
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("--env 'staging'"));
        assert!(warnings[1].contains("artifact env is 'production'"));
    }
}
