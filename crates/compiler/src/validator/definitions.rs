/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use serde_json::Value;

use crate::validator::common::validate_with_schema;
use crate::validator::error::{ValidationError, ValidationResult};
use crate::validator::type_guards::{has_name, is_flag_definition, is_flag_definitions};

/// Validate flag definitions against the definitions schema.
pub fn validate_definitions(schema: &Value, file_path: &str, data: &Value) -> ValidationResult {
    validate_with_schema(schema, file_path, data, validate_flag_specific_rules)
}

/// Validate flag-specific business rules that aren't covered by JSON schema.
fn validate_flag_specific_rules(file_path: &str, data: &Value) -> Vec<ValidationError> {
    if !is_flag_definitions(data) {
        return Vec::new();
    }

    let empty_vec = Vec::new();
    let flags = data
        .as_object()
        .and_then(|obj| obj.get("flags"))
        .and_then(|f| f.as_array())
        .unwrap_or(&empty_vec);

    let mut errors = Vec::new();
    errors.extend(validate_duplicate_flag_names(file_path, flags));
    errors.extend(validate_multivariate_flags(file_path, flags));

    errors
}

/// Validate that flag names are unique.
fn validate_duplicate_flag_names(file_path: &str, flags: &[Value]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut flag_names = std::collections::HashSet::new();

    for (index, flag) in flags.iter().enumerate() {
        if let Some(name) = has_name(flag) {
            if flag_names.contains(name) {
                errors.push(ValidationError {
                    file: file_path.to_string(),
                    line: None,
                    column: None,
                    message: format!("Duplicate flag name: '{name}'"),
                    path: Some(format!("/flags/{index}/name")),
                    suggestion: Some(
                        "Flag names must be unique. Rename this flag or remove the duplicate."
                            .to_string(),
                    ),
                });
            } else {
                flag_names.insert(name.to_string());
            }
        }
    }

    errors
}

/// Validate that multivariate flags have variations and no duplicate variation names.
fn validate_multivariate_flags(file_path: &str, flags: &[Value]) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for (index, flag) in flags.iter().enumerate() {
        if !is_flag_definition(flag) {
            continue;
        }

        let flag_type = flag
            .as_object()
            .and_then(|obj| obj.get("type"))
            .and_then(|t| t.as_str());

        if flag_type != Some("multivariate") {
            continue;
        }

        let flag_name = has_name(flag).unwrap_or("unnamed");

        // Check if variations array exists and is not empty
        let variations = flag
            .as_object()
            .and_then(|obj| obj.get("variations"))
            .and_then(|v| v.as_array());

        if variations.is_none() || variations.map(|v| v.is_empty()).unwrap_or(true) {
            errors.push(ValidationError {
                file: file_path.to_string(),
                line: None,
                column: None,
                message: format!(
                    "Multivariate flag '{flag_name}' must have at least one variation. Missing variations array."
                ),
                path: Some(format!("/flags/{index}/variations")),
                suggestion: Some("Add a 'variations' array with at least one variation.".to_string()),
            });
            continue;
        }

        // Validate duplicate variation names
        if let Some(variations) = variations {
            errors.extend(validate_duplicate_variation_names(
                file_path, flag_name, variations, index,
            ));
        }
    }

    errors
}

/// Validate that variation names are unique within a flag.
fn validate_duplicate_variation_names(
    file_path: &str,
    flag_name: &str,
    variations: &[Value],
    flag_index: usize,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut variation_names = std::collections::HashSet::new();

    for (variation_index, variation) in variations.iter().enumerate() {
        if let Some(name) = has_name(variation) {
            if variation_names.contains(name) {
                errors.push(ValidationError {
                    file: file_path.to_string(),
                    line: None,
                    column: None,
                    message: format!("Duplicate variation name '{name}' in flag '{flag_name}'"),
                    path: Some(format!(
                        "/flags/{flag_index}/variations/{variation_index}/name"
                    )),
                    suggestion: Some("Variation names must be unique within a flag.".to_string()),
                });
            } else {
                variation_names.insert(name.to_string());
            }
        }
    }

    errors
}
