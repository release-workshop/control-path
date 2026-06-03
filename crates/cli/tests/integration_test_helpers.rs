//! Test helpers for integration tests
//!
//! Integration tests spawn the CLI with `Command::current_dir(project_path)` and do not
//! mutate the process working directory. Unit tests in `src/` that need `set_current_dir`
//! use [`controlpath_cli::test_helpers::DirGuard`] with `#[serial]` instead.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

/// Repository root (`crates/cli` → workspace root). Stable under parallel integration tests.
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// Whether `runtime/typescript` has been built (`dist/ast-loader.js` present).
#[allow(dead_code)]
pub fn typescript_runtime_built() -> bool {
    workspace_root()
        .join("runtime/typescript/dist/ast-loader.js")
        .is_file()
}

fn runtime_dist_dir() -> PathBuf {
    workspace_root().join("runtime/typescript/dist")
}

#[allow(dead_code)]
fn parse_boolean_eval_result(result: &str) -> Option<bool> {
    match result {
        "true" | "True" | "ON" | "on" | "1" => Some(true),
        "false" | "False" | "OFF" | "off" | "0" => Some(false),
        _ => None,
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

    /// Create a test project with a v2 catalog (environment rules live in control-path.yaml).
    #[allow(dead_code)] // Used across multiple test files
    pub fn with_deployment(
        definitions_content: &str,
        _env: &str,
        _deployment_content: &str,
    ) -> Self {
        Self::with_definitions(definitions_content)
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

    /// Assert the compiled AST for `env` exists and is non-empty.
    #[allow(dead_code)]
    pub fn assert_ast_compiled(&self, env: &str) {
        assert!(
            self.ast_exists(env),
            "expected .controlpath/{env}.ast to exist"
        );
        let ast_path = self.path(&format!(".controlpath/{env}.ast"));
        let len = fs::metadata(&ast_path).map(|m| m.len()).unwrap_or(0);
        assert!(len > 0, "AST at {} should be non-empty", ast_path.display());
    }

    /// Assert a boolean flag evaluates to `expected` when the TypeScript runtime is built.
    ///
    /// Always checks the AST. When `runtime/typescript/dist` exists, also evaluates via Node.
    /// Locally without `dist`, evaluation is skipped. In CI (`CI` env set), missing `dist` panics.
    #[allow(dead_code)]
    pub fn assert_boolean_flag(
        &self,
        flag_name: &str,
        env: &str,
        attributes_str: &str,
        expected: bool,
    ) {
        self.assert_ast_compiled(env);

        if !typescript_runtime_built() {
            if std::env::var_os("CI").is_some() {
                panic!(
                    "CI must build runtime/typescript before workspace tests (missing {}). \
                     Local dev: cd runtime/typescript && npm ci && npm run build",
                    runtime_dist_dir().join("ast-loader.js").display()
                );
            }
            return;
        }

        let result = self
            .evaluate_flag_simple(flag_name, env, attributes_str)
            .unwrap_or_else(|| {
                panic!(
                    "TypeScript runtime is built at {} but evaluation failed for flag \
                     '{flag_name}' in environment '{env}'",
                    runtime_dist_dir().display()
                );
            });

        let parsed = parse_boolean_eval_result(&result).unwrap_or_else(|| {
            panic!("expected boolean evaluation for '{flag_name}' in '{env}', got: {result}");
        });
        assert_eq!(
            parsed, expected,
            "flag '{flag_name}' in '{env}' with attributes {attributes_str}"
        );
    }

    /// Initialize git in the project, commit `control-path.yaml`, and check out `branch`.
    ///
    /// Required for branch-mapping smart-default tests; fails fast if `git` is unavailable.
    #[allow(dead_code)]
    pub fn init_git_repo_on_branch(&self, branch: &str) {
        use std::process::Command;

        let dir = &self.project_path;
        let run = |args: &[&str]| {
            Command::new("git")
                .current_dir(dir)
                .args(args)
                .output()
                .unwrap_or_else(|e| panic!("spawn git {args:?}: {e}"))
        };
        let assert_git_ok = |step: &str, output: Output| {
            assert!(
                output.status.success(),
                "{step} failed (exit {:?}): {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
        };

        assert_git_ok("git init", run(&["init"]));
        assert_git_ok(
            "git config user.email",
            run(&["config", "user.email", "test@example.com"]),
        );
        assert_git_ok(
            "git config user.name",
            run(&["config", "user.name", "Test User"]),
        );
        self.write_file("README.md", "# Test\n");
        assert_git_ok("git add", run(&["add", "README.md", "control-path.yaml"]));
        assert_git_ok("git commit", run(&["commit", "-m", "Initial commit"]));
        assert_git_ok(
            &format!("git checkout -b {branch}"),
            run(&["checkout", "-b", branch]),
        );
    }

    /// Evaluate a flag using the compiled AST and user attributes
    /// This uses Node.js to load the AST and evaluate the flag, testing actual behavior
    /// Returns the evaluated value as a string, or `None` if the runtime is not built or evaluation fails.
    ///
    /// Prefer [`Self::assert_boolean_flag`] for workflow tests (strict in CI). Use this for optional
    /// checks (e.g. large-scale suites) when `dist/` may be absent locally.
    ///
    /// Requires `runtime/typescript` built (`npm run build`) and `node` on PATH.
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

        let runtime_dist = runtime_dist_dir();

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

/// Create a simple v2 test catalog with one environment (`production`) and an initial serve rule.
#[allow(dead_code)] // Used by other integration test crates
pub fn simple_flag_definition(flag_name: &str) -> String {
    simple_flag_definition_with_serve(flag_name, "production", true)
}

fn simple_flag_definition_with_serve(flag_name: &str, env: &str, serve: bool) -> String {
    format!(
        r"catalog:
  id: test-service
mode: local
flags:
  {flag_name}:
    default: false
    kind: release
environments:
  {env}:
    rules:
      {flag_name}:
        - serve: {serve}
"
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

/// Discover SaaS environment names from `.controlpath/*.ast` via the CLI adapter.
#[allow(dead_code)]
pub fn discover_saas_ast_environments(project_dir: &Path) -> Vec<String> {
    controlpath_cli::discover_environments_in_dir(&project_dir.join(".controlpath"))
        .expect("read .controlpath after SaaS sync")
}

/// Expected embedded SaaS poll URLs after sync: disk discovery + [`build_saas_runtime_url_maps`].
#[allow(dead_code)]
pub fn expected_saas_runtime_url_maps(
    project_dir: &Path,
    cdn_base: &str,
    saas_project: &str,
    catalog_id: &controlpath_compiler::EffectiveCatalogId,
) -> controlpath_compiler::SaasRuntimeUrlMaps {
    use controlpath_compiler::build_saas_runtime_url_maps;

    let envs = discover_saas_ast_environments(project_dir);
    assert!(
        !envs.is_empty(),
        "expected .controlpath/*.ast after SaaS sync before asserting embedded URLs"
    );
    build_saas_runtime_url_maps(cdn_base, saas_project, catalog_id, &envs)
}
