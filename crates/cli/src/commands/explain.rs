//! Explain command implementation

use crate::error::{CliError, CliResult};
use chrono::{Datelike, Timelike};
use controlpath_compiler::ast::{Artifact, BinaryOp, Expression, FuncCode, LogicalOp, Rule};
use regex::Regex;
use rmp_serde::from_slice;
use semver::Version;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub struct Options {
    pub flag: String,
    pub user: Option<String>,
    pub context: Option<String>,
    pub env: Option<String>,
    pub trace: bool,
    pub ast: Option<String>,
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
            let left_val = evaluate_expression(left, artifact, user, context);
            if *op_code == LogicalOp::Not as u8 {
                return Some(Value::Bool(!left_val));
            }
            let right_val = right
                .as_ref()
                .map(|r| evaluate_expression(r, artifact, user, context))?;
            let result = match *op_code {
                x if x == LogicalOp::And as u8 => left_val && right_val,
                x if x == LogicalOp::Or as u8 => left_val || right_val,
                _ => false,
            };
            Some(Value::Bool(result))
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
    match evaluate_expression_value(expr, artifact, user, context) {
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
    variations: &[controlpath_compiler::ast::Variation],
    artifact: &Artifact,
    user: &Value,
) -> Option<Value> {
    if variations.is_empty() {
        return None;
    }

    // Get user ID for consistent hashing
    let user_id = user
        .as_object()
        .and_then(|obj| obj.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let hash = hash_string(user_id);

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
fn select_rollout(user: &Value, pct: u8) -> bool {
    if pct == 0 {
        return false;
    }
    if pct >= 100 {
        return true;
    }

    // Get user ID for consistent hashing
    let user_id = user
        .as_object()
        .and_then(|obj| obj.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let hash = hash_string(user_id);
    let bucket = (hash % 100) as u8;

    bucket < pct
}

/// Evaluate a function call.
/// Returns the function result (which may be boolean, string, number, etc.).
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
            let list = evaluate_expression_value(&args[1], artifact, user, context)?;
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
            let arr1 = evaluate_expression_value(&args[0], artifact, user, context)?;
            let arr2 = evaluate_expression_value(&args[1], artifact, user, context)?;
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
            let id = evaluate_expression_value(&args[0], artifact, user, context)?;
            let buckets = evaluate_expression_value(&args[1], artifact, user, context)?;
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
                if let Some(value) = evaluate_expression_value(arg, artifact, user, context) {
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
            let v1 = evaluate_expression_value(&args[0], artifact, user, context)?;
            let v2 = evaluate_expression_value(&args[1], artifact, user, context)?;
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
            let v1 = evaluate_expression_value(&args[0], artifact, user, context)?;
            let v2 = evaluate_expression_value(&args[1], artifact, user, context)?;
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
            let v1 = evaluate_expression_value(&args[0], artifact, user, context)?;
            let v2 = evaluate_expression_value(&args[1], artifact, user, context)?;
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
            let v1 = evaluate_expression_value(&args[0], artifact, user, context)?;
            let v2 = evaluate_expression_value(&args[1], artifact, user, context)?;
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
            let v1 = evaluate_expression_value(&args[0], artifact, user, context)?;
            let v2 = evaluate_expression_value(&args[1], artifact, user, context)?;
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
            let start = evaluate_expression_value(&args[0], artifact, user, context)?;
            let end = evaluate_expression_value(&args[1], artifact, user, context)?;
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
            let timestamp = evaluate_expression_value(&args[0], artifact, user, context)?;
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
            let timestamp = evaluate_expression_value(&args[0], artifact, user, context)?;
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
            let _user_arg = evaluate_expression_value(&args[0], artifact, user, context);
            let segment_name = match evaluate_expression_value(&args[1], artifact, user, context) {
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

            // Evaluate segment expression (same as when clause)
            Some(Value::Bool(evaluate_expression(
                segment_expr,
                artifact,
                user,
                context,
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

/// Evaluate rule and return result with trace information
struct RuleEvaluation {
    matched: bool,
    value: Option<Value>,
    reason: String,
}

fn evaluate_rule(
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
                matched: true,
                value,
                reason: "Serve rule (no when clause)".to_string(),
            }
        }
        Rule::ServeWithWhen(when_expr, payload) => {
            let when_result = evaluate_expression(when_expr, artifact, user, context);
            if when_result {
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
            if let Some(value) = select_variation(variations, artifact, user) {
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
            let when_result = evaluate_expression(when_expr, artifact, user, context);
            if when_result {
                if let Some(value) = select_variation(variations, artifact, user) {
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
            if select_rollout(user, payload.percentage) {
                let value = match &payload.value_index {
                    controlpath_compiler::ast::RolloutValue::String(s) => {
                        Some(Value::String(s.clone()))
                    }
                    controlpath_compiler::ast::RolloutValue::Number(idx) => artifact
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
            let when_result = evaluate_expression(when_expr, artifact, user, context);
            if when_result {
                if select_rollout(user, payload.percentage) {
                    let value = match &payload.value_index {
                        controlpath_compiler::ast::RolloutValue::String(s) => {
                            Some(Value::String(s.clone()))
                        }
                        controlpath_compiler::ast::RolloutValue::Number(idx) => artifact
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

fn determine_ast_path(options: &Options) -> Result<PathBuf, CliError> {
    options.ast.as_ref().map_or_else(
        || {
            options.env.as_ref().map_or_else(
                || {
                    Err(CliError::Message(
                        "Either --ast <file> or --env <env> must be provided".to_string(),
                    ))
                },
                |env| Ok(PathBuf::from(format!(".controlpath/{env}.ast"))),
            )
        },
        |ast| Ok(PathBuf::from(ast)),
    )
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("✗ Explanation failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    // Determine AST path
    let ast_path = determine_ast_path(options)?;

    // Load AST file
    let ast_bytes = fs::read(&ast_path)
        .map_err(|e| CliError::Message(format!("Failed to read AST file: {e}")))?;

    // Deserialize AST
    let artifact: Artifact = from_slice(&ast_bytes)
        .map_err(|e| CliError::Message(format!("Failed to deserialize AST: {e}")))?;

    // Find flag by name
    let flag_index = find_flag_index(&artifact, &options.flag).ok_or_else(|| {
        CliError::Message(format!("Flag '{}' not found in artifact", options.flag))
    })?;

    let flag_rules = artifact
        .flags
        .get(flag_index)
        .ok_or_else(|| CliError::Message(format!("Flag '{}' has no rules", options.flag)))?;

    // Load user JSON
    let user_json = if let Some(user_path) = &options.user {
        let user_content = fs::read_to_string(user_path)
            .map_err(|e| CliError::Message(format!("Failed to read user file: {e}")))?;
        serde_json::from_str::<Value>(&user_content)
            .map_err(|e| CliError::Message(format!("Failed to parse user JSON: {e}")))?
    } else {
        Value::Object(serde_json::Map::new())
    };

    // Load context JSON (optional)
    let context_json = if let Some(context_path) = &options.context {
        let context_content = fs::read_to_string(context_path)
            .map_err(|e| CliError::Message(format!("Failed to read context file: {e}")))?;
        Some(
            serde_json::from_str::<Value>(&context_content)
                .map_err(|e| CliError::Message(format!("Failed to parse context JSON: {e}")))?,
        )
    } else {
        None
    };

    // Print header
    println!("Flag: {}", options.flag);
    println!("Environment: {}", artifact.environment);
    if let Some(user_obj) = user_json.as_object() {
        if let Some(id) = user_obj.get("id") {
            println!("User ID: {}", id);
        }
    }
    println!();

    // Evaluate rules
    let mut matched_rule_index = None;
    let mut final_value = None;

    for (rule_index, rule) in flag_rules.iter().enumerate() {
        let eval = evaluate_rule(rule, &artifact, &user_json, &context_json);

        if options.trace {
            println!("Rule {}:", rule_index + 1);
            match rule {
                Rule::ServeWithoutWhen(_) => println!("  Type: serve (no when clause)"),
                Rule::ServeWithWhen(when_expr, _) => {
                    println!("  Type: serve");
                    println!("  When: {}", format_expression(when_expr, &artifact, 0));
                    let when_result =
                        evaluate_expression(when_expr, &artifact, &user_json, &context_json);
                    println!("  When result: {}", when_result);
                }
                Rule::VariationsWithoutWhen(_) => {
                    println!("  Type: variations (no when clause)");
                }
                Rule::VariationsWithWhen(when_expr, _) => {
                    println!("  Type: variations");
                    println!("  When: {}", format_expression(when_expr, &artifact, 0));
                    let when_result =
                        evaluate_expression(when_expr, &artifact, &user_json, &context_json);
                    println!("  When result: {}", when_result);
                }
                Rule::RolloutWithoutWhen(payload) => {
                    println!("  Type: rollout (no when clause)");
                    println!("  Percentage: {}%", payload.percentage);
                }
                Rule::RolloutWithWhen(when_expr, payload) => {
                    println!("  Type: rollout");
                    println!("  When: {}", format_expression(when_expr, &artifact, 0));
                    let when_result =
                        evaluate_expression(when_expr, &artifact, &user_json, &context_json);
                    println!("  When result: {}", when_result);
                    println!("  Percentage: {}%", payload.percentage);
                }
            }
            println!("  Result: {}", eval.reason);
            if let Some(ref val) = eval.value {
                println!("  Value: {}", val);
            }
            println!();
        }

        if eval.matched {
            matched_rule_index = Some(rule_index);
            final_value = eval.value;
            // Always stop at first match (even in trace mode)
            // Trace mode shows all rules evaluated up to the match
            break;
        }
    }

    // Print result
    println!("Result:");
    if let Some(rule_idx) = matched_rule_index {
        println!("  ✓ Rule {} matched", rule_idx + 1);
        if let Some(ref val) = final_value {
            println!("  Value: {}", val);
        } else {
            println!("  Value: <none>");
        }
    } else {
        println!("  ✗ No rules matched");
        println!("  Value: <default or undefined>");
    }

    Ok(())
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
    fn test_determine_ast_path_with_ast() {
        let options = Options {
            flag: "test_flag".to_string(),
            user: None,
            context: None,
            env: None,
            trace: false,
            ast: Some("test.ast".to_string()),
        };
        let path = determine_ast_path(&options).unwrap();
        assert_eq!(path, PathBuf::from("test.ast"));
    }

    #[test]
    fn test_determine_ast_path_with_env() {
        let options = Options {
            flag: "test_flag".to_string(),
            user: None,
            context: None,
            env: Some("production".to_string()),
            trace: false,
            ast: None,
        };
        let path = determine_ast_path(&options).unwrap();
        assert_eq!(path, PathBuf::from(".controlpath/production.ast"));
    }

    #[test]
    fn test_determine_ast_path_without_options() {
        let options = Options {
            flag: "test_flag".to_string(),
            user: None,
            context: None,
            env: None,
            trace: false,
            ast: None,
        };
        assert!(determine_ast_path(&options).is_err());
    }

    #[test]
    fn test_format_expression_property() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["user.role".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };
        let expr = Expression::Property { prop_index: 0 };
        assert_eq!(format_expression(&expr, &artifact, 0), "user.role");
    }

    #[test]
    fn test_format_expression_literal() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };
        let expr = Expression::Literal {
            value: Value::String("admin".to_string()),
        };
        assert_eq!(format_expression(&expr, &artifact, 0), "\"admin\"");
    }

    #[test]
    fn test_evaluate_expression_property() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["user.role".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };
        let user = serde_json::json!({
            "id": "user-1",
            "role": "admin"
        });
        let expr = Expression::Property { prop_index: 0 };
        let result = evaluate_expression_value(&expr, &artifact, &user, &None);
        assert_eq!(result, Some(Value::String("admin".to_string())));
    }

    #[test]
    fn test_evaluate_expression_binary_eq() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["user.role".to_string(), "admin".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };
        let user = serde_json::json!({
            "id": "user-1",
            "role": "admin"
        });
        // Compare user.role (property) with "admin" (literal string)
        let expr = Expression::BinaryOp {
            op_code: BinaryOp::Eq as u8,
            left: Box::new(Expression::Property { prop_index: 0 }),
            right: Box::new(Expression::Literal {
                value: Value::String("admin".to_string()),
            }),
        };
        let result = evaluate_expression(&expr, &artifact, &user, &None);
        assert!(result); // user.role == "admin"
    }

    #[test]
    fn test_hash_string() {
        // Test that hash_string produces consistent results
        let hash1 = hash_string("test");
        let hash2 = hash_string("test");
        assert_eq!(hash1, hash2);

        // Test that different strings produce different hashes
        let hash3 = hash_string("different");
        assert_ne!(hash1, hash3);

        // Test empty string
        let hash_empty = hash_string("");
        assert!(hash_empty > 0);
    }

    #[test]
    fn test_select_variation() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["var1".to_string(), "var2".to_string(), "var3".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let variations = vec![
            controlpath_compiler::ast::Variation {
                var_index: 0,
                percentage: 50,
            },
            controlpath_compiler::ast::Variation {
                var_index: 1,
                percentage: 30,
            },
            controlpath_compiler::ast::Variation {
                var_index: 2,
                percentage: 20,
            },
        ];

        let user1 = serde_json::json!({"id": "user-1"});
        let user2 = serde_json::json!({"id": "user-2"});

        // Same user should get same variation
        let var1a = select_variation(&variations, &artifact, &user1);
        let var1b = select_variation(&variations, &artifact, &user1);
        assert_eq!(var1a, var1b);

        // Different users may get different variations
        let var2 = select_variation(&variations, &artifact, &user2);
        // At least one should be Some
        assert!(var1a.is_some() || var2.is_some());
    }

    #[test]
    fn test_select_variation_empty() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({"id": "user-1"});
        let result = select_variation(&[], &artifact, &user);
        assert_eq!(result, None);
    }

    #[test]
    fn test_select_variation_zero_percentage() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["var1".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let variations = vec![controlpath_compiler::ast::Variation {
            var_index: 0,
            percentage: 0,
        }];

        let user = serde_json::json!({"id": "user-1"});
        // Should return first variation when total percentage is 0
        let result = select_variation(&variations, &artifact, &user);
        assert_eq!(result, Some(Value::String("var1".to_string())));
    }

    #[test]
    fn test_select_rollout() {
        let user1 = serde_json::json!({"id": "user-1"});
        let user2 = serde_json::json!({"id": "user-2"});

        // 100% rollout should always return true
        assert!(select_rollout(&user1, 100));
        assert!(select_rollout(&user2, 100));

        // 0% rollout should always return false
        assert!(!select_rollout(&user1, 0));
        assert!(!select_rollout(&user2, 0));

        // Same user should get consistent result
        let result1a = select_rollout(&user1, 50);
        let result1b = select_rollout(&user1, 50);
        assert_eq!(result1a, result1b);
    }

    #[test]
    fn test_select_rollout_no_user_id() {
        let user = serde_json::json!({});
        // Should use empty string as user ID
        let result = select_rollout(&user, 50);
        // Should be consistent
        let result2 = select_rollout(&user, 50);
        assert_eq!(result, result2);
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
        assert_eq!(
            coerce_to_boolean(&Value::String("1".to_string())),
            Some(true)
        );
        assert_eq!(
            coerce_to_boolean(&Value::String("0".to_string())),
            Some(false)
        );
        assert_eq!(coerce_to_boolean(&Value::Number(1.into())), Some(true));
        assert_eq!(coerce_to_boolean(&Value::Number(0.into())), Some(false));
        assert_eq!(coerce_to_boolean(&Value::Null), None);
        assert_eq!(coerce_to_boolean(&Value::String("maybe".to_string())), None);
    }

    #[test]
    fn test_coerce_and_compare_with_boolean() {
        // Boolean coercion should work
        assert_eq!(
            coerce_and_compare(&Value::Bool(true), &Value::String("true".to_string())),
            0
        );
        assert_eq!(
            coerce_and_compare(&Value::Bool(false), &Value::String("false".to_string())),
            0
        );
        assert_ne!(
            coerce_and_compare(&Value::Bool(true), &Value::Bool(false)),
            0
        );
    }

    #[test]
    fn test_evaluate_expression_null_values() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let expr = Expression::Literal { value: Value::Null };
        let result = evaluate_expression(&expr, &artifact, &user, &None);
        assert!(!result); // null should evaluate to false
    }

    #[test]
    fn test_evaluate_expression_missing_property() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["user.missing".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({"id": "user-1"});
        let expr = Expression::Property { prop_index: 0 };
        let result = evaluate_expression_value(&expr, &artifact, &user, &None);
        assert_eq!(result, None); // Missing property should return None
    }

    #[test]
    fn test_evaluate_function_starts_with() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args = vec![
            Expression::Literal {
                value: Value::String("hello world".to_string()),
            },
            Expression::Literal {
                value: Value::String("hello".to_string()),
            },
        ];
        let result = evaluate_function(FuncCode::StartsWith as u8, &args, &artifact, &user, &None);
        assert_eq!(result, Some(Value::Bool(true)));

        let args2 = vec![
            Expression::Literal {
                value: Value::String("hello world".to_string()),
            },
            Expression::Literal {
                value: Value::String("world".to_string()),
            },
        ];
        let result2 =
            evaluate_function(FuncCode::StartsWith as u8, &args2, &artifact, &user, &None);
        assert_eq!(result2, Some(Value::Bool(false)));
    }

    #[test]
    fn test_evaluate_function_contains() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args = vec![
            Expression::Literal {
                value: Value::String("hello world".to_string()),
            },
            Expression::Literal {
                value: Value::String("world".to_string()),
            },
        ];
        let result = evaluate_function(FuncCode::Contains as u8, &args, &artifact, &user, &None);
        assert_eq!(result, Some(Value::Bool(true)));

        // Test with array
        let args2 = vec![
            Expression::Literal {
                value: Value::Array(vec![
                    Value::String("a".to_string()),
                    Value::String("b".to_string()),
                ]),
            },
            Expression::Literal {
                value: Value::String("a".to_string()),
            },
        ];
        let result2 = evaluate_function(FuncCode::Contains as u8, &args2, &artifact, &user, &None);
        assert_eq!(result2, Some(Value::Bool(true)));
    }

    #[test]
    fn test_evaluate_function_semver_eq() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args = vec![
            Expression::Literal {
                value: Value::String("1.0.0".to_string()),
            },
            Expression::Literal {
                value: Value::String("1.0.0".to_string()),
            },
        ];
        let result = evaluate_function(FuncCode::SemverEq as u8, &args, &artifact, &user, &None);
        assert_eq!(result, Some(Value::Bool(true)));

        let args2 = vec![
            Expression::Literal {
                value: Value::String("1.0.0".to_string()),
            },
            Expression::Literal {
                value: Value::String("2.0.0".to_string()),
            },
        ];
        let result2 = evaluate_function(FuncCode::SemverEq as u8, &args2, &artifact, &user, &None);
        assert_eq!(result2, Some(Value::Bool(false)));
    }

    #[test]
    fn test_evaluate_function_semver_comparison() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args = vec![
            Expression::Literal {
                value: Value::String("2.0.0".to_string()),
            },
            Expression::Literal {
                value: Value::String("1.0.0".to_string()),
            },
        ];

        let gt = evaluate_function(FuncCode::SemverGt as u8, &args, &artifact, &user, &None);
        assert_eq!(gt, Some(Value::Bool(true)));

        let gte = evaluate_function(FuncCode::SemverGte as u8, &args, &artifact, &user, &None);
        assert_eq!(gte, Some(Value::Bool(true)));

        let args2 = vec![
            Expression::Literal {
                value: Value::String("1.0.0".to_string()),
            },
            Expression::Literal {
                value: Value::String("2.0.0".to_string()),
            },
        ];

        let lt = evaluate_function(FuncCode::SemverLt as u8, &args2, &artifact, &user, &None);
        assert_eq!(lt, Some(Value::Bool(true)));

        let lte = evaluate_function(FuncCode::SemverLte as u8, &args2, &artifact, &user, &None);
        assert_eq!(lte, Some(Value::Bool(true)));
    }

    #[test]
    fn test_evaluate_function_temporal() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});

        // Test hourOfDay - should return a number 0-23
        let result = evaluate_function(FuncCode::HourOfDay as u8, &[], &artifact, &user, &None);
        assert!(result.is_some());
        if let Some(Value::Number(n)) = result {
            let hour = n.as_u64().unwrap();
            assert!(hour < 24);
        }

        // Test dayOfWeek - should return a day name
        let result = evaluate_function(FuncCode::DayOfWeek as u8, &[], &artifact, &user, &None);
        assert!(result.is_some());
        if let Some(Value::String(day)) = result {
            assert!([
                "SUNDAY",
                "MONDAY",
                "TUESDAY",
                "WEDNESDAY",
                "THURSDAY",
                "FRIDAY",
                "SATURDAY"
            ]
            .contains(&day.as_str()));
        }

        // Test currentTimestamp - should return ISO 8601 string
        let result = evaluate_function(
            FuncCode::CurrentTimestamp as u8,
            &[],
            &artifact,
            &user,
            &None,
        );
        assert!(result.is_some());
        if let Some(Value::String(ts)) = result {
            // Should be valid RFC3339 format
            assert!(chrono::DateTime::parse_from_rfc3339(&ts).is_ok());
        }
    }

    #[test]
    fn test_evaluate_function_hash() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args = vec![
            Expression::Literal {
                value: Value::String("test-id".to_string()),
            },
            Expression::Literal {
                value: Value::Number(10.into()),
            },
        ];

        let result = evaluate_function(FuncCode::Hash as u8, &args, &artifact, &user, &None);
        assert!(result.is_some());
        if let Some(Value::Number(ref n)) = result {
            let bucket = n.as_u64().unwrap();
            assert!(bucket < 10); // Should be in range 0-9
        }

        // Same ID should produce same bucket
        let result2 = evaluate_function(FuncCode::Hash as u8, &args, &artifact, &user, &None);
        assert_eq!(result, result2);
    }

    #[test]
    fn test_evaluate_function_coalesce() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args = vec![
            Expression::Literal { value: Value::Null },
            Expression::Literal {
                value: Value::String("fallback".to_string()),
            },
        ];

        let result = evaluate_function(FuncCode::Coalesce as u8, &args, &artifact, &user, &None);
        assert_eq!(result, Some(Value::String("fallback".to_string())));

        // First non-null should be returned
        let args2 = vec![
            Expression::Literal {
                value: Value::String("first".to_string()),
            },
            Expression::Literal {
                value: Value::String("second".to_string()),
            },
        ];
        let result2 = evaluate_function(FuncCode::Coalesce as u8, &args2, &artifact, &user, &None);
        assert_eq!(result2, Some(Value::String("first".to_string())));
    }

    #[test]
    fn test_evaluate_rule_variations_with_hashing() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["var1".to_string(), "var2".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let variations = vec![
            controlpath_compiler::ast::Variation {
                var_index: 0,
                percentage: 50,
            },
            controlpath_compiler::ast::Variation {
                var_index: 1,
                percentage: 50,
            },
        ];

        let user = serde_json::json!({"id": "user-1"});
        let rule = Rule::VariationsWithoutWhen(variations.clone());
        let eval = evaluate_rule(&rule, &artifact, &user, &None);
        assert!(eval.matched);
        assert!(eval.value.is_some());

        // Same user should get same variation
        let eval2 = evaluate_rule(&rule, &artifact, &user, &None);
        assert_eq!(eval.value, eval2.value);
    }

    #[test]
    fn test_evaluate_rule_rollout_with_hashing() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["value".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user1 = serde_json::json!({"id": "user-1"});
        let user2 = serde_json::json!({"id": "user-2"});

        let payload = controlpath_compiler::ast::RolloutPayload {
            value_index: controlpath_compiler::ast::RolloutValue::Number(0),
            percentage: 50,
        };

        let rule = Rule::RolloutWithoutWhen(payload.clone());
        let eval1 = evaluate_rule(&rule, &artifact, &user1, &None);
        let _eval2 = evaluate_rule(&rule, &artifact, &user2, &None);

        // Same user should get consistent result
        let eval1b = evaluate_rule(&rule, &artifact, &user1, &None);
        assert_eq!(eval1.matched, eval1b.matched);
    }

    #[test]
    fn test_evaluate_rule_rollout_100_percent() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["value".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({"id": "user-1"});
        let payload = controlpath_compiler::ast::RolloutPayload {
            value_index: controlpath_compiler::ast::RolloutValue::Number(0),
            percentage: 100,
        };

        let rule = Rule::RolloutWithoutWhen(payload);
        let eval = evaluate_rule(&rule, &artifact, &user, &None);
        assert!(eval.matched); // 100% should always match
    }

    #[test]
    fn test_evaluate_rule_rollout_0_percent() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["value".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({"id": "user-1"});
        let payload = controlpath_compiler::ast::RolloutPayload {
            value_index: controlpath_compiler::ast::RolloutValue::Number(0),
            percentage: 0,
        };

        let rule = Rule::RolloutWithoutWhen(payload);
        let eval = evaluate_rule(&rule, &artifact, &user, &None);
        assert!(!eval.matched); // 0% should never match
    }

    #[test]
    fn test_evaluate_expression_logical_ops() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});

        // Test AND
        let expr = Expression::LogicalOp {
            op_code: LogicalOp::And as u8,
            left: Box::new(Expression::Literal {
                value: Value::Bool(true),
            }),
            right: Some(Box::new(Expression::Literal {
                value: Value::Bool(true),
            })),
        };
        assert!(evaluate_expression(&expr, &artifact, &user, &None));

        let expr2 = Expression::LogicalOp {
            op_code: LogicalOp::And as u8,
            left: Box::new(Expression::Literal {
                value: Value::Bool(true),
            }),
            right: Some(Box::new(Expression::Literal {
                value: Value::Bool(false),
            })),
        };
        assert!(!evaluate_expression(&expr2, &artifact, &user, &None));

        // Test OR
        let expr3 = Expression::LogicalOp {
            op_code: LogicalOp::Or as u8,
            left: Box::new(Expression::Literal {
                value: Value::Bool(false),
            }),
            right: Some(Box::new(Expression::Literal {
                value: Value::Bool(true),
            })),
        };
        assert!(evaluate_expression(&expr3, &artifact, &user, &None));

        // Test NOT
        let expr4 = Expression::LogicalOp {
            op_code: LogicalOp::Not as u8,
            left: Box::new(Expression::Literal {
                value: Value::Bool(true),
            }),
            right: None,
        };
        assert!(!evaluate_expression(&expr4, &artifact, &user, &None));
    }

    #[test]
    fn test_get_property_nested() {
        let user = serde_json::json!({
            "id": "user-1",
            "profile": {
                "name": "John",
                "settings": {
                    "theme": "dark"
                }
            }
        });

        // Test nested property access
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["profile.settings.theme".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let expr = Expression::Property { prop_index: 0 };
        let result = evaluate_expression_value(&expr, &artifact, &user, &None);
        assert_eq!(result, Some(Value::String("dark".to_string())));
    }

    #[test]
    fn test_get_property_prototype_pollution() {
        let user = serde_json::json!({"id": "user-1"});
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["__proto__.polluted".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let expr = Expression::Property { prop_index: 0 };
        let result = evaluate_expression_value(&expr, &artifact, &user, &None);
        // Should return None to prevent prototype pollution
        assert_eq!(result, None);
    }

    // Additional function tests
    #[test]
    fn test_evaluate_function_ends_with() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args = vec![
            Expression::Literal {
                value: Value::String("hello world".to_string()),
            },
            Expression::Literal {
                value: Value::String("world".to_string()),
            },
        ];
        let result = evaluate_function(FuncCode::EndsWith as u8, &args, &artifact, &user, &None);
        assert_eq!(result, Some(Value::Bool(true)));

        let args2 = vec![
            Expression::Literal {
                value: Value::String("hello world".to_string()),
            },
            Expression::Literal {
                value: Value::String("hello".to_string()),
            },
        ];
        let result2 = evaluate_function(FuncCode::EndsWith as u8, &args2, &artifact, &user, &None);
        assert_eq!(result2, Some(Value::Bool(false)));
    }

    #[test]
    fn test_evaluate_function_matches() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args = vec![
            Expression::Literal {
                value: Value::String("hello123".to_string()),
            },
            Expression::Literal {
                value: Value::String(r"\d+".to_string()),
            },
        ];
        let result = evaluate_function(FuncCode::Matches as u8, &args, &artifact, &user, &None);
        assert_eq!(result, Some(Value::Bool(true)));

        // Invalid regex should return false
        let args2 = vec![
            Expression::Literal {
                value: Value::String("test".to_string()),
            },
            Expression::Literal {
                value: Value::String("[invalid".to_string()),
            },
        ];
        let result2 = evaluate_function(FuncCode::Matches as u8, &args2, &artifact, &user, &None);
        assert_eq!(result2, Some(Value::Bool(false)));
    }

    #[test]
    fn test_evaluate_function_upper_lower() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args = vec![Expression::Literal {
            value: Value::String("Hello World".to_string()),
        }];

        let upper = evaluate_function(FuncCode::Upper as u8, &args, &artifact, &user, &None);
        assert_eq!(upper, Some(Value::String("HELLO WORLD".to_string())));

        let lower = evaluate_function(FuncCode::Lower as u8, &args, &artifact, &user, &None);
        assert_eq!(lower, Some(Value::String("hello world".to_string())));
    }

    #[test]
    fn test_evaluate_function_length() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args_str = vec![Expression::Literal {
            value: Value::String("hello".to_string()),
        }];
        let result_str =
            evaluate_function(FuncCode::Length as u8, &args_str, &artifact, &user, &None);
        assert_eq!(result_str, Some(Value::Number(5.into())));

        let args_arr = vec![Expression::Literal {
            value: Value::Array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
            ]),
        }];
        let result_arr =
            evaluate_function(FuncCode::Length as u8, &args_arr, &artifact, &user, &None);
        assert_eq!(result_arr, Some(Value::Number(2.into())));
    }

    #[test]
    fn test_evaluate_function_in() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args = vec![
            Expression::Literal {
                value: Value::String("a".to_string()),
            },
            Expression::Literal {
                value: Value::Array(vec![
                    Value::String("a".to_string()),
                    Value::String("b".to_string()),
                    Value::String("c".to_string()),
                ]),
            },
        ];
        let result = evaluate_function(FuncCode::In as u8, &args, &artifact, &user, &None);
        assert_eq!(result, Some(Value::Bool(true)));

        let args2 = vec![
            Expression::Literal {
                value: Value::String("d".to_string()),
            },
            Expression::Literal {
                value: Value::Array(vec![
                    Value::String("a".to_string()),
                    Value::String("b".to_string()),
                ]),
            },
        ];
        let result2 = evaluate_function(FuncCode::In as u8, &args2, &artifact, &user, &None);
        assert_eq!(result2, Some(Value::Bool(false)));
    }

    #[test]
    fn test_evaluate_function_intersects() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args = vec![
            Expression::Literal {
                value: Value::Array(vec![
                    Value::String("a".to_string()),
                    Value::String("b".to_string()),
                ]),
            },
            Expression::Literal {
                value: Value::Array(vec![
                    Value::String("b".to_string()),
                    Value::String("c".to_string()),
                ]),
            },
        ];
        let result = evaluate_function(FuncCode::Intersects as u8, &args, &artifact, &user, &None);
        assert_eq!(result, Some(Value::Bool(true)));

        let args2 = vec![
            Expression::Literal {
                value: Value::Array(vec![Value::String("a".to_string())]),
            },
            Expression::Literal {
                value: Value::Array(vec![Value::String("b".to_string())]),
            },
        ];
        let result2 =
            evaluate_function(FuncCode::Intersects as u8, &args2, &artifact, &user, &None);
        assert_eq!(result2, Some(Value::Bool(false)));
    }

    #[test]
    fn test_evaluate_function_temporal_is_between() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let now = chrono::Utc::now();
        let start = (now - chrono::Duration::hours(1)).to_rfc3339();
        let end = (now + chrono::Duration::hours(1)).to_rfc3339();

        let args = vec![
            Expression::Literal {
                value: Value::String(start.clone()),
            },
            Expression::Literal {
                value: Value::String(end),
            },
        ];
        let result = evaluate_function(FuncCode::IsBetween as u8, &args, &artifact, &user, &None);
        assert_eq!(result, Some(Value::Bool(true)));

        // Test with past end time
        let past_end = (now - chrono::Duration::hours(2)).to_rfc3339();
        let args2 = vec![
            Expression::Literal {
                value: Value::String(start),
            },
            Expression::Literal {
                value: Value::String(past_end),
            },
        ];
        let result2 = evaluate_function(FuncCode::IsBetween as u8, &args2, &artifact, &user, &None);
        assert_eq!(result2, Some(Value::Bool(false)));
    }

    #[test]
    fn test_evaluate_function_temporal_is_after_before() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let past = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let future = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();

        let args_after = vec![Expression::Literal {
            value: Value::String(past),
        }];
        let result_after = evaluate_function(
            FuncCode::IsAfter as u8,
            &args_after,
            &artifact,
            &user,
            &None,
        );
        assert_eq!(result_after, Some(Value::Bool(true)));

        let args_before = vec![Expression::Literal {
            value: Value::String(future),
        }];
        let result_before = evaluate_function(
            FuncCode::IsBefore as u8,
            &args_before,
            &artifact,
            &user,
            &None,
        );
        assert_eq!(result_before, Some(Value::Bool(true)));
    }

    #[test]
    fn test_evaluate_function_temporal_day_of_month_month() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});

        let day_result =
            evaluate_function(FuncCode::DayOfMonth as u8, &[], &artifact, &user, &None);
        assert!(day_result.is_some());
        if let Some(Value::Number(n)) = day_result {
            let day = n.as_u64().unwrap();
            assert!((1..=31).contains(&day));
        }

        let month_result = evaluate_function(FuncCode::Month as u8, &[], &artifact, &user, &None);
        assert!(month_result.is_some());
        if let Some(Value::Number(n)) = month_result {
            let month = n.as_u64().unwrap();
            assert!((1..=12).contains(&month));
        }
    }

    #[test]
    fn test_evaluate_function_in_segment() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![
                "premium_users".to_string(),
                "user.plan".to_string(),
                "premium".to_string(),
            ],
            flags: vec![],
            flag_names: vec![],
            segments: Some(vec![(
                0, // segment name index
                Expression::BinaryOp {
                    op_code: BinaryOp::Eq as u8,
                    left: Box::new(Expression::Property { prop_index: 1 }),
                    right: Box::new(Expression::Literal {
                        value: Value::Number(2.into()),
                    }),
                },
            )]),
            signature: None,
        };

        let user = serde_json::json!({
            "id": "user-1",
            "plan": "premium"
        });

        let args = vec![
            Expression::Literal {
                value: Value::String("user".to_string()),
            },
            Expression::Literal {
                value: Value::String("premium_users".to_string()),
            },
        ];
        let result = evaluate_function(FuncCode::InSegment as u8, &args, &artifact, &user, &None);
        assert_eq!(result, Some(Value::Bool(true)));

        // Test with non-matching user
        let user2 = serde_json::json!({
            "id": "user-2",
            "plan": "free"
        });
        let result2 = evaluate_function(FuncCode::InSegment as u8, &args, &artifact, &user2, &None);
        assert_eq!(result2, Some(Value::Bool(false)));

        // Test with non-existent segment
        let args3 = vec![
            Expression::Literal {
                value: Value::String("user".to_string()),
            },
            Expression::Literal {
                value: Value::String("nonexistent".to_string()),
            },
        ];
        let result3 = evaluate_function(FuncCode::InSegment as u8, &args3, &artifact, &user, &None);
        assert_eq!(result3, Some(Value::Bool(false)));
    }

    // Additional binary operator tests
    #[test]
    fn test_evaluate_expression_binary_ne() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["user.role".to_string(), "admin".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };
        let user = serde_json::json!({
            "id": "user-1",
            "role": "admin"
        });
        let expr = Expression::BinaryOp {
            op_code: BinaryOp::Ne as u8,
            left: Box::new(Expression::Property { prop_index: 0 }),
            right: Box::new(Expression::Literal {
                value: Value::String("user".to_string()),
            }),
        };
        let result = evaluate_expression(&expr, &artifact, &user, &None);
        assert!(result); // user.role != "user"
    }

    #[test]
    fn test_evaluate_expression_binary_comparison_operators() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };
        let user = serde_json::json!({});

        // Test >
        let expr_gt = Expression::BinaryOp {
            op_code: BinaryOp::Gt as u8,
            left: Box::new(Expression::Literal {
                value: Value::Number(10.into()),
            }),
            right: Box::new(Expression::Literal {
                value: Value::Number(5.into()),
            }),
        };
        assert!(evaluate_expression(&expr_gt, &artifact, &user, &None));

        // Test <
        let expr_lt = Expression::BinaryOp {
            op_code: BinaryOp::Lt as u8,
            left: Box::new(Expression::Literal {
                value: Value::Number(5.into()),
            }),
            right: Box::new(Expression::Literal {
                value: Value::Number(10.into()),
            }),
        };
        assert!(evaluate_expression(&expr_lt, &artifact, &user, &None));

        // Test >=
        let expr_gte = Expression::BinaryOp {
            op_code: BinaryOp::Gte as u8,
            left: Box::new(Expression::Literal {
                value: Value::Number(10.into()),
            }),
            right: Box::new(Expression::Literal {
                value: Value::Number(10.into()),
            }),
        };
        assert!(evaluate_expression(&expr_gte, &artifact, &user, &None));

        // Test <=
        let expr_lte = Expression::BinaryOp {
            op_code: BinaryOp::Lte as u8,
            left: Box::new(Expression::Literal {
                value: Value::Number(5.into()),
            }),
            right: Box::new(Expression::Literal {
                value: Value::Number(10.into()),
            }),
        };
        assert!(evaluate_expression(&expr_lte, &artifact, &user, &None));
    }

    #[test]
    fn test_evaluate_expression_type_coercion() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };
        let user = serde_json::json!({});

        // String vs number coercion
        let expr = Expression::BinaryOp {
            op_code: BinaryOp::Eq as u8,
            left: Box::new(Expression::Literal {
                value: Value::String("10".to_string()),
            }),
            right: Box::new(Expression::Literal {
                value: Value::Number(10.into()),
            }),
        };
        let result = evaluate_expression(&expr, &artifact, &user, &None);
        assert!(result); // "10" == 10 should be true with coercion

        // Boolean coercion
        let expr2 = Expression::BinaryOp {
            op_code: BinaryOp::Eq as u8,
            left: Box::new(Expression::Literal {
                value: Value::Bool(true),
            }),
            right: Box::new(Expression::Literal {
                value: Value::String("true".to_string()),
            }),
        };
        let result2 = evaluate_expression(&expr2, &artifact, &user, &None);
        assert!(result2); // true == "true" should be true with coercion
    }

    // Additional edge cases
    #[test]
    fn test_select_variation_percentages_over_100() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["var1".to_string(), "var2".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let variations = vec![
            controlpath_compiler::ast::Variation {
                var_index: 0,
                percentage: 60,
            },
            controlpath_compiler::ast::Variation {
                var_index: 1,
                percentage: 50, // Total > 100
            },
        ];

        let user = serde_json::json!({"id": "user-1"});
        // Should still work, just uses cumulative percentages
        let result = select_variation(&variations, &artifact, &user);
        assert!(result.is_some());
    }

    #[test]
    fn test_select_variation_invalid_string_table_index() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["var1".to_string()], // Only one entry
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let variations = vec![controlpath_compiler::ast::Variation {
            var_index: 999, // Invalid index
            percentage: 100,
        }];

        let user = serde_json::json!({"id": "user-1"});
        let result = select_variation(&variations, &artifact, &user);
        // Should return None for invalid index
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluate_rule_rollout_invalid_string_table_index() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["value".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({"id": "user-1"});
        let payload = controlpath_compiler::ast::RolloutPayload {
            value_index: controlpath_compiler::ast::RolloutValue::Number(999), // Invalid index
            percentage: 50,
        };

        let rule = Rule::RolloutWithoutWhen(payload);
        let eval = evaluate_rule(&rule, &artifact, &user, &None);
        // Should match but value should be None
        if eval.matched {
            assert_eq!(eval.value, None);
        }
    }

    #[test]
    fn test_evaluate_function_invalid_arguments() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});

        // Function with wrong argument count
        let args_empty = vec![];
        let result = evaluate_function(
            FuncCode::StartsWith as u8,
            &args_empty,
            &artifact,
            &user,
            &None,
        );
        assert_eq!(result, Some(Value::Bool(false)));

        // Function with wrong types
        let args_wrong_type = vec![
            Expression::Literal {
                value: Value::Number(123.into()),
            },
            Expression::Literal {
                value: Value::Number(456.into()),
            },
        ];
        let result2 = evaluate_function(
            FuncCode::StartsWith as u8,
            &args_wrong_type,
            &artifact,
            &user,
            &None,
        );
        assert_eq!(result2, Some(Value::Bool(false)));
    }

    #[test]
    fn test_evaluate_function_semver_invalid() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({});
        let args = vec![
            Expression::Literal {
                value: Value::String("not-a-version".to_string()),
            },
            Expression::Literal {
                value: Value::String("1.0.0".to_string()),
            },
        ];
        let result = evaluate_function(FuncCode::SemverEq as u8, &args, &artifact, &user, &None);
        assert_eq!(result, Some(Value::Bool(false))); // Invalid semver should return false
    }

    #[test]
    fn test_get_property_deeply_nested() {
        let user = serde_json::json!({
            "id": "user-1",
            "level1": {
                "level2": {
                    "level3": {
                        "level4": {
                            "value": "deep"
                        }
                    }
                }
            }
        });

        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["level1.level2.level3.level4.value".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let expr = Expression::Property { prop_index: 0 };
        let result = evaluate_expression_value(&expr, &artifact, &user, &None);
        assert_eq!(result, Some(Value::String("deep".to_string())));
    }

    #[test]
    fn test_get_property_from_context() {
        let user = serde_json::json!({"id": "user-1"});
        let context = Some(serde_json::json!({
            "environment": "production",
            "device": {
                "type": "mobile"
            }
        }));

        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![
                "context.environment".to_string(),
                "context.device.type".to_string(),
            ],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let expr1 = Expression::Property { prop_index: 0 };
        let result1 = evaluate_expression_value(&expr1, &artifact, &user, &context);
        assert_eq!(result1, Some(Value::String("production".to_string())));

        let expr2 = Expression::Property { prop_index: 1 };
        let result2 = evaluate_expression_value(&expr2, &artifact, &user, &context);
        assert_eq!(result2, Some(Value::String("mobile".to_string())));
    }

    #[test]
    fn test_get_property_context_fallback() {
        // Test that properties fall back to context if not in user
        let user = serde_json::json!({"id": "user-1"});
        let context = Some(serde_json::json!({
            "custom_prop": "from_context"
        }));

        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec!["custom_prop".to_string()],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };

        let expr = Expression::Property { prop_index: 0 };
        let result = evaluate_expression_value(&expr, &artifact, &user, &context);
        assert_eq!(result, Some(Value::String("from_context".to_string())));
    }

    // Integration-style tests
    #[test]
    fn test_evaluate_multiple_rules_sequence() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![
                "user.role".to_string(),
                "admin".to_string(),
                "ON".to_string(),
                "OFF".to_string(),
            ],
            flags: vec![vec![
                // First rule: serve with when clause (should not match)
                Rule::ServeWithWhen(
                    Expression::BinaryOp {
                        op_code: BinaryOp::Eq as u8,
                        left: Box::new(Expression::Property { prop_index: 0 }),
                        right: Box::new(Expression::Literal {
                            value: Value::Number(1.into()),
                        }),
                    },
                    ServePayload::Number(2),
                ),
                // Second rule: serve without when (should match)
                Rule::ServeWithoutWhen(ServePayload::Number(3)),
            ]],
            flag_names: vec![0],
            segments: None,
            signature: None,
        };

        let user = serde_json::json!({"id": "user-1", "role": "user"});
        let flag_rules = &artifact.flags[0];

        let mut matched = false;
        for rule in flag_rules {
            let eval = evaluate_rule(rule, &artifact, &user, &None);
            if eval.matched {
                matched = true;
                assert_eq!(eval.value, Some(Value::String("OFF".to_string())));
                break;
            }
        }
        assert!(matched);
    }

    #[test]
    fn test_evaluate_expression_with_null_comparison() {
        let artifact = Artifact {
            version: "1.0".to_string(),
            environment: "test".to_string(),
            string_table: vec![],
            flags: vec![],
            flag_names: vec![],
            segments: None,
            signature: None,
        };
        let user = serde_json::json!({});

        // null == null should be true
        let expr_eq = Expression::BinaryOp {
            op_code: BinaryOp::Eq as u8,
            left: Box::new(Expression::Literal { value: Value::Null }),
            right: Box::new(Expression::Literal { value: Value::Null }),
        };
        assert!(evaluate_expression(&expr_eq, &artifact, &user, &None));

        // null != null should be false
        let expr_ne = Expression::BinaryOp {
            op_code: BinaryOp::Ne as u8,
            left: Box::new(Expression::Literal { value: Value::Null }),
            right: Box::new(Expression::Literal { value: Value::Null }),
        };
        assert!(!evaluate_expression(&expr_ne, &artifact, &user, &None));

        // null > anything should be false
        let expr_gt = Expression::BinaryOp {
            op_code: BinaryOp::Gt as u8,
            left: Box::new(Expression::Literal { value: Value::Null }),
            right: Box::new(Expression::Literal {
                value: Value::Number(5.into()),
            }),
        };
        assert!(!evaluate_expression(&expr_gt, &artifact, &user, &None));
    }
}
