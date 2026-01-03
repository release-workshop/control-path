/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */
use thiserror::Error;

/// Top-level error type for the compiler
#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Compilation error: {0}")]
    Compilation(#[from] CompilationError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] SerializationError),
}

/// Parse errors for YAML/JSON parsing
///
/// This is a wrapper around the parser module's `ParseError` to maintain
/// backward compatibility with the public API.
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

impl From<crate::parser::error::ParseError> for ParseError {
    fn from(err: crate::parser::error::ParseError) -> Self {
        match err {
            crate::parser::error::ParseError::InvalidYaml(msg) => Self::InvalidYaml(msg),
            crate::parser::error::ParseError::InvalidJson(msg) => Self::InvalidJson(msg),
            crate::parser::error::ParseError::MissingField(msg) => Self::MissingField(msg),
            crate::parser::error::ParseError::InvalidFieldType(msg) => Self::InvalidFieldType(msg),
        }
    }
}

/// Validation errors for schema validation
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Schema validation failed: {0}")]
    SchemaValidation(String),

    #[error("Invalid flag definition: {0}")]
    InvalidFlagDefinition(String),

    #[error("Invalid deployment: {0}")]
    InvalidDeployment(String),

    #[error("Flag not found in definitions: {0}")]
    FlagNotFound(String),

    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
}

/// Compilation errors for AST compilation
#[derive(Error, Debug)]
pub enum CompilationError {
    #[error("Expression parsing error: {0}")]
    ExpressionParsing(String),

    #[error("Invalid expression: {0}")]
    InvalidExpression(String),

    #[error("String table error: {0}")]
    StringTable(String),

    #[error("Invalid rule: {0}")]
    InvalidRule(String),

    #[error("Invalid segment: {0}")]
    InvalidSegment(String),
}

/// Serialization errors for `MessagePack` serialization
#[derive(Error, Debug)]
pub enum SerializationError {
    #[error("MessagePack serialization failed: {0}")]
    MessagePack(String),

    #[error("Invalid artifact structure: {0}")]
    InvalidArtifact(String),
}
