//! Integration tests for individual commands

mod integration_test_helpers;

use integration_test_helpers::*;
use serial_test::serial;

#[test]
#[serial]
fn test_validate_command() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Validate with --all (validates config and all environments)
    project.run_command_success(&["validate", "--all"]);

    // Validate with env (validates specific environment from config)
    project.run_command_success(&["validate", "--env", "production"]);
}

#[test]
#[serial]
fn test_validate_command_failure() {
    let project = TestProject::new();

    // Create invalid config file
    project.write_file("control-path.yaml", "invalid: yaml: content: [");

    // Validation should fail
    let output = project.run_command_failure(&["validate", "--all"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error") || stderr.contains("invalid") || stderr.contains("parse"));
}

#[test]
#[serial]
fn test_compile_command() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Compile with env (uses config)
    project.run_command_success(&["compile", "--env", "production"]);

    // Verify AST exists
    assert!(project.ast_exists("production"));

    // Compile with explicit output path
    project.run_command_success(&[
        "compile",
        "--env",
        "production",
        "--output",
        ".controlpath/production2.ast",
    ]);

    assert!(project.file_exists(".controlpath/production2.ast"));
}

#[test]
#[serial]
fn test_generate_sdk_command() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    // Generate TypeScript SDK
    project.run_command_success(&["generate-sdk", "--lang", "typescript"]);

    // Verify SDK was generated
    assert!(project.file_exists("./flags"));
}

#[test]
#[serial]
fn test_explain_command() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Compile first
    project.run_command_success(&["compile", "--env", "production"]);

    // Create user JSON file
    project.write_file("user.json", r#"{"id": "user-1", "role": "admin"}"#);

    // Explain flag
    let output = project.run_command(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        "user.json",
        "--env",
        "production",
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("my_flag") || stdout.contains("Flag"));
}

#[test]
#[serial]
fn test_explain_command_with_trace() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Compile first
    project.run_command_success(&["compile", "--env", "production"]);

    // Create user JSON file
    project.write_file("user.json", r#"{"id": "user-1"}"#);

    // Explain with trace
    let output = project.run_command(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        "user.json",
        "--env",
        "production",
        "--trace",
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Trace output should be more detailed
    assert!(!stdout.is_empty());
}

#[test]
#[serial]
fn test_init_command() {
    let project = TestProject::new();

    // Run setup with example flags to create config
    project.run_command_success(&["setup", "--skip-install"]);

    // Verify config file was created
    assert!(project.file_exists("control-path.yaml"));

    // Verify config has flags (setup creates example flags by default)
    let content = project.read_file("control-path.yaml");
    assert!(content.contains("flags"));
}

#[test]
#[serial]
fn test_init_command_with_force() {
    let project = TestProject::new();

    // Create existing config file
    project.write_file("control-path.yaml", "mode: local\nflags: []\n");

    // Setup should fail if project already exists
    let output = project.run_command_failure(&["setup", "--skip-install"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already initialized") || stderr.contains("already exists"));
}

#[test]
#[serial]
fn test_flag_list_command() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("flag1"),
        "production",
        &simple_deployment("production", "flag1", true),
    );

    // Add another flag to both config and legacy file (flag command uses legacy)
    let config_content = r"mode: local
flags:
  - name: flag1
    type: boolean
    default: false
    environments:
      production:
        - serve: true
  - name: flag2
    type: boolean
    default: true
    environments:
      production:
        - serve: true
"
    .to_string();
    project.write_file("control-path.yaml", &config_content);

    // Also update legacy files for flag command (which doesn't support config yet)
    let legacy_definitions = r"flags:
  - name: flag1
    type: boolean
    default: false
    defaultValue: false
  - name: flag2
    type: boolean
    default: true
    defaultValue: true
";
    project.write_file("flags.definitions.yaml", legacy_definitions);

    // Update deployment file to include both flags
    let legacy_deployment = r"environment: production
rules:
  flag1:
    rules:
      - serve: true
  flag2:
    rules:
      - serve: true
";
    project.write_file(".controlpath/production.deployment.yaml", legacy_deployment);

    // List flags (flag command uses legacy files)
    let output = project.run_command(&["flag", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("flag1"));
    assert!(stdout.contains("flag2"));

    // List from specific environment
    let output = project.run_command(&["flag", "list", "--deployment", "production"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("flag1"));
    assert!(stdout.contains("flag2"));
}

#[test]
#[serial]
fn test_flag_list_json_format() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    // Also create legacy file for flag command
    let legacy_definitions = r"flags:
  - name: my_flag
    type: boolean
    default: false
    defaultValue: false
";
    project.write_file("flags.definitions.yaml", legacy_definitions);

    let output = project.run_command(&["flag", "list", "--format", "json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    assert!(stdout.trim().starts_with("{") || stdout.trim().starts_with("["));
}

#[test]
#[serial]
fn test_flag_list_yaml_format() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    // Also create legacy file for flag command
    let legacy_definitions = r"flags:
  - name: my_flag
    type: boolean
    default: false
    defaultValue: false
";
    project.write_file("flags.definitions.yaml", legacy_definitions);

    let output = project.run_command(&["flag", "list", "--format", "yaml"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain YAML-like content
    assert!(stdout.contains("my_flag") || stdout.contains("flags"));
}

#[test]
#[serial]
fn test_flag_list_table_format() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    // Also create legacy file for flag command
    let legacy_definitions = r"flags:
  - name: my_flag
    type: boolean
    default: false
    defaultValue: false
";
    project.write_file("flags.definitions.yaml", legacy_definitions);

    let output = project.run_command(&["flag", "list", "--format", "table"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Table format should contain the flag name
    assert!(stdout.contains("my_flag"));
}

#[test]
#[serial]
fn test_env_list_command() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Add another environment
    project.run_command_success(&["env", "add", "--name", "staging"]);

    // List environments
    let output = project.run_command(&["env", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("production"));
    assert!(stdout.contains("staging"));
}

#[test]
#[serial]
fn test_env_remove_command() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Add a test environment to remove (env command uses legacy files)
    project.run_command_success(&["env", "add", "--name", "test_env"]);

    // Verify it exists as a legacy deployment file
    assert!(project.file_exists(".controlpath/test_env.deployment.yaml"));

    // Remove the environment (name is a flag, not positional)
    project.run_command_success(&["env", "remove", "--name", "test_env", "--force"]);

    // Verify it was removed (legacy deployment file)
    assert!(!project.file_exists(".controlpath/test_env.deployment.yaml"));

    // Verify production still exists (legacy deployment file)
    assert!(project.file_exists(".controlpath/production.deployment.yaml"));
}

#[test]
#[serial]
fn test_completion_command() {
    let project = TestProject::new();

    // Test bash completion (shell is a positional argument, not --shell)
    let output = project.run_command(&["completion", "bash"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("complete") || stdout.contains("_controlpath"));

    // Test zsh completion
    let output = project.run_command(&["completion", "zsh"]);
    assert!(output.status.success());

    // Test fish completion
    let output = project.run_command(&["completion", "fish"]);
    assert!(output.status.success());
}

#[test]
#[serial]
fn test_completion_command_invalid_shell() {
    let project = TestProject::new();

    let output = project.run_command_failure(&["completion", "powershell"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unsupported shell") || stderr.contains("powershell"));
}

#[test]
#[serial]
fn test_explain_invalid_user_json() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Compile first
    project.run_command_success(&["compile", "--env", "production"]);

    // Create invalid user JSON file
    project.write_file("user.json", r#"{"id": "user-1", invalid json}"#);

    // Explain should fail with invalid JSON
    let output = project.run_command_failure(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        "user.json",
        "--env",
        "production",
    ]);
    assert!(!output.status.success());
}

#[test]
#[serial]
fn test_explain_missing_user_file() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Compile first
    project.run_command_success(&["compile", "--env", "production"]);

    // Try to explain with non-existent user file
    let output = project.run_command_failure(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        "nonexistent.json",
        "--env",
        "production",
    ]);
    assert!(!output.status.success());
}

#[test]
#[serial]
fn test_explain_invalid_context_json() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Compile first
    project.run_command_success(&["compile", "--env", "production"]);

    // Create valid user file
    project.write_file("user.json", r#"{"id": "user-1"}"#);

    // Create invalid context JSON file
    project.write_file("context.json", r#"{"env": "prod", invalid}"#);

    // Explain should fail with invalid context JSON
    let output = project.run_command_failure(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        "user.json",
        "--context",
        "context.json",
        "--env",
        "production",
    ]);
    assert!(!output.status.success());
}
