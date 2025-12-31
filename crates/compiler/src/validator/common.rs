/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use jsonschema::JSONSchema;
use serde_json::Value;

use crate::validator::error::{convert_jsonschema_error, ValidationError, ValidationResult};

/// Common validation pattern for both definitions and deployment files.
/// Compiles schema, validates data, and combines schema errors with additional validation errors.
pub fn validate_with_schema<F>(
    schema: &Value,
    file_path: &str,
    data: &Value,
    additional_validation: F,
) -> ValidationResult
where
    F: FnOnce(&str, &Value) -> Vec<ValidationError>,
{
    // Compile schema
    let compiled = match JSONSchema::compile(schema) {
        Ok(compiled) => compiled,
        Err(err) => {
            return ValidationResult::invalid(vec![ValidationError {
                file: file_path.to_string(),
                line: None,
                column: None,
                message: format!("Failed to compile schema: {}", err),
                path: None,
                suggestion: None,
            }]);
        }
    };

    // Validate against schema
    let schema_errors: Vec<ValidationError> = if let Err(errors) = compiled.validate(data) {
        errors
            .map(|error| convert_jsonschema_error(file_path, &error))
            .collect()
    } else {
        Vec::new()
    };

    // Run additional validation
    let additional_errors = additional_validation(file_path, data);

    // Combine all errors
    let all_errors = [schema_errors, additional_errors].concat();

    ValidationResult {
        valid: all_errors.is_empty(),
        errors: all_errors,
    }
}


