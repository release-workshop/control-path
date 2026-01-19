//! Reusable compile operations

use crate::error::{CliError, CliResult};
use crate::monorepo::ServiceContext;
use controlpath_compiler::{
    compile, parse_definitions, parse_deployment, serialize, validate_definitions,
    validate_deployment,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Options for compiling environments
pub struct CompileOptions {
    /// Environment names to compile (if None, compiles all found)
    pub envs: Option<Vec<String>>,
    /// Service context for monorepo support
    pub service_context: Option<ServiceContext>,
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
    use crate::monorepo::ServicePathResolver;

    let resolver = options
        .service_context
        .as_ref()
        .map(|ctx| ServicePathResolver::new(ctx.clone()));

    // Determine which environments to compile
    let envs = if let Some(ref envs) = options.envs {
        envs.clone()
    } else {
        // Find all deployment files
        let controlpath_dir = if let Some(ref r) = resolver {
            r.base_path().join(".controlpath")
        } else {
            PathBuf::from(".controlpath")
        };

        if !controlpath_dir.exists() {
            return Err(CliError::Message(
                "No .controlpath directory found. Run 'controlpath init' first.".to_string(),
            ));
        }

        let mut found_envs = Vec::new();
        if let Ok(entries) = fs::read_dir(&controlpath_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".deployment.yaml") {
                        if let Some(env_name) = name.strip_suffix(".deployment.yaml") {
                            found_envs.push(env_name.to_string());
                        }
                    }
                }
            }
        }

        if found_envs.is_empty() {
            return Err(CliError::Message(
                "No deployment files found. Run 'controlpath env add --name <env>' to create one."
                    .to_string(),
            ));
        }

        found_envs
    };

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

    // Read and parse definitions
    let definitions_content = fs::read_to_string(&definitions_path)
        .map_err(|e| CliError::Message(format!("Failed to read definitions file: {e}")))?;
    let definitions = parse_definitions(&definitions_content)?;

    // Validate definitions
    if !options.skip_validation {
        validate_definitions(&definitions)
            .map_err(|e| CliError::Message(format!("Definitions file is invalid: {e}")))?;
    }

    // Compile each environment
    let mut compiled_envs = Vec::new();
    let mut missing_envs = Vec::new();
    for env in &envs {
        let deployment_path = if let Some(ref r) = resolver {
            r.deployment_file(env)
        } else {
            PathBuf::from(format!(".controlpath/{env}.deployment.yaml"))
        };

        if !deployment_path.exists() {
            // If environments were explicitly requested, fail fast
            if options.envs.is_some() {
                return Err(CliError::Message(format!(
                    "Deployment file not found for environment '{env}': {}",
                    deployment_path.display()
                )));
            }
            // Otherwise, just skip and collect missing ones
            missing_envs.push(env.clone());
            continue;
        }

        let output_path = if let Some(ref r) = resolver {
            r.ast_file(env)
        } else {
            PathBuf::from(format!(".controlpath/{env}.ast"))
        };

        match compile_single_env(
            &deployment_path,
            &output_path,
            &definitions,
            options.skip_validation,
        ) {
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
            "⚠ Warning: Skipped {} environment(s) with missing deployment files: {}",
            missing_envs.len(),
            missing_envs.join(", ")
        );
    }

    // Validate that we compiled at least one environment
    if compiled_envs.is_empty() {
        return Err(CliError::Message(
            "No environments were compiled. Check that deployment files exist.".to_string(),
        ));
    }

    Ok(compiled_envs)
}

/// Compile a single environment's deployment file to an AST
fn compile_single_env(
    deployment_path: &Path,
    output_path: &Path,
    definitions: &serde_json::Value,
    skip_validation: bool,
) -> CliResult<()> {
    // Read and parse deployment
    let deployment_content = fs::read_to_string(deployment_path)
        .map_err(|e| CliError::Message(format!("Failed to read deployment file: {e}")))?;
    let deployment = parse_deployment(&deployment_content)?;

    // Validate deployment (unless skipped)
    if !skip_validation {
        validate_deployment(&deployment)
            .map_err(|e| CliError::Message(format!("Deployment file is invalid: {e}")))?;
    }

    // Compile to AST
    let artifact = compile(&deployment, definitions)?;

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
