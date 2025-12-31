//! Control Path Compiler Library
//!
//! Copyright 2025 Release Workshop Ltd
//! Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
//! See the LICENSE file in the project root for details.
//!
//! This library compiles Control Path deployment YAML files into compact AST artifacts.
//! It is designed to be WASM-compatible and works only with in-memory data (no file I/O).
//!
//! # Example
//!
//! ```rust,no_run
//! use controlpath_compiler::{parse_definitions, parse_deployment, compile, serialize};
//!
//! let definitions_yaml = r#"
//! flags:
//!   - name: my_flag
//!     type: boolean
//!     defaultValue: false
//! "#;
//!
//! let deployment_yaml = r#"
//! environment: production
//! rules:
//!   my_flag:
//!     rules:
//!       - serve: true
//! "#;
//!
//! let definitions = parse_definitions(definitions_yaml)?;
//! let deployment = parse_deployment(deployment_yaml)?;
//! let artifact = compile(&deployment, &definitions)?;
//! let bytes = serialize(&artifact);
//! # Ok::<(), controlpath_compiler::CompilerError>(())
//! ```

pub mod ast;
pub mod error;
pub mod parser;
pub mod schemas;

// TODO: Add validator module
// pub mod validator;

// TODO: Add compiler module
// pub mod compiler;

use ast::Artifact;

// Re-export error types for public API
pub use error::CompilerError;

/// Parse flag definitions from YAML/JSON string
/// 
/// This function works on in-memory strings only (no file I/O).
/// The input can be YAML or JSON format.
/// 
/// # Errors
/// 
/// Returns `ParseError` if the input is invalid YAML/JSON or missing required fields.
pub fn parse_definitions(content: &str) -> Result<serde_json::Value, CompilerError> {
    parser::parse_definitions(content)
        .map_err(|e| CompilerError::Parse(e.into()))
}

/// Parse deployment from YAML/JSON string
/// 
/// This function works on in-memory strings only (no file I/O).
/// The input can be YAML or JSON format.
/// 
/// # Errors
/// 
/// Returns `ParseError` if the input is invalid YAML/JSON or missing required fields.
pub fn parse_deployment(content: &str) -> Result<serde_json::Value, CompilerError> {
    parser::parse_deployment(content)
        .map_err(|e| CompilerError::Parse(e.into()))
}

/// Validate flag definitions against JSON schema
/// 
/// # Errors
/// 
/// Returns `ValidationError` if validation fails.
pub fn validate_definitions(_definitions: &serde_json::Value) -> Result<(), CompilerError> {
    // TODO: Implement schema validation
    // For now, return an error
    Err(CompilerError::Validation(error::ValidationError::SchemaValidation(
        "Validator not yet implemented".to_string(),
    )))
}

/// Validate deployment against JSON schema
/// 
/// # Errors
/// 
/// Returns `ValidationError` if validation fails.
pub fn validate_deployment(_deployment: &serde_json::Value) -> Result<(), CompilerError> {
    // TODO: Implement schema validation
    // For now, return an error
    Err(CompilerError::Validation(error::ValidationError::SchemaValidation(
        "Validator not yet implemented".to_string(),
    )))
}

/// Compile deployment and definitions into an AST artifact
/// 
/// # Errors
/// 
/// Returns `CompilationError` if compilation fails.
pub fn compile(
    _deployment: &serde_json::Value,
    _definitions: &serde_json::Value,
) -> Result<Artifact, CompilerError> {
    // TODO: Implement AST compilation
    // For now, return an error
    Err(CompilerError::Compilation(error::CompilationError::InvalidRule(
        "Compiler not yet implemented".to_string(),
    )))
}

/// Serialize AST artifact to MessagePack bytes
/// 
/// This function never fails - it always returns a Vec<u8>.
pub fn serialize(_artifact: &Artifact) -> Vec<u8> {
    // TODO: Implement MessagePack serialization
    // For now, return empty vector
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_definitions() {
        let result = parse_definitions("flags: []");
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value["flags"].is_array());
    }

    #[test]
    fn test_parse_deployment() {
        let result = parse_deployment("environment: test\nrules: {}");
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["environment"], "test");
        assert!(value["rules"].is_object());
    }
}

