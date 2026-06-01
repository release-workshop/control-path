//! Unit tests for the TypeScript SDK generator

use crate::generator::typescript::TypeScriptGenerator;
use crate::generator::Generator;
use controlpath_compiler::{build_sdk_catalog, parse_catalog};
use std::collections::BTreeMap;
use std::fs;
use tempfile::TempDir;

fn sdk_from_flags(flags_yaml: &str) -> controlpath_compiler::SdkCatalog {
    let catalog = parse_catalog(
        &format!(
            r#"
catalog:
  id: svc
flags:
{flags_yaml}
"#
        ),
        Some("control-path.yaml"),
    )
    .unwrap();
    build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap()
}

#[test]
fn test_generator_initialization() {
    assert!(TypeScriptGenerator::new().is_ok());
}

#[test]
fn test_generate_boolean_flag_methods() {
    let sdk = sdk_from_flags(
        r#"
  new_dashboard:
    default: false
    kind: release
"#,
    );
    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(index_content.contains("async newDashboard()"));
    assert!(index_content.contains("'new_dashboard'"));
}

#[test]
fn test_generate_package_json_for_node_modules_output() {
    let sdk = sdk_from_flags(
        r#"
  test_flag:
    default: false
    kind: release
"#,
    );
    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir
        .path()
        .join("node_modules")
        .join("@controlpath")
        .join("generated");
    fs::create_dir_all(&output_path).unwrap();
    generator.generate(&sdk, &output_path).unwrap();

    let package_json_content = fs::read_to_string(output_path.join("package.json")).unwrap();
    let package_json: serde_json::Value = serde_json::from_str(&package_json_content).unwrap();
    assert_eq!(package_json["name"], "@controlpath/generated");
    assert_eq!(
        package_json["dependencies"]["@controlpath/runtime"],
        "^0.3.0"
    );
}

#[test]
fn test_generate_empty_catalog() {
    let catalog = parse_catalog(
        r#"
catalog:
  id: svc
flags: {}
"#,
        Some("control-path.yaml"),
    )
    .unwrap();
    let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();
    let generator = TypeScriptGenerator::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    generator.generate(&sdk, temp_dir.path()).unwrap();

    let types_content = fs::read_to_string(temp_dir.path().join("types.ts")).unwrap();
    let index_content = fs::read_to_string(temp_dir.path().join("index.ts")).unwrap();
    assert!(types_content.contains("export interface Attributes"));
    assert!(index_content.contains("export class Evaluator"));
}
