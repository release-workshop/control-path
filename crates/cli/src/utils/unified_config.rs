//! Utilities for reading and writing control-path.yaml configuration

use crate::error::{CliError, CliResult};
use crate::utils::atomic_write::atomic_write_string;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const UNIFIED_CONFIG_FILE: &str = "control-path.yaml";

/// Returns true when `flags` is a v2 map (not a v1 array).
pub fn is_v2_flags_format(unified: &Value) -> bool {
    unified
        .get("flags")
        .is_some_and(|flags| flags.is_object() && !flags.is_null())
}

/// Returns true when the catalog is configured for SaaS rule authority.
pub fn is_saas_mode(unified: &Value) -> bool {
    unified.get("mode").and_then(|m| m.as_str()) == Some("saas")
}

/// Get the path to the unified configuration file.
pub fn get_unified_config_path() -> PathBuf {
    PathBuf::from(UNIFIED_CONFIG_FILE)
}

/// Read and parse the unified configuration file.
pub fn read_unified_config() -> CliResult<Value> {
    let path = get_unified_config_path();
    if !path.exists() {
        return Err(CliError::Message(format!(
            "{UNIFIED_CONFIG_FILE} not found. Run 'controlpath setup' to create it."
        )));
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| CliError::Message(format!("Failed to read {}: {e}", path.display())))?;

    serde_yaml::from_str(&content)
        .map_err(|e| CliError::Message(format!("Failed to parse {}: {e}", path.display())))
}

/// Write the unified configuration file.
pub fn write_unified_config(config: &Value) -> CliResult<()> {
    let path = get_unified_config_path();
    let yaml = serde_yaml::to_string(config)
        .map_err(|e| CliError::Message(format!("Failed to serialize config: {e}")))?;
    atomic_write_string(&path, &yaml)
        .map_err(|e| CliError::Message(format!("Failed to write {}: {e}", path.display())))?;
    Ok(())
}

/// Get a sorted list of all environments defined in the unified config.
pub fn get_environments(unified: &Value) -> Vec<String> {
    let mut env_list: Vec<String> = unified
        .get("environments")
        .and_then(|e| e.as_object())
        .map(|envs| envs.keys().cloned().collect())
        .unwrap_or_default();
    env_list.sort();
    env_list
}

/// Returns true if a flag exists in the config.
pub fn flag_exists(unified: &Value, name: &str) -> bool {
    unified
        .get("flags")
        .and_then(|f| f.as_object())
        .is_some_and(|flags| flags.contains_key(name))
}

/// Returns flag lifecycle when present (`active` or `deprecated`).
pub fn flag_lifecycle<'a>(unified: &'a Value, name: &str) -> Option<&'a str> {
    unified
        .get("flags")
        .and_then(|f| f.get(name))
        .and_then(|flag| flag.get("lifecycle"))
        .and_then(|l| l.as_str())
}

pub fn is_flag_deprecated(unified: &Value, name: &str) -> bool {
    flag_lifecycle(unified, name) == Some("deprecated")
}

/// Add a boolean flag to the config (v2 map format).
pub fn add_flag(
    unified: &mut Value,
    flag_name: &str,
    default: bool,
    kind: &str,
    description: Option<&str>,
    sync_envs: &[String],
) -> CliResult<()> {
    ensure_v2_flags_map(unified)?;

    let flags = unified
        .get_mut("flags")
        .and_then(|f| f.as_object_mut())
        .expect("flags map ensured");

    if flags.contains_key(flag_name) {
        return Err(CliError::Message(format!(
            "Flag '{flag_name}' already exists"
        )));
    }

    let mut new_flag = serde_json::json!({
        "default": default,
        "kind": kind,
    });
    if let Some(desc) = description {
        new_flag["description"] = Value::String(desc.to_string());
    }
    flags.insert(flag_name.to_string(), new_flag);

    if !sync_envs.is_empty() {
        let root = unified
            .as_object_mut()
            .ok_or_else(|| CliError::Message("Invalid config root".to_string()))?;
        let envs = root
            .entry("environments")
            .or_insert_with(|| serde_json::json!({}));
        let envs_obj = envs
            .as_object_mut()
            .ok_or_else(|| CliError::Message("Invalid environments block".to_string()))?;

        for env in sync_envs {
            let env_entry = envs_obj
                .entry(env.clone())
                .or_insert_with(|| serde_json::json!({ "rules": {} }));
            if !env_entry
                .get("rules")
                .map(|r| r.is_object())
                .unwrap_or(false)
            {
                env_entry["rules"] = serde_json::json!({});
            }
            let rules = env_entry
                .get_mut("rules")
                .and_then(|r| r.as_object_mut())
                .ok_or_else(|| CliError::Message("Invalid environment rules".to_string()))?;
            rules.insert(
                flag_name.to_string(),
                serde_json::json!([{ "serve": default }]),
            );
        }
    }

    Ok(())
}

/// Mark a flag as deprecated.
pub fn deprecate_flag(unified: &mut Value, flag_name: &str) -> CliResult<()> {
    ensure_v2_flags_map(unified)?;
    let flags = unified
        .get_mut("flags")
        .and_then(|f| f.as_object_mut())
        .ok_or_else(|| CliError::Message("Invalid config: flags must be a map".to_string()))?;

    let flag = flags
        .get_mut(flag_name)
        .ok_or_else(|| CliError::Message(format!("Flag '{flag_name}' not found")))?;
    flag["lifecycle"] = Value::String("deprecated".to_string());
    Ok(())
}

/// Enable a flag in an environment by appending a rule.
pub fn enable_flag_in_environment(
    unified: &mut Value,
    flag_name: &str,
    environment: &str,
    rule_expr: Option<&str>,
    serve: bool,
    force_deprecated: bool,
) -> CliResult<()> {
    if !flag_exists(unified, flag_name) {
        return Err(CliError::Message(format!("Flag '{flag_name}' not found")));
    }

    if is_flag_deprecated(unified, flag_name) && !force_deprecated {
        return Err(CliError::Message(format!(
            "Flag '{flag_name}' is deprecated. Rule changes are blocked unless --force is set."
        )));
    }

    ensure_v2_flags_map(unified)?;

    let root = unified
        .as_object_mut()
        .ok_or_else(|| CliError::Message("Invalid config root".to_string()))?;
    let envs = root
        .entry("environments")
        .or_insert_with(|| serde_json::json!({}));
    let envs_obj = envs
        .as_object_mut()
        .ok_or_else(|| CliError::Message("Invalid environments block".to_string()))?;

    let env_entry = envs_obj
        .entry(environment.to_string())
        .or_insert_with(|| serde_json::json!({ "rules": {} }));

    if !env_entry
        .get("rules")
        .map(|r| r.is_object())
        .unwrap_or(false)
    {
        env_entry["rules"] = serde_json::json!({});
    }
    let rules = env_entry
        .get_mut("rules")
        .and_then(|r| r.as_object_mut())
        .ok_or_else(|| CliError::Message("Invalid environment rules".to_string()))?;

    let mut new_rule = serde_json::json!({ "serve": serve });
    if let Some(expr) = rule_expr {
        new_rule["when"] = Value::String(expr.to_string());
    }

    let flag_rules = rules
        .entry(flag_name.to_string())
        .or_insert_with(|| serde_json::json!([]));
    if let Some(arr) = flag_rules.as_array_mut() {
        arr.push(new_rule);
    }
    Ok(())
}

/// Remove a flag or environment rules for a flag.
pub fn remove_flag(unified: &mut Value, flag_name: &str, env: Option<&str>) -> CliResult<()> {
    ensure_v2_flags_map(unified)?;

    if let Some(target_env) = env {
        if let Some(env_rules) = unified
            .get_mut("environments")
            .and_then(|e| e.get_mut(target_env))
            .and_then(|e| e.get_mut("rules"))
            .and_then(|r| r.as_object_mut())
        {
            env_rules.remove(flag_name);
        }
        return Ok(());
    }

    let removed = unified
        .get_mut("flags")
        .and_then(|f| f.as_object_mut())
        .is_some_and(|flags| flags.remove(flag_name).is_some());

    if !removed {
        return Err(CliError::Message(format!("Flag '{flag_name}' not found.")));
    }

    if let Some(envs) = unified
        .get_mut("environments")
        .and_then(|e| e.as_object_mut())
    {
        for env in envs.values_mut() {
            if let Some(rules) = env.get_mut("rules").and_then(|r| r.as_object_mut()) {
                rules.remove(flag_name);
            }
        }
    }
    Ok(())
}

/// Add a top-level environment with an empty rules map.
pub fn add_environment(unified: &mut Value, name: &str) -> CliResult<()> {
    if get_environments(unified).iter().any(|e| e == name) {
        return Err(CliError::Message(format!(
            "Environment '{name}' already exists."
        )));
    }

    let root = unified
        .as_object_mut()
        .ok_or_else(|| CliError::Message("Invalid config root".to_string()))?;
    let envs = root
        .entry("environments")
        .or_insert_with(|| serde_json::json!({}));
    let envs_obj = envs
        .as_object_mut()
        .ok_or_else(|| CliError::Message("Invalid environments block".to_string()))?;
    envs_obj.insert(name.to_string(), serde_json::json!({ "rules": {} }));
    Ok(())
}

/// Remove a top-level environment block.
pub fn remove_environment(unified: &mut Value, name: &str) -> CliResult<()> {
    let removed = unified
        .get_mut("environments")
        .and_then(|e| e.as_object_mut())
        .and_then(|envs| envs.remove(name));

    if removed.is_none() {
        return Err(CliError::Message(format!(
            "Environment '{name}' not found."
        )));
    }
    Ok(())
}

fn ensure_v2_flags_map(unified: &mut Value) -> CliResult<()> {
    if is_v2_flags_format(unified) {
        return Ok(());
    }

    if unified.get("flags").and_then(|f| f.as_array()).is_some() {
        return Err(CliError::Message(
            "v1 flag array format is not supported for this operation; migrate to v2 map format"
                .to_string(),
        ));
    }

    unified
        .as_object_mut()
        .ok_or_else(|| CliError::Message("Invalid config root".to_string()))?
        .entry("flags")
        .or_insert_with(|| serde_json::json!({}));
    Ok(())
}

pub fn unified_config_exists() -> bool {
    get_unified_config_path().exists()
}

pub fn get_sdk_output_path(unified: &Value) -> Option<String> {
    unified
        .get("sdk")
        .and_then(|sdk| sdk.get("output"))
        .and_then(|output| output.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v2_fixture() -> Value {
        serde_yaml::from_str(
            r"catalog:
  id: test-service
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  production:
    rules:
      my_flag:
        - serve: true
",
        )
        .unwrap()
    }

    #[test]
    fn get_environments_from_v2_top_level() {
        let unified = v2_fixture();
        assert_eq!(get_environments(&unified), vec!["production"]);
    }

    #[test]
    fn add_and_enable_flag_in_v2_config() {
        let mut unified = serde_yaml::from_str(
            r"catalog:
  id: test
mode: local
flags: {}
",
        )
        .unwrap();

        add_flag(
            &mut unified,
            "new_flag",
            false,
            "release",
            None,
            &["staging".to_string()],
        )
        .unwrap();
        enable_flag_in_environment(&mut unified, "new_flag", "staging", None, true, false).unwrap();

        assert!(flag_exists(&unified, "new_flag"));
        let rules = unified
            .get("environments")
            .and_then(|e| e.get("staging"))
            .and_then(|e| e.get("rules"))
            .and_then(|r| r.get("new_flag"))
            .and_then(|r| r.as_array())
            .unwrap();
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn deprecated_flag_blocks_rule_changes_without_force() {
        let mut unified = v2_fixture();
        deprecate_flag(&mut unified, "my_flag").unwrap();
        let err =
            enable_flag_in_environment(&mut unified, "my_flag", "production", None, true, false)
                .unwrap_err();
        assert!(err.to_string().contains("deprecated"));
    }
}
