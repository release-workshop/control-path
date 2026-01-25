//! Integration tests for error handling

mod integration_test_helpers;

use integration_test_helpers::*;

#[test]
fn test_validate_missing_files() {
    let project = TestProject::new();

    // Validate should fail when no files exist
    let output = project.run_command_failure(&["validate", "--all"]);
    assert!(!output.status.success());
}

#[test]
fn test_compile_missing_deployment() {
    let project = TestProject::new();

    // Compile should fail when deployment doesn't exist
    let output = project.run_command_failure(&["compile", "--env", "production"]);
    assert!(!output.status.success());
}

#[test]
fn test_explain_missing_flag() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Compile first
    project.run_command_success(&["compile", "--env", "production"]);

    // Create user file
    project.write_file("user.json", r#"{"id": "user-1"}"#);

    // Explain non-existent flag should fail
    let output = project.run_command_failure(&[
        "explain",
        "--flag",
        "nonexistent_flag",
        "--user",
        "user.json",
        "--env",
        "production",
    ]);
    assert!(!output.status.success());
}

#[test]
fn test_flag_add_duplicate() {
    let project = TestProject::with_definitions(&simple_flag_definition("existing_flag"));

    // Try to add duplicate flag
    let output = project.run_command_failure(&[
        "flag",
        "add",
        "--name",
        "existing_flag",
        "--type",
        "boolean",
        "--default",
        "false",
    ]);
    assert!(!output.status.success());
}

#[test]
fn test_flag_remove_nonexistent() {
    let project = TestProject::new();

    // Try to remove non-existent flag
    let output = project.run_command_failure(&["flag", "remove", "nonexistent_flag", "--force"]);
    assert!(!output.status.success());
}

#[test]
fn test_env_add_duplicate() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Try to add duplicate environment
    let output = project.run_command_failure(&["env", "add", "--name", "production"]);
    assert!(!output.status.success());
}

#[test]
fn test_enable_flag_nonexistent() {
    let project = TestProject::new();

    // Try to enable non-existent flag
    let output =
        project.run_command_failure(&["enable", "nonexistent_flag", "--env", "production"]);
    assert!(!output.status.success());
}

#[test]
fn test_enable_flag_nonexistent_env() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    // Also create legacy file for enable command if it needs it
    let legacy_definitions = r"flags:
  - name: my_flag
    type: boolean
    default: false
    defaultValue: false
";
    project.write_file("flags.definitions.yaml", legacy_definitions);

    // Enable command now automatically creates environments, so this should succeed
    // (The enable command uses config and creates environments automatically)
    let output = project.run_command(&["enable", "my_flag", "--env", "nonexistent"]);
    // Should succeed - environments are created automatically
    assert!(output.status.success());

    // Verify the environment was created in config
    let config = project.read_file("control-path.yaml");
    assert!(config.contains("nonexistent"));
}

#[test]
fn test_deploy_nonexistent_env() {
    let project = TestProject::new();

    // Try to deploy non-existent environment
    let output = project.run_command_failure(&["deploy", "--env", "nonexistent"]);
    assert!(!output.status.success());
}

#[test]
fn test_generate_sdk_invalid_language() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    // Try to generate SDK with invalid language
    let output = project.run_command_failure(&["generate-sdk", "--lang", "invalid_lang"]);
    assert!(!output.status.success());
}

#[test]
fn test_compile_invalid_deployment() {
    let project = TestProject::new();

    // Create invalid deployment file
    project.write_file(
        ".controlpath/production.deployment.yaml",
        "invalid: yaml: content: [",
    );

    // Compile should fail
    let output = project.run_command_failure(&["compile", "--env", "production"]);
    assert!(!output.status.success());
}

#[test]
fn test_explain_missing_ast() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Don't compile, just try to explain
    project.write_file("user.json", r#"{"id": "user-1"}"#);

    // Explain should fail without AST
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
fn test_flag_add_invalid_name() {
    let project = TestProject::new();

    // Try to add flag with invalid name (uppercase)
    let output = project.run_command_failure(&[
        "flag",
        "add",
        "--name",
        "InvalidFlag",
        "--type",
        "boolean",
        "--default",
        "false",
    ]);
    assert!(!output.status.success());
}

#[test]
fn test_env_add_invalid_name() {
    let project = TestProject::new();

    // Try to add environment with invalid name (uppercase)
    let output = project.run_command_failure(&["env", "add", "--name", "Production"]);
    assert!(!output.status.success());
}

#[test]
fn test_env_remove_nonexistent() {
    let project = TestProject::new();

    // Try to remove non-existent environment
    let output = project.run_command_failure(&["env", "remove", "nonexistent", "--force"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("error"));
}
