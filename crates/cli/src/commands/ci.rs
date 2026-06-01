//! CI command implementation - pipeline workflow

use crate::error::{CliError, CliResult};
use crate::ops::compile as ops_compile;
use crate::ops::compile::CompileOptions;
use crate::ops::generate_sdk as ops_generate_sdk;
use crate::ops::generate_sdk::GenerateOptions;
use crate::saas::{
    build_flag_rot_report, fetch_saas_telemetry, load_saas_catalog_for_ci,
    remote_ast_options_from_catalog, sync_saas_catalog_with_catalog, warn_on_rot_findings,
    FakeSaasClient,
};
use crate::utils::catalog;
use crate::utils::language;
use crate::utils::runtime;
use crate::utils::unified_config;
use controlpath_compiler::effective_catalog_id;
use std::env;

pub struct Options {
    /// Environment names to validate/compile (if None, processes all)
    pub envs: Option<Vec<String>>,
    /// Skip SDK generation
    pub no_sdk: bool,
    /// Skip validation
    pub no_validate: bool,
}

/// Validate catalog from control-path.yaml.
fn validate_definitions_file() -> CliResult<()> {
    let base_dir = env::current_dir()
        .map_err(|e| CliError::Message(format!("Failed to resolve working directory: {e}")))?;
    catalog::load_sdk_catalog(&base_dir)?;
    Ok(())
}

/// Validate deployment environments from unified config.
fn validate_deployment_files(envs: Option<&[String]>) -> CliResult<usize> {
    let unified = unified_config::read_unified_config()?;
    let all_envs = unified_config::get_environments(&unified);

    let envs_to_validate: Vec<String> = if let Some(requested) = envs {
        let unknown: Vec<String> = requested
            .iter()
            .filter(|e| !all_envs.contains(*e))
            .cloned()
            .collect();
        if !unknown.is_empty() {
            return Err(CliError::Message(format!(
                "Unknown environment(s): {}",
                unknown.join(", ")
            )));
        }
        requested.to_vec()
    } else {
        all_envs
    };

    if envs_to_validate.is_empty() {
        return Err(CliError::Message(
            "No environments found in control-path.yaml to validate".to_string(),
        ));
    }

    Ok(envs_to_validate.len())
}

/// Run CI checks: validate, compile, and optionally regenerate SDK
pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "ok",
                        "command": "ci"
                    })
                );
            } else {
                println!("✓ CI checks passed");
                println!();
                println!("Next steps:");
                println!("  • Deploy changes:      controlpath deploy");
                println!(
                    "  • Test flag evaluation: controlpath explain --flag <flag-name> --env <env>"
                );
                println!("  • View flags:          controlpath flag list");
            }
            0
        }
        Err(e) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "command": "ci",
                        "error": e.to_string()
                    })
                );
            } else {
                eprintln!("✗ CI checks failed");
                eprintln!("  Error: {e}");
            }
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    let unified = unified_config::read_unified_config()?;
    if unified_config::is_saas_mode(&unified) {
        return run_saas_inner(options);
    }

    // CI strict mode: process all environments unless explicitly filtered.
    let envs_to_process = options.envs.clone();

    // Step 1: Validate (unless --no-validate)
    if !options.no_validate {
        if !runtime::is_json_output() {
            println!("Validating files...");
        }

        // Validate catalog
        validate_definitions_file()?;
        if !runtime::is_json_output() {
            println!("  ✓ Catalog is valid");
        }

        // Confirm requested environments exist
        let validated_count = validate_deployment_files(envs_to_process.as_deref())?;
        if !runtime::is_json_output() {
            println!("  ✓ Validated {} environment(s)", validated_count);
        }
    } else if !runtime::is_json_output() {
        println!("Skipping validation (--no-validate)");
    }

    // Step 2: Compile ASTs
    if !runtime::is_json_output() {
        println!("Compiling ASTs...");
    }
    let compile_opts = CompileOptions {
        envs: envs_to_process.clone(),
        skip_validation: options.no_validate,
    };

    let compiled = ops_compile::compile_envs(&compile_opts)?;
    if !runtime::is_json_output() {
        println!(
            "  ✓ Compiled {} environment(s): {}",
            compiled.len(),
            compiled.join(", ")
        );
    }

    // Step 3: Regenerate SDK (unless --no-sdk)
    if !options.no_sdk {
        if !runtime::is_json_output() {
            println!("Regenerating SDK...");
        }

        // Determine language (will use config/cached if available)
        let language = language::determine_language(None)?;

        let generate_opts = GenerateOptions {
            lang: Some(language),
            output: None,
            skip_validation: options.no_validate,
        };

        ops_generate_sdk::generate_sdk_helper(&generate_opts)?;
        if !runtime::is_json_output() {
            println!("  ✓ SDK regenerated");
        }
    } else if !runtime::is_json_output() {
        println!("Skipping SDK generation (--no-sdk)");
    }

    Ok(())
}

fn run_saas_inner(options: &Options) -> CliResult<()> {
    let base_dir = env::current_dir()
        .map_err(|e| CliError::Message(format!("Failed to resolve working directory: {e}")))?;

    let validate = !options.no_validate;
    let (catalog, workspace) = load_saas_catalog_for_ci(&base_dir, validate)?;
    let ast_options = remote_ast_options_from_catalog(&catalog)?;

    if validate {
        if !runtime::is_json_output() {
            println!("Validating catalog...");
            println!("  ✓ Catalog is valid");
        }
    } else if !runtime::is_json_output() {
        println!("Skipping validation (--no-validate)");
    }

    if !runtime::is_json_output() {
        println!("Syncing catalog to SaaS...");
    }
    // Offline fake boundary for issue 06; swap for an HTTP SaasClient when the backend exists.
    let mut client = FakeSaasClient::open(&base_dir)?;
    let outcome = sync_saas_catalog_with_catalog(
        &base_dir,
        &catalog,
        workspace.as_ref(),
        &mut client,
        &ast_options,
    )?;
    if !runtime::is_json_output() {
        if outcome.catalog_sync.upserted_flags.is_empty()
            && outcome.catalog_sync.retired_flags.is_empty()
        {
            println!("  ✓ Catalog is in sync");
        } else {
            if !outcome.catalog_sync.upserted_flags.is_empty() {
                println!(
                    "  ✓ Synced {} flag(s)",
                    outcome.catalog_sync.upserted_flags.len()
                );
            }
            if !outcome.catalog_sync.retired_flags.is_empty() {
                println!(
                    "  ✓ Retired {} flag(s)",
                    outcome.catalog_sync.retired_flags.len()
                );
            }
        }
        if !outcome.downloaded_envs.is_empty() {
            println!(
                "  ✓ Downloaded {} remote AST artifact(s): {}",
                outcome.downloaded_envs.len(),
                outcome.downloaded_envs.join(", ")
            );
        }
    }

    if validate && !runtime::is_json_output() {
        let sdk_catalog = catalog::load_sdk_catalog(&base_dir)?;
        let catalog_id = effective_catalog_id(&catalog.catalog, workspace.as_ref());
        let project = catalog
            .saas
            .as_ref()
            .map(|s| s.project.as_str())
            .ok_or_else(|| {
                CliError::Message(
                    "SaaS mode requires saas.project in control-path.yaml".to_string(),
                )
            })?;
        let telemetry = fetch_saas_telemetry(&client, &catalog_id, project)?;
        let entries = build_flag_rot_report(&sdk_catalog, &telemetry);
        warn_on_rot_findings(&entries);
    }

    if !options.no_sdk {
        if !runtime::is_json_output() {
            println!("Regenerating SDK...");
        }
        let language = language::determine_language(None)?;
        let generate_opts = GenerateOptions {
            lang: Some(language),
            output: None,
            skip_validation: options.no_validate,
        };
        ops_generate_sdk::generate_sdk_helper(&generate_opts)?;
        if !runtime::is_json_output() {
            println!("  ✓ SDK regenerated");
        }
    } else if !runtime::is_json_output() {
        println!("Skipping SDK generation (--no-sdk)");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{v2_test_catalog, DirGuard};
    use serial_test::serial;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_ci_validates_and_compiles() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create .controlpath directory for AST output
        fs::create_dir_all(".controlpath").unwrap();

        fs::write("control-path.yaml", v2_test_catalog("test_flag", true)).unwrap();

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

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  test_flag:
    default: false
    kind: release
environments:
  production:
    rules:
      test_flag:
        - serve: true
  staging:
    rules:
      test_flag:
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

        fs::write("control-path.yaml", v2_test_catalog("test_flag", true)).unwrap();

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

        // Invalid v2 catalog: missing required catalog block
        fs::write(
            "control-path.yaml",
            r"mode: local
flags:
  test_flag:
    default: false
    kind: release
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
