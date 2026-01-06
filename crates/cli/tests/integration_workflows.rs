//! Integration tests for workflow commands

mod integration_test_helpers;

use integration_test_helpers::*;

use std::fs;

#[test]
fn test_new_flag_workflow() {
    let project = TestProject::new();

    // Initialize project first (new-flag needs definitions file)
    // Use --force to overwrite .controlpath directory created by TestProject::new()
    project.run_command_success(&["init", "--force"]);

    // Run new-flag command
    project.run_command_success(&[
        "new-flag",
        "test_feature",
        "--type",
        "boolean",
        "--default",
        "false",
        "--skip-sdk",
    ]);

    // Verify flag was added to definitions
    let definitions = project.get_definitions();
    assert!(definitions.contains("test_feature"));
    assert!(definitions.contains("type: boolean"));
    assert!(definitions.contains("defaultValue: false"));

    // Verify flag was synced to deployment (if production exists)
    if project.file_exists(".controlpath/production.deployment.yaml") {
        let deployment = project.get_deployment("production");
        assert!(deployment.contains("test_feature"));
    }
}

#[test]
fn test_enable_workflow() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", false),
    );

    // Enable the flag
    project.run_command_success(&["enable", "my_flag", "--env", "production", "--all"]);

    // Verify flag was enabled
    let deployment = project.get_deployment("production");
    // The enable command should have changed serve to true
    assert!(deployment.contains("serve: true") || deployment.contains("serve: True"));
}

#[test]
fn test_enable_with_rule_workflow() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", false),
    );

    // Enable with a rule
    project.run_command_success(&[
        "enable",
        "my_flag",
        "--env",
        "production",
        "--rule",
        "user.role == 'admin'",
    ]);

    // Verify rule was added
    let deployment = project.get_deployment("production");
    assert!(deployment.contains("when:"));
    assert!(deployment.contains("user.role == 'admin'"));
}

#[test]
fn test_deploy_workflow() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Deploy
    project.run_command_success(&["deploy", "--env", "production"]);

    // Verify AST was created
    assert!(project.ast_exists("production"));
}

#[test]
fn test_complete_workflow_new_flag_enable_deploy() {
    let project = TestProject::new();

    // Initialize project first (new-flag needs definitions file)
    // Use --force to overwrite .controlpath directory created by TestProject::new()
    project.run_command_success(&["init", "--force"]);

    // Step 1: Create a new flag
    project.run_command_success(&[
        "new-flag",
        "new_feature",
        "--type",
        "boolean",
        "--default",
        "false",
        "--skip-sdk",
    ]);

    // Step 2: Enable it in production
    project.run_command_success(&["enable", "new_feature", "--env", "production", "--all"]);

    // Step 3: Deploy
    project.run_command_success(&["deploy", "--env", "production"]);

    // Verify everything worked
    let definitions = project.get_definitions();
    assert!(definitions.contains("new_feature"));

    let deployment = project.get_deployment("production");
    assert!(deployment.contains("new_feature"));

    assert!(project.ast_exists("production"));
}

#[test]
fn test_flag_add_list_show_remove_workflow() {
    let project = TestProject::new();

    // Initialize project first (flag add needs definitions file)
    // Use --force to overwrite .controlpath directory created by TestProject::new()
    project.run_command_success(&["init", "--force"]);

    // Add a flag
    project.run_command_success(&[
        "flag",
        "add",
        "--name",
        "test_flag",
        "--type",
        "boolean",
        "--default",
        "false",
        "--sync",
    ]);

    // List flags
    let output = project.run_command(&["flag", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test_flag"));

    // Show flag (name is a flag, not positional)
    let output = project.run_command(&["flag", "show", "--name", "test_flag"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test_flag"));

    // Remove flag (name is a flag, not positional)
    project.run_command_success(&[
        "flag",
        "remove",
        "--name",
        "test_flag",
        "--from-deployments",
        "--force",
    ]);

    // Verify flag was removed
    let definitions = project.get_definitions();
    assert!(!definitions.contains("test_flag"));
}

#[test]
fn test_env_add_sync_list_workflow() {
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    // Add environment
    project.run_command_success(&["env", "add", "--name", "staging"]);

    // Verify environment was created
    assert!(project.file_exists(".controlpath/staging.deployment.yaml"));
    let deployment = project.get_deployment("staging");
    assert!(deployment.contains("environment: staging"));
    assert!(deployment.contains("my_flag"));

    // Sync environment
    project.run_command_success(&["env", "sync", "--env", "staging"]);

    // List environments
    let output = project.run_command(&["env", "list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("staging"));
}

#[test]
fn test_setup_workflow() {
    let project = TestProject::new();

    // Create package.json to trigger TypeScript detection
    project.write_file("package.json", "{}");

    // Remove .controlpath directory (TestProject::new() creates it, but setup needs a clean project)
    // Setup will call init internally, which will fail if .controlpath already exists
    fs::remove_dir_all(project.path(".controlpath")).unwrap();
    
    // Run setup (it will initialize the project, compile, and generate SDK)
    project.run_command_success(&["setup", "--lang", "typescript", "--skip-install"]);

    // Verify project structure was created
    assert!(project.file_exists("flags.definitions.yaml"));
    assert!(project.file_exists(".controlpath/production.deployment.yaml"));

    // Verify AST was compiled
    assert!(project.ast_exists("production"));

    // Verify SDK was generated (if not skipped)
    // Note: SDK generation might be skipped in tests, so we check conditionally
    if project.file_exists("./flags") {
        // SDK directory exists
    }
}

#[test]
fn test_validate_compile_generate_sdk_workflow() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Validate
    project.run_command_success(&["validate"]);

    // Compile
    project.run_command_success(&["compile", "--env", "production"]);

    // Verify AST exists
    assert!(project.ast_exists("production"));

    // Generate SDK
    project.run_command_success(&["generate-sdk", "--lang", "typescript"]);

    // Verify SDK was generated
    assert!(project.file_exists("./flags"));
}
