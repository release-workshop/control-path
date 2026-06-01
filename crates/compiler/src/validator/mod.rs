/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

pub mod common;
pub mod constants;
pub mod error;
pub mod type_guards;

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests;

use crate::validator::error::ValidationError;

/// Format validation errors for display.
pub fn format_errors(errors: &[ValidationError]) -> String {
    if errors.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    for error in errors {
        if let Some(line) = error.line {
            if let Some(column) = error.column {
                output.push_str(&format!("  Line {line}, column {column}: "));
            } else {
                output.push_str(&format!("  Line {line}: "));
            }
        } else {
            output.push_str("  ");
        }
        output.push_str(&error.message);
        if let Some(suggestion) = &error.suggestion {
            output.push_str(&format!("\n    Suggestion: {suggestion}"));
        }
        output.push('\n');
    }
    output
}

/// Legacy validator stub retained for test module layout compatibility.
pub struct Validator;

impl Validator {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn format_errors(&self, errors: &[ValidationError]) -> String {
        format_errors(errors)
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}
