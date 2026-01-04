//! SDK code generator
//!
//! Generates type-safe SDKs from flag definitions.

pub mod typescript;

use crate::error::{CliError, CliResult};
use serde_json::Value;
use std::path::Path;

/// Trait for SDK generators
pub trait Generator {
    /// Generate SDK code from flag definitions
    fn generate(&self, definitions: &Value, output_dir: &Path) -> CliResult<()>;
}

/// Generate SDK for the specified language
pub fn generate_sdk(language: &str, definitions: &Value, output_dir: &Path) -> CliResult<()> {
    match language {
        "typescript" | "ts" => {
            let generator = typescript::TypeScriptGenerator::new();
            generator.generate(definitions, output_dir)
        }
        _ => Err(CliError::Message(format!(
            "Unsupported language: {}. Supported languages: typescript",
            language
        ))),
    }
}
