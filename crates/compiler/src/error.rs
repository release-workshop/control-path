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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::error::ParseError as ParserParseError;

    #[test]
    fn test_parse_error_from_parser_error() {
        let parser_error = ParserParseError::InvalidYaml("test error".to_string());
        let error: ParseError = parser_error.into();
        match error {
            ParseError::InvalidYaml(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Expected InvalidYaml"),
        }
    }

    #[test]
    fn test_parse_error_from_parser_error_invalid_json() {
        let parser_error = ParserParseError::InvalidJson("json error".to_string());
        let error: ParseError = parser_error.into();
        match error {
            ParseError::InvalidJson(msg) => assert_eq!(msg, "json error"),
            _ => panic!("Expected InvalidJson"),
        }
    }

    #[test]
    fn test_parse_error_from_parser_error_missing_field() {
        let parser_error = ParserParseError::MissingField("field_name".to_string());
        let error: ParseError = parser_error.into();
        match error {
            ParseError::MissingField(msg) => assert_eq!(msg, "field_name"),
            _ => panic!("Expected MissingField"),
        }
    }

    #[test]
    fn test_parse_error_from_parser_error_invalid_field_type() {
        let parser_error = ParserParseError::InvalidFieldType("type error".to_string());
        let error: ParseError = parser_error.into();
        match error {
            ParseError::InvalidFieldType(msg) => assert_eq!(msg, "type error"),
            _ => panic!("Expected InvalidFieldType"),
        }
    }

    #[test]
    fn test_compiler_error_from_parse_error() {
        let parse_error = ParseError::InvalidYaml("test".to_string());
        let compiler_error: CompilerError = parse_error.into();
        match compiler_error {
            CompilerError::Parse(e) => match e {
                ParseError::InvalidYaml(msg) => assert_eq!(msg, "test"),
                _ => panic!("Expected InvalidYaml"),
            },
            _ => panic!("Expected Parse variant"),
        }
    }

    #[test]
    fn test_compiler_error_from_validation_error() {
        let validation_error = ValidationError::SchemaValidation("schema error".to_string());
        let compiler_error: CompilerError = validation_error.into();
        match compiler_error {
            CompilerError::Validation(e) => match e {
                ValidationError::SchemaValidation(msg) => assert_eq!(msg, "schema error"),
                _ => panic!("Expected SchemaValidation"),
            },
            _ => panic!("Expected Validation variant"),
        }
    }

    #[test]
    fn test_compiler_error_from_compilation_error() {
        let compilation_error = CompilationError::ExpressionParsing("expr error".to_string());
        let compiler_error: CompilerError = compilation_error.into();
        match compiler_error {
            CompilerError::Compilation(e) => match e {
                CompilationError::ExpressionParsing(msg) => assert_eq!(msg, "expr error"),
                _ => panic!("Expected ExpressionParsing"),
            },
            _ => panic!("Expected Compilation variant"),
        }
    }

    #[test]
    fn test_compiler_error_from_serialization_error() {
        let serialization_error =
            SerializationError::MessagePack("serialization error".to_string());
        let compiler_error: CompilerError = serialization_error.into();
        match compiler_error {
            CompilerError::Serialization(e) => match e {
                SerializationError::MessagePack(msg) => assert_eq!(msg, "serialization error"),
                _ => panic!("Expected MessagePack"),
            },
            _ => panic!("Expected Serialization variant"),
        }
    }

    #[test]
    fn test_validation_error_variants() {
        let error1 = ValidationError::InvalidFlagDefinition("flag error".to_string());
        assert!(error1.to_string().contains("flag error"));

        let error2 = ValidationError::InvalidDeployment("deployment error".to_string());
        assert!(error2.to_string().contains("deployment error"));

        let error3 = ValidationError::FlagNotFound("flag_name".to_string());
        assert!(error3.to_string().contains("flag_name"));

        let error4 = ValidationError::TypeMismatch("type error".to_string());
        assert!(error4.to_string().contains("type error"));
    }

    #[test]
    fn test_compilation_error_variants() {
        let error1 = CompilationError::InvalidExpression("expr error".to_string());
        assert!(error1.to_string().contains("expr error"));

        let error2 = CompilationError::StringTable("string table error".to_string());
        assert!(error2.to_string().contains("string table error"));

        let error3 = CompilationError::InvalidRule("rule error".to_string());
        assert!(error3.to_string().contains("rule error"));

        let error4 = CompilationError::InvalidSegment("segment error".to_string());
        assert!(error4.to_string().contains("segment error"));
    }

    #[test]
    fn test_serialization_error_variants() {
        let error1 = SerializationError::InvalidArtifact("artifact error".to_string());
        assert!(error1.to_string().contains("artifact error"));
    }
}
