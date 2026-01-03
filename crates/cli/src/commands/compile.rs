//! Compile command implementation

use crate::error::{CliError, CliResult};
use controlpath_compiler::{
    compile, parse_definitions, parse_deployment, serialize, validate_definitions,
    validate_deployment,
};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Options {
    pub deployment: Option<String>,
    pub env: Option<String>,
    pub output: Option<String>,
    pub definitions: Option<String>,
}

fn determine_deployment_path(options: &Options) -> Result<PathBuf, CliError> {
    options.deployment.as_ref().map_or_else(
        || {
            options.env.as_ref().map_or_else(
                || {
                    Err(CliError::Message(
                        "Either --deployment <file> or --env <env> must be provided".to_string(),
                    ))
                },
                |env| Ok(PathBuf::from(format!(".controlpath/{env}.deployment.yaml"))),
            )
        },
        |deployment| Ok(PathBuf::from(deployment)),
    )
}

fn determine_output_path(options: &Options, deployment_path: &Path) -> PathBuf {
    options.output.as_ref().map_or_else(
        || {
            options.env.as_ref().map_or_else(
                || {
                    // Infer from deployment path
                    let deployment_dir = deployment_path.parent().unwrap_or_else(|| Path::new("."));
                    let deployment_stem = deployment_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("deployment")
                        .replace(".deployment", "");
                    deployment_dir.join(format!("{deployment_stem}.ast"))
                },
                |env| PathBuf::from(format!(".controlpath/{env}.ast")),
            )
        },
        PathBuf::from,
    )
}

fn determine_definitions_path(options: &Options) -> PathBuf {
    PathBuf::from(
        options
            .definitions
            .as_deref()
            .unwrap_or("flags.definitions.yaml"),
    )
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("✗ Compilation failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    // Determine file paths
    let deployment_path = determine_deployment_path(options)?;
    let output_path = determine_output_path(options, &deployment_path);
    let definitions_path = determine_definitions_path(options);

    // Read and parse definitions
    let definitions_content = fs::read_to_string(&definitions_path)
        .map_err(|e| CliError::Message(format!("Failed to read definitions file: {e}")))?;
    let definitions = parse_definitions(&definitions_content)?;

    // Validate definitions
    validate_definitions(&definitions)
        .map_err(|e| CliError::Message(format!("Definitions file is invalid: {e}")))?;

    // Read and parse deployment
    let deployment_content = fs::read_to_string(&deployment_path)
        .map_err(|e| CliError::Message(format!("Failed to read deployment file: {e}")))?;
    let deployment = parse_deployment(&deployment_content)?;

    // Validate deployment
    validate_deployment(&deployment)
        .map_err(|e| CliError::Message(format!("Deployment file is invalid: {e}")))?;

    // Compile to AST
    let artifact = compile(&deployment, &definitions)?;

    // Serialize to MessagePack
    let ast_bytes = serialize(&artifact)?;

    // Create output directory if needed
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Write AST file
    fs::write(&output_path, ast_bytes)?;

    println!("✓ Compiled to {}", output_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_determine_deployment_path_with_deployment() {
        let options = Options {
            deployment: Some("test.deployment.yaml".to_string()),
            env: None,
            output: None,
            definitions: None,
        };
        let path = determine_deployment_path(&options).unwrap();
        assert_eq!(path, PathBuf::from("test.deployment.yaml"));
    }

    #[test]
    fn test_determine_deployment_path_with_env() {
        let options = Options {
            deployment: None,
            env: Some("production".to_string()),
            output: None,
            definitions: None,
        };
        let path = determine_deployment_path(&options).unwrap();
        assert_eq!(
            path,
            PathBuf::from(".controlpath/production.deployment.yaml")
        );
    }

    #[test]
    fn test_determine_deployment_path_without_options() {
        let options = Options {
            deployment: None,
            env: None,
            output: None,
            definitions: None,
        };
        assert!(determine_deployment_path(&options).is_err());
    }

    #[test]
    fn test_determine_output_path_with_output() {
        let options = Options {
            deployment: None,
            env: None,
            output: Some("output.ast".to_string()),
            definitions: None,
        };
        let deployment_path = PathBuf::from("test.deployment.yaml");
        let path = determine_output_path(&options, &deployment_path);
        assert_eq!(path, PathBuf::from("output.ast"));
    }

    #[test]
    fn test_determine_output_path_with_env() {
        let options = Options {
            deployment: None,
            env: Some("production".to_string()),
            output: None,
            definitions: None,
        };
        let deployment_path = PathBuf::from("test.deployment.yaml");
        let path = determine_output_path(&options, &deployment_path);
        assert_eq!(path, PathBuf::from(".controlpath/production.ast"));
    }

    #[test]
    fn test_determine_output_path_inferred() {
        let options = Options {
            deployment: Some("test.deployment.yaml".to_string()),
            env: None,
            output: None,
            definitions: None,
        };
        let deployment_path = PathBuf::from("test.deployment.yaml");
        let path = determine_output_path(&options, &deployment_path);
        assert_eq!(path, PathBuf::from("test.ast"));
    }

    #[test]
    fn test_determine_definitions_path_with_option() {
        let options = Options {
            deployment: None,
            env: None,
            output: None,
            definitions: Some("custom.definitions.yaml".to_string()),
        };
        let path = determine_definitions_path(&options);
        assert_eq!(path, PathBuf::from("custom.definitions.yaml"));
    }

    #[test]
    fn test_determine_definitions_path_default() {
        let options = Options {
            deployment: None,
            env: None,
            output: None,
            definitions: None,
        };
        let path = determine_definitions_path(&options);
        assert_eq!(path, PathBuf::from("flags.definitions.yaml"));
    }

    #[test]
    fn test_compile_command_success() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test files
        let definitions_path = temp_path.join("flags.definitions.yaml");
        fs::write(
            &definitions_path,
            r"flags:
  - name: test_flag
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        let deployment_path = temp_path.join("test.deployment.yaml");
        fs::write(
            &deployment_path,
            r"environment: test
rules:
  test_flag:
    rules:
      - serve: true
",
        )
        .unwrap();

        let output_path = temp_path.join("test.ast");

        let options = Options {
            deployment: Some(deployment_path.to_str().unwrap().to_string()),
            env: None,
            output: Some(output_path.to_str().unwrap().to_string()),
            definitions: Some(definitions_path.to_str().unwrap().to_string()),
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
        assert!(output_path.exists());
    }
}
