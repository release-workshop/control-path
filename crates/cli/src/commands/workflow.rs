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
use crate::utils::unified_config;
use controlpath_compiler::compiler::expressions::parse_expression;
#[cfg(test)]
use controlpath_compiler::parse_deployment;
use dialoguer::Input;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

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
}

// Read config and extract definitions (for compiler compatibility)
#[cfg(test)]
fn read_definitions() -> CliResult<Value> {
    let unified = unified_config::read_unified_config()?;
    unified_config::extract_definitions(&unified)
}

// Read config and extract deployment for a specific environment
#[cfg(test)]
#[allow(dead_code)] // May be used in future tests
fn read_deployment(path: &PathBuf) -> CliResult<Value> {
    if !path.exists() {
        return Err(CliError::Message(format!(
            "Deployment file not found: {}",
            path.display()
        )));
    }
    let content = fs::read_to_string(path)
        .map_err(|e| CliError::Message(format!("Failed to read {}: {e}", path.display())))?;
    parse_deployment(&content).map_err(CliError::from)
}

// Write definitions file (for tests using legacy format)
#[cfg(test)]
fn write_definitions(definitions: &Value) -> CliResult<()> {
    let path = PathBuf::from("flags.definitions.yaml");
    let yaml = serde_yaml::to_string(definitions)
        .map_err(|e| CliError::Message(format!("Failed to serialize definitions: {e}")))?;
    fs::write(&path, yaml)
        .map_err(|e| CliError::Message(format!("Failed to write {}: {e}", path.display())))?;
    Ok(())
}

fn find_deployment_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    let controlpath_dir = PathBuf::from(".controlpath");
    if let Ok(entries) = fs::read_dir(&controlpath_dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".deployment.yaml") {
                        files.push(entry.path());
                    }
                }
            }
        }
    }
    files
}

fn get_environment_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|name| name.strip_suffix(".deployment.yaml"))
        .map(|s| s.to_string())
}

#[allow(dead_code)] // Reserved for future use
                    // Read config
fn read_unified() -> CliResult<Value> {
    unified_config::read_unified_config()
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

fn check_flag_exists(definitions: &Value, name: &str) -> bool {
    if let Some(flags) = definitions.get("flags").and_then(|f| f.as_array()) {
        flags
            .iter()
            .any(|f| f.get("name").and_then(|n| n.as_str()) == Some(name))
    } else {
        false
    }
}

// ============================================================================
// Config helper functions
// ============================================================================

/// Add a flag to the config
fn add_flag_to_unified(
    unified: &mut Value,
    flag_name: &str,
    flag_type: &str,
    default_value: &Value,
    description: Option<&str>,
) -> CliResult<()> {
    let flags = unified
        .get_mut("flags")
        .and_then(|f| f.as_array_mut())
        .ok_or_else(|| CliError::Message("Invalid config: missing flags array".to_string()))?;

    // Check if flag already exists
    if flags
        .iter()
        .any(|f| f.get("name").and_then(|n| n.as_str()) == Some(flag_name))
    {
        return Err(CliError::Message(format!(
            "Flag '{}' already exists",
            flag_name
        )));
    }

    // Create new flag object
    let mut new_flag = serde_json::json!({
        "name": flag_name,
        "type": flag_type,
        "default": default_value,
        "environments": {}
    });

    if let Some(desc) = description {
        new_flag["description"] = serde_json::json!(desc);
    }

    flags.push(new_flag);
    Ok(())
}

/// Enable a flag in a specific environment in config
fn enable_flag_in_unified_env(
    unified: &mut Value,
    flag_name: &str,
    environment: &str,
    rule_expr: Option<&str>,
    serve_value: &Value,
) -> CliResult<()> {
    let flags = unified
        .get_mut("flags")
        .and_then(|f| f.as_array_mut())
        .ok_or_else(|| CliError::Message("Invalid config: missing flags array".to_string()))?;

    // Find the flag
    let flag = flags
        .iter_mut()
        .find(|f| f.get("name").and_then(|n| n.as_str()) == Some(flag_name))
        .ok_or_else(|| CliError::Message(format!("Flag '{}' not found", flag_name)))?;

    // Get or create environments object
    let environments = flag
        .get_mut("environments")
        .and_then(|e| e.as_object_mut())
        .ok_or_else(|| {
            CliError::Message("Invalid flag structure: missing environments".to_string())
        })?;

    // Get or create rules array for this environment
    let env_rules = environments
        .entry(environment.to_string())
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .ok_or_else(|| CliError::Message("Invalid environment rules structure".to_string()))?;

    // Create new rule
    let mut new_rule = serde_json::json!({
        "serve": serve_value
    });

    if let Some(expr) = rule_expr {
        new_rule["when"] = serde_json::json!(expr);
    }

    env_rules.push(new_rule);
    Ok(())
}

pub fn run_new_flag(options: &NewFlagOptions) -> i32 {
    match run_new_flag_inner(options) {
        Ok(flag_name) => {
            println!("✓ Flag '{flag_name}' added successfully");
            println!();
            println!("Next steps:");
            println!("  • Enable in staging:    controlpath enable {flag_name} --env staging");
            println!("  • Explain flag:        controlpath explain --flag {flag_name}");
            println!("  • View flag details:    controlpath flag show {flag_name}");
            0
        }
        Err(e) => {
            eprintln!("✗ Failed to add flag");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_new_flag_inner(options: &NewFlagOptions) -> CliResult<String> {
    // Read config
    let mut unified = read_unified()?;
    let definitions = unified_config::extract_definitions(&unified)?;

    // Get flag name (interactive if not provided)
    let flag_name = if let Some(ref name) = options.name {
        validate_flag_name(name)?;
        if check_flag_exists(&definitions, name) {
            return Err(CliError::Message(format!("Flag '{}' already exists", name)));
        }
        name.clone()
    } else {
        // Interactive prompt
        let name: String = Input::new()
            .with_prompt("Flag name (snake_case)")
            .validate_with(|input: &String| -> Result<(), String> {
                validate_flag_name(input).map_err(|e| e.to_string())?;
                if check_flag_exists(&definitions, input) {
                    Err(format!("Flag '{}' already exists", input))
                } else {
                    Ok(())
                }
            })
            .interact()
            .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;
        name
    };

    // Get flag type
    let flag_type = options
        .flag_type
        .as_deref()
        .unwrap_or("boolean")
        .to_string();

    // Get default value
    let default_value = if let Some(ref default_str) = options.default {
        match flag_type.as_str() {
            "boolean" => {
                if default_str == "true" || default_str == "True" {
                    Value::Bool(true)
                } else {
                    Value::Bool(false)
                }
            }
            _ => Value::String(default_str.clone()),
        }
    } else {
        Value::Bool(false)
    };

    // Get description
    let description = options.description.as_deref();

    // Add flag to config
    add_flag_to_unified(
        &mut unified,
        &flag_name,
        &flag_type,
        &default_value,
        description,
    )?;
    write_unified(&unified)?;
    println!("✓ Added flag to configuration");

    // Enable in specified environments
    let mut enabled_envs = Vec::new();
    if let Some(ref enable_envs) = options.enable_in {
        let envs: Vec<&str> = enable_envs.split(',').map(|s| s.trim()).collect();

        // Re-read config to get latest state
        let mut unified = read_unified()?;

        for env in envs {
            // Determine serve value (opposite of default for boolean, or default for others)
            let serve_value = match &default_value {
                Value::Bool(b) => Value::Bool(!b),
                _ => default_value.clone(),
            };

            // Enable flag in this environment
            enable_flag_in_unified_env(&mut unified, &flag_name, env, None, &serve_value)?;
            println!("✓ Enabled flag in {env}");
            enabled_envs.push(env.to_string());
        }

        // Write config back
        write_unified(&unified)?;

        // Auto-compile ASTs for enabled environments
        if !enabled_envs.is_empty() {
            println!("Compiling ASTs for enabled environments...");
            let compile_opts = ops_compile::CompileOptions {
                envs: Some(enabled_envs.clone()),
                skip_validation: false,
            };
            match ops_compile::compile_envs(&compile_opts) {
                Ok(compiled) => {
                    for env in &compiled {
                        println!("✓ Compiled AST for {env}");
                    }
                }
                Err(e) => {
                    eprintln!("⚠ Warning: Failed to compile ASTs: {e}");
                    eprintln!(
                        "  You can compile manually with: controlpath compile --env {}",
                        enabled_envs.join(",")
                    );
                }
            }
        }
    }

    // Regenerate SDK (unless skipped)
    if !options.skip_sdk {
        println!("Regenerating SDK...");
        let generate_opts = ops_generate_sdk::GenerateOptions {
            lang: None, // Auto-detect from config or project files
            output: None,
            skip_validation: false,
        };
        match ops_generate_sdk::generate_sdk_helper(&generate_opts) {
            Ok(()) => {
                println!("✓ Regenerated SDK");
            }
            Err(e) => {
                eprintln!("⚠ Warning: SDK regeneration failed: {e}");
                eprintln!("  You can regenerate manually with: controlpath generate-sdk");
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
}

pub fn run_enable(options: &EnableOptions) -> i32 {
    match run_enable_inner(options) {
        Ok(envs) => {
            if envs.is_empty() {
                eprintln!("⚠ No environments were updated");
                return 1;
            }
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
                    "  • Enable in production: controlpath enable {} --env production",
                    options.name
                );
            }
            0
        }
        Err(e) => {
            eprintln!("✗ Failed to enable flag");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_enable_inner(options: &EnableOptions) -> CliResult<Vec<String>> {
    // Read config
    let mut unified = read_unified()?;
    let definitions = unified_config::extract_definitions(&unified)?;

    // Check if flag exists
    if !check_flag_exists(&definitions, &options.name) {
        return Err(CliError::Message(format!(
            "Flag '{}' not found in configuration",
            options.name
        )));
    }

    // Get flag type and default from config
    let flag_type = unified
        .get("flags")
        .and_then(|f| f.as_array())
        .and_then(|flags| {
            flags
                .iter()
                .find(|f| f.get("name").and_then(|n| n.as_str()) == Some(&options.name))
        })
        .and_then(|f| f.get("type").and_then(|t| t.as_str()))
        .unwrap_or("boolean");

    let default_value = unified
        .get("flags")
        .and_then(|f| f.as_array())
        .and_then(|flags| {
            flags
                .iter()
                .find(|f| f.get("name").and_then(|n| n.as_str()) == Some(&options.name))
        })
        .and_then(|f| f.get("default"))
        .unwrap_or(&Value::Bool(false));

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
        println!("Examples:");
        println!("  • Enable for admins: role == 'admin'");
        println!("  • Enable for percentage: id % 100 < 10");
        println!("  • Enable for specific users: id IN ['user1', 'user2']");
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
        match flag_type {
            "boolean" => {
                if val_str == "true" || val_str == "True" {
                    Value::Bool(true)
                } else {
                    Value::Bool(false)
                }
            }
            _ => Value::String(val_str.to_string()),
        }
    } else {
        // Use opposite of default for boolean, or default for others
        match default_value {
            Value::Bool(b) => Value::Bool(!b),
            _ => default_value.clone(),
        }
    };

    // Update each environment in config
    let mut updated_envs = Vec::new();
    for env in &envs {
        enable_flag_in_unified_env(
            &mut unified,
            &options.name,
            env,
            rule_expr.as_deref(),
            &serve_val,
        )?;
        updated_envs.push(env.clone());
    }

    // Write config back
    write_unified(&unified)?;

    // Auto-compile ASTs for updated environments (unless --no-compile)
    if !options.no_compile && !updated_envs.is_empty() {
        println!("Compiling ASTs for updated environments...");
        let compile_opts = ops_compile::CompileOptions {
            envs: Some(updated_envs.clone()),
            skip_validation: false,
        };
        match ops_compile::compile_envs(&compile_opts) {
            Ok(compiled) => {
                for env in &compiled {
                    println!("✓ Compiled AST for {env}");
                }
            }
            Err(e) => {
                eprintln!("⚠ Warning: Failed to compile ASTs: {e}");
                eprintln!(
                    "  You can compile manually with: controlpath compile --env {}",
                    updated_envs.join(",")
                );
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
    pub skip_validation: bool,
}

pub fn run_deploy(options: &DeployOptions) -> i32 {
    match run_deploy_inner(options) {
        Ok(envs) => {
            if options.dry_run {
                println!("✓ Dry run completed successfully");
                println!("  Would deploy to: {}", envs.join(", "));
            } else {
                println!("✓ Deployment ready");
                println!();
                println!("AST artifacts compiled:");
                for env in &envs {
                    println!("  • .controlpath/{env}.ast");
                }
                println!();
                println!("Next steps:");
                println!("  • Copy AST files to your deployment location");
                println!("  • Restart your application to load new flags");
            }
            0
        }
        Err(e) => {
            eprintln!("✗ Deployment failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_deploy_inner(options: &DeployOptions) -> CliResult<Vec<String>> {
    // Get environments
    let envs = if let Some(ref env_str) = options.env {
        env_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        // Try smart defaults: git branch mapping or defaultEnv
        if let Ok(Some(default_env)) = environment::determine_environment() {
            // Check if config exists
            if unified_config::unified_config_exists() {
                let unified = unified_config::read_unified_config()?;
                let all_envs = unified_config::get_environments(&unified);
                if all_envs.contains(&default_env) {
                    vec![default_env]
                } else {
                    // Default env doesn't exist, use all environments
                    if all_envs.is_empty() {
                        return Err(CliError::Message(
                            "No environments found in control-path.yaml. Add flags with environment rules first."
                                .to_string(),
                        ));
                    }
                    all_envs
                }
            } else {
                // Legacy: verify the default environment exists
                let deployment_path =
                    PathBuf::from(format!(".controlpath/{default_env}.deployment.yaml"));
                if deployment_path.exists() {
                    vec![default_env]
                } else {
                    // Default env doesn't exist, fall back to all environments
                    let deployment_files = find_deployment_files();
                    if deployment_files.is_empty() {
                        return Err(CliError::Message(
                            "No environments found. Run 'controlpath setup' to initialize the project."
                                .to_string(),
                        ));
                    }

                    deployment_files
                        .iter()
                        .filter_map(|p| get_environment_name(p))
                        .collect()
                }
            }
        } else {
            // No smart default found, use all environments
            if unified_config::unified_config_exists() {
                let unified = unified_config::read_unified_config()?;
                let all_envs = unified_config::get_environments(&unified);
                if all_envs.is_empty() {
                    return Err(CliError::Message(
                        "No environments found in control-path.yaml. Add flags with environment rules first."
                            .to_string(),
                    ));
                }
                all_envs
            } else {
                // Legacy: use all deployment files
                let deployment_files = find_deployment_files();
                if deployment_files.is_empty() {
                    return Err(CliError::Message(
                        "No environments found. Run 'controlpath setup' to initialize the project."
                            .to_string(),
                    ));
                }

                deployment_files
                    .iter()
                    .filter_map(|p| get_environment_name(p))
                    .collect()
            }
        }
    };

    // Validate (unless skipped)
    if !options.skip_validation {
        println!("Validating definitions and deployments...");
        let validate_opts = validate::Options {
            definitions: None,
            deployment: None,
            env: None,
            all: true,
        };
        let exit_code = validate::run(&validate_opts);
        if exit_code != 0 {
            return Err(CliError::Message("Validation failed".to_string()));
        }
        println!("✓ Validation passed");
    }

    // Compile each environment
    for env in &envs {
        if options.dry_run {
            println!("  Would compile: .controlpath/{env}.ast");
        } else {
            let compile_opts = compile::Options {
                deployment: None,
                env: Some(String::from(env)),
                output: None,
                definitions: None,
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
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create .controlpath directory
        fs::create_dir_all(".controlpath").unwrap();

        // Create config file
        let config = r#"mode: local
flags:
  - name: existing_flag
    type: boolean
    default: false
    description: An existing flag
    environments:
      production:
        - serve: false
"#;
        fs::write("control-path.yaml", config).unwrap();

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
        };

        let result = run_new_flag_inner(&options);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_flag");

        // Verify flag was added to definitions
        let definitions = read_definitions().unwrap();
        let flags = definitions.get("flags").and_then(|f| f.as_array()).unwrap();
        assert!(flags
            .iter()
            .any(|f| f.get("name").and_then(|n| n.as_str()) == Some("test_flag")));

        // Verify flag was synced to deployment
        // Verify flag was added to config
        let unified = unified_config::read_unified_config().unwrap();
        let flag = unified
            .get("flags")
            .and_then(|f| f.as_array())
            .and_then(|flags| {
                flags
                    .iter()
                    .find(|f| f.get("name").and_then(|n| n.as_str()) == Some("test_flag"))
            })
            .unwrap();
        assert!(flag.get("name").and_then(|n| n.as_str()) == Some("test_flag"));
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
        };

        let result = run_enable_inner(&enable_options);
        assert!(result.is_ok());

        // Verify flag was enabled in config
        let unified = unified_config::read_unified_config().unwrap();
        let flag = unified
            .get("flags")
            .and_then(|f| f.as_array())
            .and_then(|flags| {
                flags
                    .iter()
                    .find(|f| f.get("name").and_then(|n| n.as_str()) == Some("test_flag"))
            })
            .unwrap();

        let env_rules = flag
            .get("environments")
            .and_then(|e| e.get("production"))
            .and_then(|r| r.as_array())
            .unwrap();

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
            skip_validation: false,
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
        };

        let result = run_enable_inner(&enable_options);
        assert!(result.is_ok());

        // Verify rule was created with "when" field (not "if")
        // Verify flag was enabled with rule expression in config
        let unified = unified_config::read_unified_config().unwrap();
        let flag = unified
            .get("flags")
            .and_then(|f| f.as_array())
            .and_then(|flags| {
                flags
                    .iter()
                    .find(|f| f.get("name").and_then(|n| n.as_str()) == Some("test_flag"))
            })
            .unwrap();

        let env_rules = flag
            .get("environments")
            .and_then(|e| e.get("production"))
            .and_then(|r| r.as_array())
            .unwrap();

        // Should have a rule with "when" field
        let rule = env_rules.last().unwrap();
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
        };

        // Should succeed - environments are created automatically
        let result = run_new_flag_inner(&options);
        assert!(result.is_ok());

        // Verify flag was added and enabled in the new environment
        let unified = unified_config::read_unified_config().unwrap();
        let flag = unified
            .get("flags")
            .and_then(|f| f.as_array())
            .and_then(|flags| {
                flags
                    .iter()
                    .find(|f| f.get("name").and_then(|n| n.as_str()) == Some("test_flag"))
            })
            .unwrap();

        let env_rules = flag
            .get("environments")
            .and_then(|e| e.get("nonexistent"))
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
        let _guard = DirGuard::new(temp_path);

        let options = NewFlagOptions {
            name: Some("new_flag".to_string()),
            flag_type: Some("boolean".to_string()),
            default: Some("true".to_string()),
            description: None,
            enable_in: Some("production".to_string()),
            skip_sdk: true,
        };

        let result = run_new_flag_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_enable_flag_with_all_flag() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        let options = EnableOptions {
            name: "existing_flag".to_string(),
            env: None,
            rule: None,
            value: None,
            all: true,
            interactive: false,
            no_compile: false,
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
        let _guard = DirGuard::new(temp_path);

        let options = EnableOptions {
            name: "existing_flag".to_string(),
            env: Some("production".to_string()),
            rule: None,
            value: Some("true".to_string()),
            all: false,
            interactive: false,
            no_compile: false,
        };

        let result = run_enable_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_deploy_dry_run() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        let options = DeployOptions {
            env: None,
            dry_run: true,
            skip_validation: false,
        };

        let result = run_deploy_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_deploy_skip_validation() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        let options = DeployOptions {
            env: None,
            dry_run: false,
            skip_validation: true,
        };

        let result = run_deploy_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_deploy_specific_env() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        let options = DeployOptions {
            env: Some("production".to_string()),
            dry_run: false,
            skip_validation: false,
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
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "flags.definitions.yaml",
            r#"flags:
  - name: test_flag
    type: boolean
    default: false
"#,
        )
        .unwrap();

        let options = DeployOptions {
            env: None,
            dry_run: false,
            skip_validation: false,
        };

        let result = run_deploy_inner(&options);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No environments found"));
    }

    #[test]
    #[serial]
    fn test_new_flag_multivariate() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Add multivariate flag definition
        let mut definitions = read_definitions().unwrap();
        if let Some(flags) = definitions.get_mut("flags").and_then(|f| f.as_array_mut()) {
            flags.push(serde_json::json!({
                "name": "multivar_flag",
                "type": "multivariate",
                "defaultValue": "variant_a",
                "variations": [
                    {"name": "VARIANT_A", "value": "variant_a"},
                    {"name": "VARIANT_B", "value": "variant_b"}
                ]
            }));
        }
        write_definitions(&definitions).unwrap();

        let options = NewFlagOptions {
            name: Some("another_multivar".to_string()),
            flag_type: Some("multivariate".to_string()),
            default: Some("variant_a".to_string()),
            description: None,
            enable_in: None,
            skip_sdk: true,
        };

        // This should fail because multivariate flags need variations in definitions
        let _result = run_new_flag_inner(&options);
        // The command may fail validation, which is expected
        // We just want to test the code path
    }

    #[test]
    #[serial]
    fn test_enable_flag_multiple_envs() {
        let temp_dir = setup_test_project();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Add another environment
        fs::write(
            ".controlpath/staging.deployment.yaml",
            r"environment: staging
rules:
  existing_flag:
    rules:
      - serve: false
",
        )
        .unwrap();

        let options = EnableOptions {
            name: "existing_flag".to_string(),
            env: Some("production,staging".to_string()),
            rule: Some("user.role == 'admin'".to_string()),
            value: None,
            all: false,
            interactive: false,
            no_compile: false,
        };

        let result = run_enable_inner(&options);
        assert!(result.is_ok());
        let updated_envs = result.unwrap();
        assert_eq!(updated_envs.len(), 2);
    }
}
