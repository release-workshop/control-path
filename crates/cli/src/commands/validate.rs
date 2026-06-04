//! Validate command implementation

use crate::error::{CliError, CliResult};
use crate::utils::catalog::{self, emit_catalog_warnings};
use crate::utils::runtime;
use controlpath_compiler::validator::error::ValidationWarning;
use std::collections::BTreeSet;
use std::env;

pub struct Options {
    pub env: Option<String>,
    pub all: bool,
}

#[derive(Debug, Clone)]
enum FileToValidate {
    UnifiedConfig,
    Environment(String),
}

fn collect_files_from_options(options: &Options) -> Vec<FileToValidate> {
    let mut files = Vec::new();

    if let Some(ref env) = options.env {
        files.push(FileToValidate::Environment(env.clone()));
    } else if options.all {
        files.push(FileToValidate::UnifiedConfig);
    }

    files
}

fn auto_detect_files() -> Vec<FileToValidate> {
    vec![FileToValidate::UnifiedConfig]
}

fn validate_file(file: &FileToValidate) -> CliResult<Vec<ValidationWarning>> {
    let base_dir = env::current_dir()
        .map_err(|e| CliError::Message(format!("Failed to resolve working directory: {e}")))?;

    if let FileToValidate::Environment(env) = file {
        let unified = crate::utils::unified_config::read_unified_config()?;
        let envs = crate::utils::unified_config::get_environments(&unified);
        if !envs.iter().any(|e| e == env) {
            return Err(CliError::Message(format!(
                "Environment '{env}' not found in control-path.yaml"
            )));
        }
    }

    let bundle = catalog::load_for_explain(&base_dir)?;
    Ok(bundle.warnings)
}

fn dedupe_warnings(warnings: Vec<ValidationWarning>) -> Vec<ValidationWarning> {
    let mut seen = BTreeSet::new();
    warnings
        .into_iter()
        .filter(|w| seen.insert((w.file.clone(), w.message.clone(), w.path.clone())))
        .collect()
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok((valid_count, warnings)) => {
            if runtime::is_json_output() {
                let warning_values: Vec<serde_json::Value> = warnings
                    .iter()
                    .map(|w| {
                        serde_json::json!({
                            "file": w.file,
                            "message": w.message,
                            "path": w.path,
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::json!({
                        "status": if valid_count > 0 { "ok" } else { "error" },
                        "command": "validate",
                        "validated": valid_count,
                        "warnings": warning_values,
                        "error": if valid_count == 0 { Some("No files to validate") } else { None::<&str> }
                    })
                );
                return if valid_count > 0 { 0 } else { 1 };
            }
            emit_catalog_warnings(&warnings);
            if valid_count > 0 {
                println!(
                    "✓ Validation passed ({} file{})",
                    valid_count,
                    if valid_count > 1 { "s" } else { "" }
                );
                println!();
                println!("Next steps:");
                println!("  • Compile ASTs:        controlpath compile");
                println!("  • Generate SDK:        controlpath generate-sdk");
                println!("  • Deploy changes:      controlpath deploy");
                0
            } else {
                eprintln!("✗ No files to validate");
                eprintln!("  Run in a directory with control-path.yaml");
                1
            }
        }
        Err(e) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "command": "validate",
                        "error": e.to_string()
                    })
                );
            } else {
                eprintln!("✗ Validation failed");
                eprintln!("  Error: {e}");
            }
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<(usize, Vec<ValidationWarning>)> {
    let mut files_to_validate = collect_files_from_options(options);

    if files_to_validate.is_empty() || options.all {
        let auto_detected = auto_detect_files();
        files_to_validate.extend(auto_detected);
    }

    if files_to_validate.is_empty() {
        return Err(CliError::Message(
            "No files to validate. Run in a directory with control-path.yaml or pass --env/--all"
                .to_string(),
        ));
    }

    let mut valid_count = 0;
    let mut has_errors = false;
    let mut all_warnings = Vec::new();

    for file in &files_to_validate {
        match validate_file(file) {
            Ok(warnings) => {
                valid_count += 1;
                all_warnings.extend(warnings);
            }
            Err(e) => {
                eprintln!("✗ Failed to validate {file:?}");
                eprintln!("  Error: {e}");
                has_errors = true;
            }
        }
    }

    if has_errors {
        return Err(CliError::Message(
            "One or more files failed validation".to_string(),
        ));
    }

    Ok((valid_count, dedupe_warnings(all_warnings)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_collect_files_from_options() {
        let options = Options {
            env: None,
            all: false,
        };
        let files = collect_files_from_options(&options);
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_collect_files_with_env() {
        let options = Options {
            env: Some("production".to_string()),
            all: false,
        };
        let files = collect_files_from_options(&options);
        assert_eq!(files.len(), 1);
        match &files[0] {
            FileToValidate::Environment(env) => {
                assert_eq!(env, "production");
            }
            _ => panic!("Expected environment"),
        }
    }

    #[test]
    #[serial]
    fn test_validate_command_success() {
        use crate::test_helpers::DirGuard;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test-service
mode: local
flags:
  test_flag:
    default: false
    kind: release
environments:
  test:
    rules:
      test_flag:
        - serve: true
",
        )
        .unwrap();

        let options = Options {
            env: Some("test".to_string()),
            all: false,
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }
}
