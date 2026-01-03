/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * Parser for flag definitions from YAML/JSON strings.
 */

use crate::parser::error::ParseError;
use crate::parser::utils::parse_yaml_or_json;
use serde_json::Value;

/// Parse flag definitions from a YAML/JSON string.
///
/// Supports both YAML and JSON formats. The input can be YAML or JSON.
///
/// # Arguments
///
/// * `content` - The YAML or JSON content as a string
///
/// # Returns
///
/// Returns the parsed flag definitions as `serde_json::Value`, or a `ParseError` if parsing fails.
///
/// # Errors
///
/// Returns `ParseError` if:
/// - The content is invalid YAML/JSON
/// - The content is not an object
/// - The "flags" field is missing
/// - The "flags" field is not an array
pub fn parse_definitions(content: &str) -> Result<Value, ParseError> {
    parse_definitions_from_string(content, None)
}

/// Parse flag definitions from a string with optional file path for error messages.
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
/// Returns the parsed flag definitions as `serde_json::Value`, or a `ParseError` if parsing fails.
///
/// # Errors
///
/// Returns `ParseError` if:
/// - The content is invalid YAML/JSON
/// - The content is not an object
/// - The "flags" field is missing
/// - The "flags" field is not an array
///
/// # Panics
///
/// Panics if the parsed data is not an object (this should not happen after validation).
pub fn parse_definitions_from_string(
    content: &str,
    file_path: Option<&str>,
) -> Result<Value, ParseError> {
    let parsed_data = parse_yaml_or_json(content, file_path)?;

    // Check if it's an object (not array or primitive)
    if !parsed_data.is_object() {
        return Err(ParseError::InvalidFieldType(
            "Invalid flag definitions: expected an object".to_string(),
        ));
    }

    let definitions_obj = parsed_data.as_object().unwrap();

    // Check for required "flags" field
    if !definitions_obj.contains_key("flags") {
        return Err(ParseError::MissingField(
            "Invalid flag definitions: missing required field \"flags\"".to_string(),
        ));
    }

    // Check that "flags" is an array
    let flags_value = &definitions_obj["flags"];
    if !flags_value.is_array() {
        return Err(ParseError::InvalidFieldType(
            "Invalid flag definitions: \"flags\" must be an array".to_string(),
        ));
    }

    // Return the parsed object (full validation should be done by schema validator)
    Ok(parsed_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_yaml_definitions() {
        let yaml = r#"
flags:
  - name: new_dashboard
    type: boolean
    defaultValue: OFF
    description: "New dashboard UI feature"
  
  - name: enable_analytics
    type: boolean
    defaultValue: false
    description: "Enable analytics tracking"
"#;

        let result = parse_definitions_from_string(yaml, Some("test.yaml")).unwrap();

        assert!(result.is_object());
        assert!(result["flags"].is_array());
        assert_eq!(result["flags"].as_array().unwrap().len(), 2);
        assert_eq!(result["flags"][0]["name"], "new_dashboard");
        assert_eq!(result["flags"][0]["type"], "boolean");
        assert_eq!(result["flags"][0]["defaultValue"], "OFF");
        assert_eq!(result["flags"][1]["name"], "enable_analytics");
        assert_eq!(result["flags"][1]["defaultValue"], false);
    }

    #[test]
    fn test_parse_valid_json_definitions() {
        let json = r#"{
  "flags": [
    {
      "name": "test_flag",
      "type": "boolean",
      "defaultValue": true,
      "description": "Test flag"
    }
  ]
}"#;

        let result = parse_definitions_from_string(json, Some("test.json")).unwrap();

        assert!(result.is_object());
        assert!(result["flags"].is_array());
        assert_eq!(result["flags"].as_array().unwrap().len(), 1);
        assert_eq!(result["flags"][0]["name"], "test_flag");
        assert_eq!(result["flags"][0]["type"], "boolean");
        assert_eq!(result["flags"][0]["defaultValue"], true);
    }

    #[test]
    fn test_parse_multivariate_flag_definitions() {
        let yaml = r#"
flags:
  - name: theme
    type: multivariate
    defaultValue: light
    variations:
      - name: LIGHT
        value: light
      - name: DARK
        value: dark
      - name: AUTO
        value: auto
"#;

        let result = parse_definitions_from_string(yaml, Some("test.yaml")).unwrap();

        assert!(result["flags"].is_array());
        assert_eq!(result["flags"][0]["type"], "multivariate");
        assert!(result["flags"][0]["variations"].is_array());
        assert_eq!(
            result["flags"][0]["variations"].as_array().unwrap().len(),
            3
        );
        assert_eq!(result["flags"][0]["variations"][0]["name"], "LIGHT");
        assert_eq!(result["flags"][0]["variations"][0]["value"], "light");
    }

    #[test]
    fn test_parse_definitions_with_context_schema() {
        let yaml = r#"
context:
  user:
    age: number
    department: string
flags:
  - name: test_flag
    type: boolean
    defaultValue: false
"#;

        let result = parse_definitions_from_string(yaml, Some("test.yaml")).unwrap();

        assert!(result.is_object());
        assert!(result["context"].is_object());
        assert!(result["context"]["user"].is_object());
        assert!(result["flags"].is_array());
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let invalid_yaml = r#"
flags:
  - name: test
    type: boolean
    invalid: [unclosed
"#;

        let result = parse_definitions_from_string(invalid_yaml, Some("test.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_json() {
        let invalid_json = r#"{"flags": [{"name": "test""#;

        let result = parse_definitions_from_string(invalid_json, Some("test.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_flags_field() {
        let yaml = r#"
other_field: value
"#;

        let result = parse_definitions_from_string(yaml, Some("test.yaml"));
        assert!(result.is_err());
        match result {
            Err(ParseError::MissingField(msg)) => {
                assert!(msg.contains("flags"));
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_parse_flags_not_array() {
        let yaml = r#"
flags: not_an_array
"#;

        let result = parse_definitions_from_string(yaml, Some("test.yaml"));
        assert!(result.is_err());
        match result {
            Err(ParseError::InvalidFieldType(msg)) => {
                assert!(msg.contains("array"));
            }
            _ => panic!("Expected InvalidFieldType error"),
        }
    }

    #[test]
    fn test_parse_not_an_object() {
        let yaml = r#"
- item1
- item2
"#;

        let result = parse_definitions_from_string(yaml, Some("test.yaml"));
        assert!(result.is_err());
        match result {
            Err(ParseError::InvalidFieldType(msg)) => {
                assert!(msg.contains("object"));
            }
            _ => panic!("Expected InvalidFieldType error"),
        }
    }
}
