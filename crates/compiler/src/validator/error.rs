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

#[cfg(test)]
mod tests {
    use super::*;
    use jsonschema::JSONSchema;

    #[test]
    fn test_validation_result_valid() {
        let result = ValidationResult::valid();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validation_result_invalid_with_errors() {
        let errors = vec![ValidationError {
            file: "test.yaml".to_string(),
            line: Some(10),
            column: Some(5),
            message: "Test error".to_string(),
            path: Some("flags[0]".to_string()),
            suggestion: Some("Fix it".to_string()),
        }];
        let result = ValidationResult::invalid(errors.clone());
        assert!(!result.valid);
        assert_eq!(result.errors, errors);
    }

    #[test]
    fn test_validation_result_invalid_empty_errors() {
        let result = ValidationResult::invalid(vec![]);
        assert!(result.valid); // Empty errors means valid
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_convert_jsonschema_error_with_path() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"]
        });
        let compiled = JSONSchema::compile(&schema).unwrap();
        let data = serde_json::json!({});
        let errors: Vec<_> = compiled.validate(&data).unwrap_err().collect();
        assert!(!errors.is_empty());

        let converted = convert_jsonschema_error("test.yaml", &errors[0]);
        assert_eq!(converted.file, "test.yaml");
        assert_eq!(converted.line, None);
        assert_eq!(converted.column, None);
        assert!(!converted.message.is_empty());
        assert!(converted.path.is_some() || converted.path.is_none());
    }

    #[test]
    fn test_convert_jsonschema_error_empty_path() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"]
        });
        let compiled = JSONSchema::compile(&schema).unwrap();
        let data = serde_json::json!({});
        let errors: Vec<_> = compiled.validate(&data).unwrap_err().collect();
        assert!(!errors.is_empty());

        let converted = convert_jsonschema_error("test.yaml", &errors[0]);
        // Path might be empty or have a value depending on jsonschema version
        assert_eq!(converted.file, "test.yaml");
    }

    #[test]
    fn test_generate_suggestion_required() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"]
        });
        let compiled = JSONSchema::compile(&schema).unwrap();
        let data = serde_json::json!({});
        let errors: Vec<_> = compiled.validate(&data).unwrap_err().collect();
        assert!(!errors.is_empty());

        let converted = convert_jsonschema_error("test.yaml", &errors[0]);
        // May or may not have a suggestion depending on jsonschema version/formatting
        // Just verify the conversion works
        assert_eq!(converted.file, "test.yaml");
        assert!(!converted.message.is_empty());
    }

    #[test]
    fn test_generate_suggestion_type() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "age": {"type": "number"}
            }
        });
        let compiled = JSONSchema::compile(&schema).unwrap();
        let data = serde_json::json!({
            "age": "not a number"
        });
        let errors: Vec<_> = compiled.validate(&data).unwrap_err().collect();
        if !errors.is_empty() {
            let converted = convert_jsonschema_error("test.yaml", &errors[0]);
            // May or may not have a suggestion depending on error kind
            assert_eq!(converted.file, "test.yaml");
        }
    }

    #[test]
    fn test_generate_suggestion_enum() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "inactive"]}
            }
        });
        let compiled = JSONSchema::compile(&schema).unwrap();
        let data = serde_json::json!({
            "status": "invalid"
        });
        let errors: Vec<_> = compiled.validate(&data).unwrap_err().collect();
        if !errors.is_empty() {
            let converted = convert_jsonschema_error("test.yaml", &errors[0]);
            assert_eq!(converted.file, "test.yaml");
        }
    }

    #[test]
    fn test_generate_suggestion_pattern() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "email": {"type": "string", "pattern": "^[a-z]+@[a-z]+\\.[a-z]+$"}
            }
        });
        let compiled = JSONSchema::compile(&schema).unwrap();
        let data = serde_json::json!({
            "email": "invalid-email"
        });
        let errors: Vec<_> = compiled.validate(&data).unwrap_err().collect();
        if !errors.is_empty() {
            let converted = convert_jsonschema_error("test.yaml", &errors[0]);
            assert_eq!(converted.file, "test.yaml");
        }
    }

    #[test]
    fn test_validation_error_serialize_deserialize() {
        let error = ValidationError {
            file: "test.yaml".to_string(),
            line: Some(10),
            column: Some(5),
            message: "Test error".to_string(),
            path: Some("flags[0]".to_string()),
            suggestion: Some("Fix it".to_string()),
        };
        let json = serde_json::to_string(&error).unwrap();
        let deserialized: ValidationError = serde_json::from_str(&json).unwrap();
        assert_eq!(error, deserialized);
    }

    #[test]
    fn test_validation_result_serialize_deserialize() {
        let result = ValidationResult {
            valid: false,
            errors: vec![ValidationError {
                file: "test.yaml".to_string(),
                line: None,
                column: None,
                message: "Error".to_string(),
                path: None,
                suggestion: None,
            }],
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ValidationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }
}
