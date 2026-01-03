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
}
