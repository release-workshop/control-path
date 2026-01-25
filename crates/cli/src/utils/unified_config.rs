//! Utilities for reading and writing control-path.yaml configuration

use crate::error::{CliError, CliResult};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const UNIFIED_CONFIG_FILE: &str = "control-path.yaml";

/// Get the path to the unified configuration file.
///
/// Returns the path to `control-path.yaml` in the current directory.
pub fn get_unified_config_path() -> PathBuf {
    PathBuf::from(UNIFIED_CONFIG_FILE)
}

/// Read and parse the unified configuration file.
///
/// # Errors
///
/// Returns an error if:
/// - The configuration file doesn't exist
/// - The file can't be read
/// - The file contains invalid YAML
pub fn read_unified_config() -> CliResult<Value> {
    let path = get_unified_config_path();
    if !path.exists() {
        return Err(CliError::Message(format!(
            "{} not found. Run 'controlpath setup' to create it.",
            UNIFIED_CONFIG_FILE
        )));
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| CliError::Message(format!("Failed to read {}: {e}", path.display())))?;

    // Parse as YAML
    serde_yaml::from_str(&content)
        .map_err(|e| CliError::Message(format!("Failed to parse {}: {e}", path.display())))
}

/// Write the unified configuration file.
///
/// Serializes the provided configuration value to YAML and writes it to `control-path.yaml`.
///
/// # Errors
///
/// Returns an error if:
/// - The configuration can't be serialized to YAML
/// - The file can't be written
pub fn write_unified_config(config: &Value) -> CliResult<()> {
    let path = get_unified_config_path();
    let yaml = serde_yaml::to_string(config)
        .map_err(|e| CliError::Message(format!("Failed to serialize config: {e}")))?;
    fs::write(&path, yaml)
        .map_err(|e| CliError::Message(format!("Failed to write {}: {e}", path.display())))?;
    Ok(())
}

/// Extract flag definitions from unified config (without environment rules).
///
/// This transforms the unified format to the legacy definitions format for the compiler.
/// The returned value contains only flag definitions, without environment-specific rules.
///
/// # Errors
///
/// Returns an error if the unified config structure is invalid.
pub fn extract_definitions(unified: &Value) -> CliResult<Value> {
    let mut definitions = serde_json::json!({
        "flags": []
    });

    if let Some(flags) = unified.get("flags").and_then(|f| f.as_array()) {
        let mut def_flags = Vec::new();
        for flag in flags {
            // Clone flag but transform for compiler compatibility
            let mut flag_def = flag.clone();
            if let Some(obj) = flag_def.as_object_mut() {
                // Remove environments field (not part of definitions)
                obj.remove("environments");

                // Keep "default" and also add "defaultValue" for compiler compatibility
                if let Some(default_val) = obj.get("default").cloned() {
                    if !obj.contains_key("defaultValue") {
                        obj.insert("defaultValue".to_string(), default_val);
                    }
                }
            }
            def_flags.push(flag_def);
        }
        definitions["flags"] = serde_json::json!(def_flags);
    }

    Ok(definitions)
}

/// Extract deployment structure for a specific environment from unified config.
///
/// This transforms the unified format to the legacy deployment format for the compiler.
/// The returned value contains only the rules for the specified environment.
///
/// # Errors
///
/// Returns an error if the unified config structure is invalid or the environment doesn't exist.
pub fn extract_deployment(unified: &Value, environment: &str) -> CliResult<Value> {
    let mut deployment = serde_json::json!({
        "environment": environment,
        "rules": {}
    });

    if let Some(flags) = unified.get("flags").and_then(|f| f.as_array()) {
        let mut rules = serde_json::Map::new();

        for flag in flags {
            if let Some(flag_name) = flag.get("name").and_then(|n| n.as_str()) {
                // Get environment rules for this flag
                if let Some(env_rules) = flag
                    .get("environments")
                    .and_then(|e| e.as_object())
                    .and_then(|e| e.get(environment))
                    .and_then(|r| r.as_array())
                {
                    // Only add to rules if there are rules for this environment
                    if !env_rules.is_empty() {
                        rules.insert(
                            flag_name.to_string(),
                            serde_json::json!({
                                "rules": env_rules
                            }),
                        );
                    }
                }
            }
        }

        deployment["rules"] = serde_json::json!(rules);
    }

    // Extract segments if present in config
    if let Some(segments) = unified.get("segments") {
        deployment["segments"] = segments.clone();
    }

    Ok(deployment)
}

/// Get a sorted list of all environments defined in the unified config.
///
/// Scans all flags in the config and collects unique environment names
/// from their `environments` fields.
pub fn get_environments(unified: &Value) -> Vec<String> {
    let mut environments = std::collections::HashSet::new();

    if let Some(flags) = unified.get("flags").and_then(|f| f.as_array()) {
        for flag in flags {
            if let Some(envs) = flag.get("environments").and_then(|e| e.as_object()) {
                for env_name in envs.keys() {
                    environments.insert(env_name.clone());
                }
            }
        }
    }

    let mut env_list: Vec<String> = environments.into_iter().collect();
    env_list.sort();
    env_list
}

/// Check if the unified configuration file exists.
///
/// Returns `true` if `control-path.yaml` exists in the current directory.
pub fn unified_config_exists() -> bool {
    get_unified_config_path().exists()
}
