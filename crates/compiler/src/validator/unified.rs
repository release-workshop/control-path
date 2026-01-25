/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use serde_json::Value;

use crate::validator::common::validate_with_schema;
use crate::validator::error::{ValidationError, ValidationResult};

/// Validate control-path.yaml configuration against the schema.
pub fn validate_unified_config(schema: &Value, file_path: &str, data: &Value) -> ValidationResult {
    validate_with_schema(schema, file_path, data, validate_unified_specific_rules)
}

/// Validate config-specific business rules that aren't covered by JSON schema.
fn validate_unified_specific_rules(file_path: &str, data: &Value) -> Vec<ValidationError> {
    if !data.is_object() {
        return Vec::new();
    }

    let empty_vec = Vec::new();
    let flags = data
        .as_object()
        .and_then(|obj| obj.get("flags"))
        .and_then(|f| f.as_array())
        .unwrap_or(&empty_vec);

    let mut errors = Vec::new();

    // Validate duplicate flag names
    let mut flag_names = std::collections::HashSet::new();
    for (index, flag) in flags.iter().enumerate() {
        if let Some(name) = flag.get("name").and_then(|n| n.as_str()) {
            if flag_names.contains(name) {
                errors.push(ValidationError {
                    file: file_path.to_string(),
                    line: None,
                    column: None,
                    message: format!("Duplicate flag name: '{name}'"),
                    path: Some(format!("flags[{index}].name")),
                    suggestion: None,
                });
            } else {
                flag_names.insert(name);
            }
        }
    }

    // Validate multivariate flags have variations
    for (index, flag) in flags.iter().enumerate() {
        if let Some(flag_type) = flag.get("type").and_then(|t| t.as_str()) {
            if flag_type == "multivariate"
                && !flag
                    .get("variations")
                    .and_then(|v| v.as_array())
                    .is_some_and(|v| !v.is_empty())
            {
                if let Some(name) = flag.get("name").and_then(|n| n.as_str()) {
                    errors.push(ValidationError {
                        file: file_path.to_string(),
                        line: None,
                        column: None,
                        message: format!(
                            "Multivariate flag '{name}' must have at least one variation"
                        ),
                        path: Some(format!("flags[{index}].variations")),
                        suggestion: None,
                    });
                }
            }
        }
    }

    errors
}
