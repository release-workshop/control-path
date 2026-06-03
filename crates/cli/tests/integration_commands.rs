//! Integration tests for individual commands

mod integration_test_helpers;

use integration_test_helpers::*;
use std::fs;

#[test]
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
fn test_generate_sdk_command() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    // Generate TypeScript SDK
    project.run_command_success(&["generate-sdk", "--lang", "typescript"]);

    // Verify SDK was generated (default path is node_modules/@controlpath/generated)
    assert!(project.file_exists("node_modules/@controlpath/generated/index.ts"));
    assert!(project.file_exists("node_modules/@controlpath/generated/types.ts"));
    assert!(project.file_exists("node_modules/@controlpath/generated/package.json"));
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
fn test_explain_command_with_inline_user_json() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Compile first
    project.run_command_success(&["compile", "--env", "production"]);

    // Explain flag with inline JSON user input
    let output = project.run_command(&[
        "explain",
        "--flag",
        "my_flag",
        "--user",
        r#"{"id":"user-1","role":"admin"}"#,
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
    assert!(
        stdout.contains("User ID: user-1"),
        "trace header should show user id for rollout debugging, got: {stdout}"
    );
    assert!(stdout.contains("Rule trace:"));
}

#[test]
fn test_setup_command() {
    let project = TestProject::new();

    project.run_command_success(&["setup", "--skip-install"]);

    assert!(project.file_exists("control-path.yaml"));
    let content = project.read_file("control-path.yaml");
    assert!(content.contains("flags"));
}

#[test]
fn test_init_multi_repo() {
    let project = TestProject::new();
    project.run_command_success(&[
        "init",
        "--no-monorepo",
        "--namespace",
        "acme",
        "--service-id",
        "checkout",
    ]);
    let content = project.read_file("control-path.yaml");
    assert!(content.contains("namespace: acme"));
    assert!(content.contains("checkout"));
    assert!(!project.file_exists("control-path.workspace.yaml"));
}

#[test]
fn test_init_monorepo_workspace() {
    let project = TestProject::new();
    project.run_command_success(&["init", "--monorepo", "--namespace", "acme"]);
    assert!(project.file_exists("control-path.workspace.yaml"));
    let content = project.read_file("control-path.workspace.yaml");
    assert!(content.contains("namespace: acme"));
}

#[test]
fn test_init_service_scaffold_from_workspace() {
    let project = TestProject::new();
    project.run_command_success(&["init", "--monorepo", "--namespace", "acme"]);
    let service = project.path("checkout-service");
    fs::create_dir_all(&service).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_controlpath"))
        .current_dir(&service)
        .args(["init", "--service-id", "checkout-service"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(service.join("control-path.yaml").exists());
}

#[test]
fn test_setup_command_with_force() {
    let project = TestProject::new();

    // Create existing config file
    project.write_file("control-path.yaml", "mode: local\nflags: []\n");

    // Setup should fail if project already exists
    let output = project.run_command_failure(&["setup", "--skip-install"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already initialized") || stderr.contains("already exists"));
}

#[test]
fn test_flag_list_command() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("flag1"),
        "production",
        &simple_deployment("production", "flag1", true),
    );

    // Add another flag to config
    let config_content = r"catalog:
  id: test-service
mode: local
flags:
  flag1:
    default: false
    kind: release
  flag2:
    default: true
    kind: release
environments:
  production:
    rules:
      flag1:
        - serve: true
      flag2:
        - serve: true
"
    .to_string();
    project.write_file("control-path.yaml", &config_content);

    // List flags
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
fn test_flag_list_json_format() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    let output = project.run_command(&["flag", "list", "--format", "json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    assert!(stdout.trim().starts_with("{") || stdout.trim().starts_with("["));
}

#[test]
fn test_flag_list_yaml_format() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    let output = project.run_command(&["flag", "list", "--format", "yaml"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain YAML-like content
    assert!(stdout.contains("my_flag") || stdout.contains("flags"));
}

#[test]
fn test_flag_list_table_format() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    let output = project.run_command(&["flag", "list", "--format", "table"]);
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

    // List environments
    let output = project.run_command(&["env", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("production"));
    // env add is readiness-only in unified mode, so staging is not listed until rules exist.
    assert!(!stdout.contains("staging"));
}

#[test]
fn test_env_remove_command() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Remove existing environment from unified config.
    project.run_command_success(&["env", "remove", "--name", "production"]);

    let output = project.run_command(&["env", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("production"));
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
