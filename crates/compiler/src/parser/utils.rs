/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * Utility functions for parsing YAML and JSON content.
 * Works only with in-memory strings (no file I/O).
 */

use serde_json::Value;
use yaml_rust::{Yaml, YamlLoader};
use crate::parser::error::ParseError;

/// Parse YAML or JSON content from a string.
/// 
/// Automatically detects format based on file extension (if provided) or tries JSON first, then YAML.
/// 
/// # Arguments
/// 
/// * `content` - The YAML or JSON content as a string
/// * `file_path` - Optional file path (for error messages and format detection)
/// 
/// # Returns
/// 
/// Returns the parsed value as `serde_json::Value`, or a `ParseError` if parsing fails.
pub fn parse_yaml_or_json(content: &str, file_path: Option<&str>) -> Result<Value, ParseError> {
    // Try to detect format from file extension
    if let Some(path) = file_path {
        let path_lower = path.to_lowercase();
        if path_lower.ends_with(".json") {
            return parse_json(content).map_err(ParseError::InvalidJson);
        }
        if path_lower.ends_with(".yaml") || path_lower.ends_with(".yml") {
            return parse_yaml(content).map_err(ParseError::InvalidYaml);
        }
    }
    
    // Unknown extension: try JSON first (more strict), then YAML
    match parse_json(content) {
        Ok(value) => Ok(value),
        Err(_) => parse_yaml(content).map_err(ParseError::InvalidYaml),
    }
}

/// Parse JSON content from a string.
fn parse_json(content: &str) -> Result<Value, String> {
    serde_json::from_str(content)
        .map_err(|e| format!("JSON parse error: {}", e))
}

/// Parse YAML content from a string.
/// 
/// Uses `yaml-rust` to parse YAML, then converts to `serde_json::Value`.
fn parse_yaml(content: &str) -> Result<Value, String> {
    let docs = YamlLoader::load_from_str(content)
        .map_err(|e| format!("YAML parse error: {}", e))?;
    
    if docs.is_empty() {
        return Err("YAML document is empty".to_string());
    }
    
    // Convert first document to serde_json::Value
    yaml_to_json_value(&docs[0])
        .ok_or_else(|| "Failed to convert YAML to JSON value".to_string())
}

/// Convert a YAML value to a serde_json::Value.
/// 
/// This recursively converts yaml-rust's Yaml enum to serde_json::Value.
fn yaml_to_json_value(yaml: &Yaml) -> Option<Value> {
    match yaml {
        Yaml::Real(s) => {
            // Parse as f64, then convert to serde_json::Number
            s.parse::<f64>()
                .ok()
                .and_then(|f| {
                    // Try to convert to u64 first (if it's a whole number)
                    if f.fract() == 0.0 && f >= 0.0 && f <= u64::MAX as f64 {
                        Some(Value::Number(serde_json::Number::from(f as u64)))
                    } else {
                        // Use serde_json::to_value to handle f64 properly
                        serde_json::to_value(f).ok()
                    }
                })
                .or_else(|| Some(Value::String(s.clone())))
        }
        Yaml::Integer(i) => {
            // Convert to serde_json::Number
            // Try as i64 first (for negative numbers), then u64 (for positive)
            if *i >= 0 {
                // Positive integer: use u64
                Some(Value::Number(serde_json::Number::from(*i as u64)))
            } else {
                // Negative integer: need to check if it fits in i64
                // Since Yaml::Integer is i64, we can use it directly
                // But serde_json::Number doesn't have from_i64, so we need to convert
                // For negative numbers, we'll use from_f64 which should preserve the value
                serde_json::Number::from_f64(*i as f64)
                    .map(Value::Number)
            }
        }
        Yaml::String(s) => Some(Value::String(s.clone())),
        Yaml::Boolean(b) => Some(Value::Bool(*b)),
        Yaml::Array(arr) => {
            let json_arr: Vec<Value> = arr
                .iter()
                .filter_map(yaml_to_json_value)
                .collect();
            Some(Value::Array(json_arr))
        }
        Yaml::Hash(hash) => {
            let mut map = serde_json::Map::new();
            for (k, v) in hash {
                if let (Some(key), Some(value)) = (
                    yaml_to_string(k),
                    yaml_to_json_value(v)
                ) {
                    map.insert(key, value);
                }
            }
            Some(Value::Object(map))
        }
        Yaml::Null => Some(Value::Null),
        Yaml::BadValue => None,
        Yaml::Alias(_) => None, // Aliases not supported in JSON
    }
}

/// Convert a YAML value to a string key.
/// 
/// Used for hash map keys in YAML.
fn yaml_to_string(yaml: &Yaml) -> Option<String> {
    match yaml {
        Yaml::String(s) => Some(s.clone()),
        Yaml::Integer(i) => Some(i.to_string()),
        Yaml::Real(s) => Some(s.clone()),
        Yaml::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json() {
        let json = r#"{"key": "value", "number": 42}"#;
        let result = parse_json(json).unwrap();
        assert_eq!(result["key"], "value");
        assert_eq!(result["number"], 42);
    }

    #[test]
    fn test_parse_yaml() {
        let yaml = r#"
key: value
number: 42
boolean: true
"#;
        let result = parse_yaml(yaml).unwrap();
        assert_eq!(result["key"], "value");
        assert_eq!(result["number"], 42);
        assert_eq!(result["boolean"], true);
    }

    #[test]
    fn test_parse_yaml_or_json_detects_json() {
        let json = r#"{"test": "value"}"#;
        let result = parse_yaml_or_json(json, Some("test.json")).unwrap();
        assert_eq!(result["test"], "value");
    }

    #[test]
    fn test_parse_yaml_or_json_detects_yaml() {
        let yaml = r#"test: value"#;
        let result = parse_yaml_or_json(yaml, Some("test.yaml")).unwrap();
        assert_eq!(result["test"], "value");
    }

    #[test]
    fn test_parse_yaml_or_json_falls_back_to_json_then_yaml() {
        // Try JSON first
        let json = r#"{"test": "value"}"#;
        let result = parse_yaml_or_json(json, None).unwrap();
        assert_eq!(result["test"], "value");
        
        // Then try YAML
        let yaml = r#"test: value"#;
        let result = parse_yaml_or_json(yaml, None).unwrap();
        assert_eq!(result["test"], "value");
    }

    #[test]
    fn test_parse_invalid_json() {
        let invalid = r#"{"key": unclosed"#;
        assert!(parse_json(invalid).is_err());
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let invalid = r#"key: [unclosed"#;
        assert!(parse_yaml(invalid).is_err());
    }
}

