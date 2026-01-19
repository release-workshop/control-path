//! CI command implementation - pipeline workflow

use crate::error::{CliError, CliResult};
use crate::monorepo::ServiceContext;
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
    /// Service context for monorepo support
    pub service_context: Option<ServiceContext>,
}

/// Find all deployment files in the .controlpath directory
fn find_deployment_files(
    service_context: Option<&ServiceContext>,
) -> CliResult<Vec<(String, PathBuf)>> {
    use crate::monorepo::ServicePathResolver;

    let controlpath_dir = if let Some(ctx) = service_context {
        let resolver = ServicePathResolver::new(ctx.clone());
        resolver.base_path().join(".controlpath")
    } else {
        PathBuf::from(".controlpath")
    };

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

/// Validate definitions file
fn validate_definitions_file(service_context: Option<&ServiceContext>) -> CliResult<()> {
    use crate::monorepo::ServicePathResolver;

    let definitions_path = if let Some(ctx) = service_context {
        let resolver = ServicePathResolver::new(ctx.clone());
        resolver.definitions_file()
    } else {
        PathBuf::from("flags.definitions.yaml")
    };

    if !definitions_path.exists() {
        return Err(CliError::Message(format!(
            "Definitions file not found: {}",
            definitions_path.display()
        )));
    }

    let content = fs::read_to_string(&definitions_path)
        .map_err(|e| CliError::Message(format!("Failed to read definitions file: {e}")))?;
    let definitions = parse_definitions(&content)?;
    validate_definitions(&definitions)
        .map_err(|e| CliError::Message(format!("Definitions file is invalid: {e}")))?;

    Ok(())
}

/// Validate deployment files
fn validate_deployment_files(
    envs: Option<&[String]>,
    service_context: Option<&ServiceContext>,
) -> CliResult<usize> {
    let all_deployments = find_deployment_files(service_context)?;

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
    let envs_to_process = if options.envs.is_some() {
        options.envs.clone()
    } else {
        // Try smart defaults: git branch mapping or defaultEnv
        if let Ok(Some(default_env)) = environment::determine_environment() {
            // Verify the default environment exists
            let deployment_path =
                PathBuf::from(format!(".controlpath/{default_env}.deployment.yaml"));
            if deployment_path.exists() {
                Some(vec![default_env])
            } else {
                // Default env doesn't exist, process all environments
                None
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
        validate_definitions_file(options.service_context.as_ref())?;
        println!("  ✓ Definitions file is valid");

        // Validate deployments
        let validated_count = validate_deployment_files(
            envs_to_process.as_deref(),
            options.service_context.as_ref(),
        )?;
        println!("  ✓ Validated {} deployment file(s)", validated_count);
    } else {
        println!("Skipping validation (--no-validate)");
    }

    // Step 2: Compile ASTs
    println!("Compiling ASTs...");
    let compile_opts = CompileOptions {
        envs: envs_to_process.clone(),
        service_context: options.service_context.clone(),
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
            service_context: options.service_context.clone(),
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
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    struct DirGuard {
        original_dir: PathBuf,
    }

    impl DirGuard {
        fn new(temp_path: &std::path::Path) -> Self {
            fs::create_dir_all(temp_path).unwrap();
            let original_dir = std::env::current_dir().unwrap();
            std::env::set_current_dir(temp_path).unwrap();
            DirGuard { original_dir }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original_dir);
        }
    }

    #[test]
    #[serial]
    fn test_ci_validates_and_compiles() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create test files
        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: test_flag
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules:
  test_flag:
    rules:
      - serve: true
",
        )
        .unwrap();

        let options = Options {
            envs: None,
            no_sdk: true, // Skip SDK for this test
            no_validate: false,
            service_context: None,
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
        let _guard = DirGuard::new(temp_path);

        // Create test files
        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: test_flag
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules:
  test_flag:
    rules:
      - serve: true
",
        )
        .unwrap();
        fs::write(
            ".controlpath/staging.deployment.yaml",
            r"environment: staging
rules:
  test_flag:
    rules:
      - serve: false
",
        )
        .unwrap();

        let options = Options {
            envs: Some(vec!["production".to_string()]),
            no_sdk: true,
            no_validate: false,
            service_context: None,
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
        let _guard = DirGuard::new(temp_path);

        // Create test files
        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: test_flag
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules:
  test_flag:
    rules:
      - serve: true
",
        )
        .unwrap();

        let options = Options {
            envs: None,
            no_sdk: true,
            no_validate: true,
            service_context: None,
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
        let _guard = DirGuard::new(temp_path);

        // Create invalid definitions file
        fs::write("flags.definitions.yaml", "invalid: yaml: content: [").unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules: {}
",
        )
        .unwrap();

        let options = Options {
            envs: None,
            no_sdk: true,
            no_validate: false,
            service_context: None,
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 1);
    }
}
