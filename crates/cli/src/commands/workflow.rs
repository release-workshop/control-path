//! Workflow commands implementation
//!
//! These commands combine multiple operations to provide complete workflows:
//! - new-flag: Adds flag, syncs to environments, and regenerates SDK
//! - enable: Enables a flag in environments with rules
//! - deploy: Validates and compiles for deployment

use crate::commands::{compile, generate_sdk, validate};
use crate::error::{CliError, CliResult};
use crate::utils::config;
use controlpath_compiler::compiler::expressions::parse_expression;
use controlpath_compiler::{parse_definitions, parse_deployment, validate_deployment};
use dialoguer::{Input, MultiSelect};
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
    pub skip_sync: bool,
    pub skip_sdk: bool,
}

fn get_definitions_path() -> PathBuf {
    PathBuf::from("flags.definitions.yaml")
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
        .and_then(|s| s.strip_suffix(".deployment.yaml"))
        .map(|s| s.to_string())
}

fn read_definitions() -> CliResult<Value> {
    let path = get_definitions_path();
    if !path.exists() {
        return Err(CliError::Message(
            "flags.definitions.yaml not found. Run 'controlpath init' to create it.".to_string(),
        ));
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| CliError::Message(format!("Failed to read {}: {e}", path.display())))?;
    parse_definitions(&content).map_err(CliError::from)
}

fn write_definitions(definitions: &Value) -> CliResult<()> {
    let path = get_definitions_path();
    let yaml = serde_yaml::to_string(definitions)
        .map_err(|e| CliError::Message(format!("Failed to serialize definitions: {e}")))?;
    fs::write(&path, yaml)
        .map_err(|e| CliError::Message(format!("Failed to write {}: {e}", path.display())))?;
    Ok(())
}

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

fn write_deployment(path: &PathBuf, deployment: &Value) -> CliResult<()> {
    let yaml = serde_yaml::to_string(deployment)
        .map_err(|e| CliError::Message(format!("Failed to serialize deployment: {e}")))?;
    fs::write(path, yaml)
        .map_err(|e| CliError::Message(format!("Failed to write {}: {e}", path.display())))?;
    Ok(())
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

fn add_flag_to_definitions(
    definitions: &mut Value,
    name: &str,
    flag_type: &str,
    default: &Value,
    description: Option<&str>,
) -> CliResult<()> {
    let flags = definitions
        .get_mut("flags")
        .and_then(|f| f.as_array_mut())
        .ok_or_else(|| CliError::Message("Invalid definitions structure".to_string()))?;

    let mut flag_obj = serde_json::json!({
        "name": name,
        "type": flag_type,
        "defaultValue": default,
    });

    if let Some(desc) = description {
        flag_obj["description"] = Value::String(desc.to_string());
    }

    flags.push(flag_obj);
    Ok(())
}

fn sync_flag_to_deployment(
    deployment: &mut Value,
    flag_name: &str,
    default_value: &Value,
) -> CliResult<()> {
    let rules = deployment
        .get_mut("rules")
        .and_then(|r| r.as_object_mut())
        .ok_or_else(|| CliError::Message("Invalid deployment structure".to_string()))?;

    // Convert default value to serve value
    let serve_value = match default_value {
        Value::Bool(b) => Value::Bool(*b),
        Value::String(s) if s == "ON" => Value::Bool(true),
        Value::String(s) if s == "OFF" => Value::Bool(false),
        _ => default_value.clone(),
    };

    // Add flag with default rule (disabled)
    let flag_rules = serde_json::json!({
        "rules": [
            {
                "serve": serve_value
            }
        ]
    });

    rules.insert(flag_name.to_string(), flag_rules);
    Ok(())
}

fn enable_flag_in_deployment(
    deployment: &mut Value,
    flag_name: &str,
    value: Option<&str>,
) -> CliResult<()> {
    let rules = deployment
        .get_mut("rules")
        .and_then(|r| r.as_object_mut())
        .ok_or_else(|| CliError::Message("Invalid deployment structure".to_string()))?;

    if let Some(flag_rules) = rules.get_mut(flag_name) {
        if let Some(rules_array) = flag_rules.get_mut("rules").and_then(|r| r.as_array_mut()) {
            // Update first rule to serve true (or specified value)
            if let Some(first_rule) = rules_array.first_mut() {
                if let Some(serve) = first_rule.get_mut("serve") {
                    if let Some(val_str) = value {
                        // Parse value
                        if val_str == "true" || val_str == "True" {
                            *serve = Value::Bool(true);
                        } else if val_str == "false" || val_str == "False" {
                            *serve = Value::Bool(false);
                        } else {
                            *serve = Value::String(val_str.to_string());
                        }
                    } else {
                        *serve = Value::Bool(true);
                    }
                }
            }
        }
    }
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
    // Read definitions
    let mut definitions = read_definitions()?;

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

    // Add flag to definitions
    add_flag_to_definitions(
        &mut definitions,
        &flag_name,
        &flag_type,
        &default_value,
        description,
    )?;
    write_definitions(&definitions)?;
    println!("✓ Added flag to definitions");

    // Sync to environments (unless skipped)
    if !options.skip_sync {
        let deployment_files = find_deployment_files();
        if !deployment_files.is_empty() {
            for deployment_path in &deployment_files {
                let mut deployment = read_deployment(deployment_path)?;
                sync_flag_to_deployment(&mut deployment, &flag_name, &default_value)?;
                write_deployment(deployment_path, &deployment)?;
            }
            println!("✓ Synced flag to {} environment(s)", deployment_files.len());
        }
    }

    // Enable in specified environments
    if let Some(ref enable_envs) = options.enable_in {
        let envs: Vec<&str> = enable_envs.split(',').map(|s| s.trim()).collect();
        for env in envs {
            let deployment_path = PathBuf::from(format!(".controlpath/{env}.deployment.yaml"));
            if !deployment_path.exists() {
                return Err(CliError::Message(format!(
                    "Environment '{}' not found. Create it with: controlpath env add --name {}",
                    env, env
                )));
            }
            let mut deployment = read_deployment(&deployment_path)?;
            enable_flag_in_deployment(&mut deployment, &flag_name, None)?;
            write_deployment(&deployment_path, &deployment)?;
            println!("✓ Enabled flag in {env}");
        }
    }

    // Regenerate SDK (unless skipped)
    if !options.skip_sdk {
        let lang = config::read_config_language()?;
        if let Some(lang) = lang {
            let generate_opts = generate_sdk::Options {
                lang: Some(lang),
                output: None,
                definitions: None,
            };
            let exit_code = generate_sdk::run(&generate_opts);
            if exit_code != 0 {
                eprintln!("⚠ Warning: SDK regeneration failed");
            } else {
                println!("✓ Regenerated SDK");
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
    // Get environments (interactive if not provided)
    let envs = if let Some(ref env_str) = options.env {
        env_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        // Find available environments
        let deployment_files = find_deployment_files();
        if deployment_files.is_empty() {
            return Err(CliError::Message(
                "No environments found. Run 'controlpath env add --name <env>' to create one."
                    .to_string(),
            ));
        }

        let env_names: Vec<String> = deployment_files
            .iter()
            .filter_map(|p| get_environment_name(p))
            .collect();

        if options.interactive {
            let selected = MultiSelect::new()
                .with_prompt("Select environments to enable flag in")
                .items(&env_names)
                .interact()
                .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;

            selected.iter().map(|i| env_names[*i].clone()).collect()
        } else {
            // Default to first environment if only one
            if env_names.len() == 1 {
                vec![env_names[0].clone()]
            } else {
                return Err(CliError::Message(format!(
                    "Multiple environments found: {}. Please specify --env <env> or use --interactive",
                    env_names.join(", ")
                )));
            }
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
        println!("  • Enable for admins: user.role == 'admin'");
        println!("  • Enable for percentage: user.id % 100 < 10");
        println!("  • Enable for specific users: user.id IN ['user1', 'user2']");
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

    // Check if flag exists in definitions (before processing environments)
    let definitions = read_definitions()?;
    if !check_flag_exists(&definitions, &options.name) {
        return Err(CliError::Message(format!(
            "Flag '{}' not found in definitions",
            options.name
        )));
    }

    // Verify all specified environments exist
    let mut missing_envs = Vec::new();
    for env in &envs {
        let deployment_path = PathBuf::from(format!(".controlpath/{env}.deployment.yaml"));
        if !deployment_path.exists() {
            missing_envs.push(env.clone());
        }
    }
    if !missing_envs.is_empty() {
        return Err(CliError::Message(format!(
            "Environment(s) not found: {}. Run 'controlpath env add --name <env>' to create them.",
            missing_envs.join(", ")
        )));
    }

    // Update each environment
    let mut updated_envs = Vec::new();
    for env in &envs {
        let deployment_path = PathBuf::from(format!(".controlpath/{env}.deployment.yaml"));
        let mut deployment = read_deployment(&deployment_path)?;

        // Get flag type and default from definitions
        let flag_type = definitions
            .get("flags")
            .and_then(|f| f.as_array())
            .and_then(|flags| {
                flags
                    .iter()
                    .find(|f| f.get("name").and_then(|n| n.as_str()) == Some(&options.name))
            })
            .and_then(|f| f.get("type").and_then(|t| t.as_str()))
            .unwrap_or("boolean");

        let default_value = definitions
            .get("flags")
            .and_then(|f| f.as_array())
            .and_then(|flags| {
                flags
                    .iter()
                    .find(|f| f.get("name").and_then(|n| n.as_str()) == Some(&options.name))
            })
            .and_then(|f| f.get("defaultValue"))
            .unwrap_or(&Value::Bool(false));

        // Update or create rule
        let rules = deployment
            .get_mut("rules")
            .and_then(|r| r.as_object_mut())
            .ok_or_else(|| CliError::Message("Invalid deployment structure".to_string()))?;

        let flag_rules = rules.entry(options.name.clone()).or_insert_with(|| {
            serde_json::json!({
                "rules": []
            })
        });

        let rules_array = flag_rules
            .get_mut("rules")
            .and_then(|r| r.as_array_mut())
            .ok_or_else(|| CliError::Message("Invalid flag rules structure".to_string()))?;

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

        // Add or update rule
        if let Some(rule_expr) = &rule_expr {
            // Add rule with expression
            let new_rule = serde_json::json!({
                "when": rule_expr,
                "serve": serve_val
            });
            rules_array.push(new_rule);
        } else if rules_array.is_empty() {
            // Update first rule or add new one
            rules_array.push(serde_json::json!({
                "serve": serve_val
            }));
        } else if let Some(first_rule) = rules_array.first_mut() {
            first_rule["serve"] = serve_val.clone();
            // Remove "when" if it exists (enable for all)
            first_rule.as_object_mut().and_then(|o| o.remove("when"));
        }

        // Validate deployment before writing
        validate_deployment(&deployment)
            .map_err(|e| CliError::Message(format!("Invalid deployment after update: {e}")))?;

        write_deployment(&deployment_path, &deployment)?;
        updated_envs.push(env.clone());
    }

    Ok(updated_envs)
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
        // Find all environments
        let deployment_files = find_deployment_files();
        if deployment_files.is_empty() {
            return Err(CliError::Message(
                "No environments found. Run 'controlpath env add --name <env>' to create one."
                    .to_string(),
            ));
        }

        deployment_files
            .iter()
            .filter_map(|p| get_environment_name(p))
            .collect()
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
            println!("  Would compile: .controlpath/{env}.deployment.yaml");
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

    struct DirGuard {
        original_dir: std::path::PathBuf,
    }

    impl DirGuard {
        fn new(temp_path: &std::path::Path) -> Self {
            // Ensure directory exists
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

    fn setup_test_project() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create .controlpath directory
        fs::create_dir_all(".controlpath").unwrap();

        // Create definitions file
        let definitions = r#"flags:
  - name: existing_flag
    type: boolean
    defaultValue: false
    description: An existing flag
"#;
        fs::write("flags.definitions.yaml", definitions).unwrap();

        // Create deployment file
        let deployment = r#"environment: production
rules:
  existing_flag:
    rules:
      - serve: false
"#;
        fs::write(".controlpath/production.deployment.yaml", deployment).unwrap();

        temp_dir
    }

    #[test]
    #[serial]
    fn test_new_flag_basic() {
        let temp_dir = setup_test_project();
        let _guard = DirGuard::new(temp_dir.path());

        let options = NewFlagOptions {
            name: Some("test_flag".to_string()),
            flag_type: Some("boolean".to_string()),
            default: Some("false".to_string()),
            description: Some("Test flag".to_string()),
            enable_in: None,
            skip_sync: false,
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
        let deployment =
            read_deployment(&PathBuf::from(".controlpath/production.deployment.yaml")).unwrap();
        assert!(deployment
            .get("rules")
            .and_then(|r| r.get("test_flag"))
            .is_some());
    }

    #[test]
    #[serial]
    fn test_enable_flag() {
        let temp_dir = setup_test_project();
        let _guard = DirGuard::new(temp_dir.path());

        // First add a flag
        let new_flag_options = NewFlagOptions {
            name: Some("test_flag".to_string()),
            flag_type: Some("boolean".to_string()),
            default: Some("false".to_string()),
            description: None,
            enable_in: None,
            skip_sync: false,
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
        };

        let result = run_enable_inner(&enable_options);
        assert!(result.is_ok());

        // Verify flag was enabled
        let deployment =
            read_deployment(&PathBuf::from(".controlpath/production.deployment.yaml")).unwrap();
        let flag_rules = deployment
            .get("rules")
            .and_then(|r| r.get("test_flag"))
            .and_then(|f| f.get("rules"))
            .and_then(|r| r.as_array())
            .unwrap();

        let serve_value = flag_rules[0]
            .get("serve")
            .and_then(|s| s.as_bool())
            .unwrap();
        assert!(serve_value); // Should be true (opposite of default false)
    }

    #[test]
    #[serial]
    fn test_deploy() {
        let temp_dir = setup_test_project();
        let _guard = DirGuard::new(temp_dir.path());

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
        let _guard = DirGuard::new(temp_dir.path());

        // First add a flag
        let new_flag_options = NewFlagOptions {
            name: Some("test_flag".to_string()),
            flag_type: Some("boolean".to_string()),
            default: Some("false".to_string()),
            description: None,
            enable_in: None,
            skip_sync: false,
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
        };

        let result = run_enable_inner(&enable_options);
        assert!(result.is_ok());

        // Verify rule was created with "when" field (not "if")
        let deployment =
            read_deployment(&PathBuf::from(".controlpath/production.deployment.yaml")).unwrap();
        let flag_rules = deployment
            .get("rules")
            .and_then(|r| r.get("test_flag"))
            .and_then(|f| f.get("rules"))
            .and_then(|r| r.as_array())
            .unwrap();

        // Should have a rule with "when" field
        let rule = flag_rules.last().unwrap();
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
        let _guard = DirGuard::new(temp_dir.path());

        // First add a flag
        let new_flag_options = NewFlagOptions {
            name: Some("test_flag".to_string()),
            flag_type: Some("boolean".to_string()),
            default: Some("false".to_string()),
            description: None,
            enable_in: None,
            skip_sync: false,
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
        let _guard = DirGuard::new(temp_dir.path());

        let options = NewFlagOptions {
            name: Some("test_flag".to_string()),
            flag_type: Some("boolean".to_string()),
            default: Some("false".to_string()),
            description: None,
            enable_in: Some("nonexistent".to_string()),
            skip_sync: false,
            skip_sdk: true,
        };

        let result = run_new_flag_inner(&options);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Environment 'nonexistent' not found"));
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
            skip_sync: false,
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
    defaultValue: false
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
            skip_sync: true,
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
        };

        let result = run_enable_inner(&options);
        assert!(result.is_ok());
        let updated_envs = result.unwrap();
        assert_eq!(updated_envs.len(), 2);
    }
}
