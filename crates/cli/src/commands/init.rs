//! Init command implementation

use crate::error::{CliError, CliResult};
use std::fs;
use std::path::Path;

pub struct Options {
    pub force: bool,
    pub example_flags: bool,
    pub no_examples: bool,
}

fn check_existing_project() -> bool {
    Path::new("flags.definitions.yaml").exists() || Path::new(".controlpath").exists()
}

fn ensure_controlpath_directory() -> CliResult<()> {
    fs::create_dir_all(".controlpath").map_err(CliError::from)
}

fn create_definitions_file() -> CliResult<()> {
    let definitions_content = r"flags:
  - name: example_flag
    type: boolean
    defaultValue: false
    description: An example feature flag
";
    fs::write("flags.definitions.yaml", definitions_content).map_err(CliError::from)
}

fn create_deployment_file() -> CliResult<()> {
    let deployment_content = r"environment: production
rules:
  example_flag:
    rules:
      - serve: false
";
    fs::write(
        ".controlpath/production.deployment.yaml",
        deployment_content,
    )
    .map_err(CliError::from)
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(created_definitions) => {
            println!("✓ Project initialized");
            if created_definitions {
                println!("  Created flags.definitions.yaml");
            }
            println!("  Created .controlpath/production.deployment.yaml");
            println!();
            println!("Next steps:");
            println!("  1. Validate your files: controlpath validate");
            println!("  2. Compile AST: controlpath compile --env production");
            println!("  3. Add more flags: Edit flags.definitions.yaml");
            0
        }
        Err(e) => {
            eprintln!("✗ Initialization failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<bool> {
    let has_existing_files = check_existing_project();

    if has_existing_files && !options.force {
        return Err(CliError::Message(
            "Project already initialized. Use --force to overwrite existing files".to_string(),
        ));
    }

    ensure_controlpath_directory()?;

    // Create definitions file if:
    // - example_flags is explicitly set to true, OR
    // - no_examples is false (default behavior is to create examples)
    let create_definitions = options.example_flags || !options.no_examples;
    if create_definitions {
        create_definitions_file()?;
    }

    create_deployment_file()?;

    Ok(create_definitions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_check_existing_project() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let original_dir = std::env::current_dir().unwrap();

        // Should return false when no files exist
        std::env::set_current_dir(temp_path).unwrap();
        assert!(!check_existing_project());

        // Create definitions file (using relative path since we changed directory)
        fs::write("flags.definitions.yaml", "flags: []").unwrap();
        assert!(check_existing_project());

        // Remove and create .controlpath directory
        fs::remove_file("flags.definitions.yaml").ok();
        fs::create_dir_all(".controlpath").unwrap();
        assert!(check_existing_project());

        // Restore original directory (ignore errors if directory no longer exists)
        let _ = std::env::set_current_dir(&original_dir);
    }

    #[test]
    #[serial]
    fn test_init_command_success() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let original_dir = std::env::current_dir().unwrap();

        // Change to temp directory right before running command
        std::env::set_current_dir(temp_path).unwrap();

        let options = Options {
            force: false,
            example_flags: true,
            no_examples: false,
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);

        // Immediately check files while still in temp directory
        // Use absolute paths to avoid issues with parallel test execution
        let definitions_path = temp_path.join("flags.definitions.yaml");
        let deployment_path = temp_path.join(".controlpath/production.deployment.yaml");
        assert!(
            definitions_path.exists(),
            "flags.definitions.yaml should exist at {definitions_path:?}"
        );
        assert!(
            deployment_path.exists(),
            "production.deployment.yaml should exist at {deployment_path:?}"
        );

        // Restore original directory (ignore errors if directory no longer exists)
        let _ = std::env::set_current_dir(&original_dir);
    }

    #[test]
    #[serial]
    fn test_init_command_with_force() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let original_dir = std::env::current_dir().unwrap();

        // Change to temp directory
        std::env::set_current_dir(temp_path).unwrap();

        // Create existing file (using relative path since we changed directory)
        fs::write("flags.definitions.yaml", "flags: []").unwrap();

        let options = Options {
            force: true,
            example_flags: true,
            no_examples: false,
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);

        // Use absolute path to avoid issues with parallel test execution
        let definitions_path = temp_path.join("flags.definitions.yaml");
        assert!(
            definitions_path.exists(),
            "flags.definitions.yaml should exist at {definitions_path:?}"
        );

        // Restore original directory (ignore errors if directory no longer exists)
        let _ = std::env::set_current_dir(&original_dir);
    }

    #[test]
    #[serial]
    fn test_init_command_without_examples() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let original_dir = std::env::current_dir().unwrap();

        // Change to temp directory
        std::env::set_current_dir(temp_path).unwrap();

        // Verify temp directory doesn't have the definitions file before running
        assert!(
            !Path::new("flags.definitions.yaml").exists(),
            "Temp directory should not have flags.definitions.yaml before test"
        );

        let options = Options {
            force: false,
            example_flags: false,
            no_examples: true,
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0, "Init command should succeed");

        // Use absolute paths to avoid issues with parallel test execution
        let definitions_path = temp_path.join("flags.definitions.yaml");
        let deployment_path = temp_path.join(".controlpath/production.deployment.yaml");
        assert!(
            !definitions_path.exists(),
            "flags.definitions.yaml should not be created when --no-examples is set, but found at {definitions_path:?}"
        );
        assert!(
            deployment_path.exists(),
            "production.deployment.yaml should be created at {deployment_path:?}"
        );

        // Restore original directory (ignore errors if directory no longer exists)
        let _ = std::env::set_current_dir(&original_dir);
    }
}
