//! Debug UI command implementation
//!
//! Provides an interactive web-based UI for debugging flag evaluation.
//! The debug UI allows testing flags with different user and context values,
//! showing detailed rule matching information and evaluation results.
//!
//! # Security Notes
//! - Binds to `127.0.0.1` by default (localhost only) for security
//! - CORS is permissive for local development only
//! - Prototype pollution protection in property access
//! - Input validation for user/context JSON

use crate::error::{CliError, CliResult};
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use controlpath_compiler::ast::{Artifact, BinaryOp, Expression, FuncCode, LogicalOp, Rule};
use rmp_serde::from_slice;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::cors::CorsLayer;

/// Command options for the debug UI
pub struct Options {
    /// Port for web server (default: 8080)
    pub port: Option<u16>,
    /// Environment name (uses .controlpath/<env>.ast)
    pub env: Option<String>,
    /// Path to AST file (alternative to --env)
    pub ast: Option<String>,
    /// Open browser automatically
    pub open: bool,
}

struct AppState {
    artifact: Arc<Artifact>,
}

#[derive(Serialize)]
struct FlagInfo {
    name: String,
    index: usize,
}

#[derive(Serialize)]
struct EvaluationResult {
    flag: String,
    value: Option<Value>,
    matched_rule: Option<usize>,
    rules: Vec<RuleEvaluation>,
    environment: String,
}

#[derive(Serialize)]
struct RuleEvaluation {
    index: usize,
    matched: bool,
    reason: String,
    value: Option<Value>,
    rule_type: String,
    when_clause: Option<String>,
    when_result: Option<bool>,
}

#[derive(Deserialize)]
struct EvaluateRequest {
    flag: String,
    user: Option<Value>,
    context: Option<Value>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

/// Find flag index by name in artifact
fn find_flag_index(artifact: &Artifact, flag_name: &str) -> Option<usize> {
    artifact
        .flag_names
        .iter()
        .enumerate()
        .find_map(|(index, &name_index)| {
            artifact
                .string_table
                .get(name_index as usize)
                .and_then(|name| if name == flag_name { Some(index) } else { None })
        })
}

/// Get property value from user or context using dot notation
fn get_property(prop_path: &str, user: &Value, context: &Option<Value>) -> Option<Value> {
    let parts: Vec<&str> = prop_path.split('.').collect();
    if parts.is_empty() {
        return None;
    }

    // Reject prototype-polluting paths
    let prototype_polluting = ["__proto__", "constructor", "prototype"];
    if parts.iter().any(|part| prototype_polluting.contains(part)) {
        return None;
    }

    // First part determines the root object
    let root = parts[0];
    let obj = if root == "user" {
        Some(user)
    } else if root == "context" {
        context.as_ref()
    } else {
        // Try user first, then context
        user.get(root).or_else(|| context.as_ref()?.get(root))
    }?;

    // Navigate nested properties
    let mut current = obj;
    for part in parts.iter().skip(1) {
        current = current.get(part)?;
    }

    Some(current.clone())
}

/// Evaluate expression to a value
fn evaluate_expression_value(
    expr: &Expression,
    artifact: &Artifact,
    user: &Value,
    context: &Option<Value>,
) -> Option<Value> {
    match expr {
        Expression::Property { prop_index } => {
            let prop_path = artifact.string_table.get(*prop_index as usize)?;
            get_property(prop_path, user, context)
        }
        Expression::Literal { value } => {
            // Handle string table indices for string literals
            if let Some(num) = value.as_u64() {
                if let Some(str_val) = artifact.string_table.get(num as usize) {
                    return Some(Value::String(str_val.clone()));
                }
            }
            Some(value.clone())
        }
        Expression::BinaryOp {
            op_code,
            left,
            right,
        } => {
            let left_val = evaluate_expression_value(left, artifact, user, context)?;
            let right_val = evaluate_expression_value(right, artifact, user, context)?;
            evaluate_binary_op(*op_code, &left_val, &right_val)
        }
        Expression::LogicalOp {
            op_code,
            left,
            right,
        } => {
            if *op_code == LogicalOp::Not as u8 {
                let left_val = evaluate_expression_value(left, artifact, user, context)?;
                return Some(Value::Bool(!coerce_to_boolean(&left_val).unwrap_or(false)));
            }
            let left_val = evaluate_expression_value(left, artifact, user, context)?;
            let right_val = right
                .as_ref()
                .and_then(|r| evaluate_expression_value(r, artifact, user, context))?;
            evaluate_logical_op(*op_code, &left_val, &right_val)
        }
        Expression::Func { func_code, args } => {
            evaluate_function(*func_code, args, artifact, user, context)
        }
    }
}

/// Evaluate expression to boolean
fn evaluate_expression(
    expr: &Expression,
    artifact: &Artifact,
    user: &Value,
    context: &Option<Value>,
) -> bool {
    evaluate_expression_value(expr, artifact, user, context)
        .and_then(|v| coerce_to_boolean(&v))
        .unwrap_or(false)
}

/// Evaluate binary operation
fn evaluate_binary_op(op_code: u8, left: &Value, right: &Value) -> Option<Value> {
    let op = match op_code {
        x if x == BinaryOp::Eq as u8 => "==",
        x if x == BinaryOp::Ne as u8 => "!=",
        x if x == BinaryOp::Gt as u8 => ">",
        x if x == BinaryOp::Lt as u8 => "<",
        x if x == BinaryOp::Gte as u8 => ">=",
        x if x == BinaryOp::Lte as u8 => "<=",
        _ => return None,
    };

    let result = match op {
        "==" => {
            if left.is_null() || right.is_null() {
                left.is_null() == right.is_null()
            } else {
                coerce_and_compare(left, right) == 0
            }
        }
        "!=" => {
            if left.is_null() || right.is_null() {
                left.is_null() != right.is_null()
            } else {
                coerce_and_compare(left, right) != 0
            }
        }
        ">" => compare_values(left, right) > 0,
        ">=" => compare_values(left, right) >= 0,
        "<" => compare_values(left, right) < 0,
        "<=" => compare_values(left, right) <= 0,
        _ => return None,
    };
    Some(Value::Bool(result))
}

/// Evaluate logical operation
///
/// Note: For debugging purposes, both sides are evaluated even when
/// short-circuiting would apply (e.g., `false && ...`). This ensures
/// all evaluation results are visible in the debug UI.
fn evaluate_logical_op(op_code: u8, left: &Value, right: &Value) -> Option<Value> {
    let left_bool = coerce_to_boolean(left).unwrap_or(false);
    let right_bool = coerce_to_boolean(right).unwrap_or(false);
    match op_code {
        x if x == LogicalOp::And as u8 => Some(Value::Bool(left_bool && right_bool)),
        x if x == LogicalOp::Or as u8 => Some(Value::Bool(left_bool || right_bool)),
        _ => None,
    }
}

/// Compare two values for ordering
fn compare_values(left: &Value, right: &Value) -> i32 {
    // Try number coercion
    if let (Some(left_num), Some(right_num)) = (coerce_to_number(left), coerce_to_number(right)) {
        return (left_num - right_num).signum() as i32;
    }

    // String comparison
    let left_str = format!("{left}");
    let right_str = format!("{right}");
    left_str.cmp(&right_str) as i32
}

/// Coerce and compare two values (for equality operations)
fn coerce_and_compare(left: &Value, right: &Value) -> i32 {
    // Exact match
    if left == right {
        return 0;
    }

    // Try number coercion
    if let (Some(left_num), Some(right_num)) = (coerce_to_number(left), coerce_to_number(right)) {
        return if left_num == right_num { 0 } else { 1 };
    }

    // Try boolean coercion
    if let (Some(left_bool), Some(right_bool)) = (coerce_to_boolean(left), coerce_to_boolean(right))
    {
        return if left_bool == right_bool { 0 } else { 1 };
    }

    // String comparison
    let left_str = format!("{left}");
    let right_str = format!("{right}");
    left_str.cmp(&right_str) as i32
}

/// Coerce a value to a number if possible
fn coerce_to_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Coerce a value to a boolean if possible
fn coerce_to_boolean(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(b) => Some(*b),
        Value::String(s) => {
            let lower = s.to_lowercase();
            if lower == "true" || lower == "1" {
                Some(true)
            } else if lower == "false" || lower == "0" {
                Some(false)
            } else {
                None
            }
        }
        Value::Number(n) => n.as_f64().map(|f| f != 0.0),
        _ => None,
    }
}

/// Simple string hash function (djb2 algorithm)
fn hash_string(s: &str) -> u32 {
    let mut hash: i32 = 5381;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as i32);
    }
    hash.unsigned_abs()
}

/// Select a variation based on user ID hash
fn select_variation(
    variations: &[controlpath_compiler::ast::Variation],
    artifact: &Artifact,
    user: &Value,
) -> Option<Value> {
    if variations.is_empty() {
        return None;
    }

    let user_id = user
        .as_object()
        .and_then(|obj| obj.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let hash = hash_string(user_id);
    let total_pct: u8 = variations.iter().map(|v| v.percentage).sum();
    if total_pct == 0 {
        let first = variations.first()?;
        return artifact
            .string_table
            .get(first.var_index as usize)
            .map(|s| Value::String(s.clone()));
    }

    let bucket = (hash % 100) as u8;
    let mut cumulative: u8 = 0;

    for variation in variations {
        cumulative = cumulative.saturating_add(variation.percentage);
        if bucket < cumulative {
            return artifact
                .string_table
                .get(variation.var_index as usize)
                .map(|s| Value::String(s.clone()));
        }
    }

    let last = variations.last()?;
    artifact
        .string_table
        .get(last.var_index as usize)
        .map(|s| Value::String(s.clone()))
}

/// Select rollout based on percentage
fn select_rollout(user: &Value, pct: u8) -> bool {
    if pct == 0 {
        return false;
    }
    if pct >= 100 {
        return true;
    }

    let user_id = user
        .as_object()
        .and_then(|obj| obj.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let hash = hash_string(user_id);
    let bucket = (hash % 100) as u8;
    bucket < pct
}

/// Evaluate a function call
fn evaluate_function(
    func_code: u8,
    args: &[Expression],
    artifact: &Artifact,
    user: &Value,
    context: &Option<Value>,
) -> Option<Value> {
    match func_code {
        x if x == FuncCode::StartsWith as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let str_val = evaluate_expression_value(&args[0], artifact, user, context)?;
            let prefix = evaluate_expression_value(&args[1], artifact, user, context)?;
            if let (Value::String(s), Value::String(p)) = (str_val, prefix) {
                Some(Value::Bool(s.starts_with(&p)))
            } else {
                Some(Value::Bool(false))
            }
        }
        x if x == FuncCode::EndsWith as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let str_val = evaluate_expression_value(&args[0], artifact, user, context)?;
            let suffix = evaluate_expression_value(&args[1], artifact, user, context)?;
            if let (Value::String(s), Value::String(suf)) = (str_val, suffix) {
                Some(Value::Bool(s.ends_with(&suf)))
            } else {
                Some(Value::Bool(false))
            }
        }
        x if x == FuncCode::Contains as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let container = evaluate_expression_value(&args[0], artifact, user, context)?;
            let value = evaluate_expression_value(&args[1], artifact, user, context)?;
            match (container, value) {
                (Value::String(s), Value::String(v)) => Some(Value::Bool(s.contains(&v))),
                (Value::Array(arr), val) => Some(Value::Bool(arr.iter().any(|item| item == &val))),
                _ => Some(Value::Bool(false)),
            }
        }
        x if x == FuncCode::Matches as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let str_val = evaluate_expression_value(&args[0], artifact, user, context)?;
            let pattern = evaluate_expression_value(&args[1], artifact, user, context)?;
            if let (Value::String(s), Value::String(p)) = (str_val, pattern) {
                use regex::Regex;
                let result = Regex::new(&p)
                    .map_err(|e| {
                        // Log invalid regex patterns in debug mode
                        #[cfg(debug_assertions)]
                        eprintln!("Invalid regex pattern: {} - {}", p, e);
                        e
                    })
                    .ok()
                    .map(|re| re.is_match(&s))
                    .unwrap_or(false);
                Some(Value::Bool(result))
            } else {
                Some(Value::Bool(false))
            }
        }
        x if x == FuncCode::Upper as u8 => {
            if args.is_empty() {
                return Some(Value::String(String::new()));
            }
            let str_val = evaluate_expression_value(&args[0], artifact, user, context)?;
            let s = match str_val {
                Value::String(s) => s,
                _ => str_val.to_string(),
            };
            Some(Value::String(s.to_uppercase()))
        }
        x if x == FuncCode::Lower as u8 => {
            if args.is_empty() {
                return Some(Value::String(String::new()));
            }
            let str_val = evaluate_expression_value(&args[0], artifact, user, context)?;
            let s = match str_val {
                Value::String(s) => s,
                _ => str_val.to_string(),
            };
            Some(Value::String(s.to_lowercase()))
        }
        x if x == FuncCode::Length as u8 => {
            if args.is_empty() {
                return Some(Value::Number(0.into()));
            }
            let value = evaluate_expression_value(&args[0], artifact, user, context)?;
            let len = match value {
                Value::String(s) => s.len(),
                Value::Array(arr) => arr.len(),
                _ => 0,
            };
            Some(Value::Number(len.into()))
        }
        x if x == FuncCode::In as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let value = evaluate_expression_value(&args[0], artifact, user, context)?;
            let array = evaluate_expression_value(&args[1], artifact, user, context)?;
            if let Value::Array(arr) = array {
                Some(Value::Bool(arr.iter().any(|item| item == &value)))
            } else {
                Some(Value::Bool(false))
            }
        }
        _ => Some(Value::Bool(false)),
    }
}

/// Format expression for display
fn format_expression(expr: &Expression, artifact: &Artifact, depth: usize) -> String {
    if depth > 10 {
        return "...".to_string();
    }
    match expr {
        Expression::Property { prop_index } => artifact
            .string_table
            .get(*prop_index as usize)
            .cloned()
            .unwrap_or_else(|| format!("<prop:{prop_index}>")),
        Expression::Literal { value } => {
            if let Some(num) = value.as_u64() {
                if let Some(str_val) = artifact.string_table.get(num as usize) {
                    return format!("\"{str_val}\"");
                }
            }
            format!("{value}")
        }
        Expression::BinaryOp {
            op_code,
            left,
            right,
        } => {
            let op_str = match *op_code {
                x if x == BinaryOp::Eq as u8 => "==",
                x if x == BinaryOp::Ne as u8 => "!=",
                x if x == BinaryOp::Gt as u8 => ">",
                x if x == BinaryOp::Gte as u8 => ">=",
                x if x == BinaryOp::Lt as u8 => "<",
                x if x == BinaryOp::Lte as u8 => "<=",
                _ => "?",
            };
            format!(
                "({} {} {})",
                format_expression(left, artifact, depth + 1),
                op_str,
                format_expression(right, artifact, depth + 1)
            )
        }
        Expression::LogicalOp {
            op_code,
            left,
            right,
        } => {
            if *op_code == LogicalOp::Not as u8 {
                format!("!({})", format_expression(left, artifact, depth + 1))
            } else {
                let op_str = match *op_code {
                    x if x == LogicalOp::And as u8 => "&&",
                    x if x == LogicalOp::Or as u8 => "||",
                    _ => "?",
                };
                if let Some(right_expr) = right {
                    format!(
                        "({} {} {})",
                        format_expression(left, artifact, depth + 1),
                        op_str,
                        format_expression(right_expr, artifact, depth + 1)
                    )
                } else {
                    format!(
                        "({} {})",
                        op_str,
                        format_expression(left, artifact, depth + 1)
                    )
                }
            }
        }
        Expression::Func { func_code, args } => {
            let func_name = match *func_code {
                x if x == FuncCode::StartsWith as u8 => "startsWith",
                x if x == FuncCode::EndsWith as u8 => "endsWith",
                x if x == FuncCode::Contains as u8 => "contains",
                x if x == FuncCode::Matches as u8 => "matches",
                x if x == FuncCode::Upper as u8 => "upper",
                x if x == FuncCode::Lower as u8 => "lower",
                x if x == FuncCode::Length as u8 => "length",
                x if x == FuncCode::In as u8 => "in",
                _ => "unknown",
            };
            let args_str = args
                .iter()
                .map(|arg| format_expression(arg, artifact, depth + 1))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{func_name}({args_str})")
        }
    }
}

/// Evaluate a rule and return detailed information
fn evaluate_rule_detailed(
    rule: &Rule,
    artifact: &Artifact,
    user: &Value,
    context: &Option<Value>,
) -> RuleEvaluation {
    match rule {
        Rule::ServeWithoutWhen(payload) => {
            let value = match payload {
                controlpath_compiler::ast::ServePayload::String(s) => {
                    Some(Value::String(s.clone()))
                }
                controlpath_compiler::ast::ServePayload::Number(idx) => artifact
                    .string_table
                    .get(*idx as usize)
                    .map(|s| Value::String(s.clone())),
            };
            RuleEvaluation {
                index: 0,
                matched: true,
                reason: "Serve rule without when clause - always matches".to_string(),
                value: value.clone(),
                rule_type: "serve".to_string(),
                when_clause: None,
                when_result: None,
            }
        }
        Rule::ServeWithWhen(when_expr, payload) => {
            let when_result = evaluate_expression(when_expr, artifact, user, context);
            let value = if when_result {
                match payload {
                    controlpath_compiler::ast::ServePayload::String(s) => {
                        Some(Value::String(s.clone()))
                    }
                    controlpath_compiler::ast::ServePayload::Number(idx) => artifact
                        .string_table
                        .get(*idx as usize)
                        .map(|s| Value::String(s.clone())),
                }
            } else {
                None
            };
            RuleEvaluation {
                index: 0,
                matched: when_result,
                reason: if when_result {
                    "When clause evaluated to true".to_string()
                } else {
                    "When clause evaluated to false".to_string()
                },
                value,
                rule_type: "serve".to_string(),
                when_clause: Some(format_expression(when_expr, artifact, 0)),
                when_result: Some(when_result),
            }
        }
        Rule::VariationsWithoutWhen(variations) => {
            let value = select_variation(variations, artifact, user);
            RuleEvaluation {
                index: 0,
                matched: value.is_some(),
                reason: "Variations rule without when clause - always matches".to_string(),
                value,
                rule_type: "variations".to_string(),
                when_clause: None,
                when_result: None,
            }
        }
        Rule::VariationsWithWhen(when_expr, variations) => {
            let when_result = evaluate_expression(when_expr, artifact, user, context);
            let value = if when_result {
                select_variation(variations, artifact, user)
            } else {
                None
            };
            RuleEvaluation {
                index: 0,
                matched: when_result && value.is_some(),
                reason: if when_result {
                    "When clause evaluated to true, variation selected".to_string()
                } else {
                    "When clause evaluated to false".to_string()
                },
                value,
                rule_type: "variations".to_string(),
                when_clause: Some(format_expression(when_expr, artifact, 0)),
                when_result: Some(when_result),
            }
        }
        Rule::RolloutWithoutWhen(payload) => {
            let matched = select_rollout(user, payload.percentage);
            let value = if matched {
                match &payload.value_index {
                    controlpath_compiler::ast::RolloutValue::String(s) => {
                        Some(Value::String(s.clone()))
                    }
                    controlpath_compiler::ast::RolloutValue::Number(idx) => artifact
                        .string_table
                        .get(*idx as usize)
                        .map(|s| Value::String(s.clone())),
                }
            } else {
                None
            };
            RuleEvaluation {
                index: 0,
                matched,
                reason: format!(
                    "Rollout rule: {}% chance, {}",
                    payload.percentage,
                    if matched { "selected" } else { "not selected" }
                ),
                value,
                rule_type: "rollout".to_string(),
                when_clause: None,
                when_result: None,
            }
        }
        Rule::RolloutWithWhen(when_expr, payload) => {
            let when_result = evaluate_expression(when_expr, artifact, user, context);
            let matched = when_result && select_rollout(user, payload.percentage);
            let value = if matched {
                match &payload.value_index {
                    controlpath_compiler::ast::RolloutValue::String(s) => {
                        Some(Value::String(s.clone()))
                    }
                    controlpath_compiler::ast::RolloutValue::Number(idx) => artifact
                        .string_table
                        .get(*idx as usize)
                        .map(|s| Value::String(s.clone())),
                }
            } else {
                None
            };
            RuleEvaluation {
                index: 0,
                matched,
                reason: format!(
                    "When clause: {}, Rollout: {}% chance, {}",
                    if when_result { "true" } else { "false" },
                    payload.percentage,
                    if matched { "selected" } else { "not selected" }
                ),
                value,
                rule_type: "rollout".to_string(),
                when_clause: Some(format_expression(when_expr, artifact, 0)),
                when_result: Some(when_result),
            }
        }
    }
}

/// Determine AST path from options
fn determine_ast_path(options: &Options) -> Result<PathBuf, CliError> {
    options.ast.as_ref().map_or_else(
        || {
            options.env.as_ref().map_or_else(
                || {
                    // Try to find a default AST file
                    let default_path = PathBuf::from(".controlpath/production.ast");
                    if default_path.exists() {
                        Ok(default_path)
                    } else {
                        // Try to find any AST file
                        let controlpath_dir = PathBuf::from(".controlpath");
                        if controlpath_dir.exists() {
                            let entries = fs::read_dir(&controlpath_dir)
                                .map_err(|e| CliError::Message(format!("Failed to read .controlpath directory: {e}")))?;
                            for entry in entries {
                                let entry = entry.map_err(|e| CliError::Message(format!("Failed to read directory entry: {e}")))?;
                                let path = entry.path();
                                if path.extension().and_then(|s| s.to_str()) == Some("ast") {
                                    return Ok(path);
                                }
                            }
                        }
                        Err(CliError::Message(
                            "Either --ast <file> or --env <env> must be provided, or a .ast file must exist in .controlpath/".to_string(),
                        ))
                    }
                },
                |env| Ok(PathBuf::from(format!(".controlpath/{env}.ast"))),
            )
        },
        |ast| Ok(PathBuf::from(ast)),
    )
}

/// Load artifact from path
fn load_artifact(path: &PathBuf) -> CliResult<Artifact> {
    let ast_bytes =
        fs::read(path).map_err(|e| CliError::Message(format!("Failed to read AST file: {e}")))?;
    let artifact: Artifact = from_slice(&ast_bytes)
        .map_err(|e| CliError::Message(format!("Failed to deserialize AST: {e}")))?;
    Ok(artifact)
}

/// API handler: List all flags
///
/// Returns a list of all available flags in the artifact.
async fn list_flags(State(state): State<Arc<AppState>>) -> Json<Vec<FlagInfo>> {
    let flags: Vec<FlagInfo> = state
        .artifact
        .flag_names
        .iter()
        .enumerate()
        .filter_map(|(index, &name_index)| {
            state
                .artifact
                .string_table
                .get(name_index as usize)
                .map(|name| FlagInfo {
                    name: name.clone(),
                    index,
                })
        })
        .collect();
    Json(flags)
}

/// API handler: Evaluate a flag
///
/// Evaluates a flag with the provided user and context, returning detailed
/// information about rule matching. All rules are evaluated for debugging
/// visibility, but only the first matching rule's value is used.
///
/// # Validation
/// - Validates that user and context are valid JSON objects (if provided)
/// - Returns structured error responses for invalid input
async fn evaluate_flag(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EvaluateRequest>,
) -> Result<Json<EvaluationResult>, impl IntoResponse> {
    // Validate flag exists
    let flag_index = find_flag_index(&state.artifact, &req.flag).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Flag '{}' not found", req.flag),
            }),
        )
    })?;

    let flag_rules = state.artifact.flags.get(flag_index).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Flag '{}' has no rules", req.flag),
            }),
        )
    })?;

    // Validate and parse user JSON
    let user = if let Some(user_val) = req.user {
        if !user_val.is_object() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "User must be a JSON object".to_string(),
                }),
            ));
        }
        user_val
    } else {
        Value::Object(serde_json::Map::new())
    };

    // Validate context JSON if provided
    let context = if let Some(context_val) = req.context {
        if !context_val.is_object() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Context must be a JSON object".to_string(),
                }),
            ));
        }
        Some(context_val)
    } else {
        None
    };

    let mut matched_rule_index = None;
    let mut final_value = None;
    let mut rule_evaluations = Vec::new();

    // Evaluate all rules for debugging visibility
    // The compiler appends a default rule at the end, so at least one rule should match
    for (rule_index, rule) in flag_rules.iter().enumerate() {
        let mut eval = evaluate_rule_detailed(rule, &state.artifact, &user, &context);
        eval.index = rule_index;

        // Track first match for final value
        if eval.matched && matched_rule_index.is_none() {
            matched_rule_index = Some(rule_index);
            final_value = eval.value.clone();
        }

        rule_evaluations.push(eval);
    }

    // If no rules matched (shouldn't happen due to default rule, but handle gracefully)
    if matched_rule_index.is_none() && !rule_evaluations.is_empty() {
        // Use the last rule (default) as fallback
        let last_index = rule_evaluations.len() - 1;
        if let Some(last_eval) = rule_evaluations.get_mut(last_index) {
            matched_rule_index = Some(last_index);
            final_value = last_eval.value.clone();
            last_eval.matched = true;
        }
    }

    Ok(Json(EvaluationResult {
        flag: req.flag,
        value: final_value,
        matched_rule: matched_rule_index,
        rules: rule_evaluations,
        environment: state.artifact.environment.clone(),
    }))
}

/// Serve the debug UI HTML
async fn serve_ui() -> Html<&'static str> {
    Html(include_str!("debug_ui.html"))
}

/// Create the router
///
/// Sets up routes and middleware. CORS is permissive for local development.
/// In production, consider restricting CORS to specific origins.
fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(serve_ui))
        .route("/api/flags", get(list_flags))
        .route("/api/evaluate", post(evaluate_flag))
        // CORS is permissive for local development only
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Run the debug UI command
///
/// Starts a web server with an interactive debug UI for testing flag evaluation.
/// The server runs until interrupted (Ctrl+C) and handles graceful shutdown.
///
/// # Returns
/// Exit code: 0 on success, 1 on error
pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("✗ Debug UI failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    // Determine AST path
    let ast_path = determine_ast_path(options)?;

    // Load artifact
    let artifact = load_artifact(&ast_path)?;

    println!("✓ Loaded AST from: {}", ast_path.display());
    println!("  Environment: {}", artifact.environment);
    println!("  Flags: {}", artifact.flag_names.len());

    // Create app state
    let state = Arc::new(AppState {
        artifact: Arc::new(artifact),
    });

    // Create router
    let router = create_router(state);

    // Determine port
    let port = options.port.unwrap_or(8080);

    // Run the server
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| CliError::Message(format!("Failed to create runtime: {e}")))?;

    rt.block_on(async {
        // Bind to localhost only for security (127.0.0.1)
        let addr = format!("127.0.0.1:{port}");
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| CliError::Message(format!("Failed to bind to {addr}: {e}")))?;

        let url = format!("http://localhost:{port}");
        println!();
        println!("🚀 Debug UI running at {url}");
        println!("   Press Ctrl+C to stop");
        println!();

        // Open browser if requested
        if options.open {
            #[cfg(not(target_os = "windows"))]
            {
                let _ = std::process::Command::new("open").arg(&url).spawn();
            }
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", &url])
                    .spawn();
            }
        }

        // Setup graceful shutdown
        let server = axum::serve(listener, router);
        let graceful = server.with_graceful_shutdown(async {
            signal::ctrl_c()
                .await
                .expect("Failed to install signal handler");
            println!("\nShutting down gracefully...");
        });

        graceful
            .await
            .map_err(|e| CliError::Message(format!("Server error: {e}")))?;

        Ok::<(), CliError>(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use controlpath_compiler::ast::{Artifact, Expression, Rule, ServePayload};

    #[test]
    fn test_find_flag_index() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["test_flag".to_string(), "other_flag".to_string()],
            flags: vec![vec![], vec![]],
            flag_names: vec![0, 1],
            segments: None,
            signature: None,
        };

        assert_eq!(find_flag_index(&artifact, "test_flag"), Some(0));
        assert_eq!(find_flag_index(&artifact, "other_flag"), Some(1));
        assert_eq!(find_flag_index(&artifact, "nonexistent"), None);
    }

    #[test]
    fn test_get_property_from_user() {
        let user = serde_json::json!({"id": "user-1", "role": "admin"});
        let context = None;

        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["user.id".to_string(), "user.role".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let result1 = get_property("user.id", &user, &context);
        assert_eq!(result1, Some(Value::String("user-1".to_string())));

        let result2 = get_property("user.role", &user, &context);
        assert_eq!(result2, Some(Value::String("admin".to_string())));
    }

    #[test]
    fn test_get_property_prototype_pollution_protection() {
        let user = serde_json::json!({"id": "user-1"});
        let context = None;

        // Should reject prototype-polluting paths
        assert_eq!(get_property("__proto__", &user, &context), None);
        assert_eq!(get_property("constructor", &user, &context), None);
        assert_eq!(get_property("prototype", &user, &context), None);
        assert_eq!(get_property("user.__proto__", &user, &context), None);
    }

    #[test]
    fn test_evaluate_binary_op_equality() {
        let left = Value::String("test".to_string());
        let right = Value::String("test".to_string());
        let result = evaluate_binary_op(BinaryOp::Eq as u8, &left, &right);
        assert_eq!(result, Some(Value::Bool(true)));

        let left2 = Value::String("test".to_string());
        let right2 = Value::String("other".to_string());
        let result2 = evaluate_binary_op(BinaryOp::Eq as u8, &left2, &right2);
        assert_eq!(result2, Some(Value::Bool(false)));
    }

    #[test]
    fn test_evaluate_binary_op_null_comparison() {
        let left = Value::Null;
        let right = Value::Null;
        let result = evaluate_binary_op(BinaryOp::Eq as u8, &left, &right);
        assert_eq!(result, Some(Value::Bool(true)));

        let left2 = Value::Null;
        let right2 = Value::String("test".to_string());
        let result2 = evaluate_binary_op(BinaryOp::Eq as u8, &left2, &right2);
        assert_eq!(result2, Some(Value::Bool(false)));
    }

    #[test]
    fn test_evaluate_logical_op_and() {
        let left = Value::Bool(true);
        let right = Value::Bool(true);
        let result = evaluate_logical_op(LogicalOp::And as u8, &left, &right);
        assert_eq!(result, Some(Value::Bool(true)));

        let left2 = Value::Bool(true);
        let right2 = Value::Bool(false);
        let result2 = evaluate_logical_op(LogicalOp::And as u8, &left2, &right2);
        assert_eq!(result2, Some(Value::Bool(false)));
    }

    #[test]
    fn test_evaluate_logical_op_or() {
        let left = Value::Bool(false);
        let right = Value::Bool(true);
        let result = evaluate_logical_op(LogicalOp::Or as u8, &left, &right);
        assert_eq!(result, Some(Value::Bool(true)));

        let left2 = Value::Bool(false);
        let right2 = Value::Bool(false);
        let result2 = evaluate_logical_op(LogicalOp::Or as u8, &left2, &right2);
        assert_eq!(result2, Some(Value::Bool(false)));
    }

    #[test]
    fn test_coerce_to_boolean() {
        assert_eq!(coerce_to_boolean(&Value::Bool(true)), Some(true));
        assert_eq!(coerce_to_boolean(&Value::Bool(false)), Some(false));
        assert_eq!(
            coerce_to_boolean(&Value::String("true".to_string())),
            Some(true)
        );
        assert_eq!(
            coerce_to_boolean(&Value::String("false".to_string())),
            Some(false)
        );
        assert_eq!(coerce_to_boolean(&Value::Number(1.into())), Some(true));
        assert_eq!(coerce_to_boolean(&Value::Number(0.into())), Some(false));
    }

    #[test]
    fn test_coerce_to_number() {
        assert_eq!(coerce_to_number(&Value::Number(42.into())), Some(42.0));
        assert_eq!(
            coerce_to_number(&Value::String("42".to_string())),
            Some(42.0)
        );
        assert_eq!(coerce_to_number(&Value::Bool(true)), Some(1.0));
        assert_eq!(coerce_to_number(&Value::Bool(false)), Some(0.0));
    }

    #[test]
    fn test_hash_string() {
        // Test that hash is consistent
        let hash1 = hash_string("test");
        let hash2 = hash_string("test");
        assert_eq!(hash1, hash2);

        // Test that different strings produce different hashes
        let hash3 = hash_string("other");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_select_rollout() {
        let user = serde_json::json!({"id": "user-123"});

        // 100% should always match
        assert!(select_rollout(&user, 100));

        // 0% should never match
        assert!(!select_rollout(&user, 0));

        // 50% should be consistent for same user
        let result1 = select_rollout(&user, 50);
        let result2 = select_rollout(&user, 50);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_determine_ast_path_with_ast() {
        let options = Options {
            port: None,
            env: None,
            ast: Some("test.ast".to_string()),
            open: false,
        };
        let path = determine_ast_path(&options).unwrap();
        assert_eq!(path, PathBuf::from("test.ast"));
    }

    #[test]
    fn test_determine_ast_path_with_env() {
        let options = Options {
            port: None,
            env: Some("production".to_string()),
            ast: None,
            open: false,
        };
        let path = determine_ast_path(&options).unwrap();
        assert_eq!(path, PathBuf::from(".controlpath/production.ast"));
    }

    #[test]
    fn test_evaluate_rule_detailed_serve_without_when() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["ON".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({"id": "user-1"});
        let rule = Rule::ServeWithoutWhen(ServePayload::Number(0));

        let eval = evaluate_rule_detailed(&rule, &artifact, &user, &None);
        assert!(eval.matched);
        assert_eq!(eval.rule_type, "serve");
        assert_eq!(eval.value, Some(Value::String("ON".to_string())));
    }
}
