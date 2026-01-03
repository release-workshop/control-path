/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

pub mod common;
pub mod constants;
pub mod definitions;
pub mod deployment;
pub mod error;
pub mod type_guards;

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests;

use serde_json::Value;

use crate::schemas;
use crate::validator::definitions::validate_definitions as validate_definitions_impl;
use crate::validator::deployment::validate_deployment as validate_deployment_impl;
use crate::validator::error::{ValidationError, ValidationResult};

/// Main validator for Control Path configuration files.
/// Validates flag definitions and deployment files against JSON schemas.
pub struct Validator {
    definitions_schema: Value,
    deployment_schema: Value,
}

impl Validator {
    /// Create a new Validator instance with embedded schemas.
    ///
    /// This constructor loads schemas embedded at compile time (WASM-compatible).
    pub fn new() -> Self {
        Self {
            definitions_schema: schemas::load_definitions_schema(),
            deployment_schema: schemas::load_deployment_schema(),
        }
    }

    /// Create a new Validator instance with custom schemas.
    ///
    /// This is useful for testing or when schemas need to be provided dynamically.
    pub fn with_schemas(definitions_schema: Value, deployment_schema: Value) -> Self {
        Self {
            definitions_schema,
            deployment_schema,
        }
    }

    /// Validate a flag definitions file.
    pub fn validate_definitions(&self, file_path: &str, data: &Value) -> ValidationResult {
        validate_definitions_impl(&self.definitions_schema, file_path, data)
    }

    /// Validate a deployment file.
    pub fn validate_deployment(&self, file_path: &str, data: &Value) -> ValidationResult {
        validate_deployment_impl(&self.deployment_schema, file_path, data)
    }

    /// Format validation errors for display.
    pub fn format_errors(&self, errors: &[ValidationError]) -> String {
        if errors.is_empty() {
            return String::new();
        }

        let mut error_lines = vec!["✗ Validation failed\n".to_string()];

        for error in errors {
            error_lines.push(self.format_error_location(error));
            error_lines.push(format!("  Error: {}", error.message));

            if let Some(path) = &error.path {
                error_lines.push(format!("  Path: {path}"));
            }

            if let Some(suggestion) = &error.suggestion {
                error_lines.push(format!("  Suggestion: {suggestion}"));
            }

            error_lines.push(String::new());
        }

        error_lines.join("\n")
    }

    /// Format error location with file path and optional line/column.
    fn format_error_location(&self, error: &ValidationError) -> String {
        if let Some(line) = error.line {
            let column = error.column.map(|c| format!(":{c}")).unwrap_or_default();
            format!("{}:{line}{column}", error.file)
        } else {
            error.file.clone()
        }
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}
