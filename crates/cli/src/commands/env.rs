//! Environment management command implementation

use crate::error::{CliError, CliResult};
use controlpath_compiler::{
    parse_definitions, parse_deployment, validate_definitions, validate_deployment,
};
use dialoguer::{Confirm, Input, Select};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Options {
    pub subcommand: EnvSubcommand,
}

#[derive(Debug, Clone)]
pub enum EnvSubcommand {
    Add {
        name: Option<String>,
        template: Option<String>,
        interactive: bool,
    },
    Sync {
        env: Option<String>,
        dry_run: bool,
    },
    List {
        format: OutputFormat,
    },
    Remove {
        name: String,
        force: bool,
    },
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Result<Self, CliError> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            "yaml" => Ok(OutputFormat::Yaml),
            _ => Err(CliError::Message(format!(
                "Invalid format: {s}. Use table, json, or yaml"
            ))),
        }
    }
}

fn get_definitions_path() -> PathBuf {
    PathBuf::from("flags.definitions.yaml")
}

fn get_deployment_path(env: &str) -> PathBuf {
    PathBuf::from(format!(".controlpath/{env}.deployment.yaml"))
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

fn read_deployment(path: &PathBuf) -> CliResult<Value> {
    if !path.exists() {
        return Err(CliError::Message(format!(
            "Deployment file not found: {}\n  Suggestion: Run 'controlpath env add --name <env>' to create one",
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

/// Validates that an environment name meets the required format.
///
/// Environment names must:
/// - Not be empty
/// - Contain only lowercase letters, digits, underscores, and hyphens
fn validate_environment_name(name: &str) -> CliResult<()> {
    if name.is_empty() {
        return Err(CliError::Message(
            "Environment name cannot be empty".to_string(),
        ));
    }
    // Environment names should be valid identifiers (lowercase letters, digits, underscores, hyphens)
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(CliError::Message(
            "Environment name can only contain lowercase letters, digits, underscores, and hyphens"
                .to_string(),
        ));
    }
    Ok(())
}

/// Converts a flag's default value to a serve value for deployment rules.
///
/// Handles conversion of:
/// - Boolean values (passed through)
/// - String values "ON"/"OFF" (converted to boolean)
/// - Other string values (passed through as-is)
/// - Other types (passed through as-is)
fn convert_default_value_to_serve(default_value: &Value) -> Value {
    match default_value {
        Value::Bool(b) => Value::Bool(*b),
        Value::String(s) => match s.as_str() {
            "ON" => Value::Bool(true),
            "OFF" => Value::Bool(false),
            _ => default_value.clone(),
        },
        _ => default_value.clone(),
    }
}

fn environment_exists(env: &str) -> bool {
    get_deployment_path(env).exists()
}

/// Creates a new deployment file from flag definitions.
///
/// # Arguments
/// * `env_name` - Name of the environment
/// * `definitions` - Parsed flag definitions JSON
/// * `template_deployment` - Optional template deployment to copy rules from
///
/// # Returns
/// A new deployment JSON value ready to be written
///
/// # Behavior
/// - If template is provided, copies all rules from template
/// - Adds any flags from definitions that aren't in template (disabled by default)
/// - If no template, creates deployment with all flags from definitions (disabled by default)
/// - If definitions has no flags, creates deployment with empty rules
fn create_deployment_from_definitions(
    env_name: &str,
    definitions: &Value,
    template_deployment: Option<&Value>,
) -> CliResult<Value> {
    let mut deployment = serde_json::Map::new();
    deployment.insert(
        "environment".to_string(),
        Value::String(env_name.to_string()),
    );

    let mut rules = serde_json::Map::new();

    // If template is provided, copy rules from template
    if let Some(template) = template_deployment {
        if let Some(template_rules) = template.get("rules").and_then(|r| r.as_object()) {
            for (flag_name, flag_rules) in template_rules {
                rules.insert(flag_name.clone(), flag_rules.clone());
            }
        }
    }

    // Get flags from definitions
    let flags_array = definitions.get("flags").and_then(|f| f.as_array());
    let flag_count = flags_array.map(|f| f.len()).unwrap_or(0);

    if flag_count == 0 {
        // Explicitly handle empty definitions
        // This is valid - deployment will have empty rules
    } else if let Some(flags) = flags_array {
        for flag in flags {
            if let Some(flag_name) = flag.get("name").and_then(|n| n.as_str()) {
                // Only add if not already in rules (from template)
                if !rules.contains_key(flag_name) {
                    let default_value = flag.get("defaultValue").unwrap_or(&Value::Bool(false));

                    let default_serve = convert_default_value_to_serve(default_value);

                    let mut rule_obj = serde_json::Map::new();
                    rule_obj.insert("serve".to_string(), default_serve);

                    let mut flag_entry = serde_json::Map::new();
                    flag_entry.insert(
                        "rules".to_string(),
                        Value::Array(vec![Value::Object(rule_obj)]),
                    );
                    rules.insert(flag_name.to_string(), Value::Object(flag_entry));
                }
            }
        }
    }

    deployment.insert("rules".to_string(), Value::Object(rules));
    Ok(Value::Object(deployment))
}

/// Syncs flags from definitions to a deployment file.
///
/// # Arguments
/// * `deployment` - The deployment to sync (modified in place)
/// * `definitions` - The flag definitions to sync from
///
/// # Returns
/// A tuple of (added_count, removed_count, total_flags_count)
///
/// # Behavior
/// - Adds flags from definitions that don't exist in deployment (disabled by default)
/// - Removes flags from deployment that don't exist in definitions
/// - Preserves existing rules for flags that exist in both
fn sync_flags_to_deployment(
    deployment: &mut Value,
    definitions: &Value,
) -> CliResult<(usize, usize, usize)> {
    let deployment_rules = deployment
        .get_mut("rules")
        .and_then(|r| r.as_object_mut())
        .ok_or_else(|| {
            CliError::Message("Invalid deployment: missing 'rules' object".to_string())
        })?;

    let mut added = 0;
    let mut removed = 0;
    let mut existing_flags = std::collections::HashSet::new();

    // Collect flags from definitions
    if let Some(flags) = definitions.get("flags").and_then(|f| f.as_array()) {
        for flag in flags {
            if let Some(flag_name) = flag.get("name").and_then(|n| n.as_str()) {
                existing_flags.insert(flag_name.to_string());

                // Add flag if it doesn't exist in deployment
                if !deployment_rules.contains_key(flag_name) {
                    let default_value = flag.get("defaultValue").unwrap_or(&Value::Bool(false));

                    let default_serve = convert_default_value_to_serve(default_value);

                    let mut rule_obj = serde_json::Map::new();
                    rule_obj.insert("serve".to_string(), default_serve);

                    let mut flag_entry = serde_json::Map::new();
                    flag_entry.insert(
                        "rules".to_string(),
                        Value::Array(vec![Value::Object(rule_obj)]),
                    );
                    deployment_rules.insert(flag_name.to_string(), Value::Object(flag_entry));
                    added += 1;
                }
            }
        }
    }

    // Remove flags that don't exist in definitions
    let flags_to_remove: Vec<String> = deployment_rules
        .keys()
        .filter(|flag_name| !existing_flags.contains(*flag_name))
        .cloned()
        .collect();

    for flag_name in &flags_to_remove {
        deployment_rules.remove(flag_name);
        removed += 1;
    }

    Ok((added, removed, existing_flags.len()))
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("✗ Error: {e}");
            1
        }
    }
}

/// Prompts for environment name in interactive mode.
fn prompt_for_environment_name(existing_envs: &[String]) -> CliResult<String> {
    loop {
        let name: String = Input::new()
            .with_prompt("Environment name")
            .validate_with(|input: &String| -> Result<(), String> {
                validate_environment_name(input).map_err(|e| format!("{}", e))
            })
            .interact()
            .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;

        if existing_envs.contains(&name) {
            eprintln!("✗ Environment '{name}' already exists");
            continue;
        }

        return Ok(name);
    }
}

/// Prompts for template selection in interactive mode.
fn prompt_for_template(existing_envs: &[String]) -> CliResult<Option<String>> {
    if existing_envs.is_empty() {
        return Ok(None);
    }

    let use_template = Confirm::new()
        .with_prompt("Use a template environment?")
        .default(false)
        .interact()
        .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;

    if !use_template {
        return Ok(None);
    }

    let selection = Select::new()
        .with_prompt("Select template environment")
        .items(existing_envs)
        .interact()
        .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;

    Ok(Some(existing_envs[selection].clone()))
}

/// Main command execution logic.
///
/// Handles all four environment management subcommands:
/// - `add`: Creates a new environment deployment file
/// - `sync`: Syncs flags from definitions to deployment files
/// - `list`: Lists all environments with their flag counts
/// - `remove`: Removes an environment deployment file
fn run_inner(options: &Options) -> CliResult<()> {
    match &options.subcommand {
        EnvSubcommand::Add {
            name,
            template,
            interactive,
        } => {
            // Get existing environments for validation
            let existing_envs: Vec<String> = find_deployment_files()
                .iter()
                .filter_map(|p| get_environment_name(p.as_path()))
                .collect();

            // Interactive mode: prompt for missing values
            let (env_name, template_env) = if *interactive && name.is_none() {
                let name = prompt_for_environment_name(&existing_envs)?;
                let template = prompt_for_template(&existing_envs)?;
                (name, template)
            } else {
                let name = name.clone().ok_or_else(|| {
                    CliError::Message(
                        "Environment name is required. Use --name <name> or run in interactive mode"
                            .to_string(),
                    )
                })?;
                (name, template.clone())
            };

            validate_environment_name(&env_name)?;

            if environment_exists(&env_name) {
                return Err(CliError::Message(format!(
                    "Environment '{env_name}' already exists"
                )));
            }

            // Ensure .controlpath directory exists
            fs::create_dir_all(".controlpath").map_err(|e| {
                CliError::Message(format!("Failed to create .controlpath directory: {e}"))
            })?;

            // Read definitions
            let definitions = read_definitions()?;

            // Read template deployment if provided
            let template_deployment = if let Some(ref template_env) = template_env {
                let template_path = get_deployment_path(template_env);
                if !template_path.exists() {
                    return Err(CliError::Message(format!(
                        "Template environment '{template_env}' not found\n  Suggestion: Run 'controlpath env add --name {template_env}' to create it first"
                    )));
                }
                let template = read_deployment(&template_path)?;
                // Validate template deployment
                validate_deployment(&template)
                    .map_err(|e| CliError::Message(format!(
                        "Template environment '{template_env}' is invalid: {e}\n  Suggestion: Fix the deployment file or use a different template"
                    )))?;
                Some(template)
            } else {
                None
            };

            // Create new deployment
            let deployment = create_deployment_from_definitions(
                &env_name,
                &definitions,
                template_deployment.as_ref(),
            )?;

            // Check if definitions is empty and warn
            let flag_count = definitions
                .get("flags")
                .and_then(|f| f.as_array())
                .map(|f| f.len())
                .unwrap_or(0);
            if flag_count == 0 {
                eprintln!("  Warning: No flags found in definitions file");
            }

            // Validate deployment
            validate_deployment(&deployment)
                .map_err(|e| CliError::Message(format!("Created deployment is invalid: {e}")))?;

            // Write deployment file
            let deployment_path = get_deployment_path(&env_name);
            write_deployment(&deployment_path, &deployment)?;

            println!("✓ Created environment '{env_name}'");
            if let Some(ref template) = template_env {
                println!("  Copied from template: {template}");
            } else {
                println!("  Synced flags from definitions (all disabled by default)");
            }

            Ok(())
        }
        EnvSubcommand::Sync { env, dry_run } => {
            let definitions = read_definitions()?;
            validate_definitions(&definitions)?;

            let deployment_files = if let Some(env_name) = env {
                vec![get_deployment_path(env_name)]
            } else {
                find_deployment_files()
            };

            if deployment_files.is_empty() {
                return Err(CliError::Message(
                    "No deployment files found. Run 'controlpath env add --name <env>' to create one."
                        .to_string(),
                ));
            }

            if *dry_run {
                println!("Dry-run mode: Showing what would be synced\n");
            }

            let mut total_added = 0;
            let mut total_removed = 0;
            let mut synced_count = 0;

            for deployment_path in &deployment_files {
                match read_deployment(deployment_path) {
                    Ok(mut deployment) => {
                        let (added, removed, _total_flags) =
                            sync_flags_to_deployment(&mut deployment, &definitions)?;

                        let env_name = get_environment_name(deployment_path)
                            .unwrap_or_else(|| deployment_path.display().to_string());

                        if *dry_run {
                            // In dry-run mode, just show what would happen
                            if added > 0 || removed > 0 {
                                println!("Would sync {}", env_name);
                                if added > 0 {
                                    println!("  Would add {} flag(s)", added);
                                }
                                if removed > 0 {
                                    println!("  Would remove {} flag(s)", removed);
                                }
                            } else {
                                println!("{} is up to date", env_name);
                            }
                        } else {
                            // Validate before writing
                            validate_deployment(&deployment).map_err(|e| {
                                CliError::Message(format!(
                                    "Deployment {} is invalid after sync: {e}",
                                    deployment_path.display()
                                ))
                            })?;

                            write_deployment(deployment_path, &deployment)?;

                            if added > 0 || removed > 0 {
                                println!("✓ Synced {}", env_name);
                                if added > 0 {
                                    println!("  Added {} flag(s)", added);
                                }
                                if removed > 0 {
                                    println!("  Removed {} flag(s)", removed);
                                }
                            } else {
                                println!("✓ {} is up to date", env_name);
                            }
                        }

                        total_added += added;
                        total_removed += removed;
                        synced_count += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "  Warning: Failed to sync {}: {e}",
                            deployment_path.display()
                        );
                    }
                }
            }

            if synced_count > 0 {
                if *dry_run {
                    println!("\nDry-run summary:");
                    println!("  Environments that would be synced: {}", synced_count);
                    if total_added > 0 {
                        println!("  Flags that would be added: {}", total_added);
                    }
                    if total_removed > 0 {
                        println!("  Flags that would be removed: {}", total_removed);
                    }
                    println!("\nRun without --dry-run to apply these changes");
                } else {
                    println!("\nSync complete:");
                    println!("  Environments synced: {}", synced_count);
                    if total_added > 0 {
                        println!("  Flags added: {}", total_added);
                    }
                    if total_removed > 0 {
                        println!("  Flags removed: {}", total_removed);
                    }
                }
            }

            Ok(())
        }
        EnvSubcommand::Remove { name, force } => {
            validate_environment_name(name)?;

            let deployment_path = get_deployment_path(name);
            if !deployment_path.exists() {
                return Err(CliError::Message(format!("Environment '{name}' not found")));
            }

            // Show what will be removed
            if !*force {
                println!("This will remove environment '{name}'");
                println!("  File: {}", deployment_path.display());

                // Try to read deployment to show flag count
                if let Ok(deployment) = read_deployment(&deployment_path) {
                    let flag_count = deployment
                        .get("rules")
                        .and_then(|r| r.as_object())
                        .map(|r| r.len())
                        .unwrap_or(0);
                    println!("  Flags: {}", flag_count);
                }

                let confirmed = Confirm::new()
                    .with_prompt("Are you sure you want to remove this environment?")
                    .default(false)
                    .interact()
                    .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;

                if !confirmed {
                    println!("Cancelled");
                    return Ok(());
                }
            }

            // Remove the deployment file
            fs::remove_file(&deployment_path).map_err(|e| {
                CliError::Message(format!(
                    "Failed to remove {}: {e}",
                    deployment_path.display()
                ))
            })?;

            println!("✓ Removed environment '{name}'");
            println!("  Deleted: {}", deployment_path.display());

            Ok(())
        }
        EnvSubcommand::List { format } => {
            let deployment_files = find_deployment_files();

            if deployment_files.is_empty() {
                println!(
                    "No environments found. Run 'controlpath env add --name <env>' to create one."
                );
                return Ok(());
            }

            let mut environments = Vec::new();
            for path in &deployment_files {
                if let Some(env_name) = get_environment_name(path) {
                    match read_deployment(path) {
                        Ok(deployment) => {
                            let flag_count = deployment
                                .get("rules")
                                .and_then(|r| r.as_object())
                                .map(|r| r.len())
                                .unwrap_or(0);
                            environments.push((env_name, flag_count, path.clone()));
                        }
                        Err(_) => {
                            // Skip invalid deployments
                        }
                    }
                }
            }

            environments.sort_by_key(|(name, _, _)| name.clone());

            match format {
                OutputFormat::Table => {
                    println!("Environments:");
                    println!("  {:<20} {:<10}", "Name", "Flags");
                    println!("  {}", "-".repeat(30));
                    for (name, flag_count, _) in &environments {
                        println!("  {:<20} {:<10}", name, flag_count);
                    }
                }
                OutputFormat::Json => {
                    let json_envs: Vec<serde_json::Value> = environments
                        .iter()
                        .map(|(name, flag_count, _)| {
                            serde_json::json!({
                                "name": name,
                                "flags": flag_count
                            })
                        })
                        .collect();
                    let json = serde_json::to_string_pretty(&json_envs)
                        .map_err(|e| CliError::Message(format!("Failed to serialize: {e}")))?;
                    println!("{json}");
                }
                OutputFormat::Yaml => {
                    let yaml_envs: Vec<serde_json::Value> = environments
                        .iter()
                        .map(|(name, flag_count, _)| {
                            serde_json::json!({
                                "name": name,
                                "flags": flag_count
                            })
                        })
                        .collect();
                    let yaml = serde_yaml::to_string(&yaml_envs)
                        .map_err(|e| CliError::Message(format!("Failed to serialize: {e}")))?;
                    println!("{yaml}");
                }
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    use crate::test_helpers::DirGuard;

    fn setup_test_env() -> (TempDir, DirGuard) {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        // Create .controlpath directory before changing directory
        fs::create_dir_all(temp_path.join(".controlpath")).unwrap();
        let guard = DirGuard::new(temp_path).unwrap();
        (temp_dir, guard)
    }

    #[test]
    #[serial]
    fn test_validate_environment_name() {
        assert!(validate_environment_name("production").is_ok());
        assert!(validate_environment_name("staging").is_ok());
        assert!(validate_environment_name("dev_env").is_ok());
        assert!(validate_environment_name("test-env").is_ok());
        assert!(validate_environment_name("").is_err());
        assert!(validate_environment_name("Production").is_err()); // uppercase
        assert!(validate_environment_name("env name").is_err()); // space
    }

    #[test]
    #[serial]
    fn test_convert_default_value_to_serve() {
        // Boolean values
        assert_eq!(
            convert_default_value_to_serve(&Value::Bool(true)),
            Value::Bool(true)
        );
        assert_eq!(
            convert_default_value_to_serve(&Value::Bool(false)),
            Value::Bool(false)
        );

        // String "ON"/"OFF"
        assert_eq!(
            convert_default_value_to_serve(&Value::String("ON".to_string())),
            Value::Bool(true)
        );
        assert_eq!(
            convert_default_value_to_serve(&Value::String("OFF".to_string())),
            Value::Bool(false)
        );

        // Other strings
        assert_eq!(
            convert_default_value_to_serve(&Value::String("variation1".to_string())),
            Value::String("variation1".to_string())
        );

        // Other types
        assert_eq!(
            convert_default_value_to_serve(&Value::Number(42.into())),
            Value::Number(42.into())
        );
    }

    #[test]
    #[serial]
    fn test_create_deployment_from_definitions() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create definitions file
        let definitions_content = r"flags:
  - name: test_flag
    type: boolean
    default: false
";
        fs::write("flags.definitions.yaml", definitions_content).unwrap();

        let definitions = read_definitions().unwrap();
        let deployment = create_deployment_from_definitions("test", &definitions, None).unwrap();

        assert_eq!(
            deployment.get("environment").and_then(|e| e.as_str()),
            Some("test")
        );
        assert!(deployment.get("rules").is_some());
        assert!(deployment
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("test_flag"))
            .is_some());
    }

    #[test]
    #[serial]
    fn test_create_deployment_from_definitions_empty() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create empty definitions file
        let definitions_content = r"flags: []
";
        fs::write("flags.definitions.yaml", definitions_content).unwrap();

        let definitions = read_definitions().unwrap();
        let deployment = create_deployment_from_definitions("test", &definitions, None).unwrap();

        assert_eq!(
            deployment.get("environment").and_then(|e| e.as_str()),
            Some("test")
        );
        let rules = deployment.get("rules").and_then(|r| r.as_object()).unwrap();
        assert_eq!(rules.len(), 0); // Empty rules
    }

    #[test]
    #[serial]
    fn test_create_deployment_with_template() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create definitions file
        let definitions_content = r"flags:
  - name: flag1
    type: boolean
    default: false
  - name: flag2
    type: boolean
    default: true
";
        fs::write("flags.definitions.yaml", definitions_content).unwrap();

        // Create template deployment
        let template_content = r"environment: template
rules:
  flag1:
    rules:
      - serve: true
";
        fs::write(".controlpath/template.deployment.yaml", template_content).unwrap();

        let definitions = read_definitions().unwrap();
        let template = read_deployment(&get_deployment_path("template")).unwrap();
        let deployment =
            create_deployment_from_definitions("test", &definitions, Some(&template)).unwrap();

        // flag1 should come from template (serve: true)
        let flag1_rules = deployment
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("flag1"))
            .and_then(|f| f.get("rules"))
            .and_then(|r| r.as_array())
            .and_then(|r| r.first())
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("serve"));
        assert_eq!(flag1_rules, Some(&Value::Bool(true)));

        // flag2 should be added from definitions (default: true)
        assert!(deployment
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("flag2"))
            .is_some());
    }

    #[test]
    #[serial]
    fn test_sync_flags_to_deployment() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create definitions file
        let definitions_content = r"flags:
  - name: flag1
    type: boolean
    default: false
  - name: flag2
    type: boolean
    default: true
";
        fs::write("flags.definitions.yaml", definitions_content).unwrap();

        // Create deployment file with one flag
        let deployment_content = r"environment: test
rules:
  flag1:
    rules:
      - serve: false
  old_flag:
    rules:
      - serve: true
";
        fs::write(".controlpath/test.deployment.yaml", deployment_content).unwrap();

        let definitions = read_definitions().unwrap();
        let mut deployment = read_deployment(&get_deployment_path("test")).unwrap();

        let (added, removed, total_flags) =
            sync_flags_to_deployment(&mut deployment, &definitions).unwrap();

        assert_eq!(added, 1); // flag2 added
        assert_eq!(removed, 1); // old_flag removed
        assert_eq!(total_flags, 2); // flag1 and flag2

        // Verify flag2 was added
        assert!(deployment
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("flag2"))
            .is_some());

        // Verify old_flag was removed
        assert!(deployment
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("old_flag"))
            .is_none());
    }

    #[test]
    #[serial]
    fn test_sync_preserves_existing_rules() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create definitions file
        let definitions_content = r"flags:
  - name: flag1
    type: boolean
    default: false
";
        fs::write("flags.definitions.yaml", definitions_content).unwrap();

        // Create deployment with custom rules
        let deployment_content = r"environment: test
rules:
  flag1:
    rules:
      - name: Custom rule
        when: user.role == 'admin'
        serve: true
      - serve: false
";
        fs::write(".controlpath/test.deployment.yaml", deployment_content).unwrap();

        let definitions = read_definitions().unwrap();
        let mut deployment = read_deployment(&get_deployment_path("test")).unwrap();
        let original_rules = deployment
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("flag1"))
            .cloned();

        let (added, removed, _) = sync_flags_to_deployment(&mut deployment, &definitions).unwrap();

        assert_eq!(added, 0); // No flags added
        assert_eq!(removed, 0); // No flags removed

        // Verify rules are preserved
        let preserved_rules = deployment
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("flag1"));
        assert_eq!(preserved_rules, original_rules.as_ref());
    }

    #[test]
    #[serial]
    fn test_env_add_command() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create definitions file
        let definitions_content = r"flags:
  - name: test_flag
    type: boolean
    default: false
";
        fs::write("flags.definitions.yaml", definitions_content).unwrap();

        let opts = Options {
            subcommand: EnvSubcommand::Add {
                name: Some("staging".to_string()),
                template: None,
                interactive: false,
            },
        };

        let result = run_inner(&opts);
        assert!(result.is_ok());

        // Verify deployment file was created
        assert!(get_deployment_path("staging").exists());

        // Verify deployment content
        let deployment = read_deployment(&get_deployment_path("staging")).unwrap();
        assert_eq!(
            deployment.get("environment").and_then(|e| e.as_str()),
            Some("staging")
        );
        assert!(deployment
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("test_flag"))
            .is_some());
    }

    #[test]
    #[serial]
    fn test_env_add_duplicate_name() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create definitions file
        fs::write("flags.definitions.yaml", "flags: []").unwrap();

        // Create existing environment
        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules: {}
",
        )
        .unwrap();

        let opts = Options {
            subcommand: EnvSubcommand::Add {
                name: Some("production".to_string()),
                template: None,
                interactive: false,
            },
        };

        let result = run_inner(&opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    #[serial]
    fn test_env_add_with_template() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create definitions file
        let definitions_content = r"flags:
  - name: flag1
    type: boolean
    default: false
  - name: flag2
    type: boolean
    default: true
";
        fs::write("flags.definitions.yaml", definitions_content).unwrap();

        // Create template environment
        fs::create_dir_all(".controlpath").unwrap();
        let template_content = r"environment: production
rules:
  flag1:
    rules:
      - serve: true
";
        fs::write(".controlpath/production.deployment.yaml", template_content).unwrap();

        let opts = Options {
            subcommand: EnvSubcommand::Add {
                name: Some("staging".to_string()),
                template: Some("production".to_string()),
                interactive: false,
            },
        };

        let result = run_inner(&opts);
        assert!(result.is_ok());

        // Verify staging was created
        let staging = read_deployment(&get_deployment_path("staging")).unwrap();
        assert_eq!(
            staging.get("environment").and_then(|e| e.as_str()),
            Some("staging")
        );

        // Verify flag1 was copied from template
        let flag1_serve = staging
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("flag1"))
            .and_then(|f| f.get("rules"))
            .and_then(|r| r.as_array())
            .and_then(|r| r.first())
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("serve"));
        assert_eq!(flag1_serve, Some(&Value::Bool(true)));
    }

    #[test]
    #[serial]
    fn test_env_sync_single_environment() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create definitions file
        let definitions_content = r"flags:
  - name: flag1
    type: boolean
    default: false
  - name: flag2
    type: boolean
    default: true
";
        fs::write("flags.definitions.yaml", definitions_content).unwrap();

        // Create deployment file
        fs::create_dir_all(".controlpath").unwrap();
        let deployment_content = r"environment: test
rules:
  flag1:
    rules:
      - serve: false
";
        fs::write(".controlpath/test.deployment.yaml", deployment_content).unwrap();

        let opts = Options {
            subcommand: EnvSubcommand::Sync {
                env: Some("test".to_string()),
                dry_run: false,
            },
        };

        let result = run_inner(&opts);
        assert!(result.is_ok());

        // Verify flag2 was added
        let deployment = read_deployment(&get_deployment_path("test")).unwrap();
        assert!(deployment
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("flag2"))
            .is_some());
    }

    #[test]
    #[serial]
    fn test_env_sync_all_environments() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create definitions file
        let definitions_content = r"flags:
  - name: flag1
    type: boolean
    default: false
";
        fs::write("flags.definitions.yaml", definitions_content).unwrap();

        // Create multiple deployment files
        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules: {}
",
        )
        .unwrap();
        fs::write(
            ".controlpath/staging.deployment.yaml",
            r"environment: staging
rules: {}
",
        )
        .unwrap();

        let opts = Options {
            subcommand: EnvSubcommand::Sync {
                env: None,
                dry_run: false,
            },
        };

        let result = run_inner(&opts);
        assert!(result.is_ok());

        // Verify both environments were synced
        let prod = read_deployment(&get_deployment_path("production")).unwrap();
        let staging = read_deployment(&get_deployment_path("staging")).unwrap();

        assert!(prod
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("flag1"))
            .is_some());
        assert!(staging
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("flag1"))
            .is_some());
    }

    #[test]
    #[serial]
    fn test_env_list_table_format() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create deployment files
        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules:
  flag1:
    rules:
      - serve: false
  flag2:
    rules:
      - serve: true
",
        )
        .unwrap();
        fs::write(
            ".controlpath/staging.deployment.yaml",
            r"environment: staging
rules:
  flag1:
    rules:
      - serve: false
",
        )
        .unwrap();

        let opts = Options {
            subcommand: EnvSubcommand::List {
                format: OutputFormat::Table,
            },
        };

        let result = run_inner(&opts);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_env_list_json_format() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create deployment files
        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules:
  flag1:
    rules:
      - serve: false
",
        )
        .unwrap();

        let opts = Options {
            subcommand: EnvSubcommand::List {
                format: OutputFormat::Json,
            },
        };

        let result = run_inner(&opts);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_env_list_yaml_format() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create deployment files
        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules:
  flag1:
    rules:
      - serve: false
",
        )
        .unwrap();

        let opts = Options {
            subcommand: EnvSubcommand::List {
                format: OutputFormat::Yaml,
            },
        };

        let result = run_inner(&opts);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_env_list_empty() {
        let (_temp_dir, _guard) = setup_test_env();

        // Don't create any deployment files

        let opts = Options {
            subcommand: EnvSubcommand::List {
                format: OutputFormat::Table,
            },
        };

        let result = run_inner(&opts);
        assert!(result.is_ok()); // Should handle empty gracefully
    }

    #[test]
    #[serial]
    fn test_sync_removes_orphaned_flags() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create definitions file with one flag
        let definitions_content = r"flags:
  - name: flag1
    type: boolean
    default: false
";
        fs::write("flags.definitions.yaml", definitions_content).unwrap();

        // Create deployment with extra flag
        fs::create_dir_all(".controlpath").unwrap();
        let deployment_content = r"environment: test
rules:
  flag1:
    rules:
      - serve: false
  orphaned_flag:
    rules:
      - serve: true
";
        fs::write(".controlpath/test.deployment.yaml", deployment_content).unwrap();

        let definitions = read_definitions().unwrap();
        let mut deployment = read_deployment(&get_deployment_path("test")).unwrap();

        let (added, removed, _) = sync_flags_to_deployment(&mut deployment, &definitions).unwrap();

        assert_eq!(added, 0);
        assert_eq!(removed, 1); // orphaned_flag removed

        // Verify orphaned_flag was removed
        assert!(deployment
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("orphaned_flag"))
            .is_none());
    }

    #[test]
    #[serial]
    fn test_env_sync_dry_run() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create definitions file
        let definitions_content = r"flags:
  - name: flag1
    type: boolean
    default: false
  - name: flag2
    type: boolean
    default: true
";
        fs::write("flags.definitions.yaml", definitions_content).unwrap();

        // Create deployment file with one flag
        fs::create_dir_all(".controlpath").unwrap();
        let deployment_content = r"environment: test
rules:
  flag1:
    rules:
      - serve: false
";
        fs::write(".controlpath/test.deployment.yaml", deployment_content).unwrap();

        let opts = Options {
            subcommand: EnvSubcommand::Sync {
                env: Some("test".to_string()),
                dry_run: true,
            },
        };

        let result = run_inner(&opts);
        assert!(result.is_ok());

        // Verify deployment file was NOT modified (dry-run)
        let deployment = read_deployment(&get_deployment_path("test")).unwrap();
        assert!(deployment
            .get("rules")
            .and_then(|r| r.as_object())
            .and_then(|r| r.get("flag2"))
            .is_none()); // flag2 should not be added in dry-run
    }

    #[test]
    #[serial]
    fn test_env_remove_command() {
        let (_temp_dir, _guard) = setup_test_env();

        // Create deployment file
        fs::create_dir_all(".controlpath").unwrap();
        let deployment_content = r"environment: test
rules:
  flag1:
    rules:
      - serve: false
";
        fs::write(".controlpath/test.deployment.yaml", deployment_content).unwrap();

        assert!(get_deployment_path("test").exists());

        let opts = Options {
            subcommand: EnvSubcommand::Remove {
                name: "test".to_string(),
                force: true,
            },
        };

        let result = run_inner(&opts);
        assert!(result.is_ok());

        // Verify deployment file was removed
        assert!(!get_deployment_path("test").exists());
    }

    #[test]
    #[serial]
    fn test_env_remove_nonexistent() {
        let (_temp_dir, _guard) = setup_test_env();

        let opts = Options {
            subcommand: EnvSubcommand::Remove {
                name: "nonexistent".to_string(),
                force: true,
            },
        };

        let result = run_inner(&opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
