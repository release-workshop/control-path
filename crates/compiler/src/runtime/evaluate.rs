/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * Boolean flag evaluation against compiled AST artifacts (shared with CLI explain and TypeScript runtime).
 */

use crate::ast::{Artifact, BinaryOp, Expression, FuncCode, LogicalOp, Rule};
use chrono::{Datelike, Timelike};
use regex::Regex;
use semver::Version;
use serde_json::Value;

/// Evaluation attributes object for flag targeting (single flat JSON object).
pub struct EvaluationAttributes<'a> {
    pub attributes: &'a Value,
}

/// Result of evaluating a single AST rule.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleEvaluation {
    pub matched: bool,
    pub value: Option<Value>,
    pub reason: String,
}

/// Find flag index by qualified name in an artifact.
pub fn find_flag_index(artifact: &Artifact, flag_name: &str) -> Option<usize> {
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

/// Evaluate flag rules in order; returns first matching rule index and payload value.
pub fn evaluate_flag(
    artifact: &Artifact,
    flag_index: usize,
    attrs: &EvaluationAttributes<'_>,
) -> (Option<usize>, Option<Value>) {
    let flag_rules = match artifact.flags.get(flag_index) {
        Some(rules) => rules,
        None => return (None, None),
    };

    for (rule_index, rule) in flag_rules.iter().enumerate() {
        let eval = evaluate_rule(rule, artifact, attrs.attributes);
        if eval.matched {
            return (Some(rule_index), eval.value);
        }
    }
    (None, None)
}

/// Rollout hash bucket (0–99) for the identity `id`, if present.
pub fn rollout_bucket(attributes: &Value) -> Option<u32> {
    user_id(attributes).map(|id| hash_string(&id) % 100)
}

/// Identity string used for rollout hashing (`id` field or string attributes).
pub fn user_id(attributes: &Value) -> Option<String> {
    attributes
        .get("id")
        .and_then(|v| v.as_str().map(str::to_string))
        .or_else(|| attributes.as_str().map(str::to_string))
}

/// Strip `user.` / `context.` prefixes for legacy string-table paths.
/// Also applied at compile time in `compiler/string_table.rs` and in
/// `runtime/typescript/src/evaluator.ts` — keep all three in sync.
fn normalize_property_path(path: &str) -> &str {
    if let Some(rest) = path.strip_prefix("user.") {
        rest
    } else if let Some(rest) = path.strip_prefix("context.") {
        rest
    } else {
        path
    }
}

/// Get a property value from evaluation attributes using dot notation.
pub fn get_property(prop_path: &str, attributes: &Value) -> Option<Value> {
    let path = normalize_property_path(prop_path);
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return None;
    }

    let prototype_polluting = ["__proto__", "constructor", "prototype"];
    if parts.iter().any(|part| prototype_polluting.contains(part)) {
        return None;
    }

    let mut current = attributes;
    for part in parts {
        current = current.get(part)?;
    }

    Some(current.clone())
}

/// Evaluate expression to a value
fn evaluate_expression_value(
    expr: &Expression,
    artifact: &Artifact,
    attributes: &Value,
) -> Option<Value> {
    match expr {
        Expression::Property { prop_index } => {
            let prop_path = artifact.string_table.get(*prop_index as usize)?;
            get_property(prop_path, attributes)
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
            let left_val = evaluate_expression_value(left, artifact, attributes)?;
            let right_val = evaluate_expression_value(right, artifact, attributes)?;
            evaluate_binary_op(*op_code, &left_val, &right_val)
        }
        Expression::LogicalOp {
            op_code,
            left,
            right,
        } => {
            let left_val = evaluate_expression(left, artifact, attributes);
            if *op_code == LogicalOp::Not as u8 {
                return Some(Value::Bool(!left_val));
            }
            let right_val = right
                .as_ref()
                .map(|r| evaluate_expression(r, artifact, attributes))?;
            let result = match *op_code {
                x if x == LogicalOp::And as u8 => left_val && right_val,
                x if x == LogicalOp::Or as u8 => left_val || right_val,
                _ => false,
            };
            Some(Value::Bool(result))
        }
        Expression::Func { func_code, args } => {
            evaluate_function(*func_code, args, artifact, attributes)
        }
    }
}

/// Evaluate expression to boolean
fn evaluate_expression(expr: &Expression, artifact: &Artifact, attributes: &Value) -> bool {
    match evaluate_expression_value(expr, artifact, attributes) {
        Some(Value::Bool(b)) => b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Some(Value::Null) => false,
        Some(Value::Array(arr)) => !arr.is_empty(),
        Some(Value::Object(obj)) => !obj.is_empty(),
        None => false,
    }
}

/// Evaluate binary operator
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
        ">" => {
            if left.is_null() || right.is_null() {
                false
            } else {
                compare_values(left, right) > 0
            }
        }
        "<" => {
            if left.is_null() || right.is_null() {
                false
            } else {
                compare_values(left, right) < 0
            }
        }
        ">=" => {
            if left.is_null() || right.is_null() {
                false
            } else {
                compare_values(left, right) >= 0
            }
        }
        "<=" => {
            if left.is_null() || right.is_null() {
                false
            } else {
                compare_values(left, right) <= 0
            }
        }
        _ => false,
    };

    Some(Value::Bool(result))
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

/// Simple string hash function (djb2 algorithm).
/// Matches the TypeScript implementation for consistent hashing.
/// Uses wrapping operations to match 32-bit integer behavior.
fn hash_string(s: &str) -> u32 {
    let mut hash: i32 = 5381;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as i32);
    }
    hash.unsigned_abs()
}

/// Select a variation based on user ID hash.
/// Matches the TypeScript implementation for consistent selection.
fn select_variation(
    variations: &[crate::ast::Variation],
    artifact: &Artifact,
    attributes: &Value,
) -> Option<Value> {
    if variations.is_empty() {
        return None;
    }

    let identity = user_id(attributes).unwrap_or_default();
    let hash = hash_string(&identity);

    // Calculate total percentage
    let total_pct: u8 = variations.iter().map(|v| v.percentage).sum();
    if total_pct == 0 {
        // Return first variation if no percentages
        let first = variations.first()?;
        return artifact
            .string_table
            .get(first.var_index as usize)
            .map(|s| Value::String(s.clone()));
    }

    // Normalize hash to 0-100 range
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

    // Fallback to last variation
    let last = variations.last()?;
    artifact
        .string_table
        .get(last.var_index as usize)
        .map(|s| Value::String(s.clone()))
}

/// Select rollout based on percentage using user ID hash.
/// Matches the TypeScript implementation for consistent selection.
fn select_rollout(attributes: &Value, pct: u8) -> bool {
    if pct == 0 {
        return false;
    }
    if pct >= 100 {
        return true;
    }

    let identity = user_id(attributes).unwrap_or_default();
    let hash = hash_string(&identity);
    let bucket = (hash % 100) as u8;

    bucket < pct
}

/// Evaluate a function call.
/// Returns the function result (which may be boolean, string, number, etc.).
fn evaluate_function(
    func_code: u8,
    args: &[Expression],
    artifact: &Artifact,
    attributes: &Value,
) -> Option<Value> {
    match func_code {
        x if x == FuncCode::StartsWith as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let str_val = evaluate_expression_value(&args[0], artifact, attributes)?;
            let prefix = evaluate_expression_value(&args[1], artifact, attributes)?;
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
            let str_val = evaluate_expression_value(&args[0], artifact, attributes)?;
            let suffix = evaluate_expression_value(&args[1], artifact, attributes)?;
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
            let container = evaluate_expression_value(&args[0], artifact, attributes)?;
            let value = evaluate_expression_value(&args[1], artifact, attributes)?;
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
            let str_val = evaluate_expression_value(&args[0], artifact, attributes)?;
            let pattern = evaluate_expression_value(&args[1], artifact, attributes)?;
            if let (Value::String(s), Value::String(p)) = (str_val, pattern) {
                let result = Regex::new(&p)
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
            let str_val = evaluate_expression_value(&args[0], artifact, attributes)?;
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
            let str_val = evaluate_expression_value(&args[0], artifact, attributes)?;
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
            let value = evaluate_expression_value(&args[0], artifact, attributes)?;
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
            let value = evaluate_expression_value(&args[0], artifact, attributes)?;
            let list = evaluate_expression_value(&args[1], artifact, attributes)?;
            if let Value::Array(arr) = list {
                Some(Value::Bool(arr.iter().any(|item| item == &value)))
            } else {
                Some(Value::Bool(false))
            }
        }
        x if x == FuncCode::Intersects as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let arr1 = evaluate_expression_value(&args[0], artifact, attributes)?;
            let arr2 = evaluate_expression_value(&args[1], artifact, attributes)?;
            if let (Value::Array(a1), Value::Array(a2)) = (arr1, arr2) {
                Some(Value::Bool(a1.iter().any(|item| a2.contains(item))))
            } else {
                Some(Value::Bool(false))
            }
        }
        x if x == FuncCode::Hash as u8 => {
            // HASHED_PARTITION(id, buckets) - returns bucket number (0 to buckets-1)
            if args.len() < 2 {
                return Some(Value::Number(0.into()));
            }
            let id = evaluate_expression_value(&args[0], artifact, attributes)?;
            let buckets = evaluate_expression_value(&args[1], artifact, attributes)?;
            let id_str = id.to_string();
            let buckets_num = buckets.as_u64().unwrap_or(1) as u32;
            if buckets_num == 0 {
                return Some(Value::Number(0.into()));
            }
            let hash = hash_string(&id_str);
            Some(Value::Number((hash % buckets_num).into()))
        }
        x if x == FuncCode::Coalesce as u8 => {
            // Return first non-null, non-undefined value
            // Note: In Rust, None from evaluate_expression_value represents undefined
            for arg in args {
                if let Some(value) = evaluate_expression_value(arg, artifact, attributes) {
                    if !value.is_null() {
                        return Some(value);
                    }
                }
            }
            Some(Value::Null)
        }
        // Semver functions
        x if x == FuncCode::SemverEq as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let v1 = evaluate_expression_value(&args[0], artifact, attributes)?;
            let v2 = evaluate_expression_value(&args[1], artifact, attributes)?;
            if let (Value::String(s1), Value::String(s2)) = (v1, v2) {
                match (Version::parse(&s1), Version::parse(&s2)) {
                    (Ok(v1), Ok(v2)) => Some(Value::Bool(v1 == v2)),
                    _ => Some(Value::Bool(false)),
                }
            } else {
                Some(Value::Bool(false))
            }
        }
        x if x == FuncCode::SemverGt as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let v1 = evaluate_expression_value(&args[0], artifact, attributes)?;
            let v2 = evaluate_expression_value(&args[1], artifact, attributes)?;
            if let (Value::String(s1), Value::String(s2)) = (v1, v2) {
                match (Version::parse(&s1), Version::parse(&s2)) {
                    (Ok(v1), Ok(v2)) => Some(Value::Bool(v1 > v2)),
                    _ => Some(Value::Bool(false)),
                }
            } else {
                Some(Value::Bool(false))
            }
        }
        x if x == FuncCode::SemverGte as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let v1 = evaluate_expression_value(&args[0], artifact, attributes)?;
            let v2 = evaluate_expression_value(&args[1], artifact, attributes)?;
            if let (Value::String(s1), Value::String(s2)) = (v1, v2) {
                match (Version::parse(&s1), Version::parse(&s2)) {
                    (Ok(v1), Ok(v2)) => Some(Value::Bool(v1 >= v2)),
                    _ => Some(Value::Bool(false)),
                }
            } else {
                Some(Value::Bool(false))
            }
        }
        x if x == FuncCode::SemverLt as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let v1 = evaluate_expression_value(&args[0], artifact, attributes)?;
            let v2 = evaluate_expression_value(&args[1], artifact, attributes)?;
            if let (Value::String(s1), Value::String(s2)) = (v1, v2) {
                match (Version::parse(&s1), Version::parse(&s2)) {
                    (Ok(v1), Ok(v2)) => Some(Value::Bool(v1 < v2)),
                    _ => Some(Value::Bool(false)),
                }
            } else {
                Some(Value::Bool(false))
            }
        }
        x if x == FuncCode::SemverLte as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let v1 = evaluate_expression_value(&args[0], artifact, attributes)?;
            let v2 = evaluate_expression_value(&args[1], artifact, attributes)?;
            if let (Value::String(s1), Value::String(s2)) = (v1, v2) {
                match (Version::parse(&s1), Version::parse(&s2)) {
                    (Ok(v1), Ok(v2)) => Some(Value::Bool(v1 <= v2)),
                    _ => Some(Value::Bool(false)),
                }
            } else {
                Some(Value::Bool(false))
            }
        }
        // Temporal functions
        x if x == FuncCode::IsBetween as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            let start = evaluate_expression_value(&args[0], artifact, attributes)?;
            let end = evaluate_expression_value(&args[1], artifact, attributes)?;
            if let (Value::String(s1), Value::String(s2)) = (start, end) {
                // Parse ISO 8601 timestamps
                if let (Ok(start_time), Ok(end_time)) = (
                    chrono::DateTime::parse_from_rfc3339(&s1),
                    chrono::DateTime::parse_from_rfc3339(&s2),
                ) {
                    let now = chrono::Utc::now();
                    let start_utc = start_time.with_timezone(&chrono::Utc);
                    let end_utc = end_time.with_timezone(&chrono::Utc);
                    Some(Value::Bool(now >= start_utc && now <= end_utc))
                } else {
                    Some(Value::Bool(false))
                }
            } else {
                Some(Value::Bool(false))
            }
        }
        x if x == FuncCode::IsAfter as u8 => {
            if args.is_empty() {
                return Some(Value::Bool(false));
            }
            let timestamp = evaluate_expression_value(&args[0], artifact, attributes)?;
            if let Value::String(ts) = timestamp {
                if let Ok(ts_time) = chrono::DateTime::parse_from_rfc3339(&ts) {
                    let now = chrono::Utc::now();
                    let ts_utc = ts_time.with_timezone(&chrono::Utc);
                    Some(Value::Bool(now > ts_utc))
                } else {
                    Some(Value::Bool(false))
                }
            } else {
                Some(Value::Bool(false))
            }
        }
        x if x == FuncCode::IsBefore as u8 => {
            if args.is_empty() {
                return Some(Value::Bool(false));
            }
            let timestamp = evaluate_expression_value(&args[0], artifact, attributes)?;
            if let Value::String(ts) = timestamp {
                if let Ok(ts_time) = chrono::DateTime::parse_from_rfc3339(&ts) {
                    let now = chrono::Utc::now();
                    let ts_utc = ts_time.with_timezone(&chrono::Utc);
                    Some(Value::Bool(now < ts_utc))
                } else {
                    Some(Value::Bool(false))
                }
            } else {
                Some(Value::Bool(false))
            }
        }
        x if x == FuncCode::HourOfDay as u8 => {
            // CURRENT_HOUR_UTC - returns 0-23
            Some(Value::Number(chrono::Utc::now().hour().into()))
        }
        x if x == FuncCode::DayOfWeek as u8 => {
            // CURRENT_DAY_OF_WEEK_UTC - returns day name (MONDAY, TUESDAY, etc.)
            let days = [
                "SUNDAY",
                "MONDAY",
                "TUESDAY",
                "WEDNESDAY",
                "THURSDAY",
                "FRIDAY",
                "SATURDAY",
            ];
            let day_index = chrono::Utc::now().weekday().num_days_from_sunday() as usize;
            Some(Value::String(days[day_index].to_string()))
        }
        x if x == FuncCode::DayOfMonth as u8 => {
            // CURRENT_DAY_OF_MONTH_UTC - returns 1-31
            Some(Value::Number(chrono::Utc::now().day().into()))
        }
        x if x == FuncCode::Month as u8 => {
            // CURRENT_MONTH_UTC - returns 1-12
            Some(Value::Number(chrono::Utc::now().month().into()))
        }
        x if x == FuncCode::CurrentTimestamp as u8 => {
            // Returns ISO 8601 timestamp string in UTC
            Some(Value::String(chrono::Utc::now().to_rfc3339()))
        }
        // Segment function
        x if x == FuncCode::InSegment as u8 => {
            if args.len() < 2 {
                return Some(Value::Bool(false));
            }
            // First arg is user (we ignore it since we have user in scope)
            let _user_arg = evaluate_expression_value(&args[0], artifact, attributes);
            let segment_name = match evaluate_expression_value(&args[1], artifact, attributes) {
                Some(v) => v,
                None => return Some(Value::Bool(false)),
            };

            // Get segment name string
            let segment_name_str = match segment_name {
                Value::Number(n) => {
                    if let Some(idx) = n.as_u64() {
                        artifact.string_table.get(idx as usize).cloned()
                    } else {
                        None
                    }
                }
                Value::String(s) => Some(s),
                _ => None,
            };

            let segment_name_str = match segment_name_str {
                Some(s) => s,
                None => return Some(Value::Bool(false)),
            };

            // Look up segment in artifact
            let segments = match artifact.segments.as_ref() {
                Some(s) => s,
                None => return Some(Value::Bool(false)),
            };

            let segment = segments.iter().find(|(name_index, _)| {
                artifact
                    .string_table
                    .get(*name_index as usize)
                    .map(|name| name == &segment_name_str)
                    .unwrap_or(false)
            });

            let (_, segment_expr) = match segment {
                Some(s) => s,
                None => return Some(Value::Bool(false)),
            };

            Some(Value::Bool(evaluate_expression(
                segment_expr,
                artifact,
                attributes,
            )))
        }
        _ => {
            // Unknown function code - return None to indicate evaluation failure
            // This will cause the expression to evaluate to false in boolean context
            None
        }
    }
}

/// Format expression as readable string
#[allow(clippy::only_used_in_recursion)]
fn format_expression(expr: &Expression, artifact: &Artifact, indent: usize) -> String {
    match expr {
        Expression::Property { prop_index } => artifact
            .string_table
            .get(*prop_index as usize)
            .cloned()
            .unwrap_or_else(|| format!("<invalid prop_index: {prop_index}>")),
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
            let op = match *op_code {
                x if x == BinaryOp::Eq as u8 => "==",
                x if x == BinaryOp::Ne as u8 => "!=",
                x if x == BinaryOp::Gt as u8 => ">",
                x if x == BinaryOp::Lt as u8 => "<",
                x if x == BinaryOp::Gte as u8 => ">=",
                x if x == BinaryOp::Lte as u8 => "<=",
                _ => "?",
            };
            format!(
                "({} {} {})",
                format_expression(left, artifact, indent),
                op,
                format_expression(right, artifact, indent)
            )
        }
        Expression::LogicalOp {
            op_code,
            left,
            right,
        } => {
            let op = match *op_code {
                x if x == LogicalOp::And as u8 => "AND",
                x if x == LogicalOp::Or as u8 => "OR",
                x if x == LogicalOp::Not as u8 => "NOT",
                _ => "?",
            };
            if *op_code == LogicalOp::Not as u8 {
                format!("NOT ({})", format_expression(left, artifact, indent))
            } else {
                format!(
                    "({} {} {})",
                    format_expression(left, artifact, indent),
                    op,
                    right
                        .as_ref()
                        .map(|r| format_expression(r, artifact, indent))
                        .unwrap_or_else(|| "?".to_string())
                )
            }
        }
        Expression::Func { func_code, args } => {
            // Function names mapping (simplified)
            let func_name = match *func_code {
                0 => "startsWith",
                1 => "endsWith",
                2 => "contains",
                3 => "in",
                4 => "matches",
                5 => "upper",
                6 => "lower",
                7 => "length",
                8 => "intersects",
                9 => "semverEq",
                10 => "semverGt",
                11 => "semverGte",
                12 => "semverLt",
                13 => "semverLte",
                14 => "hash",
                15 => "coalesce",
                16 => "isBetween",
                17 => "isAfter",
                18 => "isBefore",
                19 => "dayOfWeek",
                20 => "hourOfDay",
                21 => "dayOfMonth",
                22 => "month",
                23 => "currentTimestamp",
                24 => "inSegment",
                _ => "unknown",
            };
            let args_str = args
                .iter()
                .map(|arg| format_expression(arg, artifact, indent))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{func_name}({args_str})")
        }
    }
}

/// Evaluate a single AST rule (first match wins when used via [`evaluate_flag`]).
pub fn evaluate_rule(rule: &Rule, artifact: &Artifact, attributes: &Value) -> RuleEvaluation {
    match rule {
        Rule::ServeWithoutWhen(payload) => {
            let value = match payload {
                crate::ast::ServePayload::String(s) => Some(Value::String(s.clone())),
                crate::ast::ServePayload::Number(idx) => artifact
                    .string_table
                    .get(*idx as usize)
                    .map(|s| Value::String(s.clone())),
            };
            RuleEvaluation {
                matched: true,
                value,
                reason: "Serve rule (no when clause)".to_string(),
            }
        }
        Rule::ServeWithWhen(when_expr, payload) => {
            let when_result = evaluate_expression(when_expr, artifact, attributes);
            if when_result {
                let value = match payload {
                    crate::ast::ServePayload::String(s) => Some(Value::String(s.clone())),
                    crate::ast::ServePayload::Number(idx) => artifact
                        .string_table
                        .get(*idx as usize)
                        .map(|s| Value::String(s.clone())),
                };
                RuleEvaluation {
                    matched: true,
                    value,
                    reason: "Serve rule matched (when clause evaluated to true)".to_string(),
                }
            } else {
                RuleEvaluation {
                    matched: false,
                    value: None,
                    reason: format!(
                        "Serve rule did not match (when clause evaluated to false: {})",
                        format_expression(when_expr, artifact, 0)
                    ),
                }
            }
        }
        Rule::VariationsWithoutWhen(variations) => {
            if let Some(value) = select_variation(variations, artifact, attributes) {
                RuleEvaluation {
                    matched: true,
                    value: Some(value),
                    reason: "Variations rule (no when clause)".to_string(),
                }
            } else {
                RuleEvaluation {
                    matched: false,
                    value: None,
                    reason: "Variations rule has no variations".to_string(),
                }
            }
        }
        Rule::VariationsWithWhen(when_expr, variations) => {
            let when_result = evaluate_expression(when_expr, artifact, attributes);
            if when_result {
                if let Some(value) = select_variation(variations, artifact, attributes) {
                    RuleEvaluation {
                        matched: true,
                        value: Some(value),
                        reason: "Variations rule matched (when clause evaluated to true)"
                            .to_string(),
                    }
                } else {
                    RuleEvaluation {
                        matched: false,
                        value: None,
                        reason: "Variations rule has no variations".to_string(),
                    }
                }
            } else {
                RuleEvaluation {
                    matched: false,
                    value: None,
                    reason: format!(
                        "Variations rule did not match (when clause evaluated to false: {})",
                        format_expression(when_expr, artifact, 0)
                    ),
                }
            }
        }
        Rule::RolloutWithoutWhen(payload) => {
            if select_rollout(attributes, payload.percentage) {
                let value = match &payload.value_index {
                    crate::ast::RolloutValue::String(s) => Some(Value::String(s.clone())),
                    crate::ast::RolloutValue::Number(idx) => artifact
                        .string_table
                        .get(*idx as usize)
                        .map(|s| Value::String(s.clone())),
                };
                RuleEvaluation {
                    matched: true,
                    value,
                    reason: format!("Rollout rule matched ({}% rollout)", payload.percentage),
                }
            } else {
                RuleEvaluation {
                    matched: false,
                    value: None,
                    reason: format!(
                        "Rollout rule did not match (user not in {}% rollout)",
                        payload.percentage
                    ),
                }
            }
        }
        Rule::RolloutWithWhen(when_expr, payload) => {
            let when_result = evaluate_expression(when_expr, artifact, attributes);
            if when_result {
                if select_rollout(attributes, payload.percentage) {
                    let value = match &payload.value_index {
                        crate::ast::RolloutValue::String(s) => Some(Value::String(s.clone())),
                        crate::ast::RolloutValue::Number(idx) => artifact
                            .string_table
                            .get(*idx as usize)
                            .map(|s| Value::String(s.clone())),
                    };
                    RuleEvaluation {
                        matched: true,
                        value,
                        reason: format!(
                            "Rollout rule matched (when clause evaluated to true, user in {}% rollout)",
                            payload.percentage
                        ),
                    }
                } else {
                    RuleEvaluation {
                        matched: false,
                        value: None,
                        reason: format!(
                            "Rollout rule did not match (when clause true, but user not in {}% rollout)",
                            payload.percentage
                        ),
                    }
                }
            } else {
                RuleEvaluation {
                    matched: false,
                    value: None,
                    reason: format!(
                        "Rollout rule did not match (when clause evaluated to false: {})",
                        format_expression(when_expr, artifact, 0)
                    ),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BinaryOp, Expression, RolloutPayload, RolloutValue, Rule, ServePayload};
    use serde_json::json;

    fn sample_artifact(flag_rules: Vec<Rule>) -> Artifact {
        Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![
                "ON".to_string(),
                "OFF".to_string(),
                "user.role".to_string(),
                "admin".to_string(),
            ],
            flags: vec![flag_rules],
            flag_names: vec![0],
            segments: None,
            signature: None,
        }
    }

    #[test]
    fn rollout_bucket_is_stable_for_user_id() {
        let user = json!({ "id": "stable-user" });
        let a = rollout_bucket(&user);
        let b = rollout_bucket(&user);
        assert_eq!(a, b);
        assert!(a.is_some_and(|b| b < 100));
    }

    #[test]
    fn evaluate_flag_first_serve_rule_wins() {
        let artifact = sample_artifact(vec![
            Rule::ServeWithoutWhen(ServePayload::Number(0)),
            Rule::ServeWithoutWhen(ServePayload::Number(1)),
        ]);
        let user = json!({ "id": "u1" });
        let attrs = EvaluationAttributes { attributes: &user };
        let (idx, val) = evaluate_flag(&artifact, 0, &attrs);
        assert_eq!(idx, Some(0));
        assert_eq!(val, Some(Value::String("ON".into())));
    }

    #[test]
    fn evaluate_flag_when_clause_gates_serve() {
        let artifact = sample_artifact(vec![
            Rule::ServeWithWhen(
                Expression::BinaryOp {
                    op_code: BinaryOp::Eq as u8,
                    left: Box::new(Expression::Property { prop_index: 2 }),
                    right: Box::new(Expression::Literal { value: json!(3) }),
                },
                ServePayload::Number(0),
            ),
            Rule::ServeWithoutWhen(ServePayload::Number(1)),
        ]);
        let admin = json!({ "id": "a1", "role": "admin" });
        let attrs = EvaluationAttributes { attributes: &admin };
        let (idx, val) = evaluate_flag(&artifact, 0, &attrs);
        assert_eq!(idx, Some(0));
        assert_eq!(val, Some(Value::String("ON".into())));

        let member = json!({ "id": "m1", "role": "member" });
        let attrs = EvaluationAttributes {
            attributes: &member,
        };
        let (idx, val) = evaluate_flag(&artifact, 0, &attrs);
        assert_eq!(idx, Some(1));
        assert_eq!(val, Some(Value::String("OFF".into())));
    }

    #[test]
    fn rollout_100_percent_always_matches() {
        let artifact = sample_artifact(vec![Rule::RolloutWithoutWhen(RolloutPayload {
            value_index: RolloutValue::Number(0),
            percentage: 100,
        })]);
        let user = json!({ "id": "any-user" });
        let attrs = EvaluationAttributes { attributes: &user };
        let (idx, val) = evaluate_flag(&artifact, 0, &attrs);
        assert_eq!(idx, Some(0));
        assert_eq!(val, Some(Value::String("ON".into())));
    }

    #[test]
    fn rollout_0_percent_never_matches() {
        let artifact = sample_artifact(vec![
            Rule::RolloutWithoutWhen(RolloutPayload {
                value_index: RolloutValue::Number(0),
                percentage: 0,
            }),
            Rule::ServeWithoutWhen(ServePayload::Number(1)),
        ]);
        let user = json!({ "id": "any-user" });
        let attrs = EvaluationAttributes { attributes: &user };
        let (idx, val) = evaluate_flag(&artifact, 0, &attrs);
        assert_eq!(idx, Some(1));
        assert_eq!(val, Some(Value::String("OFF".into())));
    }
}
