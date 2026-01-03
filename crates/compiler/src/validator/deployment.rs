/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use serde_json::Value;

use crate::validator::common::validate_with_schema;
use crate::validator::constants::{MAX_PERCENTAGE, MIN_PERCENTAGE};
use crate::validator::error::{ValidationError, ValidationResult};
use crate::validator::type_guards::{is_deployment, is_record, is_rollout, is_variation};

/// Validate deployment file against the deployment schema.
pub fn validate_deployment(schema: &Value, file_path: &str, data: &Value) -> ValidationResult {
    validate_with_schema(schema, file_path, data, validate_deployment_specific_rules)
}

/// Validate deployment-specific business rules that aren't covered by JSON schema.
fn validate_deployment_specific_rules(file_path: &str, data: &Value) -> Vec<ValidationError> {
    if !is_deployment(data) {
        return Vec::new();
    }

    let rules_obj = match data.as_object().and_then(|obj| obj.get("rules")) {
        Some(r) if r.is_object() => r.as_object().unwrap(),
        _ => return Vec::new(),
    };

    let mut errors = Vec::new();

    for (flag_name, flag_rules_value) in rules_obj {
        if !is_record(flag_rules_value) {
            continue;
        }

        let rules_array = flag_rules_value
            .as_object()
            .and_then(|obj| obj.get("rules"))
            .and_then(|r| r.as_array());

        if let Some(rules_array) = rules_array {
            for (rule_index, rule) in rules_array.iter().enumerate() {
                if !is_record(rule) {
                    continue;
                }

                errors.extend(validate_rule_structure(
                    file_path, flag_name, rule, rule_index,
                ));
                errors.extend(validate_variation_weights(
                    file_path, flag_name, rule, rule_index,
                ));
                errors.extend(validate_rollout_percentage(
                    file_path, flag_name, rule, rule_index,
                ));
            }
        }
    }

    errors
}

/// Validate that a rule has at least one of: serve, variations, or rollout.
fn validate_rule_structure(
    file_path: &str,
    flag_name: &str,
    rule: &Value,
    rule_index: usize,
) -> Vec<ValidationError> {
    let rule_obj = match rule.as_object() {
        Some(obj) => obj,
        None => return Vec::new(),
    };

    let has_serve = rule_obj.contains_key("serve");
    let has_variations = rule_obj
        .get("variations")
        .map(|v| v.is_array())
        .unwrap_or(false);
    let has_rollout = rule_obj.get("rollout").map(is_rollout).unwrap_or(false);

    if !has_serve && !has_variations && !has_rollout {
        return vec![ValidationError {
            file: file_path.to_string(),
            line: None,
            column: None,
            message: format!(
                "Rule in flag '{flag_name}' must have 'serve', 'variations', or 'rollout'"
            ),
            path: Some(format!("/rules/{flag_name}/rules/{rule_index}")),
            suggestion: Some("Add 'serve', 'variations', or 'rollout' to this rule.".to_string()),
        }];
    }

    Vec::new()
}

/// Validate that variation weights don't exceed 100%.
fn validate_variation_weights(
    file_path: &str,
    flag_name: &str,
    rule: &Value,
    rule_index: usize,
) -> Vec<ValidationError> {
    let rule_obj = match rule.as_object() {
        Some(obj) => obj,
        None => return Vec::new(),
    };

    let variations = match rule_obj.get("variations") {
        Some(v) if v.is_array() => v.as_array().unwrap(),
        _ => return Vec::new(),
    };

    let total_weight: u32 = variations
        .iter()
        .filter_map(|v| {
            if is_variation(v) {
                v.as_object()
                    .and_then(|obj| obj.get("weight"))
                    .and_then(|w| w.as_u64())
                    .map(|w| w as u32)
            } else {
                None
            }
        })
        .sum();

    if total_weight > MAX_PERCENTAGE {
        return vec![ValidationError {
            file: file_path.to_string(),
            line: None,
            column: None,
            message: format!(
                "Variation weights for flag '{flag_name}' exceed {MAX_PERCENTAGE}% (total: {total_weight}%)"
            ),
            path: Some(format!("/rules/{flag_name}/rules/{rule_index}/variations")),
            suggestion: Some(format!(
                "Adjust weights so they sum to {MAX_PERCENTAGE}% or less."
            )),
        }];
    }

    Vec::new()
}

/// Validate that rollout percentage is between 0 and 100.
fn validate_rollout_percentage(
    file_path: &str,
    flag_name: &str,
    rule: &Value,
    rule_index: usize,
) -> Vec<ValidationError> {
    let rule_obj = match rule.as_object() {
        Some(obj) => obj,
        None => return Vec::new(),
    };

    let rollout = match rule_obj.get("rollout") {
        Some(r) if is_rollout(r) => r,
        _ => return Vec::new(),
    };

    let percentage = rollout
        .as_object()
        .and_then(|obj| obj.get("percentage"))
        .and_then(|p| p.as_u64())
        .map(|p| p as u32);

    if let Some(percentage) = percentage {
        if !(MIN_PERCENTAGE..=MAX_PERCENTAGE).contains(&percentage) {
            return vec![ValidationError {
                file: file_path.to_string(),
                line: None,
                column: None,
                message: format!(
                    "Rollout percentage for flag '{flag_name}' must be between {MIN_PERCENTAGE} and {MAX_PERCENTAGE}"
                ),
                path: Some(format!(
                    "/rules/{flag_name}/rules/{rule_index}/rollout/percentage"
                )),
                suggestion: Some(format!(
                    "Set percentage between {MIN_PERCENTAGE} and {MAX_PERCENTAGE}."
                )),
            }];
        }
    }

    Vec::new()
}
