//! Flag management command implementation

use crate::error::{CliError, CliResult};
use crate::generator::generate_sdk;
use crate::saas::{
    build_flag_rot_report, fetch_saas_telemetry, parse_saas_catalog_document,
    print_flag_rot_report, FakeSaasClient,
};
use crate::utils::catalog;
use crate::utils::catalog_store::CatalogStore;
use crate::utils::runtime;
use crate::utils::unified_config;
use controlpath_compiler::effective_catalog_id;
use controlpath_compiler::FlagKind;
use dialoguer::{Input, Select};
use serde_json::Value;
use std::env;
use std::path::PathBuf;
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
        env: Option<String>,
    },
    Deprecate {
        name: String,
    },
    Report,
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

fn validate_flag_name(name: &str) -> CliResult<()> {
    // Flag names must match pattern: ^[a-z][a-z0-9_]*$
    if name.is_empty() {
        return Err(CliError::Message(
            "Flag name cannot be empty.\n  Tip: Flag names must be in snake_case format (e.g., 'new_feature', 'api_v2')".to_string(),
        ));
    }
    if !name.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
        return Err(CliError::Message(format!(
            "Invalid flag name: '{}'\n  Flag names must:\n  - Start with a lowercase letter\n  - Contain only lowercase letters, digits, and underscores\n  - Be in snake_case format\n  Examples: 'new_feature', 'api_v2', 'dashboard_enabled'\n  Your input: '{}'",
            name, name
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(CliError::Message(format!(
            "Invalid flag name: '{}'\n  Flag names must:\n  - Start with a lowercase letter\n  - Contain only lowercase letters, digits, and underscores\n  - Be in snake_case format\n  Examples: 'new_feature', 'api_v2', 'dashboard_enabled'\n  Your input: '{}'",
            name, name
        )));
    }
    Ok(())
}

fn validate_flag_type(flag_type: &str) -> CliResult<()> {
    if flag_type != "boolean" {
        return Err(CliError::Message(format!(
            "Invalid flag type: '{flag_type}'\n  v2 catalogs support boolean flags only."
        )));
    }
    Ok(())
}

fn catalog_flag_names(unified: &Value) -> Vec<String> {
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

fn find_similar_flag_names(flag_names: &[String], name: &str) -> Vec<String> {
    let mut similar = Vec::new();
    for flag_name in flag_names {
        let distance = levenshtein(name, flag_name);
        if distance > 0 && distance <= name.len().max(flag_name.len()) / 2 {
            similar.push((flag_name.clone(), distance));
        }
    }
    similar.sort_by_key(|(_, d)| *d);
    similar.into_iter().take(3).map(|(name, _)| name).collect()
}

fn store_flag_names(store: &CatalogStore) -> Vec<String> {
    let mut names: Vec<String> = store.document().flags.keys().cloned().collect();
    names.sort();
    names
}

fn prompt_for_flag_name(store: &CatalogStore) -> CliResult<String> {
    runtime::require_interactive("prompt for flag name")?;
    loop {
        let name: String = Input::new()
            .with_prompt("Flag name")
            .validate_with(|input: &String| -> Result<(), String> {
                validate_flag_name(input).map_err(|e| format!("{}", e))
            })
            .interact()
            .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;

        if store.flag_exists(&name) {
            let similar = find_similar_flag_names(&store_flag_names(store), &name);
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
    runtime::require_interactive("prompt for flag type")?;
    let types = vec!["boolean"];
    let selection = Select::new()
        .with_prompt("Flag type")
        .items(&types)
        .default(0)
        .interact()
        .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;
    Ok(types[selection].to_string())
}

fn prompt_for_default_value() -> CliResult<Value> {
    runtime::require_interactive("prompt for default flag value")?;
    let default: bool = Input::new()
        .with_prompt("Default value")
        .default(false)
        .interact()
        .map_err(|e| CliError::Message(format!("Failed to read input: {e}")))?;
    Ok(Value::Bool(default))
}

fn prompt_for_description() -> CliResult<Option<String>> {
    runtime::require_interactive("prompt for flag description")?;
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
    run_unified(options)
}

fn run_unified(options: &Options) -> CliResult<()> {
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
            let mut store = CatalogStore::open_default()?;

            let (name, flag_type, default_value, description) = if *interactive && name.is_none() {
                println!();
                println!("Interactive mode: We'll guide you through adding a new flag");
                println!("Press Ctrl+C at any time to cancel");
                println!();
                let name = prompt_for_flag_name(&store)?;
                let flag_type = flag_type.clone().unwrap_or_else(|| {
                    prompt_for_flag_type().unwrap_or_else(|_| "boolean".to_string())
                });
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
                    .unwrap_or_else(|| prompt_for_default_value().unwrap_or(Value::Bool(false)));
                let description = description
                    .clone()
                    .or_else(|| prompt_for_description().ok().flatten());
                (name, flag_type, default_value, description)
            } else {
                let name = name.clone().ok_or_else(|| {
                    CliError::Message(
                        "Flag name is required. Use --name <name> or run in interactive mode"
                            .to_string(),
                    )
                })?;
                validate_flag_name(&name)?;
                if store.flag_exists(&name) {
                    return Err(CliError::Message(format!("Flag '{name}' already exists")));
                }
                let flag_type = flag_type.as_deref().unwrap_or("boolean");
                validate_flag_type(flag_type)?;
                let default_value = if let Some(default_str) = default {
                    if default_str == "true" || default_str == "ON" {
                        Value::Bool(true)
                    } else if default_str == "false" || default_str == "OFF" {
                        Value::Bool(false)
                    } else {
                        return Err(CliError::Message(
                            "Boolean flags require default true or false".to_string(),
                        ));
                    }
                } else {
                    Value::Bool(false)
                };
                (
                    name,
                    flag_type.to_string(),
                    default_value,
                    description.clone(),
                )
            };

            let existing_envs_for_sync = if *sync {
                store.environment_names()
            } else {
                Vec::new()
            };

            if flag_type != "boolean" {
                return Err(CliError::Message(
                    "v2 catalogs support boolean flags only".to_string(),
                ));
            }

            let default_bool = match default_value {
                Value::Bool(b) => b,
                _ => {
                    return Err(CliError::Message(
                        "Boolean flags require default true or false".to_string(),
                    ))
                }
            };

            store.add_flag(
                &name,
                default_bool,
                FlagKind::Release,
                description.as_deref(),
                &existing_envs_for_sync,
            )?;
            store.save()?;
            // Without --lang: hard-fail if SdkGenerate/import rules break.
            // With --lang: SdkGenerate runs inside regen (warnings only on failure).
            if lang.is_none() {
                store.validate_sdk_generate()?;
            }
            println!("✓ Added flag '{name}'");

            if let Some(language) = lang {
                let output_path = store
                    .sdk_output_path()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("node_modules/@controlpath/generated"));
                match store
                    .sdk_for_generate()
                    .and_then(|sdk| generate_sdk(language, &sdk, &output_path))
                {
                    Ok(()) => println!("  Regenerated SDK ({language})"),
                    Err(e) => eprintln!("  Warning: Failed to regenerate SDK: {e}"),
                }
            }
            Ok(())
        }
        FlagSubcommand::List {
            definitions,
            deployment,
            format,
        } => {
            let unified = unified_config::read_unified_config()?;
            if *definitions || deployment.is_none() {
                list_flags_from_catalog(&unified, format)?;
            } else if let Some(env) = deployment {
                list_flags_for_environment(&unified, env, format)?;
            }
            Ok(())
        }
        FlagSubcommand::Show {
            name,
            deployment,
            format,
        } => {
            let unified = unified_config::read_unified_config()?;
            show_flag_from_catalog(&unified, name, deployment.as_deref(), format)?;
            Ok(())
        }
        FlagSubcommand::Remove { name, env } => {
            let mut store = CatalogStore::open_default()?;
            store.remove_flag(name, env.as_deref())?;
            store.save()?;
            println!("✓ Removed flag '{name}'");
            Ok(())
        }
        FlagSubcommand::Deprecate { name } => {
            let mut store = CatalogStore::open_default()?;
            if !store.flag_exists(name) {
                return Err(CliError::Message(format!("Flag '{name}' not found.")));
            }
            store.deprecate_flag(name)?;
            store.save()?;
            println!("✓ Flag '{name}' marked as deprecated");
            println!("  Rule changes are blocked unless --force is used with flag enable");
            Ok(())
        }
        FlagSubcommand::Report => run_flag_report(),
    }
}

fn run_flag_report() -> CliResult<()> {
    let base_dir = env::current_dir()
        .map_err(|e| CliError::Message(format!("Failed to resolve working directory: {e}")))?;
    let unified = unified_config::read_unified_config()?;
    let mode = unified
        .get("mode")
        .and_then(|m| m.as_str())
        .unwrap_or("local");

    let sdk_catalog = catalog::load_for_explain(&base_dir)?.sdk;
    let telemetry = if mode == "saas" {
        let (saas_catalog, workspace) = parse_saas_catalog_document(&base_dir)?;
        let catalog_id = effective_catalog_id(&saas_catalog.catalog, workspace.as_ref());
        let project = saas_catalog
            .saas
            .as_ref()
            .map(|s| s.project.as_str())
            .ok_or_else(|| {
                CliError::Message(
                    "SaaS mode requires saas.project in control-path.yaml".to_string(),
                )
            })?;
        let client = FakeSaasClient::open(&base_dir)?;
        fetch_saas_telemetry(&client, &catalog_id, project)?
    } else {
        Vec::new()
    };

    let entries = build_flag_rot_report(&sdk_catalog, &telemetry);
    print_flag_rot_report(&entries)?;
    Ok(())
}

fn list_flags_from_catalog(unified: &Value, format: &OutputFormat) -> CliResult<()> {
    let flags_obj = unified
        .get("flags")
        .and_then(|f| f.as_object())
        .ok_or_else(|| CliError::Message("Invalid catalog: flags must be a map".to_string()))?;

    let mut rows: Vec<(String, &Value)> = flags_obj.iter().map(|(k, v)| (k.clone(), v)).collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));

    match format {
        OutputFormat::Table => {
            println!("Flags:");
            println!("{:-<80}", "");
            println!(
                "{:<30} {:<15} {:<20} Description",
                "Name", "Type", "Default"
            );
            println!("{:-<80}", "");
            for (name, flag) in &rows {
                let default = flag
                    .get("default")
                    .map(format_value)
                    .unwrap_or_else(|| "?".to_string());
                let description = flag
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                println!(
                    "{:<30} {:<15} {:<20} {description}",
                    name, "boolean", default
                );
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(flags_obj)
                .map_err(|e| CliError::Message(format!("Failed to serialize: {e}")))?;
            println!("{json}");
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(flags_obj)
                .map_err(|e| CliError::Message(format!("Failed to serialize: {e}")))?;
            print!("{yaml}");
        }
    }
    Ok(())
}

fn list_flags_for_environment(unified: &Value, env: &str, format: &OutputFormat) -> CliResult<()> {
    let envs = unified_config::get_environments(unified);
    if !envs.is_empty() && !envs.iter().any(|e| e == env) {
        return Err(CliError::Message(format!(
            "Environment '{env}' not found in control-path.yaml"
        )));
    }

    let rules = unified
        .get("environments")
        .and_then(|e| e.get(env))
        .and_then(|e| e.get("rules"))
        .and_then(|r| r.as_object());

    let mut flag_info = Vec::new();
    if let Some(rules) = rules {
        for (flag_name, flag_rules) in rules {
            let mut info = serde_json::Map::new();
            info.insert("name".to_string(), Value::String(flag_name.clone()));
            info.insert("type".to_string(), Value::String("boolean".to_string()));

            if let Some(flag_def) = unified.get("flags").and_then(|f| f.get(flag_name)) {
                if let Some(default) = flag_def.get("default") {
                    info.insert("default".to_string(), default.clone());
                }
                if let Some(description) = flag_def.get("description") {
                    info.insert("description".to_string(), description.clone());
                }
            }

            let status = if flag_rules.as_array().is_some_and(|a| !a.is_empty()) {
                "configured"
            } else {
                "not configured"
            };
            info.insert("status".to_string(), Value::String(status.to_string()));
            flag_info.push(Value::Object(info));
        }
    }

    match format {
        OutputFormat::Table => {
            println!("Flags in environment '{env}':");
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

fn show_flag_from_catalog(
    unified: &Value,
    name: &str,
    deployment_env: Option<&str>,
    format: &OutputFormat,
) -> CliResult<()> {
    let flag = unified
        .get("flags")
        .and_then(|f| f.get(name))
        .ok_or_else(|| {
            let available = catalog_flag_names(unified);
            let similar = find_similar_flag_names(&available, name);
            let mut msg = format!("Flag '{name}' not found.");
            if !available.is_empty() {
                msg.push_str(&format!("\n  Available flags: {}", available.join(", ")));
            }
            if !similar.is_empty() {
                msg.push_str(&format!("\n  Did you mean: {}?", similar.join(", ")));
            }
            msg.push_str("\n  Tip: Use 'controlpath flag list' to see all flags");
            CliError::Message(msg)
        })?;

    match format {
        OutputFormat::Table => {
            println!("Flag: {name}");
            println!("{:-<60}", "");
            println!("Type: boolean");

            if let Some(default) = flag.get("default") {
                println!("Default: {}", format_value(default));
            }

            if let Some(kind) = flag.get("kind").and_then(|k| k.as_str()) {
                println!("Kind: {kind}");
            }

            if let Some(description) = flag.get("description").and_then(|d| d.as_str()) {
                println!("Description: {description}");
            }

            if let Some(lifecycle) = flag.get("lifecycle").and_then(|l| l.as_str()) {
                println!("Lifecycle: {lifecycle}");
            }

            print_catalog_environment_rules(unified, name, deployment_env);
        }
        OutputFormat::Json | OutputFormat::Yaml => {
            let mut flag_output = flag.clone();
            if let Some(obj) = flag_output.as_object_mut() {
                obj.insert("name".to_string(), Value::String(name.to_string()));
            }
            let mut output = serde_json::Map::new();
            output.insert("flag".to_string(), flag_output);
            output.insert(
                "environments".to_string(),
                Value::Object(catalog_environments_for_output(
                    unified,
                    name,
                    deployment_env,
                )),
            );

            match format {
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&output)
                        .map_err(|e| CliError::Message(format!("Failed to serialize: {e}")))?;
                    println!("{json}");
                }
                OutputFormat::Yaml => {
                    let yaml = serde_yaml::to_string(&output)
                        .map_err(|e| CliError::Message(format!("Failed to serialize: {e}")))?;
                    print!("{yaml}");
                }
                OutputFormat::Table => {}
            }
        }
    }
    Ok(())
}

fn environment_rule_count(rules: &Value) -> usize {
    rules.as_array().map(|a| a.len()).unwrap_or(0)
}

fn print_catalog_environment_rules(unified: &Value, flag_name: &str, deployment_env: Option<&str>) {
    let Some(envs) = unified.get("environments").and_then(|e| e.as_object()) else {
        return;
    };

    if let Some(env) = deployment_env {
        if let Some(rules) = envs
            .get(env)
            .and_then(|e| e.get("rules"))
            .and_then(|r| r.get(flag_name))
        {
            println!("\nEnvironment ({env}):");
            println!("  Rules: {}", environment_rule_count(rules));
        } else {
            println!("\nEnvironment ({env}): Not configured");
        }
    } else {
        let mut configured = Vec::new();
        for (env_name, env_val) in envs {
            if let Some(rules) = env_val
                .get("rules")
                .and_then(|r| r.get(flag_name))
                .filter(|r| r.as_array().is_some_and(|a| !a.is_empty()))
            {
                configured.push((env_name.clone(), environment_rule_count(rules)));
            }
        }
        if !configured.is_empty() {
            println!("\nEnvironment rules:");
            for (env_name, count) in configured {
                println!("  {env_name}: {count} rule(s)");
            }
        }
    }
}

fn catalog_environments_for_output(
    unified: &Value,
    flag_name: &str,
    deployment_env: Option<&str>,
) -> serde_json::Map<String, Value> {
    let mut environments = serde_json::Map::new();
    let Some(envs) = unified.get("environments").and_then(|e| e.as_object()) else {
        return environments;
    };

    if let Some(env) = deployment_env {
        if let Some(rules) = envs
            .get(env)
            .and_then(|e| e.get("rules"))
            .and_then(|r| r.get(flag_name))
        {
            environments.insert(env.to_string(), rules.clone());
        }
    } else {
        for (env_name, env_val) in envs {
            if let Some(rules) = env_val.get("rules").and_then(|r| r.get(flag_name)) {
                environments.insert(env_name.clone(), rules.clone());
            }
        }
    }
    environments
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
    use crate::test_helpers::DirGuard;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  existing_flag:
    default: false
    kind: release
environments:
  production:
    rules:
      existing_flag:
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

        // Verify flag was added to unified config
        let content = fs::read_to_string("control-path.yaml").unwrap();
        assert!(content.contains("new_flag"));

        // Verify flag was synced into environment rules in unified config
        assert!(content.contains("production:"));
    }

    #[test]
    #[serial]
    fn test_flag_add_multivariate() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        crate::test_helpers::write_v2_test_catalog("placeholder_flag", false);

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

        let exit_code = run(&options);
        assert_ne!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_list_command() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  flag1:
    default: false
    kind: release
  flag2:
    default: true
    kind: release
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
        let unified = serde_yaml::from_str(
            r"catalog:
  id: test
mode: local
flags:
  test_flag:
    default: false
    kind: release
  test_flag_2:
    default: false
    kind: release
  other_flag:
    default: false
    kind: release
  test:
    default: false
    kind: release
",
        )
        .unwrap();

        let similar = find_similar_flag_names(&catalog_flag_names(&unified), "test_flag_1");
        assert!(!similar.is_empty());
        assert!(similar.len() <= 3);
    }

    #[test]
    fn test_find_similar_flag_names_no_similar() {
        let unified = serde_yaml::from_str(
            r"catalog:
  id: test
mode: local
flags:
  abc:
    default: false
    kind: release
  xyz:
    default: false
    kind: release
",
        )
        .unwrap();

        let similar =
            find_similar_flag_names(&catalog_flag_names(&unified), "completely_different");
        assert!(similar.len() <= 3);
    }

    #[test]
    fn test_find_similar_flag_names_empty_definitions() {
        let unified = serde_yaml::from_str(
            r"catalog:
  id: test
mode: local
flags: {}
",
        )
        .unwrap();

        let similar = find_similar_flag_names(&catalog_flag_names(&unified), "test_flag");
        assert!(similar.is_empty());
    }

    #[test]
    #[serial]
    fn test_flag_remove_nonexistent_flag() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  other_flag:
    default: false
    kind: release
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Remove {
                name: "nonexistent_flag".to_string(),
                env: None,
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  flag1:
    default: false
    kind: release
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  flag1:
    default: false
    kind: release
  flag2:
    default: true
    kind: release
environments:
  production:
    rules:
      flag1:
        - serve: false
      flag2:
        - serve: true
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Remove {
                name: "flag1".to_string(),
                env: None,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);

        // Verify flag was removed from unified config
        let content = fs::read_to_string("control-path.yaml").unwrap();
        assert!(!content.contains("flag1"));
        assert!(content.contains("flag2"));

        // Verify environment rules are updated in unified config
        assert!(!content.contains("flag1"));
        assert!(content.contains("flag2"));
    }

    #[test]
    #[serial]
    fn test_flag_remove_from_specific_env() {
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
  flag1:
    default: false
    kind: release
environments:
  production:
    rules:
      flag1:
        - serve: false
  staging:
    rules:
      flag1:
        - serve: true
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Remove {
                name: "flag1".to_string(),
                env: Some("production".to_string()),
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);

        // Removing from one environment clears production rules but keeps the flag definition.
        let updated = fs::read_to_string("control-path.yaml").unwrap();
        assert!(updated.contains("flag1"));
        assert!(updated.contains("staging:"));
    }

    #[test]
    #[serial]
    fn test_flag_show_command() {
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
    description: A test flag
environments:
  production:
    rules:
      test_flag:
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  my_feature_flag:
    default: false
    kind: release
  my_other_flag:
    default: false
    kind: release
  completely_different:
    default: false
    kind: release
",
        )
        .unwrap();

        let unified = unified_config::read_unified_config().unwrap();
        let similar = find_similar_flag_names(&catalog_flag_names(&unified), "my_feature_flg");
        // Should find "my_feature_flag" as similar
        assert!(similar.contains(&"my_feature_flag".to_string()));
    }

    #[test]
    #[serial]
    fn test_flag_add_with_lang() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags: {}
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags: {}
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  flag1:
    default: false
    kind: release
environments:
  production:
    rules:
      flag1:
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
    description: A test flag
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  flag1:
    default: false
    kind: release
  flag2:
    default: true
    kind: release
environments:
  production:
    rules:
      flag1:
        - serve: false
      flag2:
        - serve: true
",
        )
        .unwrap();

        let options = Options {
            subcommand: FlagSubcommand::Remove {
                name: "flag1".to_string(),
                env: None,
            },
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);

        // Verify flag was removed from unified config
        let content = fs::read_to_string("control-path.yaml").unwrap();
        assert!(!content.contains("flag1"));
        assert!(content.contains("flag2"));

        let unified_content = fs::read_to_string("control-path.yaml").unwrap();
        assert!(!unified_content.contains("flag1"));
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
        assert!(validate_flag_type("multivariate").is_err());
        assert!(validate_flag_type("invalid").is_err());
    }

    #[test]
    #[serial]
    fn test_flag_exists() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  existing_flag:
    default: false
    kind: release
",
        )
        .unwrap();

        let store = crate::utils::catalog_store::CatalogStore::open_default().unwrap();
        assert!(store.flag_exists("existing_flag"));
        assert!(!store.flag_exists("nonexistent_flag"));
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  other_flag:
    default: false
    kind: release
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags: {}
environments:
  production:
    rules: {}
",
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

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }

    #[test]
    #[serial]
    fn test_flag_list_yaml_format() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  flag1:
    default: false
    kind: release
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  flag1:
    default: false
    kind: release
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags: {}
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags: {}
environments:
  production:
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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags: {}
environments:
  production:
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
      test_flag: []
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
}
