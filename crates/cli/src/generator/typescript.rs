//! TypeScript SDK generator
//!
//! Generates type-safe boolean-only TypeScript SDKs from v2 catalog projections.

use crate::error::{CliError, CliResult};
use crate::generator::Generator;
use crate::utils::atomic_write::atomic_write_string;
use controlpath_compiler::{FlagLifecycle, SdkCatalog, SdkFlag};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tera::{Context, Tera};

/// TypeScript SDK generator
pub struct TypeScriptGenerator {
    tera: Tera,
}

#[derive(Debug, Serialize)]
struct TemplateFlag {
    camel_name: String,
    method_name: String,
    qualified_name: String,
    default_value: String,
    is_deprecated: bool,
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct TemplateKillSwitchUrl {
    env: String,
    /// JSON-encoded string literal safe for TypeScript (handles quotes in URLs).
    url_json: String,
}

#[derive(Debug, Serialize)]
struct TemplateArtifactUrl {
    env: String,
    url_json: String,
}

pub(crate) fn to_ts_string_literal(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

#[cfg(test)]
mod tests {
    use super::to_ts_string_literal;

    #[test]
    fn to_ts_string_literal_escapes_single_quotes_in_urls() {
        let literal = to_ts_string_literal("https://flags.example.com/o'reilly/rules.ast");
        assert_eq!(literal, "\"https://flags.example.com/o'reilly/rules.ast\"");
    }
}

impl TypeScriptGenerator {
    pub fn new() -> Result<Self, CliError> {
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

        atomic_write_string(
            &types_template_path,
            include_str!("templates/types.ts.tera"),
        )
        .map_err(|e| {
            CliError::Message(format!(
                "Failed to write temporary types.ts template: {}",
                e
            ))
        })?;
        atomic_write_string(
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

        tera.autoescape_on(vec![]);

        Ok(Self { tera })
    }

    fn template_flags(catalog: &SdkCatalog) -> Vec<TemplateFlag> {
        catalog.flags.iter().map(Self::template_flag).collect()
    }

    fn template_flag(flag: &SdkFlag) -> TemplateFlag {
        TemplateFlag {
            camel_name: flag.sdk_method_name.clone(),
            method_name: flag.sdk_method_name.clone(),
            qualified_name: flag.qualified_name.clone(),
            default_value: flag.default.to_string(),
            is_deprecated: flag.lifecycle == FlagLifecycle::Deprecated,
            description: flag.description.clone(),
        }
    }

    fn generate_types(&self, catalog: &SdkCatalog) -> Result<String, CliError> {
        let flags = Self::template_flags(catalog);
        let flag_names: Vec<String> = flags
            .iter()
            .map(|flag| format!("'{}'", flag.camel_name))
            .collect();

        let mut tera_context = Context::new();
        tera_context.insert("flag_names", &flag_names);
        tera_context.insert("flags", &flags);

        self.tera
            .render("types.ts.tera", &tera_context)
            .map_err(|e| CliError::Message(format!("Failed to render types.ts template: {e}")))
    }

    fn template_kill_switch_urls(catalog: &SdkCatalog) -> Vec<TemplateKillSwitchUrl> {
        catalog
            .kill_switch_urls
            .iter()
            .map(|(env, url)| TemplateKillSwitchUrl {
                env: env.clone(),
                url_json: to_ts_string_literal(url),
            })
            .collect()
    }

    fn template_artifact_urls(catalog: &SdkCatalog) -> Vec<TemplateArtifactUrl> {
        catalog
            .artifact_urls
            .iter()
            .map(|(env, url)| TemplateArtifactUrl {
                env: env.clone(),
                url_json: to_ts_string_literal(url),
            })
            .collect()
    }

    fn generate_evaluator(&self, catalog: &SdkCatalog) -> Result<String, CliError> {
        let flags = Self::template_flags(catalog);
        let flag_names: Vec<String> = flags
            .iter()
            .map(|flag| format!("'{}'", flag.camel_name))
            .collect();
        let kill_switch_urls = Self::template_kill_switch_urls(catalog);
        let artifact_urls = Self::template_artifact_urls(catalog);

        let mut tera_context = Context::new();
        tera_context.insert("flags", &flags);
        tera_context.insert("flag_names", &flag_names);
        tera_context.insert("kill_switch_urls", &kill_switch_urls);
        tera_context.insert("artifact_urls", &artifact_urls);

        self.tera
            .render("index.ts.tera", &tera_context)
            .map_err(|e| CliError::Message(format!("Failed to render index.ts template: {e}")))
    }

    fn generate_package_json(&self) -> String {
        r#"{
  "name": "@controlpath/generated",
  "version": "0.1.0",
  "type": "module",
  "main": "index.ts",
  "types": "index.ts",
  "exports": {
    ".": {
      "types": "./index.ts",
      "import": "./index.ts",
      "default": "./index.ts"
    }
  },
  "dependencies": {
    "@controlpath/runtime": "^0.1.0"
  }
}"#
        .to_string()
    }
}

impl Generator for TypeScriptGenerator {
    fn generate(&self, catalog: &SdkCatalog, output_dir: &Path) -> CliResult<()> {
        fs::create_dir_all(output_dir)
            .map_err(|e| CliError::Message(format!("Failed to create output directory: {e}")))?;

        let types_content = self.generate_types(catalog)?;
        atomic_write_string(&output_dir.join("types.ts"), &types_content)
            .map_err(|e| CliError::Message(format!("Failed to write types.ts: {e}")))?;

        let index_content = self.generate_evaluator(catalog)?;
        atomic_write_string(&output_dir.join("index.ts"), &index_content)
            .map_err(|e| CliError::Message(format!("Failed to write index.ts: {e}")))?;

        let output_str = output_dir.to_string_lossy();
        if output_str.contains("node_modules") {
            let package_json_content = self.generate_package_json();
            atomic_write_string(&output_dir.join("package.json"), &package_json_content)
                .map_err(|e| CliError::Message(format!("Failed to write package.json: {e}")))?;
        }

        Ok(())
    }
}
