/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 *
 * Parser for deployments from YAML/JSON strings.
 */

use crate::parser::error::ParseError;
use crate::parser::utils::parse_yaml_or_json;
use serde_json::Value;

/// Parse deployment from a YAML/JSON string.
///
/// Supports both YAML and JSON formats. The input can be YAML or JSON.
///
/// # Arguments
///
/// * `content` - The YAML or JSON content as a string
///
/// # Returns
///
/// Returns the parsed deployment as `serde_json::Value`, or a `ParseError` if parsing fails.
///
/// # Errors
///
/// Returns `ParseError` if:
/// - The content is invalid YAML/JSON
/// - The content is not an object
/// - The "environment" field is missing or not a string
/// - The "rules" field is missing or not an object
pub fn parse_deployment(content: &str) -> Result<Value, ParseError> {
    parse_deployment_from_string(content, None)
}

/// Parse deployment from a string with optional file path for error messages.
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
/// Returns the parsed deployment as `serde_json::Value`, or a `ParseError` if parsing fails.
///
/// # Errors
///
/// Returns `ParseError` if:
/// - The content is invalid YAML/JSON
/// - The content is not an object
/// - The "environment" field is missing or not a string
/// - The "rules" field is missing or not an object
///
/// # Panics
///
/// Panics if the parsed data is not an object (this should not happen after validation).
pub fn parse_deployment_from_string(
    content: &str,
    file_path: Option<&str>,
) -> Result<Value, ParseError> {
    let parsed_data = parse_yaml_or_json(content, file_path)?;

    // Check if it's an object (not array or primitive)
    if !parsed_data.is_object() {
        return Err(ParseError::InvalidFieldType(
            "Invalid deployment: expected an object".to_string(),
        ));
    }

    let deployment_obj = parsed_data.as_object().unwrap();

    // Check for required "environment" field
    if !deployment_obj.contains_key("environment") {
        return Err(ParseError::MissingField(
            "Invalid deployment: missing required field \"environment\"".to_string(),
        ));
    }

    // Check that "environment" is a string
    let environment_value = &deployment_obj["environment"];
    if !environment_value.is_string() {
        return Err(ParseError::InvalidFieldType(
            "Invalid deployment: \"environment\" must be a string".to_string(),
        ));
    }

    // Check for required "rules" field
    if !deployment_obj.contains_key("rules") {
        return Err(ParseError::MissingField(
            "Invalid deployment: missing required field \"rules\"".to_string(),
        ));
    }

    // Check that "rules" is an object (not array or primitive)
    let rules_value = &deployment_obj["rules"];
    if !rules_value.is_object() {
        return Err(ParseError::InvalidFieldType(
            "Invalid deployment: \"rules\" must be an object".to_string(),
        ));
    }

    // Return the parsed object (full validation should be done by schema validator)
    Ok(parsed_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_yaml_deployment() {
        let yaml = r#"
environment: production
rules:
  new_dashboard:
    rules:
      - name: "Enable for admins"
        when: "user.role == 'admin'"
        serve: ON
      - name: "10% rollout"
        when: "true"
        rollout:
          variation: ON
          percentage: 10
"#;

        let result = parse_deployment_from_string(yaml, Some("test.yaml")).unwrap();

        assert!(result.is_object());
        assert_eq!(result["environment"], "production");
        assert!(result["rules"].is_object());
        assert!(result["rules"]["new_dashboard"].is_object());
        assert!(result["rules"]["new_dashboard"]["rules"].is_array());
        assert_eq!(
            result["rules"]["new_dashboard"]["rules"][0]["name"],
            "Enable for admins"
        );
        assert_eq!(result["rules"]["new_dashboard"]["rules"][0]["serve"], "ON");
        assert_eq!(
            result["rules"]["new_dashboard"]["rules"][1]["rollout"]["percentage"],
            10
        );
    }

    #[test]
    fn test_parse_valid_json_deployment() {
        let json = r#"{
  "environment": "staging",
  "rules": {
    "test_flag": {}
  }
}"#;

        let result = parse_deployment_from_string(json, Some("test.json")).unwrap();

        assert!(result.is_object());
        assert_eq!(result["environment"], "staging");
        assert!(result["rules"].is_object());
        assert!(result["rules"]["test_flag"].is_object());
    }

    #[test]
    fn test_parse_deployment_with_variations() {
        let yaml = r#"
environment: production
rules:
  theme:
    rules:
      - name: "Theme distribution"
        variations:
          - variation: LIGHT
            weight: 50
          - variation: DARK
            weight: 30
          - variation: AUTO
            weight: 20
"#;

        let result = parse_deployment_from_string(yaml, Some("test.yaml")).unwrap();

        assert!(result.is_object());
        assert!(result["rules"]["theme"]["rules"][0]["variations"].is_array());
        assert_eq!(
            result["rules"]["theme"]["rules"][0]["variations"][0]["variation"],
            "LIGHT"
        );
        assert_eq!(
            result["rules"]["theme"]["rules"][0]["variations"][0]["weight"],
            50
        );
    }

    #[test]
    fn test_parse_deployment_with_segments() {
        let yaml = r#"
environment: production
rules:
  test_flag:
segments:
  beta_users:
    when: "user.role == 'beta'"
  premium_customers:
    when: "user.subscription_tier == 'premium'"
"#;

        let result = parse_deployment_from_string(yaml, Some("test.yaml")).unwrap();

        assert!(result.is_object());
        assert!(result["segments"].is_object());
        assert_eq!(
            result["segments"]["beta_users"]["when"],
            "user.role == 'beta'"
        );
        assert_eq!(
            result["segments"]["premium_customers"]["when"],
            "user.subscription_tier == 'premium'"
        );
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let invalid_yaml = r#"
environment: production
rules:
  test: [unclosed
"#;

        let result = parse_deployment_from_string(invalid_yaml, Some("test.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_json() {
        let invalid_json = r#"{"environment": "production""#;

        let result = parse_deployment_from_string(invalid_json, Some("test.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_environment_field() {
        let yaml = r#"
rules:
  test_flag:
"#;

        let result = parse_deployment_from_string(yaml, Some("test.yaml"));
        assert!(result.is_err());
        match result {
            Err(ParseError::MissingField(msg)) => {
                assert!(msg.contains("environment"));
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_parse_missing_rules_field() {
        let yaml = r#"
environment: production
"#;

        let result = parse_deployment_from_string(yaml, Some("test.yaml"));
        assert!(result.is_err());
        match result {
            Err(ParseError::MissingField(msg)) => {
                assert!(msg.contains("rules"));
            }
            _ => panic!("Expected MissingField error"),
        }
    }

    #[test]
    fn test_parse_environment_not_string() {
        let yaml = r#"
environment: 123
rules:
  test_flag:
"#;

        let result = parse_deployment_from_string(yaml, Some("test.yaml"));
        assert!(result.is_err());
        match result {
            Err(ParseError::InvalidFieldType(msg)) => {
                assert!(msg.contains("environment"));
                assert!(msg.contains("string"));
            }
            _ => panic!("Expected InvalidFieldType error"),
        }
    }

    #[test]
    fn test_parse_rules_not_object() {
        let yaml = r#"
environment: production
rules: not_an_object
"#;

        let result = parse_deployment_from_string(yaml, Some("test.yaml"));
        assert!(result.is_err());
        match result {
            Err(ParseError::InvalidFieldType(msg)) => {
                assert!(msg.contains("rules"));
                assert!(msg.contains("object"));
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

        let result = parse_deployment_from_string(yaml, Some("test.yaml"));
        assert!(result.is_err());
        match result {
            Err(ParseError::InvalidFieldType(msg)) => {
                assert!(msg.contains("object"));
            }
            _ => panic!("Expected InvalidFieldType error"),
        }
    }
}
