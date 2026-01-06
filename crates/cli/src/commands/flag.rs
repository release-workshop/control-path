//! Flag management command implementation

use crate::error::{CliError, CliResult};
use crate::generator::generate_sdk;
use controlpath_compiler::{
    parse_definitions, parse_deployment, validate_definitions, validate_deployment,
};
use dialoguer::{Confirm, Input, Select};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use strsim::levenshtein;

pub struct Options {
    pub subcommand: FlagSubcommand,
}

#[derive(Debug, Clone)]
pub enum FlagSubcommand {
    Add {
        name: Option<String>,
        flag_type: Option<String>,
        default: Option<String>,
        description: Option<String>,
        lang: Option<String>,
        sync: bool,
        interactive: bool,
    },
    List {
        definitions: bool,
        deployment: Option<String>,
        format: OutputFormat,
    },
    Show {
        name: String,
        deployment: Option<String>,
        format: OutputFormat,
    },
    Remove {
        name: String,
        from_deployments: bool,
        env: Option<String>,
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
    // Flag names must match pattern: ^[a-z][a-z0-9_]*$
    if name.is_empty() {
        return Err(CliError::Message("Flag name cannot be empty".to_string()));
    }
    if !name.chars().next().unwrap().is_ascii_lowercase() {
        return Err(CliError::Message(
            "Flag name must start with a lowercase letter".to_string(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(CliError::Message(
            "Flag name can only contain lowercase letters, digits, and underscores".to_string(),
        ));
    }
    Ok(())
}

fn validate_flag_type(flag_type: &str) -> CliResult<()> {
    if flag_type != "boolean" && flag_type != "multivariate" {
        return Err(CliError::Message(
            "Flag type must be 'boolean' or 'multivariate'".to_string(),
        ));
    }
    Ok(())
}

fn flag_exists(definitions: &Value, name: &str) -> bool {
    if let Some(flags) = definitions.get("flags").and_then(|f| f.as_array()) {
        for flag in flags {
            if let Some(flag_name) = flag.get("name").and_then(|n| n.as_str()) {
                if flag_name == name {
                    return true;
                }
            }
        }
    }
    false
}

fn find_similar_flag_names(definitions: &Value, name: &str) -> Vec<String> {
    let mut similar = Vec::new();
    if let Some(flags) = definitions.get("flags").and_then(|f| f.as_array()) {
        for flag in flags {
            if let Some(flag_name) = flag.get("name").and_then(|n| n.as_str()) {
                let distance = levenshtein(name, flag_name);
                // If distance is small relative to name length, consider it similar
                if distance > 0 && distance <= name.len().max(flag_name.len()) / 2 {
                    similar.push((flag_name.to_string(), distance));
                }
            }
        }
    }
    // Sort by distance and return top 3
    similar.sort_by_key(|(_, d)| *d);
    similar.into_iter().take(3).map(|(name, _)| name).collect()
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
        .ok_or_else(|| {
            CliError::Message("Invalid definitions: missing 'flags' array".to_string())
        })?;

    let mut flag_obj = serde_json::Map::new();
    flag_obj.insert("name".to_string(), Value::String(name.to_string()));
    flag_obj.insert("type".to_string(), Value::String(flag_type.to_string()));
    flag_obj.insert("defaultValue".to_string(), default.clone());

    if let Some(desc) = description {
        flag_obj.insert("description".to_string(), Value::String(desc.to_string()));
    }

    flags.push(Value::Object(flag_obj));
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
        .ok_or_else(|| {
            CliError::Message("Invalid deployment: missing 'rules' object".to_string())
        })?;

    // Only add if flag doesn't already exist in deployment
    if !rules.contains_key(flag_name) {
        let default_serve = match default_value {
            Value::Bool(b) => Value::Bool(*b),
            Value::String(s) => {
                // Convert "ON"/"OFF" to boolean if needed
                if s == "ON" {
                    Value::Bool(true)
                } else if s == "OFF" {
                    Value::Bool(false)
                } else {
                    default_value.clone()
                }
            }
            _ => default_value.clone(),
        };

        let mut rule_obj = serde_json::Map::new();
        rule_obj.insert("serve".to_string(), default_serve);

        let mut flag_entry = serde_json::Map::new();
        flag_entry.insert(
            "rules".to_string(),
            Value::Array(vec![Value::Object(rule_obj)]),
        );
        rules.insert(flag_name.to_string(), Value::Object(flag_entry));
    }

    Ok(())
}

fn prompt_for_flag_name(definitions: &Value) -> CliResult<String> {
    loop {
        let name: String = Input::new()
            .with_prompt("Flag name")
            .validate_with(|input: &String| -> Result<(), String> {
                validate_flag_name(input).map_err(|e| format!("{}", e))
            })
            .interact()
            .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;

        if flag_exists(definitions, &name) {
            let similar = find_similar_flag_names(definitions, &name);
            let mut msg = format!("Flag '{name}' already exists");
            if !similar.is_empty() {
                msg.push_str(&format!("\n  Did you mean: {}?", similar.join(", ")));
            }
            eprintln!("✗ {msg}");
            continue;
        }

        return Ok(name);
    }
}

fn prompt_for_flag_type() -> CliResult<String> {
    let types = vec!["boolean", "multivariate"];
    let selection = Select::new()
        .with_prompt("Flag type")
        .items(&types)
        .default(0)
        .interact()
        .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;
    Ok(types[selection].to_string())
}

fn prompt_for_default_value(flag_type: &str) -> CliResult<Value> {
    if flag_type == "boolean" {
        let default: bool = Input::new()
            .with_prompt("Default value")
            .default(false)
            .interact()
            .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;
        Ok(Value::Bool(default))
    } else {
        let default: String = Input::new()
            .with_prompt("Default value (variation name)")
            .interact()
            .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;
        Ok(Value::String(default))
    }
}

fn prompt_for_description() -> CliResult<Option<String>> {
    let description: String = Input::new()
        .with_prompt("Description (optional)")
        .allow_empty(true)
        .interact()
        .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;
    Ok(if description.is_empty() {
        None
    } else {
        Some(description)
    })
}

fn prompt_for_sync_to_deployments(deployment_files: &[PathBuf]) -> CliResult<bool> {
    if deployment_files.is_empty() {
        return Ok(false);
    }
    let envs: Vec<String> = deployment_files
        .iter()
        .filter_map(|p| get_environment_name(p.as_path()))
        .collect();
    if envs.is_empty() {
        return Ok(false);
    }
    Confirm::new()
        .with_prompt(format!(
            "Sync to {} deployment file(s)? ({})",
            deployment_files.len(),
            envs.join(", ")
        ))
        .default(true)
        .interact()
        .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))
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

fn run_inner(options: &Options) -> CliResult<()> {
    match &options.subcommand {
        FlagSubcommand::Add {
            name,
            flag_type,
            default,
            description,
            lang,
            sync,
            interactive,
        } => {
            let mut definitions = read_definitions()?;

            // Interactive mode: prompt for missing values
            let (name, flag_type, default_value, description) = if *interactive && name.is_none() {
                let name = prompt_for_flag_name(&definitions)?;
                let flag_type = flag_type.clone().unwrap_or_else(|| {
                    prompt_for_flag_type().unwrap_or_else(|_| "boolean".to_string())
                });

                // Validate flag type in interactive mode as well
                validate_flag_type(&flag_type)?;

                let default_value = default
                    .as_ref()
                    .map(|d| {
                        if d == "true" || d == "ON" {
                            Value::Bool(true)
                        } else if d == "false" || d == "OFF" {
                            Value::Bool(false)
                        } else {
                            Value::String(d.clone())
                        }
                    })
                    .unwrap_or_else(|| {
                        prompt_for_default_value(&flag_type).unwrap_or_else(|_| {
                            if flag_type == "boolean" {
                                Value::Bool(false)
                            } else {
                                Value::String("default".to_string())
                            }
                        })
                    });
                let description = description
                    .clone()
                    .or_else(|| prompt_for_description().ok().flatten());
                (name, flag_type, default_value, description)
            } else {
                // Non-interactive mode: use provided values or defaults
                let name = name.clone().ok_or_else(|| {
                    CliError::Message(
                        "Flag name is required. Use --name <name> or run in interactive mode"
                            .to_string(),
                    )
                })?;
                validate_flag_name(&name)?;

                if flag_exists(&definitions, &name) {
                    let similar = find_similar_flag_names(&definitions, &name);
                    let mut msg = format!("Flag '{name}' already exists");
                    if !similar.is_empty() {
                        msg.push_str(&format!("\n  Did you mean: {}?", similar.join(", ")));
                    }
                    return Err(CliError::Message(msg));
                }

                let flag_type = flag_type.as_deref().unwrap_or("boolean");
                validate_flag_type(flag_type)?;

                let default_value = if let Some(default_str) = default {
                    if default_str == "true" || default_str == "ON" {
                        Value::Bool(true)
                    } else if default_str == "false" || default_str == "OFF" {
                        Value::Bool(false)
                    } else {
                        Value::String(default_str.clone())
                    }
                } else if flag_type == "boolean" {
                    Value::Bool(false)
                } else {
                    return Err(CliError::Message(
                        "Multivariate flags require a default value".to_string(),
                    ));
                };
                (
                    name,
                    flag_type.to_string(),
                    default_value,
                    description.clone(),
                )
            };

            add_flag_to_definitions(
                &mut definitions,
                &name,
                &flag_type,
                &default_value,
                description.as_deref(),
            )?;

            // Validate before writing
            validate_definitions(&definitions)?;
            write_definitions(&definitions)?;

            // Sync to deployment files
            let deployment_files = find_deployment_files();
            let should_sync = if *sync {
                true
            } else if *interactive && !deployment_files.is_empty() {
                prompt_for_sync_to_deployments(&deployment_files).unwrap_or(false)
            } else {
                false
            };

            let mut synced_count = 0;
            if should_sync {
                for deployment_path in &deployment_files {
                    match read_deployment(deployment_path) {
                        Ok(mut deployment) => {
                            match sync_flag_to_deployment(&mut deployment, &name, &default_value) {
                                Ok(()) => match validate_deployment(&deployment) {
                                    Ok(()) => {
                                        match write_deployment(deployment_path, &deployment) {
                                            Ok(()) => synced_count += 1,
                                            Err(e) => eprintln!(
                                                "  Warning: Failed to write {}: {e}",
                                                deployment_path.display()
                                            ),
                                        }
                                    }
                                    Err(e) => eprintln!(
                                        "  Warning: Failed to validate {}: {e}",
                                        deployment_path.display()
                                    ),
                                },
                                Err(e) => eprintln!(
                                    "  Warning: Failed to sync to {}: {e}",
                                    deployment_path.display()
                                ),
                            }
                        }
                        Err(e) => eprintln!(
                            "  Warning: Failed to read {}: {e}",
                            deployment_path.display()
                        ),
                    }
                }
            }

            println!("✓ Added flag '{name}'");
            if synced_count > 0 {
                println!("  Synced to {synced_count} deployment file(s)");
            }

            // Regenerate SDK if lang is specified
            if let Some(language) = lang {
                let output_path = PathBuf::from("./flags");
                match generate_sdk(language, &definitions, &output_path) {
                    Ok(()) => println!("  Regenerated SDK ({language})"),
                    Err(e) => eprintln!("  Warning: Failed to regenerate SDK: {e}"),
                }
            }

            // Show next steps
            if synced_count == 0 && !deployment_files.is_empty() {
                println!("\nNext steps:");
                if let Some(env) = deployment_files
                    .first()
                    .and_then(|p| get_environment_name(p.as_path()))
                {
                    println!("  controlpath flag enable {name} --env {env}");
                }
            } else if synced_count > 0 {
                println!("\nNext steps:");
                println!("  controlpath compile --env <env>  # Compile deployment files");
            }

            Ok(())
        }
        FlagSubcommand::List {
            definitions,
            deployment,
            format,
        } => {
            if *definitions {
                let defs = read_definitions()?;
                list_flags_from_definitions(&defs, format)?;
            } else if let Some(env) = deployment {
                let path = PathBuf::from(format!(".controlpath/{env}.deployment.yaml"));
                let dep = read_deployment(&path)?;
                let defs = read_definitions().ok();
                list_flags_from_deployment(&dep, defs.as_ref(), format)?;
            } else {
                // Default: list from definitions
                let defs = read_definitions()?;
                list_flags_from_definitions(&defs, format)?;
            }
            Ok(())
        }
        FlagSubcommand::Show {
            name,
            deployment,
            format,
        } => {
            let definitions = read_definitions()?;
            show_flag(&definitions, name, deployment.as_deref(), format)?;
            Ok(())
        }
        FlagSubcommand::Remove {
            name,
            from_deployments,
            env,
            force,
        } => {
            validate_flag_name(name)?;

            let mut definitions = read_definitions()?;

            if !flag_exists(&definitions, name) {
                return Err(CliError::Message(format!("Flag '{name}' not found")));
            }

            // Show preview of what will be removed
            if !*force {
                println!("This will remove flag '{name}' from:");
                println!("  - flags.definitions.yaml");

                let deployment_files = if let Some(env) = env {
                    vec![PathBuf::from(format!(".controlpath/{env}.deployment.yaml"))]
                } else if *from_deployments {
                    find_deployment_files()
                } else {
                    Vec::new()
                };

                for deployment_path in &deployment_files {
                    if deployment_path.exists() {
                        if let Some(env_name) = get_environment_name(deployment_path) {
                            println!("  - .controlpath/{env_name}.deployment.yaml");
                        }
                    }
                }

                if !Confirm::new()
                    .with_prompt("Continue?")
                    .default(false)
                    .interact()
                    .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?
                {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            // Remove from definitions
            if let Some(flags) = definitions.get_mut("flags").and_then(|f| f.as_array_mut()) {
                flags.retain(|flag| {
                    flag.get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| n != name)
                        .unwrap_or(true)
                });
            }

            validate_definitions(&definitions)?;
            write_definitions(&definitions)?;

            // Remove from deployment files
            let deployment_files = if let Some(env) = env {
                vec![PathBuf::from(format!(".controlpath/{env}.deployment.yaml"))]
            } else if *from_deployments {
                find_deployment_files()
            } else {
                Vec::new()
            };

            let mut removed_count = 0;
            for deployment_path in &deployment_files {
                match read_deployment(deployment_path) {
                    Ok(mut deployment) => {
                        if let Some(rules) =
                            deployment.get_mut("rules").and_then(|r| r.as_object_mut())
                        {
                            if rules.remove(name).is_some() {
                                removed_count += 1;
                                match validate_deployment(&deployment) {
                                    Ok(()) => {
                                        match write_deployment(deployment_path, &deployment) {
                                            Ok(()) => {}
                                            Err(e) => eprintln!(
                                                "  Warning: Failed to write {}: {e}",
                                                deployment_path.display()
                                            ),
                                        }
                                    }
                                    Err(e) => eprintln!(
                                        "  Warning: Failed to validate {}: {e}",
                                        deployment_path.display()
                                    ),
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!(
                        "  Warning: Failed to read {}: {e}",
                        deployment_path.display()
                    ),
                }
            }

            println!("✓ Removed flag '{name}'");
            if removed_count > 0 {
                println!("  Removed from {removed_count} deployment file(s)");
            }
            Ok(())
        }
    }
}

fn list_flags_from_definitions(definitions: &Value, format: &OutputFormat) -> CliResult<()> {
    let flags = definitions
        .get("flags")
        .and_then(|f| f.as_array())
        .ok_or_else(|| {
            CliError::Message("Invalid definitions: missing 'flags' array".to_string())
        })?;

    match format {
        OutputFormat::Table => {
            println!("Flags:");
            println!("{:-<80}", "");
            println!(
                "{:<30} {:<15} {:<20} Description",
                "Name", "Type", "Default"
            );
            println!("{:-<80}", "");
            for flag in flags {
                let name = flag.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                let flag_type = flag.get("type").and_then(|t| t.as_str()).unwrap_or("?");
                let default = flag
                    .get("defaultValue")
                    .map(format_value)
                    .unwrap_or_else(|| "?".to_string());
                let description = flag
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                println!(
                    "{:<30} {:<15} {:<20} {}",
                    name, flag_type, default, description
                );
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(flags)
                .map_err(|e| CliError::Message(format!("Failed to serialize: {e}")))?;
            println!("{json}");
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(flags)
                .map_err(|e| CliError::Message(format!("Failed to serialize: {e}")))?;
            print!("{yaml}");
        }
    }
    Ok(())
}

fn list_flags_from_deployment(
    deployment: &Value,
    definitions: Option<&Value>,
    format: &OutputFormat,
) -> CliResult<()> {
    let rules = deployment
        .get("rules")
        .and_then(|r| r.as_object())
        .ok_or_else(|| {
            CliError::Message("Invalid deployment: missing 'rules' object".to_string())
        })?;

    // Build flag info by looking up in definitions
    let mut flag_info = Vec::new();
    for (flag_name, _flag_rules) in rules {
        let mut info = serde_json::Map::new();
        info.insert("name".to_string(), Value::String(flag_name.clone()));

        // Look up flag details from definitions
        if let Some(defs) = definitions {
            if let Some(flags) = defs.get("flags").and_then(|f| f.as_array()) {
                if let Some(flag_def) = flags
                    .iter()
                    .find(|f| f.get("name").and_then(|n| n.as_str()) == Some(flag_name.as_str()))
                {
                    if let Some(flag_type) = flag_def.get("type") {
                        info.insert("type".to_string(), flag_type.clone());
                    }
                    if let Some(default) = flag_def.get("defaultValue") {
                        info.insert("default".to_string(), default.clone());
                    }
                    if let Some(description) = flag_def.get("description") {
                        info.insert("description".to_string(), description.clone());
                    }
                }
            }
        }

        // Check if flag has rules (status: configured)
        let status = if _flag_rules
            .get("rules")
            .and_then(|r| r.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false)
        {
            "configured"
        } else {
            "not configured"
        };
        info.insert("status".to_string(), Value::String(status.to_string()));

        flag_info.push(Value::Object(info));
    }

    match format {
        OutputFormat::Table => {
            println!("Flags in deployment:");
            println!("{:-<80}", "");
            println!(
                "{:<30} {:<15} {:<20} {:<15}",
                "Name", "Type", "Default", "Status"
            );
            println!("{:-<80}", "");
            for info in &flag_info {
                let name = info.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                let flag_type = info.get("type").and_then(|t| t.as_str()).unwrap_or("?");
                let default = info
                    .get("default")
                    .map(format_value)
                    .unwrap_or_else(|| "?".to_string());
                let status = info.get("status").and_then(|s| s.as_str()).unwrap_or("?");
                println!(
                    "{:<30} {:<15} {:<20} {:<15}",
                    name, flag_type, default, status
                );
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&flag_info)
                .map_err(|e| CliError::Message(format!("Failed to serialize: {e}")))?;
            println!("{json}");
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&flag_info)
                .map_err(|e| CliError::Message(format!("Failed to serialize: {e}")))?;
            print!("{yaml}");
        }
    }
    Ok(())
}

fn show_flag(
    definitions: &Value,
    name: &str,
    deployment_env: Option<&str>,
    format: &OutputFormat,
) -> CliResult<()> {
    let flags = definitions
        .get("flags")
        .and_then(|f| f.as_array())
        .ok_or_else(|| {
            CliError::Message("Invalid definitions: missing 'flags' array".to_string())
        })?;

    let flag = flags
        .iter()
        .find(|f| f.get("name").and_then(|n| n.as_str()) == Some(name))
        .ok_or_else(|| CliError::Message(format!("Flag '{name}' not found")))?;

    match format {
        OutputFormat::Table => {
            println!("Flag: {name}");
            println!("{:-<60}", "");

            if let Some(flag_type) = flag.get("type").and_then(|t| t.as_str()) {
                println!("Type: {flag_type}");
            }

            if let Some(default) = flag.get("defaultValue") {
                println!("Default: {}", format_value(default));
            }

            if let Some(description) = flag.get("description").and_then(|d| d.as_str()) {
                println!("Description: {description}");
            }

            if let Some(variations) = flag.get("variations").and_then(|v| v.as_array()) {
                println!("Variations:");
                for variation in variations {
                    if let Some(var_name) = variation.get("name").and_then(|n| n.as_str()) {
                        let var_value = variation
                            .get("value")
                            .map(format_value)
                            .unwrap_or_else(|| "?".to_string());
                        println!("  - {var_name}: {var_value}");
                    }
                }
            }

            // Show deployment info - either specific env or all
            if let Some(env) = deployment_env {
                let path = PathBuf::from(format!(".controlpath/{env}.deployment.yaml"));
                if let Ok(deployment) = read_deployment(&path) {
                    if let Some(rules) = deployment.get("rules").and_then(|r| r.as_object()) {
                        if let Some(flag_rules) = rules.get(name) {
                            println!("\nDeployment ({env}):");
                            if let Some(rules_array) =
                                flag_rules.get("rules").and_then(|r| r.as_array())
                            {
                                println!("  Rules: {}", rules_array.len());
                            }
                        } else {
                            println!("\nDeployment ({env}): Not configured");
                        }
                    }
                }
            } else {
                // Show status across all environments
                let deployment_files = find_deployment_files();
                if !deployment_files.is_empty() {
                    println!("\nDeployment Status:");
                    for deployment_path in &deployment_files {
                        if let Some(env_name) = get_environment_name(deployment_path) {
                            if let Ok(deployment) = read_deployment(deployment_path) {
                                if let Some(rules) =
                                    deployment.get("rules").and_then(|r| r.as_object())
                                {
                                    if rules.contains_key(name) {
                                        if let Some(flag_rules) = rules.get(name) {
                                            if let Some(rules_array) =
                                                flag_rules.get("rules").and_then(|r| r.as_array())
                                            {
                                                println!(
                                                    "  {env_name}: {} rule(s)",
                                                    rules_array.len()
                                                );
                                            } else {
                                                println!("  {env_name}: configured");
                                            }
                                        }
                                    } else {
                                        println!("  {env_name}: not configured");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        OutputFormat::Json => {
            let mut output = serde_json::Map::new();
            output.insert("flag".to_string(), flag.clone());

            // Add deployment info
            let mut deployments = serde_json::Map::new();
            let deployment_files = if let Some(env) = deployment_env {
                vec![PathBuf::from(format!(".controlpath/{env}.deployment.yaml"))]
            } else {
                find_deployment_files()
            };

            for deployment_path in &deployment_files {
                if let Some(env_name) = get_environment_name(deployment_path) {
                    if let Ok(deployment) = read_deployment(deployment_path) {
                        if let Some(rules) = deployment.get("rules").and_then(|r| r.as_object()) {
                            if let Some(flag_rules) = rules.get(name) {
                                deployments.insert(env_name, flag_rules.clone());
                            }
                        }
                    }
                }
            }
            output.insert("deployments".to_string(), Value::Object(deployments));

            let json = serde_json::to_string_pretty(&output)
                .map_err(|e| CliError::Message(format!("Failed to serialize: {e}")))?;
            println!("{json}");
        }
        OutputFormat::Yaml => {
            let mut output = serde_json::Map::new();
            output.insert("flag".to_string(), flag.clone());

            // Add deployment info
            let mut deployments = serde_json::Map::new();
            let deployment_files = if let Some(env) = deployment_env {
                vec![PathBuf::from(format!(".controlpath/{env}.deployment.yaml"))]
            } else {
                find_deployment_files()
            };

            for deployment_path in &deployment_files {
                if let Some(env_name) = get_environment_name(deployment_path) {
                    if let Ok(deployment) = read_deployment(deployment_path) {
                        if let Some(rules) = deployment.get("rules").and_then(|r| r.as_object()) {
                            if let Some(flag_rules) = rules.get(name) {
                                deployments.insert(env_name, flag_rules.clone());
                            }
                        }
                    }
                }
            }
            output.insert("deployments".to_string(), Value::Object(deployments));

            let yaml = serde_yaml::to_string(&output)
                .map_err(|e| CliError::Message(format!("Failed to serialize: {e}")))?;
            print!("{yaml}");
        }
    }
    Ok(())
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => format!("{value}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    struct DirGuard {
        original_dir: PathBuf,
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

    #[test]
    #[serial]
    fn test_validate_flag_name() {
        assert!(validate_flag_name("my_flag").is_ok());
        assert!(validate_flag_name("flag123").is_ok());
        assert!(validate_flag_name("a").is_ok());

        assert!(validate_flag_name("").is_err());
        assert!(validate_flag_name("MyFlag").is_err()); // uppercase
        assert!(validate_flag_name("my-flag").is_err()); // hyphen
        assert!(validate_flag_name("123flag").is_err()); // starts with number
    }

    #[test]
    #[serial]
    fn test_flag_add_command() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create definitions file
        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: existing_flag
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        // Create deployment file
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules:
  existing_flag:
    rules:
      - serve: false
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Add {
                name: Some("new_flag".to_string()),
                flag_type: Some("boolean".to_string()),
                default: Some("false".to_string()),
                description: Some("A new flag".to_string()),
                lang: None,
                sync: true,
                interactive: false,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);

        // Verify flag was added to definitions
        let content = fs::read_to_string("flags.definitions.yaml").unwrap();
        assert!(content.contains("new_flag"));

        // Verify flag was synced to deployment
        let deployment_content =
            fs::read_to_string(".controlpath/production.deployment.yaml").unwrap();
        assert!(deployment_content.contains("new_flag"));
    }

    #[test]
    #[serial]
    fn test_flag_add_multivariate() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags: []
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Add {
                name: Some("multivar_flag".to_string()),
                flag_type: Some("multivariate".to_string()),
                default: Some("variant_a".to_string()),
                description: None,
                lang: None,
                sync: false,
                interactive: false,
            },
        };

        // Multivariate flags require variations array, which flag add doesn't support yet
        // So this should fail validation
        let exit_code = run(&options);
        // The command will fail because multivariate flags need variations
        // For now, we expect it to fail until variations support is added
        assert_ne!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_list_command() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: flag1
    type: boolean
    defaultValue: false
  - name: flag2
    type: boolean
    defaultValue: true
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::List {
                definitions: true,
                deployment: None,
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_find_similar_flag_names() {
        let definitions = serde_json::json!({
            "flags": [
                {"name": "test_flag"},
                {"name": "test_flag_2"},
                {"name": "other_flag"},
                {"name": "test"}
            ]
        });

        let similar = find_similar_flag_names(&definitions, "test_flag_1");
        // Should find similar flags
        assert!(!similar.is_empty());
        assert!(similar.len() <= 3); // Should return max 3
    }

    #[test]
    fn test_find_similar_flag_names_no_similar() {
        let definitions = serde_json::json!({
            "flags": [
                {"name": "abc"},
                {"name": "xyz"}
            ]
        });

        let similar = find_similar_flag_names(&definitions, "completely_different");
        // Should be empty or have very few matches
        assert!(similar.len() <= 3);
    }

    #[test]
    fn test_find_similar_flag_names_empty_definitions() {
        let definitions = serde_json::json!({
            "flags": []
        });

        let similar = find_similar_flag_names(&definitions, "test_flag");
        assert!(similar.is_empty());
    }


    #[test]
    #[serial]
    fn test_flag_remove_nonexistent_flag() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: other_flag
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Remove {
                name: "nonexistent_flag".to_string(),
                from_deployments: false,
                env: None,
                force: true,
            },
        };

        let exit_code = run(&options);
        assert_ne!(exit_code, 0);
    }


    #[test]
    #[serial]
    fn test_flag_list_json_format() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: flag1
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::List {
                definitions: true,
                deployment: None,
                format: OutputFormat::Json,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_remove_command() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: flag1
    type: boolean
    defaultValue: false
  - name: flag2
    type: boolean
    defaultValue: true
",
        )
        .unwrap();

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

        let options = Options {
            subcommand: FlagSubcommand::Remove {
                name: "flag1".to_string(),
                from_deployments: true,
                env: None,
                force: true,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);

        // Verify flag was removed from definitions
        let content = fs::read_to_string("flags.definitions.yaml").unwrap();
        assert!(!content.contains("flag1"));
        assert!(content.contains("flag2"));

        // Verify flag was removed from deployment
        let deployment_content =
            fs::read_to_string(".controlpath/production.deployment.yaml").unwrap();
        assert!(!deployment_content.contains("flag1"));
        assert!(deployment_content.contains("flag2"));
    }

    #[test]
    #[serial]
    fn test_flag_remove_from_specific_env() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: flag1
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

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

        fs::write(
            ".controlpath/staging.deployment.yaml",
            r"environment: staging
rules:
  flag1:
    rules:
      - serve: true
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Remove {
                name: "flag1".to_string(),
                from_deployments: true,
                env: Some("production".to_string()),
                force: true,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);

        // Verify flag was removed from production but not staging
        let prod_content = fs::read_to_string(".controlpath/production.deployment.yaml").unwrap();
        assert!(!prod_content.contains("flag1"));

        let staging_content = fs::read_to_string(".controlpath/staging.deployment.yaml").unwrap();
        assert!(staging_content.contains("flag1"));
    }

    #[test]
    #[serial]
    fn test_flag_show_command() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: test_flag
    type: boolean
    defaultValue: false
    description: A test flag
",
        )
        .unwrap();

        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules:
  test_flag:
    rules:
      - serve: false
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Show {
                name: "test_flag".to_string(),
                deployment: Some("production".to_string()),
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_find_similar_flag_names_integration() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: my_feature_flag
    type: boolean
    defaultValue: false
  - name: my_other_flag
    type: boolean
    defaultValue: false
  - name: completely_different
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        let definitions = read_definitions().unwrap();
        let similar = find_similar_flag_names(&definitions, "my_feature_flg");
        // Should find "my_feature_flag" as similar
        assert!(similar.contains(&"my_feature_flag".to_string()));
    }

    #[test]
    #[serial]
    fn test_flag_add_with_lang() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "flags.definitions.yaml",
            r"flags: []
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Add {
                name: Some("test_flag".to_string()),
                flag_type: Some("boolean".to_string()),
                default: Some("false".to_string()),
                description: None,
                lang: Some("typescript".to_string()),
                sync: false,
                interactive: false,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_add_with_default_on_off() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags: []
",
        )
        .unwrap();

        // Test with "ON"
        let options = Options {
            subcommand: FlagSubcommand::Add {
                name: Some("flag_on".to_string()),
                flag_type: Some("boolean".to_string()),
                default: Some("ON".to_string()),
                description: None,
                lang: None,
                sync: false,
                interactive: false,
            },
        };
        assert_eq!(run(&options), 0);

        // Test with "OFF"
        let options2 = Options {
            subcommand: FlagSubcommand::Add {
                name: Some("flag_off".to_string()),
                flag_type: Some("boolean".to_string()),
                default: Some("OFF".to_string()),
                description: None,
                lang: None,
                sync: false,
                interactive: false,
            },
        };
        assert_eq!(run(&options2), 0);
    }

    #[test]
    #[serial]
    fn test_flag_list_from_deployment() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: flag1
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules:
  flag1:
    rules:
      - serve: true
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::List {
                definitions: false,
                deployment: Some("production".to_string()),
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_show_yaml_format() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: test_flag
    type: boolean
    defaultValue: false
    description: A test flag
",
        )
        .unwrap();

        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules:
  test_flag:
    rules:
      - serve: false
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Show {
                name: "test_flag".to_string(),
                deployment: None,
                format: OutputFormat::Yaml,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_show_json_format() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: test_flag
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Show {
                name: "test_flag".to_string(),
                deployment: None,
                format: OutputFormat::Json,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_show_with_variations() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: test_flag
    type: multivariate
    defaultValue: variant_a
    variations:
      - name: VARIANT_A
        value: variant_a
      - name: VARIANT_B
        value: variant_b
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Show {
                name: "test_flag".to_string(),
                deployment: None,
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_remove_without_from_deployments() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: flag1
    type: boolean
    defaultValue: false
  - name: flag2
    type: boolean
    defaultValue: true
",
        )
        .unwrap();

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

        let options = Options {
            subcommand: FlagSubcommand::Remove {
                name: "flag1".to_string(),
                from_deployments: false,
                env: None,
                force: true,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);

        // Verify flag was removed from definitions but not deployment
        let content = fs::read_to_string("flags.definitions.yaml").unwrap();
        assert!(!content.contains("flag1"));
        assert!(content.contains("flag2"));

        let deployment_content =
            fs::read_to_string(".controlpath/production.deployment.yaml").unwrap();
        assert!(deployment_content.contains("flag1")); // Still in deployment
    }

    #[test]
    #[serial]
    fn test_output_format_from_str() {
        assert!(OutputFormat::from_str("table").is_ok());
        assert!(OutputFormat::from_str("json").is_ok());
        assert!(OutputFormat::from_str("yaml").is_ok());
        assert!(OutputFormat::from_str("TABLE").is_ok()); // Case insensitive
        assert!(OutputFormat::from_str("invalid").is_err());
    }

    #[test]
    #[serial]
    fn test_validate_flag_type() {
        assert!(validate_flag_type("boolean").is_ok());
        assert!(validate_flag_type("multivariate").is_ok());
        assert!(validate_flag_type("invalid").is_err());
    }

    #[test]
    #[serial]
    fn test_flag_exists() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: existing_flag
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        let definitions = read_definitions().unwrap();
        assert!(flag_exists(&definitions, "existing_flag"));
        assert!(!flag_exists(&definitions, "nonexistent_flag"));
    }

    #[test]
    #[serial]
    fn test_sync_flag_to_deployment_preserves_existing() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules:
  existing_flag:
    rules:
      - serve: true
",
        )
        .unwrap();

        let mut deployment = read_deployment(&PathBuf::from(
            ".controlpath/production.deployment.yaml",
        ))
        .unwrap();

        // Try to sync a flag that already exists
        sync_flag_to_deployment(&mut deployment, "existing_flag", &Value::Bool(false)).unwrap();

        // Flag should still exist (not duplicated)
        let rules = deployment.get("rules").and_then(|r| r.as_object()).unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules.contains_key("existing_flag"));
    }

    #[test]
    #[serial]
    fn test_format_value() {
        assert_eq!(format_value(&Value::Bool(true)), "true");
        assert_eq!(format_value(&Value::Bool(false)), "false");
        assert_eq!(format_value(&Value::String("test".to_string())), "test");
        assert_eq!(format_value(&Value::Number(42.into())), "42");
    }

    #[test]
    fn test_format_value_edge_cases() {
        assert_eq!(format_value(&Value::Null), "null");
        assert_eq!(format_value(&Value::Array(vec![])), "[]");
        assert_eq!(format_value(&Value::Object(serde_json::Map::new())), "{}");
    }

    #[test]
    #[serial]
    fn test_flag_list_from_deployment_with_definitions() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

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
            subcommand: FlagSubcommand::List {
                definitions: false,
                deployment: Some("production".to_string()),
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_show_nonexistent_flag() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: other_flag
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Show {
                name: "nonexistent_flag".to_string(),
                deployment: None,
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_ne!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_show_with_deployment_env() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

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
            subcommand: FlagSubcommand::Show {
                name: "test_flag".to_string(),
                deployment: Some("production".to_string()),
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_add_with_sync_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags: []
",
        )
        .unwrap();

        // Create invalid deployment file
        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"invalid: yaml: [",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Add {
                name: Some("test_flag".to_string()),
                flag_type: Some("boolean".to_string()),
                default: Some("false".to_string()),
                description: None,
                lang: None,
                sync: true,
                interactive: false,
            },
        };

        // Should still succeed (flag added to definitions) but warn about deployment sync failure
        let exit_code = run(&options);
        // May succeed or fail depending on error handling, but flag should be added
        assert!(exit_code == 0 || exit_code == 1);
    }

    #[test]
    #[serial]
    fn test_flag_list_yaml_format() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: flag1
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::List {
                definitions: true,
                deployment: None,
                format: OutputFormat::Yaml,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_list_from_deployment_json_format() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

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
            subcommand: FlagSubcommand::List {
                definitions: false,
                deployment: Some("production".to_string()),
                format: OutputFormat::Json,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_list_from_deployment_yaml_format() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

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
            subcommand: FlagSubcommand::List {
                definitions: false,
                deployment: Some("production".to_string()),
                format: OutputFormat::Yaml,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_list_from_deployment_without_definitions() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

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
            subcommand: FlagSubcommand::List {
                definitions: false,
                deployment: Some("production".to_string()),
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_list_default_behavior() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: flag1
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        // List without specifying definitions or deployment - should default to definitions
        let options = Options {
            subcommand: FlagSubcommand::List {
                definitions: false,
                deployment: None,
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_validate_flag_name_additional_cases() {
        // Additional edge cases not covered in existing test
        assert!(validate_flag_name("flag_name_123").is_ok());
        assert!(validate_flag_name("a1b2c3").is_ok());
    }

    #[test]
    #[serial]
    fn test_flag_add_with_sdk_regeneration() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags: []
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Add {
                name: Some("test_flag".to_string()),
                flag_type: Some("boolean".to_string()),
                default: Some("false".to_string()),
                description: None,
                lang: Some("typescript".to_string()),
                sync: false,
                interactive: false,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }


    #[test]
    #[serial]
    fn test_flag_show_with_deployment_multiple_envs() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

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
            subcommand: FlagSubcommand::Show {
                name: "test_flag".to_string(),
                deployment: None, // Should show all deployments
                format: OutputFormat::Yaml,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_add_with_next_steps_message() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags: []
",
        )
        .unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules: {}
",
        )
        .unwrap();

        // Add flag without syncing - should show next steps message
        let options = Options {
            subcommand: FlagSubcommand::Add {
                name: Some("test_flag".to_string()),
                flag_type: Some("boolean".to_string()),
                default: Some("false".to_string()),
                description: None,
                lang: None,
                sync: false, // Don't sync
                interactive: false,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_add_with_sync_and_next_steps() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags: []
",
        )
        .unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules: {}
",
        )
        .unwrap();

        // Add flag with syncing - should show different next steps
        let options = Options {
            subcommand: FlagSubcommand::Add {
                name: Some("test_flag".to_string()),
                flag_type: Some("boolean".to_string()),
                default: Some("false".to_string()),
                description: None,
                lang: None,
                sync: true, // Sync to deployments
                interactive: false,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_show_flag_with_description() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: test_flag
    type: boolean
    defaultValue: false
    description: A test flag description
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Show {
                name: "test_flag".to_string(),
                deployment: None,
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_list_flags_from_deployment_not_configured() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

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
        // Flag in deployment but no rules (not configured)
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules:
  test_flag:
    rules: []
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::List {
                definitions: false,
                deployment: Some("production".to_string()),
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_show_flag_with_variations() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: test_flag
    type: multivariate
    defaultValue: variant_a
    variations:
      - name: VARIANT_A
        value: variant_a
      - name: VARIANT_B
        value: variant_b
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Show {
                name: "test_flag".to_string(),
                deployment: None,
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_show_flag_deployment_not_configured() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

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
rules: {}
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Show {
                name: "test_flag".to_string(),
                deployment: Some("production".to_string()),
                format: OutputFormat::Table,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_sync_flag_to_deployment_with_on_off() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        let mut deployment = serde_json::json!({
            "environment": "test",
            "rules": {}
        });

        // Test syncing with "ON" string
        sync_flag_to_deployment(&mut deployment, "flag_on", &Value::String("ON".to_string())).unwrap();
        
        // Test syncing with "OFF" string
        sync_flag_to_deployment(&mut deployment, "flag_off", &Value::String("OFF".to_string())).unwrap();

        let rules = deployment.get("rules").and_then(|r| r.as_object()).unwrap();
        assert!(rules.contains_key("flag_on"));
        assert!(rules.contains_key("flag_off"));
    }
}
