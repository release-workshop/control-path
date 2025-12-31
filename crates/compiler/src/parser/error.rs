/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * Parser-specific error types.
 */

use thiserror::Error;

/// Parser error type for YAML/JSON parsing failures.
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid YAML: {0}")]
    InvalidYaml(String),
    
    #[error("Invalid JSON: {0}")]
    InvalidJson(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Invalid field type: {0}")]
    InvalidFieldType(String),
}

