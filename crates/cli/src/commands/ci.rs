//! CI command implementation - pipeline workflow

use crate::error::{CliError, CliResult};
use crate::ops::compile as ops_compile;
use crate::ops::compile::CompileOptions;
use crate::ops::generate_sdk as ops_generate_sdk;
use crate::ops::generate_sdk::GenerateOptions;
use crate::utils::environment;
use crate::utils::language;
use controlpath_compiler::{
    parse_definitions, parse_deployment, validate_definitions, validate_deployment,
};
use std::fs;
use std::path::PathBuf;

pub struct Options {
    /// Environment names to validate/compile (if None, processes all)
    pub envs: Option<Vec<String>>,
    /// Skip SDK generation
    pub no_sdk: bool,
    /// Skip validation
    pub no_validate: bool,
}

/// Find all deployment files in the .controlpath directory (legacy support)
fn find_deployment_files() -> CliResult<Vec<(String, PathBuf)>> {
    let controlpath_dir = PathBuf::from(".controlpath");

    if !controlpath_dir.exists() {
        return Ok(Vec::new());
    }

    let mut deployments = Vec::new();
    if let Ok(entries) = fs::read_dir(&controlpath_dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".deployment.yaml") {
                        if let Some(env_name) = name.strip_suffix(".deployment.yaml") {
                            deployments.push((env_name.to_string(), entry.path()));
                        }
                    }
                }
            }
        }
    }

    Ok(deployments)
}

/// Validate definitions file (from config or legacy file)
fn validate_definitions_file() -> CliResult<()> {
    use crate::utils::unified_config;

    if unified_config::unified_config_exists() {
        // Use config
        let unified = unified_config::read_unified_config()?;
        let definitions = unified_config::extract_definitions(&unified)?;
        validate_definitions(&definitions)
            .map_err(|e| CliError::Message(format!("Config is invalid: {e}")))?;
    } else {
        // Use legacy file-based approach
        let definitions_path = PathBuf::from("flags.definitions.yaml");
        if !definitions_path.exists() {
            return Err(CliError::Message(format!(
                "Definitions file not found: {}\n  Run 'controlpath setup' to initialize the project.",
                definitions_path.display()
            )));
        }
        let content = fs::read_to_string(&definitions_path)
            .map_err(|e| CliError::Message(format!("Failed to read definitions file: {e}")))?;
        let definitions = parse_definitions(&content)?;
        validate_definitions(&definitions)
            .map_err(|e| CliError::Message(format!("Definitions file is invalid: {e}")))?;
    }

    Ok(())
}

/// Validate deployment files (from config or legacy files)
fn validate_deployment_files(envs: Option<&[String]>) -> CliResult<usize> {
    use crate::utils::unified_config;

    if unified_config::unified_config_exists() {
        // Use config
        let unified = unified_config::read_unified_config()?;
        let all_envs = unified_config::get_environments(&unified);

        let envs_to_validate: Vec<_> = if let Some(envs) = envs {
            all_envs.into_iter().filter(|e| envs.contains(e)).collect()
        } else {
            all_envs
        };

        if envs_to_validate.is_empty() {
            return Err(CliError::Message(
                "No environments found in config to validate".to_string(),
            ));
        }

        let mut validated_count = 0;
        for env_name in &envs_to_validate {
            let deployment = unified_config::extract_deployment(&unified, env_name)?;
            validate_deployment(&deployment).map_err(|e| {
                CliError::Message(format!(
                    "Deployment for environment '{env_name}' is invalid: {e}"
                ))
            })?;
            validated_count += 1;
        }

        Ok(validated_count)
    } else {
        // Use legacy file-based approach
        let all_deployments = find_deployment_files()?;

        let deployments_to_validate: Vec<_> = if let Some(envs) = envs {
            all_deployments
                .into_iter()
                .filter(|(env_name, _)| envs.contains(env_name))
                .collect()
        } else {
            all_deployments
        };

        if deployments_to_validate.is_empty() {
            return Err(CliError::Message(
                "No deployment files found to validate".to_string(),
            ));
        }

        let mut validated_count = 0;
        for (env_name, deployment_path) in &deployments_to_validate {
            let content = fs::read_to_string(deployment_path)
                .map_err(|e| CliError::Message(format!("Failed to read deployment file: {e}")))?;
            let deployment = parse_deployment(&content)?;
            validate_deployment(&deployment).map_err(|e| {
                CliError::Message(format!("Deployment file for '{env_name}' is invalid: {e}"))
            })?;
            validated_count += 1;
        }

        Ok(validated_count)
    }
}

/// Run CI checks: validate, compile, and optionally regenerate SDK
pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => {
            println!("✓ CI checks passed");
            0
        }
        Err(e) => {
            eprintln!("✗ CI checks failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    // Determine environments to process (use smart defaults if not specified)
    use crate::utils::unified_config;

    let envs_to_process = if options.envs.is_some() {
        options.envs.clone()
    } else {
        // Try smart defaults: git branch mapping or defaultEnv
        if let Ok(Some(default_env)) = environment::determine_environment() {
            // Verify the default environment exists in config
            if unified_config::unified_config_exists() {
                let unified = unified_config::read_unified_config().ok();
                if let Some(unified) = unified {
                    let envs = unified_config::get_environments(&unified);
                    if envs.contains(&default_env) {
                        Some(vec![default_env])
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                // Legacy: check for deployment file
                let deployment_path =
                    PathBuf::from(format!(".controlpath/{default_env}.deployment.yaml"));
                if deployment_path.exists() {
                    Some(vec![default_env])
                } else {
                    None
                }
            }
        } else {
            // No smart default found, process all environments
            None
        }
    };

    // Step 1: Validate (unless --no-validate)
    if !options.no_validate {
        println!("Validating files...");

        // Validate definitions
        validate_definitions_file()?;
        println!("  ✓ Definitions file is valid");

        // Validate deployments
        let validated_count = validate_deployment_files(envs_to_process.as_deref())?;
        println!("  ✓ Validated {} deployment file(s)", validated_count);
    } else {
        println!("Skipping validation (--no-validate)");
    }

    // Step 2: Compile ASTs
    println!("Compiling ASTs...");
    let compile_opts = CompileOptions {
        envs: envs_to_process.clone(),
        skip_validation: options.no_validate,
    };

    let compiled = ops_compile::compile_envs(&compile_opts)?;
    println!(
        "  ✓ Compiled {} environment(s): {}",
        compiled.len(),
        compiled.join(", ")
    );

    // Step 3: Regenerate SDK (unless --no-sdk)
    if !options.no_sdk {
        println!("Regenerating SDK...");

        // Determine language (will use config/cached if available)
        let language = language::determine_language(None)?;

        let generate_opts = GenerateOptions {
            lang: Some(language),
            output: None,
            skip_validation: options.no_validate,
        };

        ops_generate_sdk::generate_sdk_helper(&generate_opts)?;
        println!("  ✓ SDK regenerated");
    } else {
        println!("Skipping SDK generation (--no-sdk)");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::DirGuard;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_ci_validates_and_compiles() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create .controlpath directory for AST output
        fs::create_dir_all(".controlpath").unwrap();

        // Create config file
        fs::write(
            "control-path.yaml",
            r"mode: local
flags:
  - name: test_flag
    type: boolean
    default: false
    environments:
      production:
        - serve: true
",
        )
        .unwrap();

        let options = Options {
            envs: Some(vec!["production".to_string()]), // Explicitly specify environment
            no_sdk: true,                               // Skip SDK for this test
            no_validate: false,
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);

        // Verify AST was created
        assert!(PathBuf::from(".controlpath/production.ast").exists());
    }

    #[test]
    #[serial]
    fn test_ci_respects_env_filter() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create config file
        fs::write(
            "control-path.yaml",
            r"mode: local
flags:
  - name: test_flag
    type: boolean
    default: false
    environments:
      production:
        - serve: true
      staging:
        - serve: false
",
        )
        .unwrap();

        let options = Options {
            envs: Some(vec!["production".to_string()]),
            no_sdk: true,
            no_validate: false,
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);

        // Verify only production AST was created
        assert!(PathBuf::from(".controlpath/production.ast").exists());
        assert!(!PathBuf::from(".controlpath/staging.ast").exists());
    }

    #[test]
    #[serial]
    fn test_ci_respects_no_validate() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create .controlpath directory for AST output
        fs::create_dir_all(".controlpath").unwrap();

        // Create config file
        fs::write(
            "control-path.yaml",
            r"mode: local
flags:
  - name: test_flag
    type: boolean
    default: false
    environments:
      production:
        - serve: true
",
        )
        .unwrap();

        let options = Options {
            envs: Some(vec!["production".to_string()]), // Explicitly specify environment
            no_sdk: true,
            no_validate: true,
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);

        // Verify AST was created even without validation
        assert!(PathBuf::from(".controlpath/production.ast").exists());
    }

    #[test]
    #[serial]
    fn test_ci_fails_on_invalid_definitions() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create invalid config file (missing required "default" field)
        fs::write(
            "control-path.yaml",
            r"mode: local
flags:
  - name: test_flag
    type: boolean
",
        )
        .unwrap();

        let options = Options {
            envs: None,
            no_sdk: true,
            no_validate: false,
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 1);
    }
}
