//! Test helpers for integration tests

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

/// Guard for changing the current working directory in tests.
/// Automatically restores the original directory when dropped.
///
/// This is useful for tests that need to run in a temporary directory
/// but want to ensure cleanup happens even if the test panics.
///
/// # Example
///
/// ```rust,no_run
/// use tempfile::TempDir;
/// use integration_test_helpers::DirGuard;
///
/// let temp_dir = TempDir::new().unwrap();
/// let _guard = DirGuard::new(temp_dir.path()).unwrap();
/// // Now we're in temp_dir, and will be restored when _guard drops
/// ```
#[allow(dead_code)] // May be used by integration tests
pub struct DirGuard {
    original_dir: PathBuf,
}

impl DirGuard {
    /// Create a new DirGuard and change to the specified directory.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The directory doesn't exist and can't be created
    /// - The current directory can't be determined
    /// - The directory can't be changed to
    #[allow(dead_code)] // May be used by integration tests
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let path = path.as_ref();
        fs::create_dir_all(path)?;
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(path)?;
        Ok(DirGuard { original_dir })
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original_dir);
    }
}

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

        // Note: Don't create .controlpath directory here - let commands create it as needed
        // This allows setup command to work properly

        Self {
            temp_dir,
            project_path,
        }
    }

    /// Create a new test project with .controlpath directory (for tests that need it)
    #[allow(dead_code)] // May be used in future tests
    pub fn with_controlpath() -> Self {
        let project = Self::new();
        fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();
        project
    }

    /// Create a test project with config file
    pub fn with_definitions(definitions_content: &str) -> Self {
        let project = Self::new();
        fs::write(
            project.project_path.join("control-path.yaml"),
            definitions_content,
        )
        .unwrap();
        project
    }

    /// Create a test project with config (with_definitions is now for config)
    /// Also creates legacy files for commands that don't support config yet
    #[allow(dead_code)] // Used across multiple test files
    pub fn with_deployment(
        definitions_content: &str,
        env: &str,
        _deployment_content: &str,
    ) -> Self {
        // Create config
        let project = Self::with_definitions(definitions_content);

        // Also create legacy files for commands that don't support config yet (flag, env)
        // Parse config and extract flags for legacy format
        if let Ok(unified) = serde_yaml::from_str::<serde_json::Value>(definitions_content) {
            if let Some(flags) = unified.get("flags").and_then(|f| f.as_array()) {
                // Create legacy definitions format
                let mut legacy_flags = Vec::new();
                let mut deployment_rules = serde_json::Map::new();

                for flag in flags {
                    let mut legacy_flag = flag.clone();
                    if let Some(flag_name) = flag.get("name").and_then(|n| n.as_str()) {
                        if let Some(obj) = legacy_flag.as_object_mut() {
                            // Remove environments (not in definitions)
                            obj.remove("environments");
                            // Ensure both default and defaultValue exist
                            if let Some(default_val) = obj.get("default").cloned() {
                                if !obj.contains_key("defaultValue") {
                                    obj.insert("defaultValue".to_string(), default_val.clone());
                                }
                            }

                            // Add flag to deployment rules if it has environment rules
                            if let Some(envs) = flag.get("environments").and_then(|e| e.as_object())
                            {
                                if envs.contains_key(env) {
                                    // Flag has rules for this environment, add to deployment
                                    let mut flag_rules_obj = serde_json::Map::new();
                                    let mut rules_array = Vec::new();
                                    if let Some(env_rules) =
                                        envs.get(env).and_then(|r| r.as_array())
                                    {
                                        for rule in env_rules {
                                            rules_array.push(rule.clone());
                                        }
                                    }
                                    flag_rules_obj.insert(
                                        "rules".to_string(),
                                        serde_json::json!(rules_array),
                                    );
                                    deployment_rules.insert(
                                        flag_name.to_string(),
                                        serde_json::json!(flag_rules_obj),
                                    );
                                }
                            }
                        }
                    }
                    legacy_flags.push(legacy_flag);
                }

                let legacy_definitions = serde_yaml::to_string(&serde_json::json!({
                    "flags": legacy_flags
                }))
                .unwrap();
                fs::write(
                    project.project_path.join("flags.definitions.yaml"),
                    legacy_definitions,
                )
                .unwrap();

                // Also create legacy deployment file with rules
                fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();
                let deployment = serde_yaml::to_string(&serde_json::json!({
                    "environment": env,
                    "rules": deployment_rules
                }))
                .unwrap();
                fs::write(
                    project
                        .project_path
                        .join(".controlpath")
                        .join(format!("{}.deployment.yaml", env)),
                    deployment,
                )
                .unwrap();
            }
        }

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

    /// Get config content
    #[allow(dead_code)] // Used across multiple test files
    pub fn get_definitions(&self) -> String {
        self.read_file("control-path.yaml")
    }

    /// Get config content (same as get_definitions for config format)
    #[allow(dead_code)] // Used across multiple test files
    pub fn get_deployment(&self, _env: &str) -> String {
        // For config, deployment is part of the config file
        self.read_file("control-path.yaml")
    }

    /// Check if AST file exists for environment
    #[allow(dead_code)] // Used across multiple test files
    pub fn ast_exists(&self, env: &str) -> bool {
        self.file_exists(&format!(".controlpath/{}.ast", env))
    }

    /// Evaluate a flag using the compiled AST and user attributes
    /// This uses Node.js to load the AST and evaluate the flag, testing actual behavior
    /// Returns the evaluated value as a string, or None if evaluation fails
    /// This is a behavior-focused test helper that verifies flags work correctly
    ///
    /// Note: This requires the TypeScript runtime to be built (run `cd runtime/typescript && npm run build`)
    /// If the runtime is not available, this will return None (tests should handle this gracefully)
    #[allow(dead_code)] // Used in integration tests when runtime is available
    pub fn evaluate_flag(
        &self,
        flag_name: &str,
        env: &str,
        attributes: &serde_json::Value,
    ) -> Option<String> {
        // Ensure AST exists
        if !self.ast_exists(env) {
            return None;
        }

        // Find workspace root by looking for Cargo.toml or runtime directory
        // Tests typically run from workspace root
        let workspace_root = std::env::current_dir().ok()?;
        let runtime_dist = workspace_root
            .join("runtime")
            .join("typescript")
            .join("dist");

        // Check if runtime is built
        if !runtime_dist.join("ast-loader.js").exists() {
            // Runtime not built - return None (tests can skip or use alternative verification)
            return None;
        }

        // Create a temporary Node.js script to evaluate the flag
        let ast_path = self.path(&format!(".controlpath/{}.ast", env));
        let attributes_json =
            serde_json::to_string(attributes).unwrap_or_else(|_| "{}".to_string());

        // Use absolute paths for requires to avoid path issues
        let loader_path = runtime_dist.join("ast-loader.js");
        let evaluator_path = runtime_dist.join("evaluator.js");

        let script_content = format!(
            r#"
const {{ loadFromFile }} = require('{}');
const {{ evaluate }} = require('{}');
const path = require('path');

async function main() {{
    const astPath = '{}';
    const attributes = {};
    
    try {{
        const artifact = await loadFromFile(astPath);
        
        // Find flag index by name
        let flagIndex = -1;
        for (let i = 0; i < artifact.flagNames.length; i++) {{
            const nameIndex = artifact.flagNames[i];
            const name = artifact.strs[nameIndex];
            if (name === '{}') {{
                flagIndex = i;
                break;
            }}
        }}
        
        if (flagIndex === -1) {{
            console.error('Flag not found');
            process.exit(1);
        }}
        
        const result = evaluate(flagIndex, artifact, attributes);
        if (result === undefined) {{
            console.error('Evaluation returned undefined');
            process.exit(1);
        }}
        
        // Convert result to string
        const resultStr = typeof result === 'string' ? result : JSON.stringify(result);
        console.log(resultStr);
    }} catch (error) {{
        console.error(error.message);
        process.exit(1);
    }}
}}

main();
"#,
            loader_path.to_string_lossy().replace('\\', "/"),
            evaluator_path.to_string_lossy().replace('\\', "/"),
            ast_path.to_string_lossy().replace('\\', "/"),
            attributes_json,
            flag_name
        );

        // Write script to temp file
        let script_path = self.path("evaluate_flag_temp.js");
        fs::write(&script_path, script_content).ok()?;

        // Run Node.js script from project directory
        let output = Command::new("node")
            .current_dir(&self.project_path)
            .arg("evaluate_flag_temp.js")
            .output()
            .ok()?;

        // Clean up script
        let _ = fs::remove_file(&script_path);

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Some(result)
        } else {
            // Log error for debugging
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Flag evaluation failed: {}", stderr);
            None
        }
    }

    /// Evaluate a flag with simple attributes (convenience method)
    /// attributes_str should be a JSON object string like `{{"role": "admin", "id": "user1"}}`
    #[allow(dead_code)] // Used in integration tests when runtime is available
    pub fn evaluate_flag_simple(
        &self,
        flag_name: &str,
        env: &str,
        attributes_str: &str,
    ) -> Option<String> {
        let attributes: serde_json::Value = serde_json::from_str(attributes_str).ok()?;
        self.evaluate_flag(flag_name, env, &attributes)
    }
}

/// Create a simple test flag definition (config format)
pub fn simple_flag_definition(flag_name: &str) -> String {
    format!(
        r"mode: local
flags:
  - name: {}
    type: boolean
    default: false
    environments:
      production:
        - serve: true
",
        flag_name
    )
}

/// Create a simple deployment (now part of config, so this is a no-op)
/// The environments are already in the config format
#[allow(dead_code)] // Used across multiple test files
pub fn simple_deployment(_env: &str, _flag_name: &str, _serve: bool) -> String {
    // This is no longer used - environments are in config
    String::new()
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
