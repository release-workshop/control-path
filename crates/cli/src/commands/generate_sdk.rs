//! Generate SDK command implementation

use crate::error::{CliError, CliResult};
use crate::generator::generate_sdk;
use crate::utils::language;
use crate::utils::unified_config;
use controlpath_compiler::{parse_definitions, validate_definitions};
use std::fs;
use std::path::PathBuf;

pub struct Options {
    pub lang: Option<String>,
    pub output: Option<String>,
    pub definitions: Option<String>,
}

fn determine_definitions_path(options: &Options) -> PathBuf {
    PathBuf::from(
        options
            .definitions
            .as_deref()
            .unwrap_or("flags.definitions.yaml"),
    )
}

fn determine_output_path(options: &Options) -> PathBuf {
    PathBuf::from(options.output.as_deref().unwrap_or("./flags"))
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => {
            println!("✓ SDK generated successfully");
            0
        }
        Err(e) => {
            eprintln!("✗ SDK generation failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    let output_path = determine_output_path(options);

    // Check if using config (when no explicit definitions file is specified)
    let definitions = if options.definitions.is_none() && unified_config::unified_config_exists() {
        // Use config
        let unified = unified_config::read_unified_config()?;
        unified_config::extract_definitions(&unified)?
    } else {
        // Use legacy file-based approach
        let definitions_path = determine_definitions_path(options);
        let definitions_content = fs::read_to_string(&definitions_path)
            .map_err(|e| CliError::Message(format!("Failed to read definitions file: {e}")))?;
        parse_definitions(&definitions_content)?
    };

    // Validate definitions
    validate_definitions(&definitions)?;

    // Determine language (priority: CLI flag > Config > Auto-detect > Default)
    let language = language::determine_language(options.lang.clone())?.to_lowercase();

    // Generate SDK
    generate_sdk(&language, &definitions, &output_path)?;

    println!("  Generated SDK to {}", output_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::generate_sdk;
    use tempfile::TempDir;

    #[test]
    fn test_generate_sdk() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("generated");

        let definitions_content = r#"flags:
  - name: test_flag
    type: boolean
    default: false
"#;

        let definitions = parse_definitions(definitions_content).unwrap();
        let result = generate_sdk("typescript", &definitions, &output_path);
        assert!(result.is_ok());

        // Verify files were created
        assert!(output_path.join("index.ts").exists());
        assert!(output_path.join("types.ts").exists());
        assert!(output_path.join("package.json").exists());
    }
}
