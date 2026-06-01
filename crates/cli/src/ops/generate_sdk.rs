//! Reusable SDK generation operations

use crate::error::CliResult;
use crate::generator::generate_sdk;
use crate::utils::catalog;
use crate::utils::language;
use crate::utils::unified_config;
use std::env;
use std::path::PathBuf;

/// Options for generating SDK
pub struct GenerateOptions {
    /// Language to generate (if None, auto-detect)
    pub lang: Option<String>,
    /// Output directory (if None, uses config sdk.output or default node_modules/@controlpath/generated)
    pub output: Option<String>,
    /// Skip validation before generation
    pub skip_validation: bool,
}

/// Generate SDK from v2 catalog flag definitions and imports.
pub fn generate_sdk_helper(options: &GenerateOptions) -> CliResult<()> {
    let unified = unified_config::read_unified_config().ok();

    let base_dir = env::current_dir().map_err(|e| {
        crate::error::CliError::Message(format!("Failed to resolve working directory: {e}"))
    })?;

    let sdk_catalog = if options.skip_validation {
        catalog::load_sdk_catalog_unchecked_for_generate(&base_dir)?
    } else {
        catalog::load_sdk_catalog_for_generate(&base_dir)?
    };

    let output_path = if let Some(ref output) = options.output {
        PathBuf::from(output)
    } else if let Some(ref config) = unified {
        unified_config::get_sdk_output_path(config)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("node_modules/@controlpath/generated"))
    } else {
        PathBuf::from("node_modules/@controlpath/generated")
    };

    let language = language::determine_language(options.lang.clone())?.to_lowercase();
    generate_sdk(&language, &sdk_catalog, &output_path)?;

    Ok(())
}
