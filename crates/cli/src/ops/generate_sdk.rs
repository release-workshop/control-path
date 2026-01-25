//! Reusable SDK generation operations

use crate::error::{CliError, CliResult};
use crate::generator::generate_sdk;
use crate::utils::language;
use crate::utils::unified_config;
use controlpath_compiler::validate_definitions;
use std::path::PathBuf;

/// Options for generating SDK
pub struct GenerateOptions {
    /// Language to generate (if None, auto-detect)
    pub lang: Option<String>,
    /// Output directory (if None, uses default ./flags)
    pub output: Option<String>,
    /// Skip validation before generation
    pub skip_validation: bool,
}

/// Generate SDK from flag definitions
///
/// This function:
/// 1. Reads and validates flag definitions
/// 2. Determines language (CLI flag > Config > Auto-detect > Default)
/// 3. Generates SDK to output directory
pub fn generate_sdk_helper(options: &GenerateOptions) -> CliResult<()> {
    // Read config
    let unified = unified_config::read_unified_config()?;

    // Extract definitions from config
    let definitions = unified_config::extract_definitions(&unified)?;

    // Validate definitions (unless skipped)
    if !options.skip_validation {
        validate_definitions(&definitions)
            .map_err(|e| CliError::Message(format!("Config is invalid: {e}")))?;
    }

    let output_path = if let Some(ref output) = options.output {
        PathBuf::from(output)
    } else {
        PathBuf::from("./flags")
    };

    // Determine language (priority: CLI flag > Config > Auto-detect > Default)
    let language = language::determine_language(options.lang.clone())?.to_lowercase();

    // Generate SDK
    generate_sdk(&language, &definitions, &output_path)?;

    Ok(())
}
