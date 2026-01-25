//! Compiler module - compiles parsed deployment/definitions into AST artifacts
//!
//! This module handles:
//! - Expression parsing (from string to Expression AST)
//! - AST compilation (from parsed JSON to Artifact)
//! - String table building

pub mod expressions;
pub mod string_table;

use crate::ast::{
    Artifact, Expression, RolloutPayload, RolloutValue, Rule, ServePayload, Variation,
};
use crate::compiler::expressions::parse_expression;
use crate::compiler::string_table::StringTable;
use crate::error::{CompilationError, CompilerError};
use serde_json::Value;

/// Compile a deployment to an AST artifact.
///
/// # Arguments
///
/// * `deployment` - Parsed deployment JSON value
/// * `definitions` - Parsed flag definitions JSON value
///
/// # Returns
///
/// Compiled AST artifact
///
/// # Errors
///
/// Returns `CompilerError::Compilation` if compilation fails.
pub fn compile(deployment: &Value, definitions: &Value) -> Result<Artifact, CompilerError> {
    let mut string_table = StringTable::new();
    let mut flags: Vec<Vec<Rule>> = Vec::new();
    let mut segments: Vec<(u16, Expression)> = Vec::new();

    // Extract flag definitions
    let flag_defs = definitions
        .get("flags")
        .and_then(|f| f.as_array())
        .ok_or_else(|| {
            CompilerError::Compilation(CompilationError::InvalidRule(
                "Missing or invalid 'flags' array in definitions".to_string(),
            ))
        })?;

    // Build flag index map from definitions (order matters)
    let mut flag_index_map = std::collections::HashMap::new();
    for (index, flag) in flag_defs.iter().enumerate() {
        if let Some(name) = flag.get("name").and_then(|n| n.as_str()) {
            flag_index_map.insert(name.to_string(), index);
        }
    }

    // Initialize flags array (one entry per flag definition)
    flags.resize(flag_defs.len(), Vec::new());

    // Compile segments if present
    if let Some(segments_obj) = deployment.get("segments").and_then(|s| s.as_object()) {
        for (segment_name, segment_def) in segments_obj {
            if let Some(when_str) = segment_def.get("when").and_then(|w| w.as_str()) {
                let segment_expr = parse_expression(when_str)?;
                let processed_expr = string_table.process_expression(&segment_expr)?;
                let name_index = string_table.add(segment_name)?;
                segments.push((name_index, processed_expr));
            }
        }
    }

    // Extract deployment rules
    let rules_obj = deployment
        .get("rules")
        .and_then(|r| r.as_object())
        .ok_or_else(|| {
            CompilerError::Compilation(CompilationError::InvalidRule(
                "Missing or invalid 'rules' object in deployment".to_string(),
            ))
        })?;

    // Compile flag rules
    for (flag_name, flag_rules) in rules_obj {
        let flag_index = flag_index_map.get(flag_name.as_str()).ok_or_else(|| {
            CompilerError::Compilation(CompilationError::InvalidRule(format!(
                "Flag \"{flag_name}\" not found in flag definitions"
            )))
        })?;

        let flag_def = &flag_defs[*flag_index];
        let mut compiled_rules: Vec<Rule> = Vec::new();

        // Compile each rule
        if let Some(rules_array) = flag_rules.get("rules").and_then(|r| r.as_array()) {
            for rule in rules_array {
                if let Some(compiled_rule) =
                    compile_rule(rule, flag_name, flag_def, &mut string_table)?
                {
                    compiled_rules.push(compiled_rule);
                }
            }
        }

        flags[*flag_index] = compiled_rules;
    }

    // Append default serve rule for every flag using its definition defaultValue.
    for (flag_index, flag_def) in flag_defs.iter().enumerate() {
        let default_value = normalize_value(flag_def.get("defaultValue"), flag_def);
        let default_index = string_table.add(&default_value)?;
        flags[flag_index].push(Rule::ServeWithoutWhen(ServePayload::Number(default_index)));
    }

    // Build flag names array (string table indices) for automatic flag name map inference
    let mut flag_names: Vec<u16> = Vec::new();
    for flag_def in flag_defs {
        if let Some(name) = flag_def.get("name").and_then(|n| n.as_str()) {
            let name_index = string_table.add(name)?;
            flag_names.push(name_index);
        }
    }

    // Extract environment name
    let environment = deployment
        .get("environment")
        .and_then(|e| e.as_str())
        .ok_or_else(|| {
            CompilerError::Compilation(CompilationError::InvalidRule(
                "Missing 'environment' field in deployment".to_string(),
            ))
        })?;

    // Build artifact
    let artifact = Artifact {
        version: "1.0".to_string(),
        environment: environment.to_string(),
        string_table: string_table.to_vec(),
        flags,
        flag_names,
        segments: if segments.is_empty() {
            None
        } else {
            Some(segments)
        },
        signature: None,
    };

    Ok(artifact)
}

/// Compile an environment from control-path.yaml configuration
///
/// This function extracts definitions and deployment for the specified environment
/// from the config and compiles them into an AST artifact.
///
/// # Arguments
///
/// * `unified_config` - Parsed control-path.yaml configuration
/// * `environment` - Environment name to compile (e.g., "production", "staging")
///
/// # Returns
///
/// Compiled AST artifact for the specified environment
///
/// # Errors
///
/// Returns `CompilerError` if:
/// - The config is invalid
/// - The environment is not found in the config
/// - Compilation fails
pub fn compile_from_unified(
    unified_config: &Value,
    environment: &str,
) -> Result<Artifact, CompilerError> {
    // Extract definitions from config (without environment rules)
    let mut definitions = serde_json::json!({
        "flags": []
    });

    if let Some(flags) = unified_config.get("flags").and_then(|f| f.as_array()) {
        let mut def_flags = Vec::new();
        for flag in flags {
            // Clone flag but transform for compiler compatibility
            let mut flag_def = flag.clone();
            if let Some(obj) = flag_def.as_object_mut() {
                // Remove environments field (not part of definitions)
                obj.remove("environments");

                // Transform "default" to "defaultValue" for compiler compatibility
                if let Some(default_val) = obj.remove("default") {
                    obj.insert("defaultValue".to_string(), default_val);
                }
            }
            def_flags.push(flag_def);
        }
        definitions["flags"] = serde_json::json!(def_flags);
    }

    // Extract deployment structure for the specified environment
    let mut deployment = serde_json::json!({
        "environment": environment,
        "rules": {}
    });

    if let Some(flags) = unified_config.get("flags").and_then(|f| f.as_array()) {
        let mut rules = serde_json::Map::new();

        for flag in flags {
            if let Some(flag_name) = flag.get("name").and_then(|n| n.as_str()) {
                // Get environment rules for this flag
                if let Some(env_rules) = flag
                    .get("environments")
                    .and_then(|e| e.as_object())
                    .and_then(|e| e.get(environment))
                    .and_then(|r| r.as_array())
                {
                    // Only add to rules if there are rules for this environment
                    if !env_rules.is_empty() {
                        rules.insert(
                            flag_name.to_string(),
                            serde_json::json!({
                                "rules": env_rules
                            }),
                        );
                    }
                }
            }
        }

        deployment["rules"] = serde_json::json!(rules);
    }

    // Extract segments if present in config
    if let Some(segments) = unified_config.get("segments") {
        deployment["segments"] = segments.clone();
    }

    // Compile using the extracted definitions and deployment
    compile(&deployment, &definitions)
}

#[cfg(test)]
mod tests;

/// Compile a single deployment rule to an AST rule.
fn compile_rule(
    rule: &Value,
    flag_name: &str,
    flag_def: &Value,
    string_table: &mut StringTable,
) -> Result<Option<Rule>, CompilerError> {
    // Parse when clause if present
    let when_expr: Option<Expression> =
        if let Some(when_str) = rule.get("when").and_then(|w| w.as_str()) {
            let parsed_expr = parse_expression(when_str)?;
            Some(string_table.process_expression(&parsed_expr)?)
        } else {
            None
        };

    // Compile serve rule
    if let Some(serve_value) = rule.get("serve") {
        let flag_type = flag_def
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("boolean");

        // For serve rules, we store the value as-is (don't look up variations)
        // This allows serve: BLUE to store "BLUE" (variation name) rather than "blue" (variation value)
        // For multivariate flags, if serve_value is a string, we keep it as-is (it might be a variation name)
        // For boolean flags, normalize_value will handle the conversion (true -> "ON", etc.)
        let value = if flag_type == "multivariate" {
            // For multivariate flags, if serve_value is a string, use it directly
            // This preserves variation names like "BLUE" instead of looking them up
            if let Some(serve_str) = serve_value.as_str() {
                serve_str.to_string()
            } else {
                // Not a string, normalize normally (for numbers, etc.)
                normalize_value(Some(serve_value), flag_def)
            }
        } else {
            // For boolean flags, normalize normally (true -> "ON", etc.)
            normalize_value(Some(serve_value), flag_def)
        };

        let value_index = string_table.add(&value)?;
        let payload = ServePayload::Number(value_index);

        return Ok(Some(if let Some(when) = when_expr {
            Rule::ServeWithWhen(when, payload)
        } else {
            Rule::ServeWithoutWhen(payload)
        }));
    }

    // Compile variations rule
    if let Some(variations_array) = rule.get("variations").and_then(|v| v.as_array()) {
        if !variations_array.is_empty() {
            let variations: Result<Vec<Variation>, CompilerError> = variations_array
                .iter()
                .map(|var| {
                    let variation_name = var
                        .get("variation")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            CompilerError::Compilation(CompilationError::InvalidRule(
                                "Missing 'variation' in variations rule".to_string(),
                            ))
                        })?;

                    let flag_variations = flag_def
                        .get("variations")
                        .and_then(|v| v.as_array())
                        .ok_or_else(|| {
                            CompilerError::Compilation(CompilationError::InvalidRule(format!(
                                "Flag \"{flag_name}\" does not have variations defined, but rule uses variations"
                            )))
                        })?;

                    let var_def = flag_variations
                        .iter()
                        .find(|v| v.get("name").and_then(|n| n.as_str()) == Some(variation_name))
                        .ok_or_else(|| {
                            CompilerError::Compilation(CompilationError::InvalidRule(format!(
                                "Variation \"{variation_name}\" not found in flag \"{flag_name}\""
                            )))
                        })?;

                    let var_value = normalize_value(var_def.get("value"), flag_def);
                    let var_index = string_table.add(&var_value)?;

                    let weight = var
                        .get("weight")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0);
                    // Match TypeScript behavior: just round, don't clamp
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let percentage = weight.round() as u8;

                    Ok(Variation {
                        var_index,
                        percentage,
                    })
                })
                .collect();

            let variations = variations?;

            return Ok(Some(if let Some(when) = when_expr {
                Rule::VariationsWithWhen(when, variations)
            } else {
                Rule::VariationsWithoutWhen(variations)
            }));
        }
    }

    // Compile rollout rule
    if let Some(rollout_obj) = rule.get("rollout").and_then(|r| r.as_object()) {
        let variation_name = rollout_obj
            .get("variation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                CompilerError::Compilation(CompilationError::InvalidRule(
                    "Missing 'variation' in rollout rule".to_string(),
                ))
            })?;

        let flag_type = flag_def
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("boolean");

        let value_index: u16 = if flag_type == "boolean" {
            // For boolean flags, rollout.variation is the value (ON/OFF), not a variation name
            let value = normalize_value(
                Some(&serde_json::Value::String(variation_name.to_string())),
                flag_def,
            );
            string_table.add(&value)?
        } else {
            // For multivariate flags, rollout.variation is a variation name
            let flag_variations = flag_def
                .get("variations")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    CompilerError::Compilation(CompilationError::InvalidRule(format!(
                        "Flag \"{flag_name}\" does not have variations defined, but rule uses rollout"
                    )))
                })?;

            let var_def = flag_variations
                .iter()
                .find(|v| v.get("name").and_then(|n| n.as_str()) == Some(variation_name))
                .ok_or_else(|| {
                    CompilerError::Compilation(CompilationError::InvalidRule(format!(
                        "Variation \"{variation_name}\" not found in flag \"{flag_name}\""
                    )))
                })?;

            let var_value = normalize_value(var_def.get("value"), flag_def);
            string_table.add(&var_value)?
        };

        let percentage = rollout_obj
            .get("percentage")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        // Match TypeScript behavior: just round, don't clamp
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let percentage = percentage.round() as u8;

        let payload = RolloutPayload {
            value_index: RolloutValue::Number(value_index),
            percentage,
        };

        return Ok(Some(if let Some(when) = when_expr {
            Rule::RolloutWithWhen(when, payload)
        } else {
            Rule::RolloutWithoutWhen(payload)
        }));
    }

    // No valid rule type found
    Ok(None)
}

/// Normalize a flag value to a string representation.
/// For boolean flags, converts boolean to string.
fn normalize_value(value: Option<&Value>, flag_def: &Value) -> String {
    let flag_type = flag_def
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("boolean");

    if flag_type == "boolean" {
        // For boolean flags, normalize to string representation
        if let Some(val) = value {
            if let Some(b) = val.as_bool() {
                return if b {
                    "ON".to_string()
                } else {
                    "OFF".to_string()
                };
            }
            if let Some(s) = val.as_str() {
                let upper = s.to_uppercase();
                if upper == "ON" || upper == "TRUE" || upper == "1" {
                    return "ON".to_string();
                }
                if upper == "OFF" || upper == "FALSE" || upper == "0" {
                    return "OFF".to_string();
                }
            }
        }
    }

    // For other types, convert to string
    value.map_or_else(String::new, |val| match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        _ => serde_json::to_string(val).unwrap_or_else(|_| String::new()),
    })
}
