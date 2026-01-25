//! TypeScript SDK generator
//!
//! Generates type-safe TypeScript SDKs from flag definitions.

use crate::error::{CliError, CliResult};
use crate::generator::Generator;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tera::{Context, Tera};

/// TypeScript SDK generator
pub struct TypeScriptGenerator {
    tera: Tera,
}

#[derive(Debug, Serialize)]
struct VariationType {
    name: String,
    values: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FlagInfo {
    camel_name: String,
    method_name: String,
    snake_name: String,
    flag_type: String,
    return_type: String,
    default_value: String,
    default_string: String,
}

impl TypeScriptGenerator {
    pub fn new() -> Result<Self, CliError> {
        // Always use embedded templates by writing them to temporary files
        // This ensures consistency and works in all environments (including tests)
        // Use a unique directory per instance to avoid race conditions in concurrent tests
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let temp_dir = std::env::temp_dir();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let unique_id = format!("{}_{}", timestamp, counter);
        let temp_template_dir = temp_dir.join(format!("controlpath_templates_{}", unique_id));
        if fs::create_dir_all(&temp_template_dir).is_err() {
            return Err(CliError::Message(
                "Failed to create temporary template directory".to_string(),
            ));
        }

        let types_template_path = temp_template_dir.join("types.ts.tera");
        let index_template_path = temp_template_dir.join("index.ts.tera");

        fs::write(
            &types_template_path,
            include_str!("templates/types.ts.tera"),
        )
        .map_err(|e| {
            CliError::Message(format!(
                "Failed to write temporary types.ts template: {}",
                e
            ))
        })?;
        fs::write(
            &index_template_path,
            include_str!("templates/index.ts.tera"),
        )
        .map_err(|e| {
            CliError::Message(format!(
                "Failed to write temporary index.ts template: {}",
                e
            ))
        })?;

        let pattern = temp_template_dir.to_string_lossy().replace('\\', "/") + "/**/*.tera";
        let mut tera = Tera::new(&pattern).map_err(|e| {
            CliError::Message(format!("Failed to initialize Tera with templates: {}", e))
        })?;

        // Auto-escape is disabled for TypeScript code generation
        tera.autoescape_on(vec![]);

        Ok(Self { tera })
    }

    /// Convert snake_case to camelCase
    fn to_camel_case(s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;
        for c in s.chars() {
            if c == '_' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(c.to_uppercase().next().unwrap_or(c));
                capitalize_next = false;
            } else {
                result.push(c);
            }
        }
        result
    }

    /// Get variation type name from flag name
    fn get_variation_type_name(flag_name: &str) -> String {
        let camel = Self::to_camel_case(flag_name);
        let mut chars = camel.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str() + "Variation",
        }
    }

    /// Format default value for TypeScript code
    fn format_default_value(value: &Value) -> String {
        match value {
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => {
                // Check if it's a boolean string
                match s.as_str() {
                    "ON" | "true" => "true".to_string(),
                    "OFF" | "false" => "false".to_string(),
                    _ => format!("'{}'", s),
                }
            }
            _ => "false".to_string(), // Fallback
        }
    }

    /// Get default value from flag definition
    fn get_default_value(flag: &Value) -> &Value {
        flag.get("defaultValue").unwrap_or(&Value::Bool(false))
    }

    /// Extract flag information from definitions
    fn extract_flag_info(flag: &Value) -> Option<FlagInfo> {
        let flag_name = flag.get("name")?.as_str()?;
        let camel_name = Self::to_camel_case(flag_name);
        let flag_type = flag
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("boolean");
        let default_value = Self::get_default_value(flag);
        let default_value_str = Self::format_default_value(default_value);
        let default_string = default_value.as_str().unwrap_or("").to_string();
        let return_type = if flag_type == "boolean" {
            "boolean".to_string()
        } else {
            Self::get_variation_type_name(flag_name)
        };

        Some(FlagInfo {
            camel_name: camel_name.clone(),
            method_name: camel_name,
            snake_name: flag_name.to_string(),
            flag_type: flag_type.to_string(),
            return_type,
            default_value: default_value_str,
            default_string,
        })
    }

    /// Generate types.ts file
    fn generate_types(&self, definitions: &Value) -> Result<String, CliError> {
        let empty_flags: Vec<Value> = Vec::new();
        let flags = definitions
            .get("flags")
            .and_then(|f| f.as_array())
            .unwrap_or(&empty_flags);

        // Extract variation types
        let mut variation_types = Vec::new();
        for flag in flags {
            if let Some(flag_type) = flag.get("type").and_then(|t| t.as_str()) {
                if flag_type == "multivariate" {
                    if let Some(variations) = flag.get("variations").and_then(|v| v.as_array()) {
                        if let Some(flag_name) = flag.get("name").and_then(|n| n.as_str()) {
                            let variation_type_name = Self::get_variation_type_name(flag_name);
                            let variation_values: Vec<String> = variations
                                .iter()
                                .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
                                .map(|n| format!("'{}'", n))
                                .collect();
                            if !variation_values.is_empty() {
                                variation_types.push(VariationType {
                                    name: variation_type_name,
                                    values: variation_values,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Extract flag names
        let flag_names: Vec<String> = flags
            .iter()
            .filter_map(|f| f.get("name").and_then(|n| n.as_str()))
            .map(Self::to_camel_case)
            .map(|n| format!("'{}'", n))
            .collect();

        // Extract flag info
        let flag_infos: Vec<FlagInfo> = flags.iter().filter_map(Self::extract_flag_info).collect();

        let mut tera_context = Context::new();
        tera_context.insert("variation_types", &variation_types);
        tera_context.insert("flag_names", &flag_names);
        tera_context.insert("flags", &flag_infos);

        self.tera
            .render("types.ts.tera", &tera_context)
            .map_err(|e| CliError::Message(format!("Failed to render types.ts template: {e}")))
    }

    /// Generate index.ts file (evaluator class)
    fn generate_evaluator(&self, definitions: &Value) -> Result<String, CliError> {
        let empty_flags: Vec<Value> = Vec::new();
        let flags = definitions
            .get("flags")
            .and_then(|f| f.as_array())
            .unwrap_or(&empty_flags);

        // Extract variation types
        let mut variation_types = Vec::new();
        for flag in flags {
            if let Some(flag_type) = flag.get("type").and_then(|t| t.as_str()) {
                if flag_type == "multivariate" {
                    if let Some(variations) = flag.get("variations").and_then(|v| v.as_array()) {
                        if let Some(flag_name) = flag.get("name").and_then(|n| n.as_str()) {
                            let variation_type_name = Self::get_variation_type_name(flag_name);
                            let variation_values: Vec<String> = variations
                                .iter()
                                .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
                                .map(|n| format!("'{}'", n))
                                .collect();
                            if !variation_values.is_empty() {
                                variation_types.push(VariationType {
                                    name: variation_type_name,
                                    values: variation_values,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Extract flag info
        let flag_infos: Vec<FlagInfo> = flags.iter().filter_map(Self::extract_flag_info).collect();

        // Extract flag names for evaluateAll
        let flag_names: Vec<String> = flag_infos
            .iter()
            .map(|f| format!("'{}'", f.camel_name))
            .collect();

        let mut tera_context = Context::new();
        tera_context.insert("has_variations", &!variation_types.is_empty());
        tera_context.insert("variation_types", &variation_types);
        tera_context.insert("flags", &flag_infos);
        tera_context.insert("flag_names", &flag_names);

        self.tera
            .render("index.ts.tera", &tera_context)
            .map_err(|e| CliError::Message(format!("Failed to render index.ts template: {e}")))
    }

    /// Generate package.json for the generated SDK
    fn generate_package_json(&self) -> String {
        r#"{
  "name": "generated-flags",
  "version": "0.1.0",
  "main": "index.js",
  "types": "index.d.ts",
  "dependencies": {
    "@controlpath/runtime": "^0.1.0"
  }
}"#
        .to_string()
    }
}

impl Generator for TypeScriptGenerator {
    fn generate(&self, definitions: &Value, output_dir: &Path) -> CliResult<()> {
        // Create output directory if it doesn't exist
        fs::create_dir_all(output_dir)
            .map_err(|e| CliError::Message(format!("Failed to create output directory: {e}")))?;

        // Generate types.ts
        let types_content = self.generate_types(definitions)?;
        fs::write(output_dir.join("types.ts"), types_content)
            .map_err(|e| CliError::Message(format!("Failed to write types.ts: {e}")))?;

        // Generate index.ts
        let index_content = self.generate_evaluator(definitions)?;
        fs::write(output_dir.join("index.ts"), index_content)
            .map_err(|e| CliError::Message(format!("Failed to write index.ts: {e}")))?;

        // Generate package.json
        let package_json_content = self.generate_package_json();
        fs::write(output_dir.join("package.json"), package_json_content)
            .map_err(|e| CliError::Message(format!("Failed to write package.json: {e}")))?;

        Ok(())
    }
}
