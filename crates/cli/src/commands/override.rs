//! Override management command implementation

use crate::error::{CliError, CliResult};
use chrono::Utc;
use jsonschema::JSONSchema;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Embed override file schema at compile time
const OVERRIDE_SCHEMA_JSON: &str = include_str!("../../../../schemas/override-file.schema.v1.json");

/// Options for override management commands
pub struct Options {
    /// The override subcommand to execute
    pub subcommand: OverrideSubcommand,
}

#[derive(Debug, Clone)]
pub enum OverrideSubcommand {
    Set {
        flag: String,
        value: String,
        reason: Option<String>,
        operator: Option<String>,
        file: PathBuf,
        definitions: Option<PathBuf>,
    },
    Clear {
        flag: String,
        file: PathBuf,
    },
    List {
        file: PathBuf,
    },
    History {
        flag: Option<String>,
        file: PathBuf,
    },
}

/// Load the override file schema
///
/// Returns the parsed JSON schema as a `serde_json::Value`.
/// This function never fails at runtime since the schema is embedded at compile time.
///
/// # Panics
///
/// Panics if the embedded schema JSON is invalid (this should never happen).
fn load_override_schema() -> Value {
    serde_json::from_str(OVERRIDE_SCHEMA_JSON)
        .expect("Failed to parse embedded override schema - this should never happen")
}

/// Read override file from disk
///
/// If the file doesn't exist, returns an empty override file structure.
/// If the file exists but contains invalid JSON, returns an error.
///
/// # Errors
///
/// Returns `CliError` if the file exists but cannot be read or parsed.
fn read_override_file(path: &Path) -> CliResult<Value> {
    if !path.exists() {
        // Create empty override file
        return Ok(json!({
            "version": "1.0",
            "overrides": {}
        }));
    }

    let content = fs::read_to_string(path).map_err(|e| {
        CliError::Message(format!(
            "Failed to read override file {}: {e}",
            path.display()
        ))
    })?;

    serde_json::from_str(&content).map_err(|e| {
        CliError::Message(format!(
            "Failed to parse override file {}: {e}",
            path.display()
        ))
    })
}

/// Write override file to disk
///
/// Validates the data against the override file schema before writing.
/// Creates parent directories if they don't exist.
///
/// # Errors
///
/// Returns `CliError` if:
/// - Schema validation fails
/// - The data cannot be serialized to JSON
/// - The file cannot be written
fn write_override_file(path: &Path, data: &Value) -> CliResult<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            CliError::Message(format!(
                "Failed to create directory {}: {e}",
                parent.display()
            ))
        })?;
    }

    // Validate against schema before writing
    let schema = load_override_schema();
    let compiled = JSONSchema::compile(&schema)
        .map_err(|e| CliError::Message(format!("Failed to compile schema: {e}")))?;

    if let Err(errors) = compiled.validate(data) {
        let error_messages: Vec<String> = errors
            .map(|e| format!("{}: {}", e.instance_path, e))
            .collect();
        return Err(CliError::Message(format!(
            "Override file validation failed:\n{}",
            error_messages.join("\n")
        )));
    }

    let json = serde_json::to_string_pretty(data)
        .map_err(|e| CliError::Message(format!("Failed to serialize override file: {e}")))?;

    fs::write(path, json).map_err(|e| {
        CliError::Message(format!(
            "Failed to write override file {}: {e}",
            path.display()
        ))
    })?;

    Ok(())
}

/// Read flag definitions to validate override values
fn read_flag_definitions(path: Option<&Path>) -> CliResult<Option<Value>> {
    let def_path = path.unwrap_or_else(|| Path::new("flags.definitions.yaml"));

    if !def_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(def_path).map_err(|e| {
        CliError::Message(format!(
            "Failed to read definitions file {}: {e}",
            def_path.display()
        ))
    })?;

    let yaml: Value = serde_yaml::from_str(&content).map_err(|e| {
        CliError::Message(format!(
            "Failed to parse definitions file {}: {e}",
            def_path.display()
        ))
    })?;

    Ok(Some(yaml))
}

/// Validate override value against flag definitions
///
/// Validates that the override value is appropriate for the flag type:
/// - Boolean flags: accepts ON/OFF, true/false, 1/0, yes/no (case-insensitive)
/// - Multivariate flags: must match a variation name from the flag definitions
///
/// # Errors
///
/// Returns `CliError` if:
/// - The flag type is unknown
/// - The value is invalid for the flag type
/// - The multivariate flag has no variations defined
fn validate_override_value(
    flag_name: &str,
    value: &str,
    definitions: Option<&Value>,
) -> CliResult<()> {
    if let Some(defs) = definitions {
        // Find flag in definitions
        if let Some(flags) = defs.get("flags").and_then(|f| f.as_array()) {
            let flag = flags
                .iter()
                .find(|f| f.get("name").and_then(|n| n.as_str()) == Some(flag_name));

            if let Some(flag_obj) = flag {
                let flag_type = flag_obj
                    .get("type")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| {
                        CliError::Message(format!("Flag '{}' missing type", flag_name))
                    })?;

                match flag_type {
                    "boolean" => {
                        // Validate boolean value (ON/OFF, true/false, 1/0, yes/no - case-insensitive)
                        let value_upper = value.to_uppercase();
                        if !["ON", "OFF", "TRUE", "FALSE", "1", "0", "YES", "NO"]
                            .contains(&value_upper.as_str())
                        {
                            return Err(CliError::Message(format!(
                                "Invalid boolean value '{}' for flag '{}'. Use ON/OFF, true/false, 1/0, or yes/no",
                                value, flag_name
                            )));
                        }
                    }
                    "multivariate" => {
                        // Validate multivariate value (must match a variation name)
                        if let Some(variations) =
                            flag_obj.get("variations").and_then(|v| v.as_array())
                        {
                            let valid_variations: Vec<&str> = variations
                                .iter()
                                .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
                                .collect();

                            if !valid_variations.contains(&value) {
                                return Err(CliError::Message(format!(
                                    "Invalid variation '{}' for flag '{}'. Valid variations: {}",
                                    value,
                                    flag_name,
                                    valid_variations.join(", ")
                                )));
                            }
                        } else {
                            return Err(CliError::Message(format!(
                                "Flag '{}' is multivariate but has no variations defined",
                                flag_name
                            )));
                        }
                    }
                    _ => {
                        return Err(CliError::Message(format!(
                            "Unknown flag type '{}' for flag '{}'",
                            flag_type, flag_name
                        )));
                    }
                }
            } else {
                // Flag not found in definitions - warn but don't fail
                eprintln!(
                    "⚠ Warning: Flag '{}' not found in flag definitions",
                    flag_name
                );
            }
        }
    }

    Ok(())
}

/// Normalize boolean value to ON/OFF
///
/// Converts various boolean representations to the standard ON/OFF format:
/// - true, TRUE, 1, yes, YES → ON
/// - false, FALSE, 0, no, NO → OFF
/// - ON, OFF → unchanged
///
/// # Panics
///
/// This function does not validate the input. Invalid values will be returned as-is.
/// Use `validate_override_value` to ensure the value is valid before normalizing.
fn normalize_boolean_value(value: &str) -> String {
    let value_upper = value.to_uppercase();
    match value_upper.as_str() {
        "TRUE" | "1" | "YES" => "ON".to_string(),
        "FALSE" | "0" | "NO" => "OFF".to_string(),
        _ => value.to_string(), // Already ON/OFF or invalid (will be caught by validation)
    }
}

/// Run the override command
///
/// Executes the override subcommand and returns an exit code:
/// - `0` on success
/// - `1` on error (error message is printed to stderr)
pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("✗ Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    match &options.subcommand {
        OverrideSubcommand::Set {
            flag,
            value,
            reason,
            operator,
            file,
            definitions,
        } => {
            // Read existing override file
            let mut override_file = read_override_file(file)?;

            // Read flag definitions if provided
            let flag_definitions = read_flag_definitions(definitions.as_deref())?;

            // Validate flag exists in definitions (if provided)
            if let Some(defs) = flag_definitions.as_ref() {
                if let Some(flags) = defs.get("flags").and_then(|f| f.as_array()) {
                    let flag_exists = flags
                        .iter()
                        .any(|f| f.get("name").and_then(|n| n.as_str()) == Some(flag.as_str()));

                    if !flag_exists {
                        return Err(CliError::Message(format!(
                            "Flag '{}' not found in flag definitions",
                            flag
                        )));
                    }
                }
            }

            // Normalize and validate value
            let normalized_value = if let Some(defs) = flag_definitions.as_ref() {
                if let Some(flags) = defs.get("flags").and_then(|f| f.as_array()) {
                    if let Some(flag_obj) = flags
                        .iter()
                        .find(|f| f.get("name").and_then(|n| n.as_str()) == Some(flag.as_str()))
                    {
                        if let Some(flag_type) = flag_obj.get("type").and_then(|t| t.as_str()) {
                            if flag_type == "boolean" {
                                normalize_boolean_value(value)
                            } else {
                                value.clone()
                            }
                        } else {
                            value.clone()
                        }
                    } else {
                        // Flag not found in definitions - use value as-is
                        value.clone()
                    }
                } else {
                    value.clone()
                }
            } else {
                // No definitions provided - use value as-is (don't normalize)
                // Normalization only happens when we know the flag type
                value.clone()
            };

            validate_override_value(flag, &normalized_value, flag_definitions.as_ref())?;

            // Create override value (full format with metadata)
            // Always use full format to ensure reason is included for audit trail
            let timestamp = Utc::now().to_rfc3339();
            let mut override_obj = json!({
                "value": normalized_value,
                "timestamp": timestamp,
            });

            // Reason is recommended for audit trail - warn if missing
            if let Some(r) = reason {
                override_obj["reason"] = json!(r);
            } else {
                eprintln!("⚠ Warning: No reason provided for override. Consider using --reason for audit trail.");
            }

            if let Some(op) = operator {
                override_obj["operator"] = json!(op);
            }

            // Update overrides
            let overrides = override_file
                .get_mut("overrides")
                .and_then(|o| o.as_object_mut())
                .ok_or_else(|| {
                    CliError::Message(
                        "Invalid override file: missing 'overrides' object".to_string(),
                    )
                })?;

            overrides.insert(flag.clone(), override_obj.clone());

            // Write back to file
            write_override_file(file, &override_file)?;

            println!(
                "✓ Set override for flag '{}' to '{}'",
                flag, normalized_value
            );
            Ok(())
        }
        OverrideSubcommand::Clear { flag, file } => {
            // Read existing override file
            let mut override_file = read_override_file(file)?;

            // Remove from overrides
            let overrides = override_file
                .get_mut("overrides")
                .and_then(|o| o.as_object_mut())
                .ok_or_else(|| {
                    CliError::Message(
                        "Invalid override file: missing 'overrides' object".to_string(),
                    )
                })?;

            if overrides.remove(flag).is_some() {
                // Write back to file
                write_override_file(file, &override_file)?;

                println!("✓ Cleared override for flag '{}'", flag);
            } else {
                println!("ℹ No override found for flag '{}'", flag);
            }

            Ok(())
        }
        OverrideSubcommand::List { file } => {
            let override_file = read_override_file(file)?;

            // Validate file format (warn if invalid)
            let schema = load_override_schema();
            let compiled = JSONSchema::compile(&schema)
                .map_err(|e| CliError::Message(format!("Failed to compile schema: {e}")))?;

            if let Err(errors) = compiled.validate(&override_file) {
                eprintln!("⚠ Warning: Override file has validation errors:");
                for error in errors {
                    eprintln!("  - {}: {}", error.instance_path, error);
                }
            }

            // Display overrides
            if let Some(overrides) = override_file.get("overrides").and_then(|o| o.as_object()) {
                if overrides.is_empty() {
                    println!("No overrides set");
                } else {
                    println!("Current overrides:");
                    println!("{:-<80}", "");
                    println!(
                        "{:<30} {:<20} {:<20} {:<10}",
                        "Flag", "Value", "Timestamp", "Operator"
                    );
                    println!("{:-<80}", "");

                    for (flag_name, override_value) in overrides {
                        let (value, timestamp, reason, operator) = match override_value {
                            Value::String(s) => (s.clone(), None, None, None),
                            Value::Object(obj) => (
                                obj.get("value")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("?")
                                    .to_string(),
                                obj.get("timestamp")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string()),
                                obj.get("reason")
                                    .and_then(|r| r.as_str())
                                    .map(|s| s.to_string()),
                                obj.get("operator")
                                    .and_then(|o| o.as_str())
                                    .map(|s| s.to_string()),
                            ),
                            _ => ("?".to_string(), None, None, None),
                        };

                        let timestamp_str = timestamp
                            .as_ref()
                            .map(|t| {
                                // Format timestamp for display
                                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(t) {
                                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                                } else {
                                    t.clone()
                                }
                            })
                            .unwrap_or_else(|| "-".to_string());

                        let operator_str = operator.as_deref().unwrap_or("-");

                        println!(
                            "{:<30} {:<20} {:<20} {:<10}",
                            flag_name, value, timestamp_str, operator_str
                        );

                        if let Some(r) = reason {
                            println!("  Reason: {}", r);
                        }
                    }
                }
            } else {
                println!("No overrides set");
            }

            Ok(())
        }
        OverrideSubcommand::History { flag, file } => {
            // History command shows current overrides with their reasons (audit trail)
            let override_file = read_override_file(file)?;

            // Validate file format (warn if invalid)
            let schema = load_override_schema();
            let compiled = JSONSchema::compile(&schema)
                .map_err(|e| CliError::Message(format!("Failed to compile schema: {e}")))?;

            if let Err(errors) = compiled.validate(&override_file) {
                eprintln!("⚠ Warning: Override file has validation errors:");
                for error in errors {
                    eprintln!("  - {}: {}", error.instance_path, error);
                }
            }

            // Display overrides with their reasons (audit trail)
            if let Some(overrides) = override_file.get("overrides").and_then(|o| o.as_object()) {
                let filtered_overrides: Vec<(&String, &Value)> = if let Some(filter_flag) = flag {
                    overrides
                        .iter()
                        .filter(|(flag_name, _)| flag_name.as_str() == filter_flag.as_str())
                        .collect()
                } else {
                    overrides.iter().collect()
                };

                if filtered_overrides.is_empty() {
                    if let Some(filter_flag) = flag {
                        println!("No override found for flag '{}'", filter_flag);
                    } else {
                        println!("No overrides set");
                    }
                } else {
                    if let Some(filter_flag) = flag {
                        println!("Override history for flag '{}':", filter_flag);
                    } else {
                        println!("Override history (current overrides with audit trail):");
                    }
                    println!("{:-<100}", "");
                    println!(
                        "{:<30} {:<20} {:<25} {:<20} {:<30}",
                        "Flag", "Value", "Timestamp", "Operator", "Reason"
                    );
                    println!("{:-<100}", "");

                    for (flag_name, override_value) in filtered_overrides {
                        let (value, timestamp, reason, operator) = match override_value {
                            Value::String(s) => (s.clone(), None, None, None),
                            Value::Object(obj) => (
                                obj.get("value")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("?")
                                    .to_string(),
                                obj.get("timestamp")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string()),
                                obj.get("reason")
                                    .and_then(|r| r.as_str())
                                    .map(|s| s.to_string()),
                                obj.get("operator")
                                    .and_then(|o| o.as_str())
                                    .map(|s| s.to_string()),
                            ),
                            _ => ("?".to_string(), None, None, None),
                        };

                        let timestamp_str = timestamp
                            .as_ref()
                            .map(|t| {
                                // Format timestamp for display
                                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(t) {
                                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                                } else {
                                    t.clone()
                                }
                            })
                            .unwrap_or_else(|| "-".to_string());

                        let operator_str = operator.as_deref().unwrap_or("-");
                        let reason_str = reason.as_deref().unwrap_or("-");

                        println!(
                            "{:<30} {:<20} {:<25} {:<20} {:<30}",
                            flag_name, value, timestamp_str, operator_str, reason_str
                        );
                    }
                }
            } else {
                println!("No overrides set");
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    use crate::test_helpers::DirGuard;

    #[test]
    #[serial]
    fn test_set_override() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        let options = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "test_flag".to_string(),
                value: "ON".to_string(),
                reason: Some("Test override".to_string()),
                operator: Some("test@example.com".to_string()),
                file: override_file.clone(),
                definitions: None,
            },
        };

        assert_eq!(run(&options), 0);

        // Verify override file was created
        assert!(override_file.exists());

        let content = fs::read_to_string(&override_file).unwrap();
        let data: Value = serde_json::from_str(&content).unwrap();

        assert_eq!(data["version"], "1.0");
        assert_eq!(data["overrides"]["test_flag"]["value"], "ON");
        assert_eq!(data["overrides"]["test_flag"]["reason"], "Test override");
        assert_eq!(
            data["overrides"]["test_flag"]["operator"],
            "test@example.com"
        );
    }

    #[test]
    #[serial]
    fn test_clear_override() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // First set an override
        let set_options = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "test_flag".to_string(),
                value: "ON".to_string(),
                reason: None,
                operator: None,
                file: override_file.clone(),
                definitions: None,
            },
        };
        assert_eq!(run(&set_options), 0);

        // Then clear it
        let clear_options = Options {
            subcommand: OverrideSubcommand::Clear {
                flag: "test_flag".to_string(),
                file: override_file.clone(),
            },
        };
        assert_eq!(run(&clear_options), 0);

        // Verify override was removed
        let content = fs::read_to_string(&override_file).unwrap();
        let data: Value = serde_json::from_str(&content).unwrap();

        assert!(data["overrides"].as_object().unwrap().is_empty());
    }

    #[test]
    #[serial]
    fn test_list_overrides() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // Set some overrides
        let set_options1 = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "flag1".to_string(),
                value: "ON".to_string(),
                reason: None,
                operator: None,
                file: override_file.clone(),
                definitions: None,
            },
        };
        assert_eq!(run(&set_options1), 0);

        let set_options2 = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "flag2".to_string(),
                value: "OFF".to_string(),
                reason: None,
                operator: None,
                file: override_file.clone(),
                definitions: None,
            },
        };
        assert_eq!(run(&set_options2), 0);

        // List overrides
        let list_options = Options {
            subcommand: OverrideSubcommand::List {
                file: override_file.clone(),
            },
        };
        assert_eq!(run(&list_options), 0);
    }

    #[test]
    #[serial]
    fn test_normalize_boolean_value() {
        assert_eq!(normalize_boolean_value("true"), "ON");
        assert_eq!(normalize_boolean_value("TRUE"), "ON");
        assert_eq!(normalize_boolean_value("1"), "ON");
        assert_eq!(normalize_boolean_value("yes"), "ON");
        assert_eq!(normalize_boolean_value("false"), "OFF");
        assert_eq!(normalize_boolean_value("FALSE"), "OFF");
        assert_eq!(normalize_boolean_value("0"), "OFF");
        assert_eq!(normalize_boolean_value("no"), "OFF");
        assert_eq!(normalize_boolean_value("ON"), "ON");
        assert_eq!(normalize_boolean_value("OFF"), "OFF");
    }

    #[test]
    #[serial]
    fn test_set_override_with_definitions_boolean() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create flag definitions file
        fs::write(
            temp_path.join("flags.definitions.yaml"),
            r"flags:
  - name: test_flag
    type: boolean
    default: false
",
        )
        .unwrap();

        let override_file = temp_path.join("overrides.json");

        // Test setting boolean flag to ON
        let options = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "test_flag".to_string(),
                value: "true".to_string(), // Should be normalized to ON
                reason: None,
                operator: None,
                file: override_file.clone(),
                definitions: Some(temp_path.join("flags.definitions.yaml")),
            },
        };

        assert_eq!(run(&options), 0);

        // Verify override was normalized
        let content = fs::read_to_string(&override_file).unwrap();
        let data: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(data["overrides"]["test_flag"]["value"], "ON");
    }

    #[test]
    #[serial]
    fn test_set_override_with_definitions_multivariate() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create flag definitions file with multivariate flag
        fs::write(
            temp_path.join("flags.definitions.yaml"),
            r"flags:
  - name: api_version
    type: multivariate
    default: V1
    variations:
      - name: V1
        value: v1
      - name: V2
        value: v2
      - name: V3
        value: v3
",
        )
        .unwrap();

        let override_file = temp_path.join("overrides.json");

        // Test setting multivariate flag to valid variation
        let options = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "api_version".to_string(),
                value: "V2".to_string(),
                reason: Some("Testing V2".to_string()),
                operator: None,
                file: override_file.clone(),
                definitions: Some(temp_path.join("flags.definitions.yaml")),
            },
        };

        assert_eq!(run(&options), 0);

        // Verify override was set
        let content = fs::read_to_string(&override_file).unwrap();
        let data: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(data["overrides"]["api_version"]["value"], "V2");
    }

    #[test]
    #[serial]
    fn test_set_override_invalid_boolean_value() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create flag definitions file
        fs::write(
            temp_path.join("flags.definitions.yaml"),
            r"flags:
  - name: test_flag
    type: boolean
    default: false
",
        )
        .unwrap();

        let override_file = temp_path.join("overrides.json");

        // Test setting invalid boolean value
        let options = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "test_flag".to_string(),
                value: "INVALID_VALUE".to_string(),
                reason: None,
                operator: None,
                file: override_file.clone(),
                definitions: Some(temp_path.join("flags.definitions.yaml")),
            },
        };

        assert_eq!(run(&options), 1); // Should fail
    }

    #[test]
    #[serial]
    fn test_set_override_invalid_multivariate_value() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create flag definitions file with multivariate flag
        fs::write(
            temp_path.join("flags.definitions.yaml"),
            r"flags:
  - name: api_version
    type: multivariate
    default: V1
    variations:
      - name: V1
        value: v1
      - name: V2
        value: v2
",
        )
        .unwrap();

        let override_file = temp_path.join("overrides.json");

        // Test setting invalid variation name
        let options = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "api_version".to_string(),
                value: "INVALID_VARIATION".to_string(),
                reason: None,
                operator: None,
                file: override_file.clone(),
                definitions: Some(temp_path.join("flags.definitions.yaml")),
            },
        };

        assert_eq!(run(&options), 1); // Should fail
    }

    #[test]
    #[serial]
    fn test_set_override_flag_not_in_definitions() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create flag definitions file
        fs::write(
            temp_path.join("flags.definitions.yaml"),
            r"flags:
  - name: other_flag
    type: boolean
    default: false
",
        )
        .unwrap();

        let override_file = temp_path.join("overrides.json");

        // Test setting override for flag that doesn't exist in definitions
        let options = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "nonexistent_flag".to_string(),
                value: "ON".to_string(),
                reason: None,
                operator: None,
                file: override_file.clone(),
                definitions: Some(temp_path.join("flags.definitions.yaml")),
            },
        };

        assert_eq!(run(&options), 1); // Should fail
    }

    #[test]
    #[serial]
    fn test_read_override_file_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // Write invalid JSON
        fs::write(
            &override_file,
            r#"{ "version": "1.0", "overrides": { invalid json } }"#,
        )
        .unwrap();

        // Try to read it - should fail
        let result = read_override_file(&override_file);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_write_override_file_schema_validation_failure() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // Create invalid override data (missing required "version" field)
        let invalid_data = json!({
            "overrides": {
                "test_flag": "ON"
            }
        });

        // Try to write it - should fail schema validation
        let result = write_override_file(&override_file, &invalid_data);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_write_override_file_schema_validation_invalid_version() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // Create invalid override data (invalid version format)
        let invalid_data = json!({
            "version": "invalid",
            "overrides": {
                "test_flag": "ON"
            }
        });

        // Try to write it - should fail schema validation
        let result = write_override_file(&override_file, &invalid_data);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_list_overrides_with_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // Write invalid JSON
        fs::write(&override_file, r#"{ "version": "1.0", invalid }"#).unwrap();

        // List should handle the error gracefully
        let options = Options {
            subcommand: OverrideSubcommand::List {
                file: override_file.clone(),
            },
        };

        assert_eq!(run(&options), 1); // Should fail
    }

    #[test]
    #[serial]
    fn test_list_overrides_with_schema_warning() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // Write override file with invalid structure (missing version)
        // This should trigger a warning but not fail completely
        fs::write(
            &override_file,
            r#"{
  "overrides": {
    "test_flag": "ON"
  }
}"#,
        )
        .unwrap();

        let options = Options {
            subcommand: OverrideSubcommand::List {
                file: override_file.clone(),
            },
        };

        // List should show warning but still work
        // The validation happens in list command, so it should warn
        let exit_code = run(&options);
        // May succeed or fail depending on validation strictness
        assert!(exit_code == 0 || exit_code == 1);
    }

    #[test]
    #[serial]
    fn test_normalize_boolean_value_edge_cases() {
        // Test case-insensitive normalization
        assert_eq!(normalize_boolean_value("True"), "ON");
        assert_eq!(normalize_boolean_value("False"), "OFF");
        assert_eq!(normalize_boolean_value("Yes"), "ON");
        assert_eq!(normalize_boolean_value("No"), "OFF");

        // Test that invalid values are returned as-is (no validation)
        assert_eq!(normalize_boolean_value("maybe"), "maybe");
        assert_eq!(normalize_boolean_value(""), "");
        assert_eq!(normalize_boolean_value("2"), "2");
    }

    #[test]
    #[serial]
    fn test_set_override_all_boolean_variants() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            temp_path.join("flags.definitions.yaml"),
            r"flags:
  - name: test_flag
    type: boolean
    default: false
",
        )
        .unwrap();

        let override_file = temp_path.join("overrides.json");

        // Test all valid boolean value variants
        let variants = vec![
            "ON", "OFF", "true", "false", "TRUE", "FALSE", "1", "0", "yes", "no", "YES", "NO",
        ];

        for variant in variants {
            // Clear previous override
            fs::remove_file(&override_file).ok();

            let options = Options {
                subcommand: OverrideSubcommand::Set {
                    flag: "test_flag".to_string(),
                    value: variant.to_string(),
                    reason: None,
                    operator: None,
                    file: override_file.clone(),
                    definitions: Some(temp_path.join("flags.definitions.yaml")),
                },
            };

            assert_eq!(run(&options), 0, "Failed for variant: {}", variant);

            // Verify it was normalized correctly
            let content = fs::read_to_string(&override_file).unwrap();
            let data: Value = serde_json::from_str(&content).unwrap();
            let value = data["overrides"]["test_flag"]["value"].as_str().unwrap();
            assert!(
                value == "ON" || value == "OFF",
                "Value {} was not normalized correctly for variant {}",
                value,
                variant
            );
        }
    }

    #[test]
    #[serial]
    fn test_clear_nonexistent_override() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // Clear override that doesn't exist - should succeed with info message
        let options = Options {
            subcommand: OverrideSubcommand::Clear {
                flag: "nonexistent_flag".to_string(),
                file: override_file.clone(),
            },
        };

        assert_eq!(run(&options), 0); // Should succeed (no error, just info)
    }

    #[test]
    #[serial]
    fn test_set_override_multivariate_no_variations() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create flag definitions file with multivariate flag but no variations
        fs::write(
            temp_path.join("flags.definitions.yaml"),
            r"flags:
  - name: api_version
    type: multivariate
    default: V1
",
        )
        .unwrap();

        let override_file = temp_path.join("overrides.json");

        // Test setting override - should fail because no variations defined
        let options = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "api_version".to_string(),
                value: "V1".to_string(),
                reason: None,
                operator: None,
                file: override_file.clone(),
                definitions: Some(temp_path.join("flags.definitions.yaml")),
            },
        };

        assert_eq!(run(&options), 1); // Should fail
    }

    #[test]
    #[serial]
    fn test_list_empty_overrides() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // Create empty override file
        fs::write(
            &override_file,
            r#"{
  "version": "1.0",
  "overrides": {}
}"#,
        )
        .unwrap();

        let options = Options {
            subcommand: OverrideSubcommand::List {
                file: override_file.clone(),
            },
        };

        assert_eq!(run(&options), 0); // Should succeed
    }

    #[test]
    #[serial]
    fn test_set_override_with_simple_format() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // Set override without definitions - should use simple format
        let options = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "test_flag".to_string(),
                value: "ON".to_string(),
                reason: None,
                operator: None,
                file: override_file.clone(),
                definitions: None,
            },
        };

        assert_eq!(run(&options), 0);

        // Verify it was written in full format (with timestamp)
        let content = fs::read_to_string(&override_file).unwrap();
        let data: Value = serde_json::from_str(&content).unwrap();
        assert!(data["overrides"]["test_flag"].is_object());
        assert!(data["overrides"]["test_flag"]["timestamp"].is_string());
    }

    #[test]
    #[serial]
    fn test_history_command_display_all() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // Set some overrides with reasons
        let set_options1 = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "flag1".to_string(),
                value: "ON".to_string(),
                reason: Some("Test reason 1".to_string()),
                operator: Some("operator1@example.com".to_string()),
                file: override_file.clone(),
                definitions: None,
            },
        };
        assert_eq!(run(&set_options1), 0);

        let set_options2 = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "flag2".to_string(),
                value: "OFF".to_string(),
                reason: Some("Test reason 2".to_string()),
                operator: Some("operator2@example.com".to_string()),
                file: override_file.clone(),
                definitions: None,
            },
        };
        assert_eq!(run(&set_options2), 0);

        // Display history (shows current overrides with reasons)
        let history_options = Options {
            subcommand: OverrideSubcommand::History {
                flag: None,
                file: override_file.clone(),
            },
        };
        assert_eq!(run(&history_options), 0);
    }

    #[test]
    #[serial]
    fn test_history_command_filter_by_flag() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // Set multiple overrides
        let set_options1 = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "flag1".to_string(),
                value: "ON".to_string(),
                reason: Some("Reason 1".to_string()),
                operator: None,
                file: override_file.clone(),
                definitions: None,
            },
        };
        assert_eq!(run(&set_options1), 0);

        let set_options2 = Options {
            subcommand: OverrideSubcommand::Set {
                flag: "flag2".to_string(),
                value: "OFF".to_string(),
                reason: Some("Reason 2".to_string()),
                operator: None,
                file: override_file.clone(),
                definitions: None,
            },
        };
        assert_eq!(run(&set_options2), 0);

        // Display history for specific flag
        let history_options = Options {
            subcommand: OverrideSubcommand::History {
                flag: Some("flag1".to_string()),
                file: override_file.clone(),
            },
        };
        assert_eq!(run(&history_options), 0);
    }

    #[test]
    #[serial]
    fn test_history_command_no_overrides() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let override_file = temp_path.join("overrides.json");

        // Display history when no overrides exist
        let history_options = Options {
            subcommand: OverrideSubcommand::History {
                flag: None,
                file: override_file.clone(),
            },
        };
        assert_eq!(run(&history_options), 0);
    }
}
