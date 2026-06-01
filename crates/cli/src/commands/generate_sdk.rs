//! Generate SDK command implementation

use crate::error::CliResult;
use crate::generator::generate_sdk;
use crate::utils::catalog;
use crate::utils::language;
use crate::utils::runtime;
use crate::utils::unified_config;
use std::env;
use std::path::PathBuf;

pub struct Options {
    pub lang: Option<String>,
    pub output: Option<String>,
}

fn determine_output_path(options: &Options, unified: Option<&serde_json::Value>) -> PathBuf {
    if let Some(ref output) = options.output {
        PathBuf::from(output)
    } else if let Some(config) = unified {
        if let Some(config_output) = unified_config::get_sdk_output_path(config) {
            PathBuf::from(config_output)
        } else {
            PathBuf::from("node_modules/@controlpath/generated")
        }
    } else {
        PathBuf::from("node_modules/@controlpath/generated")
    }
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(output_path) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "ok",
                        "command": "generate-sdk",
                        "artifacts": [output_path.display().to_string()],
                        "warnings": [],
                        "errors": []
                    })
                );
            } else {
                println!("✓ SDK generated successfully");
            }
            0
        }
        Err(e) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "command": "generate-sdk",
                        "artifacts": [],
                        "warnings": [],
                        "errors": [e.to_string()]
                    })
                );
            } else {
                eprintln!("✗ SDK generation failed");
                eprintln!("  Error: {e}");
            }
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<PathBuf> {
    let unified = unified_config::read_unified_config().ok();
    let base_dir = env::current_dir().map_err(|e| {
        crate::error::CliError::Message(format!("Failed to resolve working directory: {e}"))
    })?;
    let sdk_catalog = catalog::load_sdk_catalog_for_generate(&base_dir)?;
    let output_path = determine_output_path(options, unified.as_ref());
    let language = language::determine_language(options.lang.clone())?.to_lowercase();

    generate_sdk(&language, &sdk_catalog, &output_path)?;

    if !runtime::is_json_output() {
        println!("  Generated SDK to {}", output_path.display());
        println!();
        println!("Next steps:");
        println!(
            "  • Import SDK in your code: import {{ evaluator }} from '@controlpath/generated'"
        );
        println!(
            "  • Initialize evaluator:   await evaluator.init({{ artifact: './.controlpath/<env>.ast' }})"
        );
        println!("  • Use flags in code:       const enabled = await evaluator.<flagName>(user)");
    }
    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use crate::generator::generate_sdk;
    use controlpath_compiler::{build_sdk_catalog, parse_catalog};
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    #[test]
    fn test_generate_sdk() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("generated");

        let catalog = parse_catalog(
            r#"
catalog:
  id: svc
flags:
  test_flag:
    default: false
    kind: release
"#,
            Some("control-path.yaml"),
        )
        .unwrap();
        let sdk = build_sdk_catalog(&catalog, &BTreeMap::new()).unwrap();

        let result = generate_sdk("typescript", &sdk, &output_path);
        assert!(result.is_ok());
        assert!(output_path.join("index.ts").exists());
        assert!(output_path.join("types.ts").exists());
    }
}
