//! Reusable SDK generation operations

use crate::error::{CliError, CliResult};
use crate::generator::generate_sdk;
use crate::monorepo::ServiceContext;
use crate::utils::language;
use controlpath_compiler::{parse_definitions, validate_definitions};
use std::fs;
use std::path::PathBuf;

/// Options for generating SDK
pub struct GenerateOptions {
    /// Language to generate (if None, auto-detect)
    pub lang: Option<String>,
    /// Output directory (if None, uses default ./flags)
    pub output: Option<String>,
    /// Service context for monorepo support
    pub service_context: Option<ServiceContext>,
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
    use crate::monorepo::ServicePathResolver;

    let resolver = options
        .service_context
        .as_ref()
        .map(|ctx| ServicePathResolver::new(ctx.clone()));

    // Determine paths
    let definitions_path = if let Some(ref r) = resolver {
        r.definitions_file()
    } else {
        PathBuf::from("flags.definitions.yaml")
    };

    if !definitions_path.exists() {
        return Err(CliError::Message(format!(
            "Definitions file not found: {}",
            definitions_path.display()
        )));
    }

    let output_path = if let Some(ref output) = options.output {
        if let Some(ref r) = resolver {
            r.base_path().join(output)
        } else {
            PathBuf::from(output)
        }
    } else if let Some(ref r) = resolver {
        r.sdk_output("flags")
    } else {
        PathBuf::from("./flags")
    };

    // Read and parse definitions
    let definitions_content = fs::read_to_string(&definitions_path)
        .map_err(|e| CliError::Message(format!("Failed to read definitions file: {e}")))?;
    let definitions = parse_definitions(&definitions_content)?;

    // Validate definitions (unless skipped)
    if !options.skip_validation {
        validate_definitions(&definitions)
            .map_err(|e| CliError::Message(format!("Definitions file is invalid: {e}")))?;
    }

    // Determine language (priority: CLI flag > Config > Auto-detect > Default)
    let language = language::determine_language(options.lang.clone())?.to_lowercase();

    // Generate SDK
    generate_sdk(&language, &definitions, &output_path)?;

    Ok(())
}
