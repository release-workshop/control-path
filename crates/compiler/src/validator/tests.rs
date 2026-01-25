/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;

    #[test]
    fn test_validate_valid_definitions() {
        let validator = Validator::new();
        let valid_data = json!({
            "flags": [
                {
                    "name": "new_dashboard",
                    "type": "boolean",
                    "default": false,
                    "defaultValue": false,
                    "description": "New dashboard UI feature"
                }
            ]
        });

        let result = validator.validate_definitions("test.yaml", &valid_data);
        assert!(result.valid, "Valid definitions should pass validation");
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_validate_definitions_missing_required_fields() {
        let validator = Validator::new();
        let invalid_data = json!({
            "flags": [
                {
                    "name": "new_dashboard"
                    // missing type and defaultValue
                }
            ]
        });

        let result = validator.validate_definitions("test.yaml", &invalid_data);
        assert!(!result.valid, "Invalid definitions should fail validation");
        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| e.message.contains("required")));
    }

    #[test]
    fn test_validate_duplicate_flag_names() {
        let validator = Validator::new();
        let invalid_data = json!({
            "flags": [
                {
                    "name": "duplicate_flag",
                    "type": "boolean",
                    "defaultValue": false
                },
                {
                    "name": "duplicate_flag",
                    "type": "boolean",
                    "defaultValue": true
                }
            ]
        });

        let result = validator.validate_definitions("test.yaml", &invalid_data);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("Duplicate flag name")));
    }

    #[test]
    fn test_validate_multivariate_flag_without_variations() {
        let validator = Validator::new();
        let invalid_data = json!({
            "flags": [
                {
                    "name": "multivariate_flag",
                    "type": "multivariate",
                    "defaultValue": "variation_a"
                    // missing variations
                }
            ]
        });

        let result = validator.validate_definitions("test.yaml", &invalid_data);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("variations")));
    }

    #[test]
    fn test_validate_multivariate_flag_with_duplicate_variation_names() {
        let validator = Validator::new();
        let invalid_data = json!({
            "flags": [
                {
                    "name": "multivariate_flag",
                    "type": "multivariate",
                    "defaultValue": "variation_a",
                    "variations": [
                        { "name": "VARIATION_A", "value": "a" },
                        { "name": "VARIATION_A", "value": "b" } // duplicate
                    ]
                }
            ]
        });

        let result = validator.validate_definitions("test.yaml", &invalid_data);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("Duplicate variation name")));
    }

    #[test]
    fn test_validate_valid_deployment() {
        let validator = Validator::new();
        let valid_data = json!({
            "environment": "production",
            "rules": {
                "new_dashboard": {
                    "rules": [
                        {
                            "serve": true
                        }
                    ]
                }
            }
        });

        let result = validator.validate_deployment("test.yaml", &valid_data);
        assert!(result.valid, "Valid deployment should pass validation");
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_validate_deployment_rule_without_serve_variations_or_rollout() {
        let validator = Validator::new();
        let invalid_data = json!({
            "environment": "production",
            "rules": {
                "new_dashboard": {
                    "rules": [
                        {
                            // missing serve, variations, and rollout
                        }
                    ]
                }
            }
        });

        let result = validator.validate_deployment("test.yaml", &invalid_data);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("serve")
            || e.message.contains("variations")
            || e.message.contains("rollout")));
    }

    #[test]
    fn test_validate_variation_weights_exceed_100() {
        let validator = Validator::new();
        let invalid_data = json!({
            "environment": "production",
            "rules": {
                "multivariate_flag": {
                    "rules": [
                        {
                            "variations": [
                                { "name": "VARIATION_A", "weight": 60 },
                                { "name": "VARIATION_B", "weight": 50 } // total: 110%
                            ]
                        }
                    ]
                }
            }
        });

        let result = validator.validate_deployment("test.yaml", &invalid_data);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("exceed") && e.message.contains('%')));
    }

    #[test]
    fn test_validate_rollout_percentage_out_of_range() {
        let validator = Validator::new();
        let invalid_data = json!({
            "environment": "production",
            "rules": {
                "new_dashboard": {
                    "rules": [
                        {
                            "rollout": {
                                "percentage": 150 // out of range
                            }
                        }
                    ]
                }
            }
        });

        let result = validator.validate_deployment("test.yaml", &invalid_data);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("percentage") && e.message.contains("between")));
    }

    #[test]
    fn test_validator_with_schemas() {
        let custom_definitions_schema = json!({
            "type": "object",
            "properties": {
                "flags": {
                    "type": "array"
                }
            }
        });
        let custom_deployment_schema = json!({
            "type": "object",
            "properties": {
                "environment": {
                    "type": "string"
                }
            }
        });
        let custom_unified_schema = json!({
            "type": "object",
            "properties": {
                "flags": {
                    "type": "array"
                }
            }
        });

        let validator = Validator::with_schemas(
            custom_definitions_schema,
            custom_deployment_schema,
            custom_unified_schema,
        );
        let data = json!({"flags": []});
        let result = validator.validate_definitions("test.yaml", &data);
        assert!(result.valid);
    }

    #[test]
    fn test_validator_default() {
        let validator = Validator::default();
        let valid_data = json!({
            "flags": [
                {
                    "name": "test_flag",
                    "type": "boolean",
                    "default": false,
                    "defaultValue": false
                }
            ]
        });
        let result = validator.validate_definitions("test.yaml", &valid_data);
        assert!(result.valid);
    }

    #[test]
    fn test_format_errors_empty() {
        let validator = Validator::new();
        let errors = vec![];
        let formatted = validator.format_errors(&errors);
        assert_eq!(formatted, "");
    }

    #[test]
    fn test_format_errors_with_all_fields() {
        use super::super::error::ValidationError;
        let validator = Validator::new();
        let errors = vec![ValidationError {
            file: "test.yaml".to_string(),
            line: Some(5),
            column: Some(10),
            path: Some("flags[0].name".to_string()),
            message: "Invalid flag name".to_string(),
            suggestion: Some("Use lowercase letters only".to_string()),
        }];
        let formatted = validator.format_errors(&errors);
        assert!(formatted.contains("✗ Validation failed"));
        assert!(formatted.contains("test.yaml:5:10"));
        assert!(formatted.contains("Invalid flag name"));
        assert!(formatted.contains("Path: flags[0].name"));
        assert!(formatted.contains("Suggestion: Use lowercase letters only"));
    }

    #[test]
    fn test_format_errors_without_line_column() {
        use super::super::error::ValidationError;
        let validator = Validator::new();
        let errors = vec![ValidationError {
            file: "test.yaml".to_string(),
            line: None,
            column: None,
            path: None,
            message: "Invalid file".to_string(),
            suggestion: None,
        }];
        let formatted = validator.format_errors(&errors);
        assert!(formatted.contains("test.yaml"));
        assert!(formatted.contains("Invalid file"));
    }

    #[test]
    fn test_format_errors_with_line_no_column() {
        use super::super::error::ValidationError;
        let validator = Validator::new();
        let errors = vec![ValidationError {
            file: "test.yaml".to_string(),
            line: Some(5),
            column: None,
            path: None,
            message: "Invalid file".to_string(),
            suggestion: None,
        }];
        let formatted = validator.format_errors(&errors);
        assert!(formatted.contains("test.yaml:5"));
    }
}
