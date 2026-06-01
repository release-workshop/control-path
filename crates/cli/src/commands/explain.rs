//! Minimal `explain` command: loads inputs and formats shared compiler explain traces.

use crate::error::{CliError, CliResult};
use crate::utils::{catalog, kill_switch, runtime};
use controlpath_compiler::ast::Artifact;
use controlpath_compiler::catalog::model::CatalogMode;
use controlpath_compiler::{
    explain_flag, find_flag_index, user_id, ExplainError, ExplainLayer, ExplainRequest,
    ExplainTrace, KillSwitchOverrides,
};
use rmp_serde::from_slice;
use serde_json::{json, Value};
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

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok((trace, user)) => {
            if runtime::is_json_output() {
                print_json(options, &trace);
            } else {
                if options.trace {
                    print_trace_header(options, &trace, &user);
                    print_rule_trace(&trace);
                }
                print_human(options, &trace);
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

fn run_inner(options: &Options) -> CliResult<(ExplainTrace, Value)> {
    let base_dir = std::env::current_dir()
        .map_err(|e| CliError::Message(format!("Failed to resolve working directory: {e}")))?;

    let environment = resolve_environment(options)?;
    let ast_path = determine_ast_path(options, &environment)?;
    let artifact = load_artifact(&ast_path)?;

    if find_flag_index(&artifact, &options.flag).is_none() {
        return Err(CliError::Message(format!(
            "Flag '{}' not found in artifact {}",
            options.flag,
            ast_path.display()
        )));
    }

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
    let kill_switch = kill_switch_overrides(&kill_file);

    let mut trace = explain_flag(ExplainRequest {
        artifact: &artifact,
        flag: &options.flag,
        environment: &environment,
        catalog: &bundle.catalog,
        imports: &bundle.imports,
        sdk_flag,
        user: &user_json,
        context: context_json.as_ref(),
        kill_switch: (!kill_switch.is_empty()).then_some(&kill_switch),
        saas_mode: bundle.catalog.mode == CatalogMode::Saas,
        include_rule_trace: options.trace && !runtime::is_json_output(),
    })
    .map_err(|e| map_explain_error(&ast_path, e))?;

    trace
        .warnings
        .extend(env_ast_warnings(options, &environment, &artifact));

    Ok((trace, user_json))
}

fn map_explain_error(ast_path: &Path, err: ExplainError) -> CliError {
    match err {
        ExplainError::FlagNotInArtifact { flag } => CliError::Message(format!(
            "Flag '{flag}' not found in artifact {}",
            ast_path.display()
        )),
        ExplainError::NoRuleMatched { flag } => CliError::Message(format!(
            "No rules matched for flag '{flag}' in artifact {} (artifact may be corrupt)",
            ast_path.display()
        )),
    }
}

fn kill_switch_overrides(kill_file: &Value) -> KillSwitchOverrides {
    let mut overrides = KillSwitchOverrides::new();
    if let Some(flags) = kill_file.get("flags").and_then(Value::as_object) {
        for (name, value) in flags {
            if let Some(b) = value.as_bool() {
                overrides.insert(name.clone(), b);
            }
        }
    }
    overrides
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

fn layer_label(layer: ExplainLayer) -> &'static str {
    match layer {
        ExplainLayer::KillSwitch => "kill switch file",
        ExplainLayer::EnvironmentRule => "environment rule",
        ExplainLayer::CatalogDefault => "catalog default",
    }
}

fn print_human(options: &Options, trace: &ExplainTrace) {
    println!("Flag: {}", options.flag);
    println!("Environment: {}", trace.environment);
    if trace.imported {
        println!("Source: imported catalog projection");
    }
    if trace.deprecated {
        println!("Warning: flag lifecycle is deprecated");
    }
    for warning in &trace.warnings {
        println!("Warning: {warning}");
    }
    println!();
    println!("Result:");
    println!("  Layer: {}", layer_label(trace.layer));
    println!("  Value: {}", trace.value);
    if let Some(idx) = trace.rule_index {
        if trace.layer == ExplainLayer::EnvironmentRule {
            println!("  Rule: {} (1-based in environment)", idx + 1);
        } else if trace.layer == ExplainLayer::CatalogDefault {
            println!(
                "  Rule: catalog default (compiled trailing rule {})",
                idx + 1
            );
        }
    }
    if let Some(rule) = &trace.catalog_rule {
        if let Some(when) = &rule.when {
            println!("  When: {when}");
        }
        if let Some(rollout) = &rule.rollout {
            println!(
                "  Rollout: {}% → serve {}",
                rollout.percentage, rollout.serve
            );
            if let Some(bucket) = trace.rollout_bucket {
                println!("  Rollout bucket: {bucket} (0–99, user id hash)");
            }
        }
        if let Some(reason) = &rule.reason {
            println!("  Reason: {reason}");
        }
    }
    if trace.missing_user_id {
        println!();
        println!("  ⚠ Missing user.id — rollout rules need a stable identity for bucketing");
    }
    println!();
}

fn print_json(options: &Options, trace: &ExplainTrace) {
    let mut body = json!({
        "status": "ok",
        "command": "explain",
        "flag": options.flag,
        "environment": trace.environment,
        "layer": layer_label(trace.layer),
        "value": trace.value,
        "imported": trace.imported,
        "deprecated": trace.deprecated,
        "matchedRule": trace.rule_index.map(|i| i + 1),
        "warnings": [],
        "errors": []
    });
    let warnings = body["warnings"].as_array_mut().unwrap();
    if trace.deprecated {
        warnings.push(json!("flag lifecycle is deprecated"));
    }
    for warning in &trace.warnings {
        warnings.push(json!(warning));
    }
    if trace.missing_user_id {
        warnings.push(json!("missing user.id for rollout bucketing"));
    }
    if let Some(rule) = &trace.catalog_rule {
        body["catalogRule"] = json!({
            "when": rule.when,
            "serve": rule.serve,
            "rollout": rule.rollout,
            "reason": rule.reason,
        });
    }
    if let Some(bucket) = trace.rollout_bucket {
        body["rolloutBucket"] = json!(bucket);
    }
    println!("{body}");
}

/// Line printed under `--trace` when the user has a stable id (object `id` or string user).
fn trace_user_line(user: &Value) -> Option<String> {
    user_id(user).map(|id| format!("User ID: {id}"))
}

fn print_trace_header(options: &Options, trace: &ExplainTrace, user: &Value) {
    println!("Flag: {}", options.flag);
    println!("Environment: {}", trace.environment);
    if let Some(line) = trace_user_line(user) {
        println!("{line}");
    }
    println!();
    println!("Rule trace:");
}

fn print_rule_trace(trace: &ExplainTrace) {
    for entry in &trace.rule_trace {
        println!(
            "  Rule {}: {}",
            entry.rule_index + 1,
            if entry.matched { "matched" } else { "skipped" }
        );
        println!("    {}", entry.evaluation_reason);
        if let Some(reason) = &entry.catalog_reason {
            println!("    Catalog reason: {reason}");
        } else if let Some(note) = &entry.catalog_note {
            println!("    Catalog: {note}");
        }
        if let Some(ref val) = entry.value {
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
    fn trace_user_line_uses_user_id_semantics() {
        assert_eq!(
            trace_user_line(&json!({ "id": "user-1" })).as_deref(),
            Some("User ID: user-1")
        );
        assert_eq!(
            trace_user_line(&json!("abc")).as_deref(),
            Some("User ID: abc")
        );
        assert_eq!(trace_user_line(&json!({ "plan": "beta" })), None);
    }

    #[test]
    fn kill_switch_overrides_reads_boolean_map() {
        let file = json!({ "version": "2.0", "flags": { "my_flag": true } });
        let overrides = kill_switch_overrides(&file);
        assert_eq!(overrides.get("my_flag"), Some(&true));
        assert_eq!(overrides.get("other"), None);
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
