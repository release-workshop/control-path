//! Integration tests for individual commands

mod integration_test_helpers;

use integration_test_helpers::*;

#[test]
fn test_validate_command() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Validate with --all
    project.run_command_success(&["validate", "--all"]);

    // Validate specific files
    project.run_command_success(&[
        "validate",
        "--definitions",
        "flags.definitions.yaml",
        "--deployment",
        ".controlpath/production.deployment.yaml",
    ]);

    // Validate with env
    project.run_command_success(&["validate", "--env", "production"]);
}

#[test]
fn test_validate_command_failure() {
    let project = TestProject::new();

    // Create invalid definitions file
    project.write_file("flags.definitions.yaml", "invalid: yaml: content: [");

    // Validation should fail
    let output = project.run_command_failure(&["validate", "--all"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error") || stderr.contains("invalid"));
}

#[test]
fn test_compile_command() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Compile with env
    project.run_command_success(&["compile", "--env", "production"]);

    // Verify AST exists
    assert!(project.ast_exists("production"));

    // Compile with explicit paths
    project.run_command_success(&[
        "compile",
        "--deployment",
        ".controlpath/production.deployment.yaml",
        "--output",
        ".controlpath/production2.ast",
        "--definitions",
        "flags.definitions.yaml",
    ]);

    assert!(project.file_exists(".controlpath/production2.ast"));
}

#[test]
fn test_generate_sdk_command() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    // Generate TypeScript SDK
    project.run_command_success(&["generate-sdk", "--lang", "typescript"]);

    // Verify SDK was generated
    assert!(project.file_exists("./flags"));
}

#[test]
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
fn test_init_command() {
    let project = TestProject::new();

    // Run init with --force to ensure clean state (in case .controlpath already exists)
    project.run_command_success(&["init", "--example-flags", "--force"]);

    // Verify files were created
    assert!(project.file_exists("flags.definitions.yaml"));
    assert!(project.file_exists(".controlpath/production.deployment.yaml"));
}

#[test]
fn test_init_command_with_force() {
    let project = TestProject::new();

    // Create existing file
    project.write_file("flags.definitions.yaml", "flags: []");

    // Run init with force
    project.run_command_success(&["init", "--force", "--example-flags"]);

    // Verify file was overwritten
    let content = project.get_definitions();
    // Should have example flags, not just empty array
    assert!(content.len() > 10);
}

#[test]
fn test_flag_list_command() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("flag1"),
        "production",
        &simple_deployment("production", "flag1", true),
    );

    // Add another flag
    project.write_file(
        "flags.definitions.yaml",
        r"flags:
  - name: flag1
    type: boolean
    defaultValue: false
  - name: flag2
    type: boolean
    defaultValue: true
",
    );

    // List from definitions
    let output = project.run_command(&["flag", "list", "--definitions"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("flag1"));
    assert!(stdout.contains("flag2"));

    // List from deployment
    let output = project.run_command(&["flag", "list", "--deployment", "production"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("flag1"));
}

#[test]
fn test_flag_list_json_format() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    let output = project.run_command(&["flag", "list", "--definitions", "--format", "json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    assert!(stdout.trim().starts_with("{") || stdout.trim().starts_with("["));
}

#[test]
fn test_flag_list_yaml_format() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    let output = project.run_command(&["flag", "list", "--definitions", "--format", "yaml"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain YAML-like content
    assert!(stdout.contains("my_flag") || stdout.contains("flags"));
}

#[test]
fn test_flag_list_table_format() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    let output = project.run_command(&["flag", "list", "--definitions", "--format", "table"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Table format should contain the flag name
    assert!(stdout.contains("my_flag"));
}

#[test]
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
fn test_env_remove_command() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Add a test environment to remove
    project.run_command_success(&["env", "add", "--name", "test_env"]);

    // Verify it exists
    assert!(project.file_exists(".controlpath/test_env.deployment.yaml"));

    // Remove the environment (name is a flag, not positional)
    project.run_command_success(&["env", "remove", "--name", "test_env", "--force"]);

    // Verify it was removed
    assert!(!project.file_exists(".controlpath/test_env.deployment.yaml"));

    // Verify production still exists
    assert!(project.file_exists(".controlpath/production.deployment.yaml"));
}

#[test]
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
fn test_completion_command_invalid_shell() {
    let project = TestProject::new();

    let output = project.run_command_failure(&["completion", "powershell"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unsupported shell") || stderr.contains("powershell"));
}

#[test]
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
