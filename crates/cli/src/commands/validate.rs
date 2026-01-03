//! Validate command implementation

use crate::error::{CliError, CliResult};
use controlpath_compiler::{
    parse_definitions, parse_deployment, validate_definitions, validate_deployment,
};
use std::fs;
use std::path::PathBuf;

pub struct Options {
    pub definitions: Option<String>,
    pub deployment: Option<String>,
    pub env: Option<String>,
    pub all: bool,
}

#[derive(Debug, Clone)]
enum FileToValidate {
    Definitions(PathBuf),
    Deployment(PathBuf),
}

fn collect_files_from_options(options: &Options) -> Vec<FileToValidate> {
    let mut files = Vec::new();

    if let Some(ref definitions) = options.definitions {
        files.push(FileToValidate::Definitions(PathBuf::from(definitions)));
    }

    if let Some(ref deployment) = options.deployment {
        files.push(FileToValidate::Deployment(PathBuf::from(deployment)));
    }

    if let Some(ref env) = options.env {
        files.push(FileToValidate::Deployment(PathBuf::from(format!(
            ".controlpath/{env}.deployment.yaml"
        ))));
    }

    files
}

fn find_definitions_file(files: &mut Vec<FileToValidate>) {
    let path = PathBuf::from("flags.definitions.yaml");
    if path.exists() {
        files.push(FileToValidate::Definitions(path));
    }
}

fn find_deployment_files(files: &mut Vec<FileToValidate>) {
    let controlpath_dir = PathBuf::from(".controlpath");
    if let Ok(entries) = fs::read_dir(&controlpath_dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".deployment.yaml") {
                        files.push(FileToValidate::Deployment(entry.path()));
                    }
                }
            }
        }
    }
}

fn auto_detect_files() -> Vec<FileToValidate> {
    let mut files = Vec::new();
    find_definitions_file(&mut files);
    find_deployment_files(&mut files);
    files
}

fn validate_file(file: &FileToValidate) -> CliResult<()> {
    match file {
        FileToValidate::Definitions(path) => {
            let content = fs::read_to_string(path).map_err(|e| {
                CliError::Message(format!("Failed to read {}: {e}", path.display()))
            })?;
            let definitions = parse_definitions(&content)?;
            validate_definitions(&definitions)?;
            Ok(())
        }
        FileToValidate::Deployment(path) => {
            let content = fs::read_to_string(path).map_err(|e| {
                CliError::Message(format!("Failed to read {}: {e}", path.display()))
            })?;
            let deployment = parse_deployment(&content)?;
            validate_deployment(&deployment)?;
            Ok(())
        }
    }
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(valid_count) => {
            if valid_count > 0 {
                println!(
                    "✓ Validation passed ({} file{})",
                    valid_count,
                    if valid_count > 1 { "s" } else { "" }
                );
                0
            } else {
                eprintln!("✗ No files to validate");
                eprintln!("  Use --definitions <file> or --deployment <file> to specify files");
                eprintln!("  Or run in a directory with flags.definitions.yaml or .controlpath/*.deployment.yaml");
                1
            }
        }
        Err(e) => {
            eprintln!("✗ Validation failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<usize> {
    // Collect files to validate
    let mut files_to_validate = collect_files_from_options(options);

    // Auto-detect if no flags provided or --all flag is used
    if files_to_validate.is_empty() || options.all {
        let auto_detected = auto_detect_files();
        files_to_validate.extend(auto_detected);
    }

    if files_to_validate.is_empty() {
        return Err(CliError::Message(
            "No files to validate. Use --definitions <file> or --deployment <file> to specify files, or run in a directory with flags.definitions.yaml or .controlpath/*.deployment.yaml".to_string(),
        ));
    }

    // Validate each file
    let mut valid_count = 0;
    let mut has_errors = false;

    for file in &files_to_validate {
        match validate_file(file) {
            Ok(()) => {
                valid_count += 1;
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

    Ok(valid_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_collect_files_from_options() {
        let options = Options {
            definitions: Some("test.definitions.yaml".to_string()),
            deployment: Some("test.deployment.yaml".to_string()),
            env: None,
            all: false,
        };
        let files = collect_files_from_options(&options);
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_collect_files_with_env() {
        let options = Options {
            definitions: None,
            deployment: None,
            env: Some("production".to_string()),
            all: false,
        };
        let files = collect_files_from_options(&options);
        assert_eq!(files.len(), 1);
        match &files[0] {
            FileToValidate::Deployment(path) => {
                assert!(path.to_str().unwrap().contains("production"));
            }
            FileToValidate::Definitions(_) => panic!("Expected deployment file"),
        }
    }

    #[test]
    fn test_validate_command_success() {
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

        let options = Options {
            definitions: Some(definitions_path.to_str().unwrap().to_string()),
            deployment: Some(deployment_path.to_str().unwrap().to_string()),
            env: None,
            all: false,
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
    }
}
