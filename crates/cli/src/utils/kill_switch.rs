//! Kill switch file helpers (v2 local runtime artifacts).
//!
//! Environment resolution (`resolve_kill_switch_env`): explicit `--env` wins; else
//! `defaultEnv` from `.controlpath/config.yaml` when that env exists in the catalog (or
//! when the catalog defines no environments); else the first name in top-level
//! `environments`; else `production`. A stale `defaultEnv` that is not in the catalog
//! is rejected with an error.

use crate::error::{CliError, CliResult};
use crate::utils::atomic_write::atomic_write_string;
use crate::utils::catalog;
use crate::utils::unified_config;
use controlpath_compiler::catalog::FlagKind;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const KILL_SWITCH_VERSION: &str = "2.0";

/// Default path for a local kill switch file.
pub fn kill_switch_path(env: &str) -> PathBuf {
    PathBuf::from(format!(".controlpath/{env}.kill-switches.json"))
}

fn validate_env_in_catalog(env: &str) -> CliResult<()> {
    let unified = unified_config::read_unified_config()?;
    let envs = unified_config::get_environments(&unified);
    if envs.is_empty() {
        return Ok(());
    }
    if !envs.iter().any(|e| e == env) {
        return Err(CliError::Message(format!(
            "Environment '{env}' not found in control-path.yaml. Available: {}",
            envs.join(", ")
        )));
    }
    Ok(())
}

/// Resolve environment for commands that infer env from config defaults.
pub fn resolve_kill_switch_env(explicit: Option<&str>) -> CliResult<String> {
    if let Some(env) = explicit {
        validate_env_in_catalog(env)?;
        return Ok(env.to_string());
    }

    let unified = unified_config::read_unified_config()?;
    let envs = unified_config::get_environments(&unified);

    if let Ok(content) = fs::read_to_string(".controlpath/config.yaml") {
        if let Ok(cfg) = serde_yaml::from_str::<Value>(&content) {
            if let Some(default) = cfg.get("defaultEnv").and_then(|v| v.as_str()) {
                if envs.is_empty() {
                    return Ok(default.to_string());
                }
                if envs.iter().any(|e| e == default) {
                    return Ok(default.to_string());
                }
                return Err(CliError::Message(format!(
                    "defaultEnv '{default}' in .controlpath/config.yaml is not defined in control-path.yaml. Available: {}",
                    envs.join(", ")
                )));
            }
        }
    }

    if let Some(first) = envs.first() {
        return Ok(first.clone());
    }

    Ok("production".to_string())
}

/// Resolve environment for mutating kill-switch operations.
///
/// Incident controls require explicit targeting to avoid accidental writes.
pub fn require_kill_switch_env(explicit: Option<&str>) -> CliResult<String> {
    let env = explicit.ok_or_else(|| {
        CliError::Message("Missing required --env for kill-switch command.".to_string())
    })?;
    validate_env_in_catalog(env)?;
    Ok(env.to_string())
}

pub fn read_kill_switch_file(path: &Path) -> CliResult<Value> {
    if !path.exists() {
        return Ok(json!({
            "version": KILL_SWITCH_VERSION,
            "flags": {}
        }));
    }

    let content = fs::read_to_string(path)
        .map_err(|e| CliError::Message(format!("Failed to read {}: {e}", path.display())))?;
    serde_json::from_str(&content)
        .map_err(|e| CliError::Message(format!("Failed to parse {}: {e}", path.display())))
}

pub fn write_kill_switch_file(path: &Path, data: &Value) -> CliResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            CliError::Message(format!(
                "Failed to create directory {}: {e}",
                parent.display()
            ))
        })?;
    }

    let mut value = data.clone();
    if value.get("version").is_none() {
        value["version"] = json!(KILL_SWITCH_VERSION);
    }
    if value.get("flags").is_none() {
        value["flags"] = json!({});
    }

    let serialized = serde_json::to_string_pretty(&value)
        .map_err(|e| CliError::Message(format!("Failed to serialize kill switch file: {e}")))?;
    atomic_write_string(path, &format!("{serialized}\n"))
        .map_err(|e| CliError::Message(format!("Failed to write {}: {e}", path.display())))
}

fn parse_bool_value(value: &str) -> CliResult<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "on" | "1" | "yes" => Ok(true),
        "false" | "off" | "0" | "no" => Ok(false),
        _ => Err(CliError::Message(format!(
            "Invalid boolean kill switch value: '{value}'. Use true/false or ON/OFF."
        ))),
    }
}

pub fn set_kill_switch_flag(path: &Path, flag: &str, value: &str) -> CliResult<()> {
    let base_dir = std::env::current_dir()
        .map_err(|e| CliError::Message(format!("Failed to resolve working directory: {e}")))?;
    let sdk = catalog::load_for_explain(&base_dir)?.sdk;
    let Some(flag_meta) = sdk.flags.iter().find(|f| f.qualified_name == flag) else {
        return Err(CliError::Message(format!(
            "Flag '{flag}' not found in control-path.yaml catalog"
        )));
    };
    if flag_meta.kind != FlagKind::KillSwitch {
        return Err(CliError::Message(format!(
            "Flag '{flag}' is kind '{:?}' and cannot be used as a kill switch. Use kind 'kill_switch'.",
            flag_meta.kind
        )));
    };

    let mut file = read_kill_switch_file(path)?;
    let flags = file
        .get_mut("flags")
        .and_then(|f| f.as_object_mut())
        .ok_or_else(|| CliError::Message("Invalid kill switch file: missing flags".to_string()))?;
    flags.insert(flag.to_string(), json!(parse_bool_value(value)?));
    write_kill_switch_file(path, &file)?;
    Ok(())
}

pub fn clear_kill_switch_flag(path: &Path, flag: &str) -> CliResult<()> {
    let mut file = read_kill_switch_file(path)?;
    let flags = file
        .get_mut("flags")
        .and_then(|f| f.as_object_mut())
        .ok_or_else(|| CliError::Message("Invalid kill switch file: missing flags".to_string()))?;
    if flags.remove(flag).is_none() {
        return Err(CliError::Message(format!(
            "Kill switch for flag '{flag}' not found in {}",
            path.display()
        )));
    }
    write_kill_switch_file(path, &file)?;
    Ok(())
}

pub fn list_kill_switches(path: &Path) -> CliResult<Map<String, Value>> {
    let file = read_kill_switch_file(path)?;
    Ok(file
        .get("flags")
        .and_then(|f| f.as_object())
        .cloned()
        .unwrap_or_default())
}
