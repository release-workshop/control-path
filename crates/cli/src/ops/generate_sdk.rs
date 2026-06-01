//! Reusable SDK generation operations

use crate::error::CliResult;
use crate::generator::generate_sdk;
use crate::utils::catalog;
use crate::utils::language;
use crate::utils::unified_config;
use serde_json::Value;
use std::env;
use std::path::PathBuf;

const DEFAULT_SDK_OUTPUT: &str = "node_modules/@controlpath/generated";

/// Options for generating SDK
pub struct GenerateOptions {
    /// Language to generate (if None, auto-detect)
    pub lang: Option<String>,
    /// Output directory (if None, uses config sdk.output or default node_modules/@controlpath/generated)
    pub output: Option<String>,
}

/// Resolve SDK output directory: CLI `--output`, then `sdk.output` in catalog, then default.
pub(crate) fn resolve_sdk_output_path(
    cli_output: Option<&str>,
    unified: Option<&Value>,
) -> PathBuf {
    if let Some(output) = cli_output {
        PathBuf::from(output)
    } else if let Some(config) = unified {
        unified_config::get_sdk_output_path(config)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SDK_OUTPUT))
    } else {
        PathBuf::from(DEFAULT_SDK_OUTPUT)
    }
}

/// Generate SDK from v2 catalog flag definitions and imports.
///
/// Returns the output directory where artifacts were written.
pub fn generate_sdk_helper(options: &GenerateOptions) -> CliResult<PathBuf> {
    let unified = unified_config::read_unified_config().ok();

    let base_dir = env::current_dir().map_err(|e| {
        crate::error::CliError::Message(format!("Failed to resolve working directory: {e}"))
    })?;

    let sdk_catalog = catalog::load_for_sdk_generate(&base_dir)?;
    let output_path = resolve_sdk_output_path(options.output.as_deref(), unified.as_ref());
    let language = language::determine_language(options.lang.clone())?.to_lowercase();
    generate_sdk(&language, &sdk_catalog, &output_path)?;

    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_sdk_output_path_prefers_cli_output() {
        let unified: Value = serde_yaml::from_str("sdk:\n  output: from-config\n").unwrap();
        let path = resolve_sdk_output_path(Some("from-cli"), Some(&unified));
        assert_eq!(path, PathBuf::from("from-cli"));
    }

    #[test]
    fn resolve_sdk_output_path_uses_config_sdk_output() {
        let unified: Value = serde_yaml::from_str("sdk:\n  output: custom/generated\n").unwrap();
        let path = resolve_sdk_output_path(None, Some(&unified));
        assert_eq!(path, PathBuf::from("custom/generated"));
    }

    #[test]
    fn resolve_sdk_output_path_defaults_without_config() {
        let path = resolve_sdk_output_path(None, None);
        assert_eq!(path, PathBuf::from(DEFAULT_SDK_OUTPUT));
    }
}
