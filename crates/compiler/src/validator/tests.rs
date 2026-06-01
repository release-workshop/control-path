/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

use crate::validator::error::{ValidationError, ValidationResult};
use crate::validator::Validator;

#[test]
fn test_validator_default() {
    let validator = Validator;
    assert!(validator.format_errors(&[]).is_empty());
}

#[test]
fn test_format_errors_empty() {
    let validator = Validator::new();
    assert_eq!(validator.format_errors(&[]), "");
}

#[test]
fn test_format_errors_with_line_column() {
    let validator = Validator::new();
    let errors = vec![ValidationError {
        file: "test.yaml".to_string(),
        message: "Test error".to_string(),
        path: Some("field".to_string()),
        line: Some(1),
        column: Some(5),
        suggestion: Some("Fix it".to_string()),
    }];
    let formatted = validator.format_errors(&errors);
    assert!(formatted.contains("Line 1, column 5"));
    assert!(formatted.contains("Test error"));
    assert!(formatted.contains("Fix it"));
}

#[test]
fn test_format_errors_without_line_column() {
    let validator = Validator::new();
    let errors = vec![ValidationError {
        file: "test.yaml".to_string(),
        message: "Test error".to_string(),
        path: None,
        line: None,
        column: None,
        suggestion: None,
    }];
    let formatted = validator.format_errors(&errors);
    assert!(formatted.contains("Test error"));
}

#[test]
fn test_validation_result_valid() {
    let result = ValidationResult {
        valid: true,
        errors: vec![],
    };
    assert!(result.valid);
}

#[test]
fn test_validation_result_invalid_with_errors() {
    let result = ValidationResult {
        valid: false,
        errors: vec![ValidationError {
            file: "test.yaml".to_string(),
            message: "error".to_string(),
            path: None,
            line: None,
            column: None,
            suggestion: None,
        }],
    };
    assert!(!result.valid);
    assert_eq!(result.errors.len(), 1);
}
