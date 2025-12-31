//! Compilation tests for the compiler module

use crate::ast::{Expression, Rule};
use crate::compiler::compile;
use serde_json::json;

#[test]
fn test_compile_simple_boolean_flag() {
    let definitions = json!({
        "flags": [
            {
                "name": "my_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "my_flag": {
                "rules": [
                    {
                        "serve": true
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    assert_eq!(artifact.version, "1.0");
    assert_eq!(artifact.environment, "production");
    assert_eq!(artifact.flags.len(), 1);
    assert_eq!(artifact.flags[0].len(), 2); // One serve rule + one default rule
    
    // Check that flag name is in string table
    assert!(artifact.string_table.contains(&"my_flag".to_string()));
    assert_eq!(artifact.flag_names.len(), 1);
}

#[test]
fn test_compile_boolean_flag_with_when() {
    let definitions = json!({
        "flags": [
            {
                "name": "my_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "my_flag": {
                "rules": [
                    {
                        "when": "user.role == 'admin'",
                        "serve": true
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    assert_eq!(artifact.flags[0].len(), 2); // One serve rule with when + one default rule
    
    // Check that the rule has a when expression
    match &artifact.flags[0][0] {
        Rule::ServeWithWhen(expr, _) => {
            // Expression should be a binary op
            match expr {
                Expression::BinaryOp { .. } => {}
                _ => panic!("Expected BinaryOp expression"),
            }
        }
        _ => panic!("Expected ServeWithWhen rule"),
    }
}

#[test]
fn test_compile_multivariate_flag_with_variations() {
    let definitions = json!({
        "flags": [
            {
                "name": "theme",
                "type": "multivariate",
                "defaultValue": "light",
                "variations": [
                    {
                        "name": "light",
                        "value": "light"
                    },
                    {
                        "name": "dark",
                        "value": "dark"
                    }
                ]
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "theme": {
                "rules": [
                    {
                        "variations": [
                            {
                                "variation": "light",
                                "weight": 50
                            },
                            {
                                "variation": "dark",
                                "weight": 50
                            }
                        ]
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    assert_eq!(artifact.flags[0].len(), 2); // One variations rule + one default rule
    
    // Check that the rule is a variations rule
    match &artifact.flags[0][0] {
        Rule::VariationsWithoutWhen(variations) => {
            assert_eq!(variations.len(), 2);
            assert_eq!(variations[0].percentage, 50);
            assert_eq!(variations[1].percentage, 50);
        }
        _ => panic!("Expected VariationsWithoutWhen rule"),
    }
}

#[test]
fn test_compile_rollout_rule() {
    let definitions = json!({
        "flags": [
            {
                "name": "my_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "my_flag": {
                "rules": [
                    {
                        "rollout": {
                            "variation": "ON",
                            "percentage": 25
                        }
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    assert_eq!(artifact.flags[0].len(), 2); // One rollout rule + one default rule
    
    // Check that the rule is a rollout rule
    match &artifact.flags[0][0] {
        Rule::RolloutWithoutWhen(payload) => {
            assert_eq!(payload.percentage, 25);
        }
        _ => panic!("Expected RolloutWithoutWhen rule"),
    }
}

#[test]
fn test_compile_with_segments() {
    let definitions = json!({
        "flags": [
            {
                "name": "my_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "segments": {
            "beta_users": {
                "when": "user.beta == true"
            }
        },
        "rules": {
            "my_flag": {
                "rules": []
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    assert!(artifact.segments.is_some());
    let segments = artifact.segments.as_ref().unwrap();
    assert_eq!(segments.len(), 1);
    
    // Check segment name is in string table
    assert!(artifact.string_table.contains(&"beta_users".to_string()));
}

#[test]
fn test_compile_multiple_flags() {
    let definitions = json!({
        "flags": [
            {
                "name": "flag1",
                "type": "boolean",
                "defaultValue": false
            },
            {
                "name": "flag2",
                "type": "boolean",
                "defaultValue": true
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "flag1": {
                "rules": [
                    {
                        "serve": true
                    }
                ]
            },
            "flag2": {
                "rules": [
                    {
                        "serve": false
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    assert_eq!(artifact.flags.len(), 2);
    assert_eq!(artifact.flag_names.len(), 2);
    
    // Each flag should have 2 rules (one serve + one default)
    assert_eq!(artifact.flags[0].len(), 2);
    assert_eq!(artifact.flags[1].len(), 2);
}

#[test]
fn test_compile_error_flag_not_found() {
    let definitions = json!({
        "flags": [
            {
                "name": "my_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "unknown_flag": {
                "rules": []
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_err());
}

#[test]
fn test_compile_error_variation_not_found() {
    let definitions = json!({
        "flags": [
            {
                "name": "theme",
                "type": "multivariate",
                "defaultValue": "light",
                "variations": [
                    {
                        "name": "light",
                        "value": "light"
                    }
                ]
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "theme": {
                "rules": [
                    {
                        "variations": [
                            {
                                "variation": "dark",
                                "weight": 100
                            }
                        ]
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_err());
}

#[test]
fn test_compile_default_rule_appended() {
    let definitions = json!({
        "flags": [
            {
                "name": "my_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "my_flag": {
                "rules": [
                    {
                        "serve": true
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    // Should have 2 rules: one serve rule + one default rule
    assert_eq!(artifact.flags[0].len(), 2);
    
    // Last rule should be the default serve rule
    match &artifact.flags[0][1] {
        Rule::ServeWithoutWhen(payload) => {
            match payload {
                crate::ast::ServePayload::Number(index) => {
                    // Default value "OFF" should be in string table
                    assert!(artifact.string_table[*index as usize] == "OFF");
                }
                _ => panic!("Expected Number payload"),
            }
        }
        _ => panic!("Expected ServeWithoutWhen rule for default"),
    }
}

#[test]
fn test_compile_string_table_deduplication() {
    let definitions = json!({
        "flags": [
            {
                "name": "my_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "my_flag": {
                "rules": [
                    {
                        "when": "user.role == 'admin'",
                        "serve": true
                    },
                    {
                        "when": "user.role == 'admin'",
                        "serve": true
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    // "admin" should only appear once in string table
    let admin_count = artifact.string_table.iter().filter(|s| s.as_str() == "admin").count();
    assert_eq!(admin_count, 1);
}

#[test]
fn test_compile_flag_with_no_rules() {
    let definitions = json!({
        "flags": [
            {
                "name": "my_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "my_flag": {
                "rules": []
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    // Should only have the default rule
    assert_eq!(artifact.flags[0].len(), 1);
    match &artifact.flags[0][0] {
        Rule::ServeWithoutWhen(_) => {}
        _ => panic!("Expected default ServeWithoutWhen rule"),
    }
}

#[test]
fn test_compile_variation_weight_rounding() {
    let definitions = json!({
        "flags": [
            {
                "name": "theme",
                "type": "multivariate",
                "defaultValue": "light",
                "variations": [
                    {
                        "name": "light",
                        "value": "light"
                    },
                    {
                        "name": "dark",
                        "value": "dark"
                    }
                ]
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "theme": {
                "rules": [
                    {
                        "variations": [
                            {
                                "variation": "light",
                                "weight": 49.7  // Should round to 50
                            },
                            {
                                "variation": "dark",
                                "weight": 50.3  // Should round to 50
                            }
                        ]
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    match &artifact.flags[0][0] {
        Rule::VariationsWithoutWhen(variations) => {
            assert_eq!(variations.len(), 2);
            // Should round, not clamp
            assert_eq!(variations[0].percentage, 50);
            assert_eq!(variations[1].percentage, 50);
        }
        _ => panic!("Expected VariationsWithoutWhen rule"),
    }
}

#[test]
fn test_compile_rollout_percentage_rounding() {
    let definitions = json!({
        "flags": [
            {
                "name": "my_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "my_flag": {
                "rules": [
                    {
                        "rollout": {
                            "variation": "ON",
                            "percentage": 25.7  // Should round to 26
                        }
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    match &artifact.flags[0][0] {
        Rule::RolloutWithoutWhen(payload) => {
            // Should round, not clamp
            assert_eq!(payload.percentage, 26);
        }
        _ => panic!("Expected RolloutWithoutWhen rule"),
    }
}

#[test]
fn test_compile_boolean_flag_with_string_true() {
    let definitions = json!({
        "flags": [
            {
                "name": "my_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "my_flag": {
                "rules": [
                    {
                        "serve": "true"  // String "true" should normalize to "ON"
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    match &artifact.flags[0][0] {
        Rule::ServeWithoutWhen(payload) => {
            match payload {
                crate::ast::ServePayload::Number(index) => {
                    // Should be normalized to "ON"
                    assert_eq!(artifact.string_table[*index as usize], "ON");
                }
                _ => panic!("Expected Number payload"),
            }
        }
        _ => panic!("Expected ServeWithoutWhen rule"),
    }
}

#[test]
fn test_compile_boolean_flag_with_string_false() {
    let definitions = json!({
        "flags": [
            {
                "name": "my_flag",
                "type": "boolean",
                "defaultValue": true
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "my_flag": {
                "rules": [
                    {
                        "serve": "false"  // String "false" should normalize to "OFF"
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    match &artifact.flags[0][0] {
        Rule::ServeWithoutWhen(payload) => {
            match payload {
                crate::ast::ServePayload::Number(index) => {
                    // Should be normalized to "OFF"
                    assert_eq!(artifact.string_table[*index as usize], "OFF");
                }
                _ => panic!("Expected Number payload"),
            }
        }
        _ => panic!("Expected ServeWithoutWhen rule"),
    }
}

#[test]
fn test_compile_multivariate_flag_with_rollout() {
    let definitions = json!({
        "flags": [
            {
                "name": "theme",
                "type": "multivariate",
                "defaultValue": "light",
                "variations": [
                    {
                        "name": "light",
                        "value": "light"
                    },
                    {
                        "name": "dark",
                        "value": "dark"
                    }
                ]
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "rules": {
            "theme": {
                "rules": [
                    {
                        "rollout": {
                            "variation": "dark",  // Variation name, not value
                            "percentage": 25
                        }
                    }
                ]
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    match &artifact.flags[0][0] {
        Rule::RolloutWithoutWhen(payload) => {
            assert_eq!(payload.percentage, 25);
            // Value should be "dark" (from variation definition)
            match &payload.value_index {
                crate::ast::RolloutValue::Number(index) => {
                    assert_eq!(artifact.string_table[*index as usize], "dark");
                }
                _ => panic!("Expected Number value_index"),
            }
        }
        _ => panic!("Expected RolloutWithoutWhen rule"),
    }
}

#[test]
fn test_compile_empty_segments() {
    let definitions = json!({
        "flags": [
            {
                "name": "my_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let deployment = json!({
        "environment": "production",
        "segments": {},
        "rules": {
            "my_flag": {
                "rules": []
            }
        }
    });

    let result = compile(&deployment, &definitions);
    assert!(result.is_ok());
    
    let artifact = result.unwrap();
    // Segments should be None (not Some(empty vec))
    assert!(artifact.segments.is_none());
}

