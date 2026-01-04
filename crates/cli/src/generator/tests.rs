//! Unit tests for the TypeScript SDK generator

use crate::generator::typescript::TypeScriptGenerator;
use crate::generator::Generator;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_generator_initialization() {
    let generator = TypeScriptGenerator::new();
    assert!(generator.is_ok());
}

#[test]
fn test_to_camel_case() {
    // Test the private function indirectly through generation
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "new_dashboard",
                "type": "boolean",
                "defaultValue": false
            },
            {
                "name": "checkout_experiment",
                "type": "multivariate",
                "defaultValue": "CONTROL",
                "variations": [
                    {"name": "CONTROL", "value": "control"},
                    {"name": "VARIANT_A", "value": "variant_a"}
                ]
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    // Verify that camelCase conversion worked by checking generated code
    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("newDashboard"));
    assert!(index_content.contains("checkoutExperiment"));
}

#[test]
fn test_format_default_value_boolean() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "test_flag",
                "type": "boolean",
                "defaultValue": true
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    // Should contain the default value as a boolean literal
    assert!(index_content.contains("return true") || index_content.contains("return false"));
}

#[test]
fn test_format_default_value_string() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "test_flag",
                "type": "multivariate",
                "defaultValue": "CONTROL",
                "variations": [
                    {"name": "CONTROL", "value": "control"},
                    {"name": "VARIANT_A", "value": "variant_a"}
                ]
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    // Should contain the default value as a string literal
    assert!(index_content.contains("'CONTROL'"));
}

#[test]
fn test_generate_types_boolean_flag() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "new_dashboard",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();

    // Should contain User interface
    assert!(types_content.contains("export interface User"));
    assert!(types_content.contains("id: string"));

    // Should contain Context interface
    assert!(types_content.contains("export interface Context"));

    // Should contain FlagName type
    assert!(types_content.contains("export type FlagName"));
    assert!(types_content.contains("'newDashboard'"));

    // Should contain FlagReturnTypes
    assert!(types_content.contains("export type FlagReturnTypes"));
    assert!(types_content.contains("newDashboard: boolean"));
}

#[test]
fn test_generate_types_multivariate_flag() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "checkout_experiment",
                "type": "multivariate",
                "defaultValue": "CONTROL",
                "variations": [
                    {"name": "CONTROL", "value": "control"},
                    {"name": "VARIANT_A", "value": "variant_a"},
                    {"name": "VARIANT_B", "value": "variant_b"}
                ]
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();

    // Should contain variation type
    assert!(types_content.contains("export type CheckoutExperimentVariation"));
    assert!(types_content.contains("'CONTROL'"));
    assert!(types_content.contains("'VARIANT_A'"));
    assert!(types_content.contains("'VARIANT_B'"));

    // Should contain flag return type
    assert!(types_content.contains("checkoutExperiment: CheckoutExperimentVariation"));
}

#[test]
fn test_generate_evaluator_boolean_flag() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "new_dashboard",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();

    // Should contain Evaluator class
    assert!(index_content.contains("export class Evaluator"));

    // Should contain method for the flag
    assert!(index_content.contains("async newDashboard()"));
    assert!(index_content.contains("async newDashboard(user: User)"));
    assert!(index_content.contains("async newDashboard(user: User, context: Context)"));

    // Should contain method implementation
    assert!(index_content.contains("async newDashboard(user?: User, context?: Context)"));

    // Should contain Provider import
    assert!(index_content.contains("import { Provider } from '@controlpath/runtime'"));

    // Should contain context management methods
    assert!(index_content.contains("setContext"));
    assert!(index_content.contains("clearContext"));

    // Should contain batch evaluation methods
    assert!(index_content.contains("evaluateBatch"));
    assert!(index_content.contains("evaluateAll"));
}

#[test]
fn test_generate_evaluator_multivariate_flag() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "checkout_experiment",
                "type": "multivariate",
                "defaultValue": "CONTROL",
                "variations": [
                    {"name": "CONTROL", "value": "control"},
                    {"name": "VARIANT_A", "value": "variant_a"}
                ]
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();

    // Should contain method for the multivariate flag
    assert!(index_content.contains("async checkoutExperiment()"));
    assert!(index_content.contains("Promise<CheckoutExperimentVariation>"));

    // Should use string evaluation for multivariate
    assert!(index_content.contains("resolveStringEvaluation"));

    // Should cast return value to variation type
    assert!(index_content.contains("as CheckoutExperimentVariation"));
}

#[test]
fn test_generate_package_json() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "test_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let package_json_content = fs::read_to_string(temp_dir.path().join("package.json")).unwrap();
    let package_json: serde_json::Value = serde_json::from_str(&package_json_content).unwrap();

    assert_eq!(package_json["name"], "generated-flags");
    assert_eq!(package_json["version"], "0.1.0");
    assert_eq!(package_json["main"], "index.js");
    assert_eq!(package_json["types"], "index.d.ts");

    // Check dependencies
    let deps = package_json["dependencies"].as_object().unwrap();
    assert!(deps.contains_key("@controlpath/runtime"));
    assert!(deps.contains_key("@openfeature/server-sdk"));
}

#[test]
fn test_generate_multiple_flags() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "new_dashboard",
                "type": "boolean",
                "defaultValue": false
            },
            {
                "name": "enable_analytics",
                "type": "boolean",
                "defaultValue": true
            },
            {
                "name": "checkout_experiment",
                "type": "multivariate",
                "defaultValue": "CONTROL",
                "variations": [
                    {"name": "CONTROL", "value": "control"},
                    {"name": "VARIANT_A", "value": "variant_a"}
                ]
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();

    // All flags should have methods
    assert!(index_content.contains("newDashboard"));
    assert!(index_content.contains("enableAnalytics"));
    assert!(index_content.contains("checkoutExperiment"));

    // All flags should be in types
    assert!(types_content.contains("'newDashboard'"));
    assert!(types_content.contains("'enableAnalytics'"));
    assert!(types_content.contains("'checkoutExperiment'"));

    // evaluateAll should include all flags
    assert!(index_content.contains("'newDashboard'"));
    assert!(index_content.contains("'enableAnalytics'"));
    assert!(index_content.contains("'checkoutExperiment'"));
}

#[test]
fn test_generate_empty_flags() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": []
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    // Should still generate valid files
    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();
    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();

    assert!(types_content.contains("export interface User"));
    assert!(index_content.contains("export class Evaluator"));
}

#[test]
fn test_generate_flag_without_default_value() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "test_flag",
                "type": "boolean"
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    // Should default to false for boolean flags
    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("return false"));
}

#[test]
fn test_generate_flag_info_map() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "new_dashboard",
                "type": "boolean",
                "defaultValue": false
            },
            {
                "name": "checkout_experiment",
                "type": "multivariate",
                "defaultValue": "CONTROL",
                "variations": [
                    {"name": "CONTROL", "value": "control"},
                    {"name": "VARIANT_A", "value": "variant_a"}
                ]
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();

    // Should contain getFlagInfo method
    assert!(index_content.contains("getFlagInfo"));

    // Should contain flag info map with both flags
    assert!(index_content.contains("'newDashboard'"));
    assert!(index_content.contains("'checkoutExperiment'"));

    // Should contain correct types and default values
    assert!(index_content.contains("type: 'boolean'"));
    assert!(index_content.contains("type: 'multivariate'"));
    assert!(index_content.contains("defaultValue: false"));
    assert!(index_content.contains("defaultValue: 'CONTROL'"));
}

#[test]
fn test_generate_observability_methods() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "test_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();

    // Should contain observability methods
    assert!(index_content.contains("setLogger"));
    assert!(index_content.contains("setTracer"));
    assert!(index_content.contains("setMetrics"));
}

#[test]
fn test_generate_type_casting_for_multivariate() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "checkout_experiment",
                "type": "multivariate",
                "defaultValue": "CONTROL",
                "variations": [
                    {"name": "CONTROL", "value": "control"},
                    {"name": "VARIANT_A", "value": "variant_a"}
                ]
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();

    // Should cast default values to return type in catch block
    assert!(index_content.contains("as CheckoutExperimentVariation"));

    // Should cast default values to return type in early return
    assert!(index_content.contains("return 'CONTROL' as CheckoutExperimentVariation"));
}

#[test]
fn test_generate_snake_case_conversion() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "very_long_flag_name",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();

    // Method name should be camelCase
    assert!(index_content.contains("veryLongFlagName"));

    // But should use snake_case for provider calls
    assert!(index_content.contains("'very_long_flag_name'"));
}

#[test]
fn test_generate_error_handling() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "test_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();

    // Should have try-catch blocks
    assert!(index_content.contains("try {"));
    assert!(index_content.contains("} catch (error) {"));

    // Should return defaults on error
    assert!(index_content.contains("return false"));
}

#[test]
fn test_generate_context_resolution() {
    let generator = TypeScriptGenerator::new().unwrap();
    let definitions = json!({
        "flags": [
            {
                "name": "test_flag",
                "type": "boolean",
                "defaultValue": false
            }
        ]
    });

    let temp_dir = TempDir::new().unwrap();
    let result = generator.generate(&definitions, temp_dir.path());
    assert!(result.is_ok());

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();

    // Should contain resolveContext method
    assert!(index_content.contains("resolveContext"));

    // Should contain flattenUserAttributes
    assert!(index_content.contains("flattenUserAttributes"));

    // Should contain flattenContextAttributes
    assert!(index_content.contains("flattenContextAttributes"));

    // Should handle user.id
    assert!(index_content.contains("targetingKey"));
    assert!(index_content.contains("user."));
}
