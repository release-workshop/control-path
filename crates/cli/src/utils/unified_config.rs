//! Read-only helpers for `control-path.yaml` (legacy `serde_json::Value` access).
//!
//! Catalog **mutations** go through [`super::catalog_store::CatalogStore`].
//! `read_unified_config` remains for read-only callers until issue 03 consolidates loaders:
//! `ci`, `dev`, `validate`, `watch`, `generate-sdk`, `kill_switch`, and `flag` list/show/report.

use crate::error::{CliError, CliResult};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const UNIFIED_CONFIG_FILE: &str = "control-path.yaml";

/// Returns true when the catalog is configured for SaaS rule authority.
pub fn is_saas_mode(unified: &Value) -> bool {
    unified.get("mode").and_then(|m| m.as_str()) == Some("saas")
}

/// Get the path to the unified configuration file.
pub fn get_unified_config_path() -> PathBuf {
    PathBuf::from(UNIFIED_CONFIG_FILE)
}

/// Read and parse the unified configuration file (read-only).
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

    #[test]
    fn get_environments_from_v2_top_level() {
        let unified: Value = serde_yaml::from_str(
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
        .unwrap();
        assert_eq!(get_environments(&unified), vec!["production"]);
    }
}
