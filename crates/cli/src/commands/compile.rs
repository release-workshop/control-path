//! Compile command implementation

use crate::error::{CliError, CliResult};
use crate::ops::compile::{self, CompileOptions};
use crate::utils::atomic_write::atomic_write;
use crate::utils::runtime;
use std::path::PathBuf;

pub struct Options {
    pub env: Option<String>,
    pub output: Option<String>,
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(output_path) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "ok",
                        "command": "compile",
                        "artifact": output_path.display().to_string()
                    })
                );
            }
            0
        }
        Err(e) => {
            if runtime::is_json_output() {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "command": "compile",
                        "error": e.to_string()
                    })
                );
            } else {
                eprintln!("✗ Compilation failed");
                eprintln!("  Error: {e}");
            }
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<PathBuf> {
    let env = options
        .env
        .as_ref()
        .ok_or_else(|| CliError::Message("Use --env <env> with control-path.yaml".to_string()))?;

    compile::compile_envs(&CompileOptions {
        envs: Some(vec![env.to_string()]),
        skip_validation: false,
    })?;

    let default_output = PathBuf::from(format!(".controlpath/{env}.ast"));
    let output_path = options
        .output
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or(default_output.clone());

    if output_path != default_output {
        let ast_bytes = std::fs::read(&default_output).map_err(|e| {
            CliError::Message(format!(
                "Failed to read compiled AST at {}: {e}",
                default_output.display()
            ))
        })?;
        atomic_write(&output_path, &ast_bytes).map_err(|e| {
            CliError::Message(format!(
                "Failed to write AST to {}: {e}",
                output_path.display()
            ))
        })?;
    }

    if !runtime::is_json_output() {
        println!("✓ Compiled to {}", output_path.display());
        println!();
        println!("Next steps:");
        println!("  • Test flag evaluation: controlpath explain --flag <flag-name> --env {env}");
        println!("  • Deploy changes:      controlpath deploy --env {env}");
    }

    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{write_v2_test_catalog, DirGuard};
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_compile_command_success() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::create_dir_all(".controlpath").unwrap();

        write_v2_test_catalog("test_flag", true);

        let output_path = temp_path.join("production.ast");

        let options = Options {
            env: Some("production".to_string()),
            output: Some(output_path.to_str().unwrap().to_string()),
        };

        let exit_code = run(&options);

        assert_eq!(exit_code, 0);
        assert!(output_path.exists());
    }
}
