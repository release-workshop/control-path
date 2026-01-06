//! Test helpers for integration tests

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use tempfile::TempDir;

// CARGO_BIN_EXE_controlpath is set by Cargo when running integration tests
// This allows us to find the binary to test
const BINARY_NAME: &str = env!("CARGO_BIN_EXE_controlpath");

/// Test project setup helper
pub struct TestProject {
    #[allow(dead_code)] // Used to keep temp directory alive during tests
    pub temp_dir: TempDir,
    pub project_path: PathBuf,
}

impl Default for TestProject {
    fn default() -> Self {
        Self::new()
    }
}

impl TestProject {
    /// Create a new test project with basic structure
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();

        // Create .controlpath directory
        fs::create_dir_all(project_path.join(".controlpath")).unwrap();

        Self {
            temp_dir,
            project_path,
        }
    }

    /// Create a test project with definitions file
    pub fn with_definitions(definitions_content: &str) -> Self {
        let project = Self::new();
        fs::write(
            project.project_path.join("flags.definitions.yaml"),
            definitions_content,
        )
        .unwrap();
        project
    }

    /// Create a test project with definitions and deployment files
    #[allow(dead_code)] // Used across multiple test files
    pub fn with_deployment(definitions_content: &str, env: &str, deployment_content: &str) -> Self {
        let project = Self::with_definitions(definitions_content);
        fs::write(
            project
                .project_path
                .join(".controlpath")
                .join(format!("{}.deployment.yaml", env)),
            deployment_content,
        )
        .unwrap();
        project
    }

    /// Get path to a file in the project
    #[allow(dead_code)] // Used across multiple test files
    pub fn path(&self, relative_path: &str) -> PathBuf {
        self.project_path.join(relative_path)
    }

    /// Check if a file exists
    #[allow(dead_code)] // Used across multiple test files
    pub fn file_exists(&self, relative_path: &str) -> bool {
        self.path(relative_path).exists()
    }

    /// Read file content
    #[allow(dead_code)] // Used across multiple test files
    pub fn read_file(&self, relative_path: &str) -> String {
        fs::read_to_string(self.path(relative_path)).unwrap()
    }

    /// Write file content
    #[allow(dead_code)] // Used across multiple test files
    pub fn write_file(&self, relative_path: &str, content: &str) {
        let path = self.path(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    /// Run controlpath command and return output
    pub fn run_command(&self, args: &[&str]) -> Output {
        let mut cmd = Command::new(BINARY_NAME);
        cmd.current_dir(&self.project_path);
        cmd.args(args);
        cmd.output().unwrap()
    }

    /// Run controlpath command and assert success
    #[allow(dead_code)] // Used across multiple test files
    pub fn run_command_success(&self, args: &[&str]) {
        let output = self.run_command(args);
        if !output.status.success() {
            eprintln!("Command failed: controlpath {}", args.join(" "));
            eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            panic!("Command failed with exit code: {:?}", output.status.code());
        }
    }

    /// Run controlpath command and assert failure
    #[allow(dead_code)] // Used across multiple test files
    pub fn run_command_failure(&self, args: &[&str]) -> Output {
        let output = self.run_command(args);
        assert!(!output.status.success(), "Command should have failed");
        output
    }

    /// Get definitions content
    #[allow(dead_code)] // Used across multiple test files
    pub fn get_definitions(&self) -> String {
        self.read_file("flags.definitions.yaml")
    }

    /// Get deployment content for an environment
    #[allow(dead_code)] // Used across multiple test files
    pub fn get_deployment(&self, env: &str) -> String {
        self.read_file(&format!(".controlpath/{}.deployment.yaml", env))
    }

    /// Check if AST file exists for environment
    #[allow(dead_code)] // Used across multiple test files
    pub fn ast_exists(&self, env: &str) -> bool {
        self.file_exists(&format!(".controlpath/{}.ast", env))
    }
}

/// Create a simple test flag definition
pub fn simple_flag_definition(flag_name: &str) -> String {
    format!(
        r"flags:
  - name: {}
    type: boolean
    defaultValue: false
",
        flag_name
    )
}

/// Create a simple deployment file
#[allow(dead_code)] // Used across multiple test files
pub fn simple_deployment(env: &str, flag_name: &str, serve: bool) -> String {
    format!(
        r"environment: {}
rules:
  {}:
    rules:
      - serve: {}
",
        env, flag_name, serve
    )
}

/// Create a deployment with a rule
#[allow(dead_code)] // May be used in future tests
pub fn deployment_with_rule(env: &str, flag_name: &str, when: &str, serve: bool) -> String {
    format!(
        r"environment: {}
rules:
  {}:
    rules:
      - when: {}
        serve: {}
",
        env, flag_name, when, serve
    )
}
