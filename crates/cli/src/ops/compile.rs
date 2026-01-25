//! Reusable compile operations

use crate::error::{CliError, CliResult};
use crate::utils::unified_config;
use controlpath_compiler::{compile, serialize, validate_definitions, validate_deployment};
use std::fs;
use std::path::{Path, PathBuf};

/// Options for compiling environments
pub struct CompileOptions {
    /// Environment names to compile (if None, compiles all found)
    pub envs: Option<Vec<String>>,
    /// Skip validation before compilation
    pub skip_validation: bool,
}

/// Compile ASTs for one or more environments
///
/// This function:
/// 1. Validates deployment files (unless skip_validation is true)
/// 2. Compiles each environment's deployment to an AST file
/// 3. Writes AST files to `.controlpath/<env>.ast`
pub fn compile_envs(options: &CompileOptions) -> CliResult<Vec<String>> {
    // Path resolution simplified - always use project root

    // Read config
    let unified = unified_config::read_unified_config()?;

    // Extract definitions from config
    let definitions = unified_config::extract_definitions(&unified)?;

    // Validate definitions
    if !options.skip_validation {
        validate_definitions(&definitions)
            .map_err(|e| CliError::Message(format!("Config is invalid: {e}")))?;
    }

    // Determine which environments to compile
    let envs = if let Some(ref envs) = options.envs {
        envs.clone()
    } else {
        // Get all environments from config
        let found_envs = unified_config::get_environments(&unified);
        if found_envs.is_empty() {
            return Err(CliError::Message(
                "No environments found in config. Add flags with environment rules first."
                    .to_string(),
            ));
        }
        found_envs
    };

    // Compile each environment
    let mut compiled_envs = Vec::new();
    let mut missing_envs = Vec::new();
    for env in &envs {
        // Extract deployment for this environment
        let deployment = match unified_config::extract_deployment(&unified, env) {
            Ok(dep) => dep,
            Err(e) => {
                if options.envs.is_some() {
                    return Err(CliError::Message(format!(
                        "Failed to extract deployment for environment '{env}': {e}"
                    )));
                }
                missing_envs.push(env.clone());
                continue;
            }
        };

        // Validate deployment (unless skipped)
        if !options.skip_validation {
            validate_deployment(&deployment)
                .map_err(|e| CliError::Message(format!("Deployment for {env} is invalid: {e}")))?;
        }

        let output_path = PathBuf::from(format!(".controlpath/{env}.ast"));

        match compile_single_env_from_values(&deployment, &output_path, &definitions) {
            Ok(()) => {
                compiled_envs.push(env.clone());
            }
            Err(e) => {
                return Err(CliError::Message(format!("Failed to compile {env}: {e}")));
            }
        }
    }

    // Warn about missing environments if we were auto-discovering
    if !missing_envs.is_empty() && options.envs.is_none() {
        eprintln!(
            "⚠ Warning: Skipped {} environment(s) with no rules defined: {}",
            missing_envs.len(),
            missing_envs.join(", ")
        );
    }

    // Validate that we compiled at least one environment
    if compiled_envs.is_empty() {
        return Err(CliError::Message(
            "No environments were compiled. Check that flags have environment rules defined."
                .to_string(),
        ));
    }

    Ok(compiled_envs)
}

/// Compile a single environment from already-parsed deployment and definitions
fn compile_single_env_from_values(
    deployment: &serde_json::Value,
    output_path: &Path,
    definitions: &serde_json::Value,
) -> CliResult<()> {
    // Compile to AST
    let artifact = compile(deployment, definitions)?;

    // Serialize to MessagePack
    let ast_bytes = serialize(&artifact)?;

    // Create output directory if needed
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Write AST file
    fs::write(output_path, ast_bytes)?;

    Ok(())
}
