//! SDK code generator
//!
//! Generates type-safe SDKs from v2 boolean flag catalogs.

pub mod typescript;

#[cfg(test)]
mod catalog_tests;

#[cfg(test)]
mod tests;

use crate::error::{CliError, CliResult};
use controlpath_compiler::SdkCatalog;
use std::path::Path;

/// Trait for SDK generators
pub trait Generator {
    /// Generate SDK code from a v2 catalog projection
    fn generate(&self, catalog: &SdkCatalog, output_dir: &Path) -> CliResult<()>;
}

/// Generate SDK for the specified language
pub fn generate_sdk(language: &str, catalog: &SdkCatalog, output_dir: &Path) -> CliResult<()> {
    match language {
        "typescript" | "ts" => {
            let generator = typescript::TypeScriptGenerator::new()?;
            generator.generate(catalog, output_dir)
        }
        _ => Err(CliError::Message(format!(
            "Unsupported language: {}. Supported languages: typescript",
            language
        ))),
    }
}
