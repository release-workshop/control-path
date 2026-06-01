//! Workflow commands implementation
//!
//! These commands combine multiple operations to provide complete workflows:
//! - new-flag: Adds flag, syncs to environments, and regenerates SDK
//! - enable: Enables a flag in environments with rules
//! - deploy: Validates and compiles for deployment

use crate::commands::{compile, validate};
use crate::error::{CliError, CliResult};
use crate::ops::{compile as ops_compile, generate_sdk as ops_generate_sdk};
use crate::utils::environment;
use crate::utils::runtime;
use crate::utils::unified_config;
use controlpath_compiler::compiler::expressions::parse_expression;
use dialoguer::Input;
use serde_json::Value;

// ============================================================================
// new-flag command
// ============================================================================

pub struct NewFlagOptions {
    pub name: Option<String>,
    pub flag_type: Option<String>,
    pub default: Option<String>,
    pub description: Option<String>,
    pub enable_in: Option<String>, // Comma-separated environments
    pub skip_sdk: bool,
    pub best_effort: bool,
}

// Write config
fn write_unified(config: &Value) -> CliResult<()> {
    unified_config::write_unified_config(config)
}

fn validate_flag_name(name: &str) -> CliResult<()> {
    if name.is_empty() {
        return Err(CliError::Message("Flag name cannot be empty".to_string()));
    }
    // Flag names should be snake_case
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(CliError::Message(
            "Flag name must be snake_case (lowercase letters, digits, and underscores only)"
                .to_string(),
        ));
    }
    if !name.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
        return Err(CliError::Message(
            "Flag name must start with a lowercase letter".to_string(),
        ));
    }
    Ok(())
}

fn check_flag_exists(unified: &Value, name: &str) -> bool {
    unified_config::flag_exists(unified, name)
}

// ============================================================================
// Config helper functions
// ============================================================================

fn catalog_flag_names_for_error(unified: &Value) -> Vec<String> {
    unified
        .get("flags")
        .and_then(|f| f.as_object())
        .map(|flags| {
            let mut names: Vec<String> = flags.keys().cloned().collect();
            names.sort();
            names
        })
        .unwrap_or_default()
}

fn default_bool_from_flag(unified: &Value, flag_name: &str) -> Value {
    unified
        .get("flags")
        .and_then(|f| f.get(flag_name))
        .and_then(|flag| flag.get("default"))
        .cloned()
        .unwrap_or(Value::Bool(false))
}

pub fn run_new_flag(options: &NewFlagOptions) -> i32 {
    match run_new_flag_inner(options) {
        Ok(flag_name) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "ok",
                        "command": "new-flag",
                        "flag": flag_name,
                        "warnings": [],
                        "errors": []
                    })
                );
            } else {
                println!("✓ Flag '{flag_name}' added successfully");
                println!();
                println!("Next steps:");
                println!(
                    "  • Enable in staging:    controlpath flag enable {flag_name} --env staging"
                );
                println!("  • Explain flag:        controlpath explain --flag {flag_name}");
                println!("  • View flag details:    controlpath flag show {flag_name}");
            }
            0
        }
        Err(e) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "command": "new-flag",
                        "warnings": [],
                        "errors": [e.to_string()]
                    })
                );
            } else {
                eprintln!("✗ Failed to add flag");
                eprintln!("  Error: {e}");
            }
            1
        }
    }
}

fn run_new_flag_inner(options: &NewFlagOptions) -> CliResult<String> {
    let mut unified = unified_config::read_unified_config()?;

    let flag_name = if let Some(ref name) = options.name {
        validate_flag_name(name)?;
        if check_flag_exists(&unified, name) {
            return Err(CliError::Message(format!("Flag '{}' already exists", name)));
        }
        name.clone()
    } else {
        runtime::require_interactive("prompt for a flag name")?;
        let name: String = Input::new()
            .with_prompt("Flag name (snake_case)")
            .validate_with(|input: &String| -> Result<(), String> {
                validate_flag_name(input).map_err(|e| e.to_string())?;
                if check_flag_exists(&unified, input) {
                    Err(format!("Flag '{}' already exists", input))
                } else {
                    Ok(())
                }
            })
            .interact()
            .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;
        name
    };

    let flag_type = options
        .flag_type
        .as_deref()
        .unwrap_or("boolean")
        .to_string();

    if flag_type != "boolean" {
        return Err(CliError::Message(
            "v2 catalogs support boolean flags only".to_string(),
        ));
    }

    let default_value = if let Some(ref default_str) = options.default {
        if default_str == "true" || default_str == "True" {
            Value::Bool(true)
        } else {
            Value::Bool(false)
        }
    } else {
        Value::Bool(false)
    };

    let default_bool = match default_value {
        Value::Bool(b) => b,
        _ => {
            return Err(CliError::Message(
                "v2 catalogs require a boolean default (true/false)".to_string(),
            ))
        }
    };

    let description = options.description.as_deref();

    unified_config::add_flag(
        &mut unified,
        &flag_name,
        default_bool,
        "release",
        description,
        &[],
    )?;
    write_unified(&unified)?;
    if !runtime::is_json_output() {
        println!("✓ Added flag to configuration");
    }

    // Enable in specified environments
    let mut enabled_envs = Vec::new();
    if let Some(ref enable_envs) = options.enable_in {
        let envs: Vec<&str> = enable_envs.split(',').map(|s| s.trim()).collect();

        // Re-read config to get latest state
        let mut unified = unified_config::read_unified_config()?;

        for env in envs {
            // Determine serve value (opposite of default for boolean, or default for others)
            let serve_value = match &default_value {
                Value::Bool(b) => Value::Bool(!b),
                _ => default_value.clone(),
            };

            let serve_bool = match serve_value {
                Value::Bool(b) => b,
                _ => default_bool,
            };
            unified_config::enable_flag_in_environment(
                &mut unified,
                &flag_name,
                env,
                None,
                serve_bool,
                false,
            )?;
            if !runtime::is_json_output() {
                println!("✓ Enabled flag in {env}");
            }
            enabled_envs.push(env.to_string());
        }

        // Write config back
        write_unified(&unified)?;

        // Auto-compile ASTs for enabled environments
        if !enabled_envs.is_empty() {
            if !runtime::is_json_output() {
                println!("Compiling ASTs for enabled environments...");
            }
            let compile_opts = ops_compile::CompileOptions {
                envs: Some(enabled_envs.clone()),
            };
            match ops_compile::compile_envs(&compile_opts) {
                Ok(compiled) => {
                    for env in &compiled {
                        if !runtime::is_json_output() {
                            println!("✓ Compiled AST for {env}");
                        }
                    }
                }
                Err(e) => {
                    if options.best_effort {
                        eprintln!("⚠ Warning: Failed to compile ASTs: {e}");
                        eprintln!(
                            "  You can compile manually with: controlpath compile --env {}",
                            enabled_envs.join(",")
                        );
                    } else {
                        return Err(CliError::Message(format!(
                            "Failed to compile ASTs after adding flag '{}': {e}",
                            flag_name
                        )));
                    }
                }
            }
        }
    }

    // Regenerate SDK (unless skipped)
    if !options.skip_sdk {
        if !runtime::is_json_output() {
            println!("Regenerating SDK...");
        }
        let generate_opts = ops_generate_sdk::GenerateOptions {
            lang: None,
            output: None,
        };
        match ops_generate_sdk::generate_sdk_helper(&generate_opts) {
            Ok(()) => {
                if !runtime::is_json_output() {
                    println!("✓ Regenerated SDK");
                }
            }
            Err(e) => {
                if options.best_effort {
                    eprintln!("⚠ Warning: SDK regeneration failed: {e}");
                    eprintln!("  You can regenerate manually with: controlpath generate-sdk");
                } else {
                    return Err(CliError::Message(format!(
                        "SDK regeneration failed after adding flag '{}': {e}",
                        flag_name
                    )));
                }
            }
        }
    }

    Ok(flag_name)
}

// ============================================================================
// enable command
// ============================================================================

pub struct EnableOptions {
    pub name: String,
    pub env: Option<String>, // Comma-separated environments
    pub rule: Option<String>,
    pub all: bool,             // Enable for all users (no rule)
    pub value: Option<String>, // Value to serve
    pub interactive: bool,
    pub no_compile: bool, // Skip automatic compilation
    pub best_effort: bool,
    pub force: bool,
}

pub fn run_enable(options: &EnableOptions) -> i32 {
    match run_enable_inner(options) {
        Ok(envs) => {
            if envs.is_empty() {
                if runtime::is_json_output() {
                    println!(
                        "{}",
                        serde_json::json!({
                            "status": "error",
                            "command": "enable",
                            "warnings": [],
                            "errors": ["No environments were updated"]
                        })
                    );
                } else {
                    eprintln!("⚠ No environments were updated");
                }
                return 1;
            }
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "ok",
                        "command": "enable",
                        "flag": options.name,
                        "environments": envs,
                        "warnings": [],
                        "errors": []
                    })
                );
            } else {
                println!("✓ Flag '{}' enabled in: {}", options.name, envs.join(", "));
                println!();
                println!("Next steps:");
                if let Some(first_env) = envs.first() {
                    println!(
                        "  • Explain flag:        controlpath explain --flag {} --env {}",
                        options.name, first_env
                    );
                }
                println!(
                    "  • Deploy changes:      controlpath deploy --env {}",
                    envs.join(",")
                );
                if envs.len() == 1 {
                    println!(
                        "  • Enable in production: controlpath flag enable {} --env production",
                        options.name
                    );
                }
            }
            0
        }
        Err(e) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "command": "enable",
                        "flag": options.name,
                        "warnings": [],
                        "errors": [e.to_string()]
                    })
                );
            } else {
                eprintln!("✗ Failed to enable flag");
                eprintln!("  Error: {e}");
            }
            1
        }
    }
}

fn run_enable_inner(options: &EnableOptions) -> CliResult<Vec<String>> {
    let mut unified = unified_config::read_unified_config()?;

    if !unified_config::flag_exists(&unified, &options.name) {
        let available_flags = catalog_flag_names_for_error(&unified);
        let mut msg = format!("Flag '{}' not found in configuration.", options.name);
        if !available_flags.is_empty() {
            msg.push_str(&format!(
                "\n  Available flags: {}",
                available_flags.join(", ")
            ));
        }
        msg.push_str("\n  Tip: Use 'controlpath flag list' to see all flags");
        msg.push_str(&format!(
            "\n  Or add it with: controlpath new-flag {}",
            options.name
        ));
        return Err(CliError::Message(msg));
    }

    let default_value = default_bool_from_flag(&unified, &options.name);

    // Get environments (interactive if not provided)
    let envs = if let Some(ref env_str) = options.env {
        env_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        // Try smart defaults: git branch mapping or defaultEnv
        if let Ok(Some(default_env)) = environment::determine_environment() {
            // Check if environment exists in config
            let all_envs = unified_config::get_environments(&unified);
            if all_envs.contains(&default_env) {
                vec![default_env]
            } else {
                // Default env doesn't exist, fall through to finding available environments
                find_envs_for_enable_unified(&unified, options)?
            }
        } else {
            // No smart default found, find available environments
            find_envs_for_enable_unified(&unified, options)?
        }
    };

    // Get rule expression
    let rule_expr = if options.all {
        None // No rule, just serve default
    } else if let Some(ref rule) = options.rule {
        // Validate rule expression before using it
        parse_expression(rule)
            .map_err(|e| CliError::Message(format!("Invalid rule expression: {e}")))?;
        Some(rule.clone())
    } else if options.interactive {
        // Interactive rule builder
        runtime::require_interactive("prompt for a rule expression")?;
        if !runtime::is_json_output() {
            println!();
            println!("Interactive mode: We'll guide you through enabling the flag");
            println!();
            println!("Rule expression examples:");
            println!("  • Enable for admins:        user.role == 'admin'");
            println!("  • Enable for percentage:     user.id % 100 < 10");
            println!("  • Enable for specific users: user.id IN ['user1', 'user2']");
            println!("  • Enable for all users:      (leave empty)");
            println!();
        }
        let rule: String = Input::new()
            .with_prompt("Rule expression (leave empty to enable for all)")
            .allow_empty(true)
            .validate_with(|input: &String| -> Result<(), String> {
                if input.is_empty() {
                    return Ok(());
                }
                parse_expression(input).map_err(|e| format!("Invalid expression: {e}"))?;
                Ok(())
            })
            .interact()
            .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;
        if rule.is_empty() {
            None
        } else {
            Some(rule)
        }
    } else {
        None // Default: enable for all (no rule)
    };

    // Get value to serve
    let serve_value = options.value.as_deref();

    // Determine serve value
    let serve_val = if let Some(val_str) = serve_value {
        if val_str == "true" || val_str == "True" {
            Value::Bool(true)
        } else {
            Value::Bool(false)
        }
    } else {
        match &default_value {
            Value::Bool(b) => Value::Bool(!*b),
            _ => default_value.clone(),
        }
    };

    let serve_bool = match serve_val {
        Value::Bool(b) => b,
        _ => {
            return Err(CliError::Message(
                "v2 catalogs only support boolean serve values".to_string(),
            ))
        }
    };

    let mut updated_envs = Vec::new();
    for env in &envs {
        unified_config::enable_flag_in_environment(
            &mut unified,
            &options.name,
            env,
            rule_expr.as_deref(),
            serve_bool,
            options.force,
        )?;
        updated_envs.push(env.clone());
    }

    // Write config back
    write_unified(&unified)?;

    // Auto-compile ASTs for updated environments (unless --no-compile)
    if !options.no_compile && !updated_envs.is_empty() {
        if !runtime::is_json_output() {
            println!("Compiling ASTs for updated environments...");
        }
        let compile_opts = ops_compile::CompileOptions {
            envs: Some(updated_envs.clone()),
        };
        match ops_compile::compile_envs(&compile_opts) {
            Ok(compiled) => {
                for env in &compiled {
                    if !runtime::is_json_output() {
                        println!("✓ Compiled AST for {env}");
                    }
                }
            }
            Err(e) => {
                if options.best_effort {
                    eprintln!("⚠ Warning: Failed to compile ASTs: {e}");
                    eprintln!(
                        "  You can compile manually with: controlpath compile --env {}",
                        updated_envs.join(",")
                    );
                } else {
                    return Err(CliError::Message(format!(
                        "Failed to compile ASTs after enabling flag '{}': {e}",
                        options.name
                    )));
                }
            }
        }
    }

    Ok(updated_envs)
}

/// Helper function to find environments for enable command when no env specified (config)
fn find_envs_for_enable_unified(
    unified: &Value,
    _options: &EnableOptions,
) -> CliResult<Vec<String>> {
    let envs = unified_config::get_environments(unified);
    if envs.is_empty() {
        return Err(CliError::Message(
            "No environments found. Add flags with environment rules first.".to_string(),
        ));
    }
    Ok(envs)
}

// ============================================================================
// deploy command
// ============================================================================

pub struct DeployOptions {
    pub env: Option<String>, // Comma-separated environments
    pub dry_run: bool,
}

pub fn run_deploy(options: &DeployOptions) -> i32 {
    match run_deploy_inner(options) {
        Ok(envs) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "ok",
                        "command": "deploy",
                        "dryRun": options.dry_run,
                        "environments": envs,
                        "warnings": [],
                        "errors": []
                    })
                );
            } else if options.dry_run {
                println!("✓ Dry run completed successfully");
                println!("  Would deploy to: {}", envs.join(", "));
            } else {
                println!("✓ Deployment ready");
                println!();
                println!("AST artifacts compiled:");
                for env in &envs {
                    println!("  • .controlpath/{env}.ast");
                    println!("  • .controlpath/{env}.kill-switches.json");
                }
                println!();
                println!("Next steps:");
                println!("  • Copy AST files to your deployment location");
                println!("  • Restart your application to load new flags");
            }
            0
        }
        Err(e) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "command": "deploy",
                        "warnings": [],
                        "errors": [e.to_string()]
                    })
                );
            } else {
                eprintln!("✗ Deployment failed");
                eprintln!("  Error: {e}");
            }
            1
        }
    }
}

fn run_deploy_inner(options: &DeployOptions) -> CliResult<Vec<String>> {
    let unified = unified_config::read_unified_config()?;
    let all_envs = unified_config::get_environments(&unified);

    // Get environments
    let envs = if let Some(ref env_str) = options.env {
        env_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        // Try smart defaults: git branch mapping or defaultEnv
        if let Ok(Some(default_env)) = environment::determine_environment() {
            if all_envs.contains(&default_env) {
                vec![default_env]
            } else {
                if all_envs.is_empty() {
                    return Err(CliError::Message(
                        "No environments found in control-path.yaml.\n  Tip: Add flags with environment rules first using:\n    controlpath new-flag <flag-name> --enable-in <env>\n  Or enable an existing flag:\n    controlpath flag enable <flag-name> --env <env>".to_string(),
                    ));
                }
                all_envs
            }
        } else {
            if all_envs.is_empty() {
                return Err(CliError::Message(
                    "No environments found in control-path.yaml.\n  Tip: Add flags with environment rules first using:\n    controlpath new-flag <flag-name> --enable-in <env>\n  Or enable an existing flag:\n    controlpath flag enable <flag-name> --env <env>".to_string(),
                ));
            }
            all_envs
        }
    };

    if !runtime::is_json_output() {
        println!("Validating catalog and environments...");
    }
    let validate_opts = validate::Options {
        env: None,
        all: true,
    };
    let exit_code = validate::run(&validate_opts);
    if exit_code != 0 {
        return Err(CliError::Message("Validation failed".to_string()));
    }
    if !runtime::is_json_output() {
        println!("✓ Validation passed");
    }

    // Compile each environment
    for env in &envs {
        if options.dry_run {
            if !runtime::is_json_output() {
                println!("  Would compile: .controlpath/{env}.ast");
            }
        } else {
            let compile_opts = compile::Options {
                env: Some(String::from(env)),
                output: None,
            };
            let exit_code = compile::run(&compile_opts);
            if exit_code != 0 {
                return Err(CliError::Message(format!(
                    "Compilation failed for environment: {env}"
                )));
            }
        }
    }

    Ok(envs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    use crate::test_helpers::DirGuard;

    fn setup_test_project() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join(".controlpath")).unwrap();

        let config = r#"catalog:
  id: test-service
mode: local
flags:
  existing_flag:
    default: false
    kind: release
    description: An existing flag
environments:
  production:
    rules:
      existing_flag:
        - serve: false
"#;
        fs::write(temp_path.join("control-path.yaml"), config).unwrap();

        temp_dir
    }

    #[test]
    #[serial]
    fn test_new_flag_basic() {
        let temp_dir = setup_test_project();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();

        let options = NewFlagOptions {
            name: Some("test_flag".to_string()),
            flag_type: Some("boolean".to_string()),
            default: Some("false".to_string()),
            description: Some("Test flag".to_string()),
            enable_in: None,
            skip_sdk: true,
            best_effort: false,
        };

        let result = run_new_flag_inner(&options);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_flag");

        let unified = unified_config::read_unified_config().unwrap();
        assert!(unified_config::flag_exists(&unified, "test_flag"));
    }

    #[test]
    #[serial]
    fn test_enable_flag() {
        let temp_dir = setup_test_project();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();

        // First add a flag
        let new_flag_options = NewFlagOptions {
            name: Some("test_flag".to_string()),
            flag_type: Some("boolean".to_string()),
            default: Some("false".to_string()),
            description: None,
            enable_in: None,
            skip_sdk: true,
            best_effort: false,
        };
        run_new_flag_inner(&new_flag_options).unwrap();

        // Now enable it
        let enable_options = EnableOptions {
            name: "test_flag".to_string(),
            env: Some("production".to_string()),
            rule: None,
            all: true,
            value: None,
            interactive: false,
            no_compile: false,
            best_effort: false,
            force: false,
        };

        let result = run_enable_inner(&enable_options);
        assert!(result.is_ok());

        let unified = unified_config::read_unified_config().unwrap();
        let env_rules = unified
            .get("environments")
            .and_then(|e| e.get("production"))
            .and_then(|e| e.get("rules"))
            .and_then(|r| r.get("test_flag"))
            .and_then(|r| r.as_array())
            .expect("production rules for test_flag");

        let serve_value = env_rules[0].get("serve").and_then(|s| s.as_bool()).unwrap();
        assert!(serve_value); // Should be true (opposite of default false)
    }

    #[test]
    #[serial]
    fn test_deploy() {
        let temp_dir = setup_test_project();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();

        let options = DeployOptions {
            env: Some("production".to_string()),
            dry_run: true,
        };

        let result = run_deploy_inner(&options);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["production"]);
    }

    #[test]
    #[serial]
    fn test_enable_with_rule_expression() {
        let temp_dir = setup_test_project();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();

        // First add a flag
        let new_flag_options = NewFlagOptions {
            name: Some("test_flag".to_string()),
            flag_type: Some("boolean".to_string()),
            default: Some("false".to_string()),
            description: None,
            enable_in: None,
            skip_sdk: true,
            best_effort: false,
        };
        run_new_flag_inner(&new_flag_options).unwrap();

        // Enable with a rule expression
        let enable_options = EnableOptions {
            name: "test_flag".to_string(),
            env: Some("production".to_string()),
            rule: Some("user.role == 'admin'".to_string()),
            all: false,
            value: None,
            interactive: false,
            no_compile: false,
            best_effort: false,
            force: false,
        };

        let result = run_enable_inner(&enable_options);
        assert!(result.is_ok());

        // Verify rule was created with "when" field (not "if")
        // Verify flag was enabled with rule expression in config
        let unified = unified_config::read_unified_config().unwrap();
        let env_rules = unified
            .get("environments")
            .and_then(|e| e.get("production"))
            .and_then(|e| e.get("rules"))
            .and_then(|r| r.get("test_flag"))
            .and_then(|r| r.as_array())
            .expect("production rules for test_flag");

        let rule = env_rules.last().expect("at least one rule");
        assert!(rule.get("when").is_some(), "Rule should have 'when' field");
        assert_eq!(
            rule.get("when").and_then(|w| w.as_str()),
            Some("user.role == 'admin'")
        );
        assert!(rule.get("if").is_none(), "Rule should not have 'if' field");
    }

    #[test]
    #[serial]
    fn test_enable_invalid_rule_expression() {
        let temp_dir = setup_test_project();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();

        // First add a flag
        let new_flag_options = NewFlagOptions {
            name: Some("test_flag".to_string()),
            flag_type: Some("boolean".to_string()),
            default: Some("false".to_string()),
            description: None,
            enable_in: None,
            skip_sdk: true,
            best_effort: false,
        };
        run_new_flag_inner(&new_flag_options).unwrap();

        // Try to enable with invalid rule expression
        let enable_options = EnableOptions {
            name: "test_flag".to_string(),
            env: Some("production".to_string()),
            rule: Some("invalid expression syntax".to_string()),
            all: false,
            value: None,
            interactive: false,
            no_compile: false,
            best_effort: false,
            force: false,
        };

        let result = run_enable_inner(&enable_options);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid rule expression"));
    }

    #[test]
    #[serial]
    fn test_new_flag_enable_in_nonexistent_env() {
        let temp_dir = setup_test_project();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();

        let options = NewFlagOptions {
            name: Some("test_flag".to_string()),
            flag_type: Some("boolean".to_string()),
            default: Some("false".to_string()),
            description: None,
            enable_in: Some("nonexistent".to_string()),
            skip_sdk: true,
            best_effort: false,
        };

        // Should succeed - environments are created automatically
        let result = run_new_flag_inner(&options);
        assert!(result.is_ok());

        let unified = unified_config::read_unified_config().unwrap();
        let env_rules = unified
            .get("environments")
            .and_then(|e| e.get("nonexistent"))
            .and_then(|e| e.get("rules"))
            .and_then(|r| r.get("test_flag"))
            .and_then(|r| r.as_array());
        assert!(
            env_rules.is_some(),
            "Environment 'nonexistent' should have been created"
        );
    }

    #[test]
    #[serial]
    fn test_new_flag_with_enable_in() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let options = NewFlagOptions {
            name: Some("new_flag".to_string()),
            flag_type: Some("boolean".to_string()),
            default: Some("true".to_string()),
            description: None,
            enable_in: Some("production".to_string()),
            skip_sdk: true,
            best_effort: false,
        };

        let result = run_new_flag_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_enable_flag_with_all_flag() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let options = EnableOptions {
            name: "existing_flag".to_string(),
            env: None,
            rule: None,
            value: None,
            all: true,
            interactive: false,
            no_compile: false,
            best_effort: false,
            force: false,
        };

        let result = run_enable_inner(&options);
        assert!(result.is_ok());
        let updated_envs = result.unwrap();
        assert!(!updated_envs.is_empty());
    }

    #[test]
    #[serial]
    fn test_enable_flag_with_value() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let options = EnableOptions {
            name: "existing_flag".to_string(),
            env: Some("production".to_string()),
            rule: None,
            value: Some("true".to_string()),
            all: false,
            interactive: false,
            no_compile: false,
            best_effort: false,
            force: false,
        };

        let result = run_enable_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_deploy_dry_run() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let options = DeployOptions {
            env: None,
            dry_run: true,
        };

        let result = run_deploy_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_deploy_specific_env() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let options = DeployOptions {
            env: Some("production".to_string()),
            dry_run: false,
        };

        let result = run_deploy_inner(&options);
        assert!(result.is_ok());
        let envs = result.unwrap();
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0], "production");
    }

    #[test]
    #[serial]
    fn test_deploy_no_environments() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  test_flag:
    default: false
    kind: release
",
        )
        .unwrap();

        let options = DeployOptions {
            env: None,
            dry_run: false,
        };

        let result = run_deploy_inner(&options);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No environments found") || err.contains("control-path.yaml"));
    }

    #[test]
    #[serial]
    fn test_new_flag_multivariate() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let options = NewFlagOptions {
            name: Some("multivar_flag".to_string()),
            flag_type: Some("multivariate".to_string()),
            default: Some("variant_a".to_string()),
            description: None,
            enable_in: None,
            skip_sdk: true,
            best_effort: false,
        };

        let result = run_new_flag_inner(&options);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("v2 catalogs support boolean flags only"));
    }

    #[test]
    #[serial]
    fn test_enable_flag_multiple_envs() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Add another environment to the v2 catalog
        let config = r#"catalog:
  id: test-service
mode: local
flags:
  existing_flag:
    default: false
    kind: release
    description: An existing flag
environments:
  production:
    rules:
      existing_flag:
        - serve: false
  staging:
    rules:
      existing_flag:
        - serve: false
"#;
        fs::write("control-path.yaml", config).unwrap();

        let options = EnableOptions {
            name: "existing_flag".to_string(),
            env: Some("production,staging".to_string()),
            rule: Some("user.role == 'admin'".to_string()),
            value: None,
            all: false,
            interactive: false,
            no_compile: false,
            best_effort: false,
            force: false,
        };

        let result = run_enable_inner(&options);
        assert!(result.is_ok());
        let updated_envs = result.unwrap();
        assert_eq!(updated_envs.len(), 2);
    }
}
