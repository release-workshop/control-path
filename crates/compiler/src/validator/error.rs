/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use serde::{Deserialize, Serialize};

/// Validation error matching TypeScript ValidationError interface
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationError {
    pub file: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub message: String,
    pub path: Option<String>,
    pub suggestion: Option<String>,
}

/// Validation result matching TypeScript ValidationResult interface
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    /// Create a valid result with no errors
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    /// Create an invalid result with errors
    pub fn invalid(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: errors.is_empty(),
            errors,
        }
    }
}

/// Convert a jsonschema ValidationError to our ValidationError format.
///
/// This function converts errors from the `jsonschema` crate to our `ValidationError` format
/// that matches the TypeScript `ValidationError` interface.
///
/// # Note on Line/Column Information
///
/// The `jsonschema` crate does not provide line/column information in the same way that
/// AJV (used by TypeScript) does. Therefore, `line` and `column` fields are always `None`.
/// This is a known limitation and acceptable for WASM compatibility.
///
/// Error messages may also differ slightly in formatting compared to TypeScript due to
/// the different JSON Schema libraries (AJV vs jsonschema crate), but they are
/// functionally equivalent.
pub fn convert_jsonschema_error(
    file_path: &str,
    error: &jsonschema::ValidationError,
) -> ValidationError {
    let instance_path = error.instance_path.to_string();
    let message = error.to_string();

    ValidationError {
        file: file_path.to_string(),
        // Note: jsonschema doesn't provide line/column info like AJV does
        // This is a known limitation - see function documentation
        line: None,
        column: None,
        message,
        path: if instance_path.is_empty() {
            None
        } else {
            Some(instance_path)
        },
        suggestion: generate_suggestion_from_error(error),
    }
}

/// Generate a helpful suggestion based on jsonschema error.
///
/// This attempts to provide helpful suggestions similar to the TypeScript implementation,
/// though they may be less specific due to differences between AJV and jsonschema error formats.
fn generate_suggestion_from_error(error: &jsonschema::ValidationError) -> Option<String> {
    // Use Debug formatting to get the keyword name
    let keyword = format!("{:?}", error.kind);

    if keyword.contains("required") {
        if let Some(instance_path) = error.instance_path.last() {
            Some(format!("Add missing required field '{instance_path:?}'"))
        } else {
            Some("Add missing required field".to_string())
        }
    } else if keyword.contains("type") {
        // The error message usually contains the type information
        Some("Check the field type matches the schema".to_string())
    } else if keyword.contains("enum") {
        // The error message usually contains allowed values
        Some("Check allowed values in the schema".to_string())
    } else if keyword.contains("pattern") {
        Some("Check that the value matches the required pattern".to_string())
    } else {
        None
    }
}
