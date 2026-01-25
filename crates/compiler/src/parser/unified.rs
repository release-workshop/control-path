/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * Parser for control-path.yaml configuration.
 */

use crate::parser::error::ParseError;
use crate::parser::utils::parse_yaml_or_json;
use serde_json::Value;

/// Parse control-path.yaml configuration from a YAML/JSON string.
///
/// Supports both YAML and JSON formats. The input can be YAML or JSON.
///
/// # Arguments
///
/// * `content` - The YAML or JSON content as a string
///
/// # Returns
///
/// Returns the parsed configuration as `serde_json::Value`, or a `ParseError` if parsing fails.
///
/// # Errors
///
/// Returns `ParseError` if:
/// - The content is invalid YAML/JSON
/// - The content is not an object
/// - The "flags" field is missing or not an array
pub fn parse_unified_config(content: &str) -> Result<Value, ParseError> {
    parse_unified_config_from_string(content, None)
}

/// Parse configuration from a string with optional file path for error messages.
///
/// Supports both YAML and JSON formats.
///
/// # Arguments
///
/// * `content` - The YAML or JSON content as a string
/// * `file_path` - Optional file path (for error messages and format detection)
///
/// # Returns
///
/// Returns the parsed configuration as `serde_json::Value`, or a `ParseError` if parsing fails.
///
/// # Errors
///
/// Returns `ParseError` if:
/// - The content is invalid YAML/JSON
/// - The content is not an object
/// - The "flags" field is missing or not an array
///
/// # Panics
///
/// Panics if the parsed data is not an object (this should not happen after validation).
pub fn parse_unified_config_from_string(
    content: &str,
    file_path: Option<&str>,
) -> Result<Value, ParseError> {
    let parsed_data = parse_yaml_or_json(content, file_path)?;

    // Check if it's an object (not array or primitive)
    if !parsed_data.is_object() {
        return Err(ParseError::InvalidFieldType(
            "Invalid config: expected an object".to_string(),
        ));
    }

    let config_obj = parsed_data.as_object().unwrap();

    // Check for required "flags" field
    if !config_obj.contains_key("flags") {
        return Err(ParseError::MissingField(
            "Invalid config: missing required field \"flags\"".to_string(),
        ));
    }

    // Check that "flags" is an array
    let flags_value = &config_obj["flags"];
    if !flags_value.is_array() {
        return Err(ParseError::InvalidFieldType(
            "Invalid config: \"flags\" must be an array".to_string(),
        ));
    }

    // Return the parsed object (full validation should be done by schema validator)
    Ok(parsed_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_yaml_unified_config() {
        let yaml = r#"
mode: local
flags:
  - name: test_flag
    type: boolean
    default: false
"#;
        let result = parse_unified_config(yaml);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value["flags"].is_array());
        assert_eq!(value["flags"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_parse_valid_json_unified_config() {
        let json = r#"{"flags": [{"name": "test_flag", "type": "boolean", "default": false}]}"#;
        let result = parse_unified_config(json);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value["flags"].is_array());
    }

    #[test]
    fn test_parse_unified_config_missing_flags() {
        let yaml = r#"
mode: local
"#;
        let result = parse_unified_config(yaml);
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::MissingField(_) => {}
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_parse_unified_config_flags_not_array() {
        let yaml = r#"
flags: not_an_array
"#;
        let result = parse_unified_config(yaml);
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidFieldType(_) => {}
            _ => panic!("Expected InvalidFieldType error"),
        }
    }

    #[test]
    fn test_parse_unified_config_with_environments() {
        let yaml = r#"
flags:
  - name: test_flag
    type: boolean
    default: false
    environments:
      production:
        - serve: true
"#;
        let result = parse_unified_config(yaml);
        assert!(result.is_ok());
        let value = result.unwrap();
        let flag = &value["flags"].as_array().unwrap()[0];
        assert!(flag["environments"].is_object());
    }
}
