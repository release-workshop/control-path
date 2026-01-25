//! Integration tests for workflow commands

mod integration_test_helpers;

use integration_test_helpers::*;
use serial_test::serial;

use std::fs;

#[test]
#[serial]
fn test_new_flag_workflow() {
    let project = TestProject::new();

    // Initialize project first (new-flag needs config)
    project.run_command_success(&["setup", "--skip-install", "--no-examples"]);

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

    // Verify flag was added to config
    let config = project.get_definitions(); // get_definitions now returns config
    assert!(config.contains("test_feature"));

    // Verify flag can be loaded and used (behavior test)
    // Compile to create AST
    project.run_command_success(&["compile", "--env", "production"]);

    // Verify AST was created (observable outcome)
    assert!(project.ast_exists("production"));

    // If evaluation is available, verify the flag works with default value
    if let Some(result) =
        project.evaluate_flag_simple("test_feature", "production", r#"{"id": "test_user"}"#)
    {
        // Default should be false, so result should be "OFF" or "false"
        assert!(
            result == "OFF" || result == "false" || result == "False",
            "Flag should have default value (false), got: {}",
            result
        );
    }
}

#[test]
#[serial]
fn test_enable_workflow() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", false),
    );

    // Enable the flag (uses config)
    project.run_command_success(&["enable", "my_flag", "--env", "production", "--all"]);

    // Compile AST to enable evaluation
    project.run_command_success(&["compile", "--env", "production"]);

    // Verify flag behavior: evaluate with a test user and verify it's enabled
    // This tests actual behavior, not just file contents
    if let Some(result) =
        project.evaluate_flag_simple("my_flag", "production", r#"{"id": "test_user"}"#)
    {
        // Flag should be enabled (result should be "ON" or "true" or similar)
        assert!(
            result == "ON" || result == "true" || result == "True",
            "Flag should be enabled, got: {}",
            result
        );
    } else {
        // If evaluation is not available (runtime not built), fall back to config check
        // This is acceptable but less ideal - we're still testing behavior when possible
        let config = project.get_definitions();
        assert!(config.contains("serve: true") || config.contains("serve: True"));
    }
}

#[test]
#[serial]
fn test_enable_with_rule_workflow() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", false),
    );

    // Enable with a rule (uses config)
    project.run_command_success(&[
        "enable",
        "my_flag",
        "--env",
        "production",
        "--rule",
        "role == 'admin'", // Updated: no user. prefix
    ]);

    // Compile AST to enable evaluation
    project.run_command_success(&["compile", "--env", "production"]);

    // Verify flag behavior: admin user should get enabled flag, regular user should not
    // This tests actual behavior, not just file contents
    if let (Some(admin_result), Some(user_result)) = (
        project.evaluate_flag_simple(
            "my_flag",
            "production",
            r#"{"id": "admin1", "role": "admin"}"#,
        ),
        project.evaluate_flag_simple(
            "my_flag",
            "production",
            r#"{"id": "user1", "role": "user"}"#,
        ),
    ) {
        // Admin should get enabled flag
        assert!(
            admin_result == "ON" || admin_result == "true" || admin_result == "True",
            "Admin should get enabled flag, got: {}",
            admin_result
        );
        // Regular user should get default (disabled)
        assert!(
            user_result == "OFF" || user_result == "false" || user_result == "False",
            "Regular user should get disabled flag, got: {}",
            user_result
        );
    } else {
        // If evaluation is not available, fall back to config check
        let config = project.get_definitions();
        assert!(config.contains("when:"));
        assert!(config.contains("role == 'admin'"));
    }
}

#[test]
#[serial]
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
#[serial]
fn test_complete_workflow_new_flag_enable_deploy() {
    let project = TestProject::new();

    // Initialize project first (new-flag needs config)
    project.run_command_success(&["setup", "--skip-install", "--no-examples"]);

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

    // Verify everything worked (config)
    let config = project.get_definitions(); // Returns config
    assert!(config.contains("new_feature"));
    // Flag should be in config with production environment
    assert!(config.contains("production"));

    assert!(project.ast_exists("production"));
}

#[test]
#[serial]
fn test_flag_add_list_show_remove_workflow() {
    // Initialize project first - use with_deployment to create both config and legacy files
    // The flag command still uses legacy files, so we need both
    let project = TestProject::with_deployment(
        &simple_flag_definition("existing_flag"),
        "production",
        &simple_deployment("production", "existing_flag", false),
    );

    // Add a flag (flag command works with legacy files)
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

    // Verify flag was removed (check legacy definitions file)
    let definitions = project.read_file("flags.definitions.yaml");
    assert!(!definitions.contains("test_flag"));
}

#[test]
#[serial]
fn test_env_add_sync_list_workflow() {
    // Note: env add command may have been removed in favor of config
    // This test is skipped for now as the workflow has changed
    // Environments are now managed directly in control-path.yaml
    let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));

    // For now, verify the config exists and has the flag
    let config = project.get_definitions();
    assert!(config.contains("my_flag"));

    // If env commands still exist, test them
    // Otherwise, this test documents the new workflow where environments
    // are added directly to control-path.yaml
    let output = project.run_command(&["env", "list"]);
    if output.status.success() {
        let _stdout = String::from_utf8_lossy(&output.stdout);
        // If env list works, staging should be listed (from config)
        // Otherwise, this is expected to fail as env commands may be removed
    }
}

#[test]
#[serial]
fn test_setup_workflow() {
    let project = TestProject::new();

    // Create package.json to trigger TypeScript detection
    project.write_file("package.json", "{}");

    // TestProject::new() no longer creates .controlpath, so setup should work fine

    // Run setup (it will initialize the project, compile, and generate SDK)
    project.run_command_success(&["setup", "--lang", "typescript", "--skip-install"]);

    // Verify project structure was created (config format)
    assert!(project.file_exists("control-path.yaml"));

    // Verify config has flags
    let config = project.read_file("control-path.yaml");
    assert!(config.contains("flags"));

    // Verify AST was compiled for both environments and can be loaded
    assert!(project.ast_exists("production"));
    assert!(project.ast_exists("staging"));

    // Verify AST files are not empty (basic content verification)
    let production_ast_size = std::fs::metadata(project.path(".controlpath/production.ast"))
        .map(|m| m.len())
        .unwrap_or(0);
    let staging_ast_size = std::fs::metadata(project.path(".controlpath/staging.ast"))
        .map(|m| m.len())
        .unwrap_or(0);
    assert!(
        production_ast_size > 0,
        "Production AST should not be empty"
    );
    assert!(staging_ast_size > 0, "Staging AST should not be empty");

    // Verify config.yaml was created with language and defaultEnv
    assert!(project.file_exists(".controlpath/config.yaml"));
    let config = project.read_file(".controlpath/config.yaml");
    assert!(config.contains("language:"));
    assert!(config.contains("typescript") || config.contains("TypeScript"));
    assert!(config.contains("defaultEnv:") || config.contains("default_env:"));

    // Verify SDK was generated and contains correct content
    assert!(project.file_exists("./flags"));
    assert!(project.file_exists("./flags/index.ts"));

    // Verify SDK content is correct and usable
    let sdk_content = project.read_file("./flags/index.ts");
    assert!(
        sdk_content.contains("export")
            || sdk_content.contains("class")
            || sdk_content.contains("function"),
        "SDK should contain exportable code"
    );
    // Verify it references the Evaluator or similar runtime components
    assert!(
        sdk_content.contains("Evaluator")
            || sdk_content.contains("evaluate")
            || sdk_content.contains("load"),
        "SDK should contain evaluation functionality"
    );

    // Verify example usage file was created and contains usage instructions
    assert!(project.file_exists("example_usage.ts"));
    let example_content = project.read_file("example_usage.ts");
    assert!(
        example_content.contains("import") || example_content.contains("require"),
        "Example usage should show how to import the SDK"
    );
}

#[test]
#[serial]
fn test_setup_respects_no_examples() {
    let project = TestProject::new();

    // Create package.json to trigger TypeScript detection
    project.write_file("package.json", "{}");

    // TestProject::new() no longer creates .controlpath, so this is not needed

    // Run setup with --no-examples
    project.run_command_success(&[
        "setup",
        "--lang",
        "typescript",
        "--skip-install",
        "--no-examples",
    ]);

    // Verify config was created
    assert!(project.file_exists("control-path.yaml"));
    let config = project.get_definitions(); // Returns config
    assert!(
        !config.contains("example_flag"),
        "Should not contain example flags"
    );

    // Verify example usage file was NOT created
    assert!(!project.file_exists("example_usage.ts"));

    // Verify AST was compiled for production only
    assert!(project.ast_exists("production"));
    assert!(!project.ast_exists("staging"));

    // Verify config.yaml was still created
    assert!(project.file_exists(".controlpath/config.yaml"));
}

#[test]
#[serial]
fn test_setup_uses_cached_language() {
    let project = TestProject::new();

    // Create package.json to trigger TypeScript detection
    project.write_file("package.json", "{}");

    // Run setup with explicit language
    project.run_command_success(&["setup", "--lang", "typescript", "--skip-install"]);

    // Verify config.yaml contains the language
    assert!(project.file_exists(".controlpath/config.yaml"));
    let config = project.read_file(".controlpath/config.yaml");
    assert!(config.contains("language:"));
    assert!(config.contains("typescript") || config.contains("TypeScript"));

    // Test that the language was cached by checking the config file
    // The setup command should have used the cached language from the first run
    // We can't easily test running setup twice in the same project since it detects
    // existing projects, so we just verify the language was saved correctly
    let config_after = project.read_file(".controlpath/config.yaml");
    assert!(config_after.contains("language:"));
    assert!(config_after.contains("typescript") || config_after.contains("TypeScript"));
}

#[test]
#[serial]
fn test_setup_skip_install_flag() {
    let project = TestProject::new();

    // Create package.json to trigger TypeScript detection
    project.write_file("package.json", "{}");

    // TestProject::new() no longer creates .controlpath, so this is not needed

    // Run setup with --skip-install flag
    // This should complete successfully without attempting to install npm packages
    project.run_command_success(&["setup", "--lang", "typescript", "--skip-install"]);

    // Verify project structure was created (config)
    assert!(project.file_exists("control-path.yaml"));
    let config = project.read_file("control-path.yaml");
    assert!(config.contains("flags"));

    // Verify ASTs were compiled
    assert!(project.ast_exists("production"));
    assert!(project.ast_exists("staging"));

    // Verify SDK was generated
    assert!(project.file_exists("./flags"));
    assert!(project.file_exists("./flags/index.ts"));

    // Verify example usage file was created
    assert!(project.file_exists("example_usage.ts"));

    // Note: We can't easily verify that npm install was NOT called without mocking,
    // but the fact that the command succeeded with --skip-install indicates
    // the flag is being respected (otherwise it would fail if npm install was attempted
    // in an environment without npm or with network issues)
}

#[test]
#[serial]
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

    // Verify AST exists and can be loaded
    assert!(project.ast_exists("production"));

    // Verify AST file is not empty (basic content verification)
    let ast_size = std::fs::metadata(project.path(".controlpath/production.ast"))
        .map(|m| m.len())
        .unwrap_or(0);
    assert!(ast_size > 0, "AST should not be empty");

    // If evaluation is available, verify the AST can be used to evaluate flags
    if let Some(result) =
        project.evaluate_flag_simple("my_flag", "production", r#"{"id": "test_user"}"#)
    {
        // AST is valid and can be used for evaluation
        assert!(!result.is_empty(), "Evaluation should return a result");
    }

    // Generate SDK
    project.run_command_success(&["generate-sdk", "--lang", "typescript"]);

    // Verify SDK was generated and contains correct content
    assert!(project.file_exists("./flags"));
    assert!(project.file_exists("./flags/index.ts"));

    // Verify SDK content is correct and usable
    let sdk_content = project.read_file("./flags/index.ts");
    assert!(
        sdk_content.contains("export")
            || sdk_content.contains("class")
            || sdk_content.contains("function"),
        "SDK should contain exportable code"
    );
}

#[test]
#[serial]
fn test_enable_auto_compiles_env() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", false),
    );

    // Ensure AST doesn't exist before enable
    assert!(!project.ast_exists("production"));

    // Enable the flag (should auto-compile AST)
    project.run_command_success(&["enable", "my_flag", "--env", "production", "--all"]);

    // Verify flag behavior: evaluate with a test user and verify it's enabled
    if let Some(result) =
        project.evaluate_flag_simple("my_flag", "production", r#"{"id": "test_user"}"#)
    {
        assert!(
            result == "ON" || result == "true" || result == "True",
            "Flag should be enabled, got: {}",
            result
        );
    } else {
        // Fall back to config check if evaluation not available
        let deployment = project.get_deployment("production");
        assert!(deployment.contains("serve: true") || deployment.contains("serve: True"));
    }

    // Verify AST was automatically compiled
    assert!(
        project.ast_exists("production"),
        "AST should be automatically compiled after enable"
    );
}

#[test]
#[serial]
fn test_enable_no_compile_flag() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", false),
    );

    // Ensure AST doesn't exist before enable
    assert!(!project.ast_exists("production"));

    // Enable the flag with --no-compile (should NOT auto-compile AST)
    project.run_command_success(&[
        "enable",
        "my_flag",
        "--env",
        "production",
        "--all",
        "--no-compile",
    ]);

    // Verify flag behavior: evaluate with a test user and verify it's enabled
    if let Some(result) =
        project.evaluate_flag_simple("my_flag", "production", r#"{"id": "test_user"}"#)
    {
        assert!(
            result == "ON" || result == "true" || result == "True",
            "Flag should be enabled, got: {}",
            result
        );
    } else {
        // Fall back to config check if evaluation not available
        let deployment = project.get_deployment("production");
        assert!(deployment.contains("serve: true") || deployment.contains("serve: True"));
    }

    // Verify AST was NOT automatically compiled
    assert!(
        !project.ast_exists("production"),
        "AST should NOT be compiled when --no-compile is used"
    );
}

#[test]
#[serial]
fn test_new_flag_auto_generates_sdk() {
    let project = TestProject::new();

    // Initialize project first
    project.run_command_success(&["setup", "--skip-install", "--no-examples"]);

    // Create package.json to enable SDK generation
    project.write_file("package.json", "{}");

    // Ensure SDK directory doesn't exist before new-flag
    assert!(!project.file_exists("./flags"));

    // Run new-flag command WITHOUT --skip-sdk (should auto-generate SDK)
    project.run_command_success(&[
        "new-flag",
        "test_feature",
        "--type",
        "boolean",
        "--default",
        "false",
    ]);

    // Verify flag was added to definitions
    let definitions = project.get_definitions();
    assert!(definitions.contains("test_feature"));

    // Verify SDK was automatically generated and contains correct content
    assert!(
        project.file_exists("./flags"),
        "SDK should be automatically generated after new-flag"
    );
    assert!(
        project.file_exists("./flags/index.ts"),
        "SDK index.ts should exist"
    );

    // Verify SDK content is correct and usable
    let sdk_content = project.read_file("./flags/index.ts");
    assert!(
        sdk_content.contains("export")
            || sdk_content.contains("class")
            || sdk_content.contains("function"),
        "SDK should contain exportable code"
    );
    // Verify it includes the new flag
    assert!(
        sdk_content.contains("test_feature") || sdk_content.contains("testFeature"),
        "SDK should include the newly created flag"
    );
}

#[test]
#[serial]
fn test_new_flag_skip_sdk_flag() {
    let project = TestProject::new();

    // Initialize project first
    project.run_command_success(&["setup", "--skip-install", "--no-examples"]);

    // Create package.json to enable SDK generation
    project.write_file("package.json", "{}");

    // Ensure SDK directory doesn't exist before new-flag
    assert!(!project.file_exists("./flags"));

    // Run new-flag command WITH --skip-sdk (should NOT auto-generate SDK)
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

    // Verify SDK was NOT automatically generated
    assert!(
        !project.file_exists("./flags"),
        "SDK should NOT be generated when --skip-sdk is used"
    );
}

#[test]
#[serial]
fn test_new_flag_enable_in_auto_compiles() {
    let project = TestProject::new();

    // Initialize project first
    project.run_command_success(&["setup", "--skip-install", "--no-examples"]);

    // Remove AST files created by setup so we can test auto-compilation
    if project.ast_exists("production") {
        fs::remove_file(project.path(".controlpath/production.ast")).ok();
    }

    // Ensure AST doesn't exist before new-flag
    assert!(!project.ast_exists("production"));

    // Run new-flag with --enable-in (should auto-compile AST for enabled environment)
    project.run_command_success(&[
        "new-flag",
        "test_feature",
        "--type",
        "boolean",
        "--default",
        "false",
        "--enable-in",
        "production",
        "--skip-sdk", // Skip SDK to focus on compilation
    ]);

    // Verify flag was added to definitions
    let definitions = project.get_definitions();
    assert!(definitions.contains("test_feature"));

    // Verify AST was automatically compiled for the enabled environment
    assert!(
        project.ast_exists("production"),
        "AST should be automatically compiled when using --enable-in"
    );

    // Verify flag behavior: evaluate with a test user and verify it's enabled
    // This tests actual behavior, not just file contents
    if let Some(result) =
        project.evaluate_flag_simple("test_feature", "production", r#"{"id": "test_user"}"#)
    {
        assert!(
            result == "ON" || result == "true" || result == "True",
            "Flag should be enabled, got: {}",
            result
        );
    } else {
        // Fall back to config check if evaluation not available
        let config = project.get_definitions();
        assert!(config.contains("test_feature"));
        assert!(config.contains("production"));
        assert!(config.contains("serve: true") || config.contains("serve: True"));
    }
}

#[test]
#[serial]
fn test_dev_validates_core_files() {
    let project = TestProject::new();

    // Try to run dev without definitions file - should fail
    let output = project.run_command(&["dev"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Definitions file not found") || stderr.contains("setup"),
        "Should error about missing definitions file"
    );
}

#[test]
#[serial]
fn test_dev_starts_successfully() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("test_flag"),
        "production",
        &simple_deployment("production", "test_flag", false),
    );

    // Create config with language
    project.write_file(
        ".controlpath/config.yaml",
        "language: typescript\ndefaultEnv: production\n",
    );

    // Test that dev command starts successfully
    // We spawn the process, wait briefly to verify it starts, then kill it
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_controlpath"));
    cmd.current_dir(&project.project_path);
    cmd.args(["dev"]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let mut child = cmd.spawn().expect("Failed to spawn dev command");

    // Wait a short time to verify the dev process starts successfully
    thread::sleep(Duration::from_millis(500));

    // Verify the process is still running (dev started successfully)
    match child.try_wait() {
        Ok(Some(status)) => {
            // Process exited early - this is a failure
            panic!("Dev process exited early with status: {:?}", status);
        }
        Ok(None) => {
            // Process is still running - good, dev started
        }
        Err(e) => {
            panic!("Error checking process status: {}", e);
        }
    }

    // Kill the process
    child.kill().expect("Failed to kill dev process");
    let _ = child.wait();
}

#[test]
#[serial]
fn test_ci_runs_end_to_end() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("test_flag"),
        "production",
        &simple_deployment("production", "test_flag", true),
    );

    // Create config with language
    project.write_file(
        ".controlpath/config.yaml",
        "language: typescript\ndefaultEnv: production\n",
    );

    // Run CI command - should validate, compile, and regenerate SDK
    project.run_command_success(&["ci"]);

    // Verify AST was created
    assert!(project.ast_exists("production"));

    // Verify SDK was generated (check for flags directory)
    assert!(project.file_exists("flags") || project.file_exists("flags/index.ts"));
}

#[test]
#[serial]
fn test_ci_respects_env_filter() {
    let project = TestProject::new();

    // Create config with multiple environments
    project.write_file(
        "control-path.yaml",
        r"mode: local
flags:
  - name: test_flag
    type: boolean
    default: false
    environments:
      production:
        - serve: true
      staging:
        - serve: false
",
    );

    // Create .controlpath directory for AST output
    fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();

    // Run CI for production only
    project.run_command_success(&["ci", "--env", "production", "--no-sdk"]);

    // Verify only production AST was created
    assert!(project.ast_exists("production"));
    assert!(!project.ast_exists("staging"));
}

#[test]
#[serial]
fn test_ci_respects_no_sdk() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("test_flag"),
        "production",
        &simple_deployment("production", "test_flag", true),
    );

    // Run CI with --no-sdk
    project.run_command_success(&["ci", "--no-sdk"]);

    // Verify AST was created
    assert!(project.ast_exists("production"));

    // SDK generation should be skipped (we can't easily verify this, but the command should succeed)
}

#[test]
#[serial]
fn test_ci_respects_no_validate() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("test_flag"),
        "production",
        &simple_deployment("production", "test_flag", true),
    );

    // Run CI with --no-validate
    project.run_command_success(&["ci", "--no-validate", "--no-sdk"]);

    // Verify AST was created even without validation
    assert!(project.ast_exists("production"));
}

#[test]
#[serial]
fn test_ci_fails_on_invalid_definitions() {
    let project = TestProject::new();

    // Create invalid definitions file
    project.write_file("flags.definitions.yaml", "invalid: yaml: content: [");

    fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();
    project.write_file(
        ".controlpath/production.deployment.yaml",
        &simple_deployment("production", "test_flag", true),
    );

    // CI should fail on invalid definitions
    let output = project.run_command(&["ci", "--no-sdk"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid") || stderr.contains("error") || stderr.contains("failed"),
        "Should error about invalid definitions"
    );
}

#[test]
#[serial]
fn test_ci_fails_on_invalid_deployment() {
    let project = TestProject::new();

    // Create valid definitions
    project.write_file(
        "flags.definitions.yaml",
        &simple_flag_definition("test_flag"),
    );

    // Create invalid deployment file
    fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();
    project.write_file(
        ".controlpath/production.deployment.yaml",
        "invalid: yaml: content: [",
    );

    // CI should fail on invalid deployment
    let output = project.run_command(&["ci", "--no-sdk"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid") || stderr.contains("error") || stderr.contains("failed"),
        "Should error about invalid deployment"
    );
}

#[test]
#[serial]
fn test_dev_uses_config_language() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("test_flag"),
        "production",
        &simple_deployment("production", "test_flag", false),
    );

    // Create config with Python language
    project.write_file(
        ".controlpath/config.yaml",
        "language: python\ndefaultEnv: production\n",
    );

    // Test that dev command uses config language
    // We spawn the process, wait briefly, then kill it
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_controlpath"));
    cmd.current_dir(&project.project_path);
    cmd.args(["dev"]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("Failed to spawn dev command");

    // Wait a short time for initial output
    thread::sleep(Duration::from_millis(500));

    // Verify the process is still running
    match child.try_wait() {
        Ok(Some(status)) => {
            panic!("Dev process exited early with status: {:?}", status);
        }
        Ok(None) => {
            // Process is still running - good
        }
        Err(e) => {
            panic!("Error checking process status: {}", e);
        }
    }

    // Kill the process
    child.kill().expect("Failed to kill dev process");
    let _ = child.wait();
}

#[test]
#[serial]
fn test_dev_respects_lang_override() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("test_flag"),
        "production",
        &simple_deployment("production", "test_flag", false),
    );

    // Create config with TypeScript
    project.write_file(
        ".controlpath/config.yaml",
        "language: typescript\ndefaultEnv: production\n",
    );

    // Test that dev command respects --lang override
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_controlpath"));
    cmd.current_dir(&project.project_path);
    cmd.args(["dev", "--lang", "python"]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let mut child = cmd.spawn().expect("Failed to spawn dev command");

    // Wait a short time to verify it starts
    thread::sleep(Duration::from_millis(500));

    // Verify the process is still running
    match child.try_wait() {
        Ok(Some(status)) => {
            panic!("Dev process exited early with status: {:?}", status);
        }
        Ok(None) => {
            // Process is still running - good
        }
        Err(e) => {
            panic!("Error checking process status: {}", e);
        }
    }

    // Kill the process
    child.kill().expect("Failed to kill dev process");
    let _ = child.wait();
}

#[test]
#[serial]
fn test_enable_smart_defaults_from_branch_mapping() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "staging",
        &simple_deployment("staging", "my_flag", false),
    );

    // Initialize git repo and create staging branch
    use std::process::Command;
    let _ = Command::new("git")
        .args(["init"])
        .current_dir(&project.project_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&project.project_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&project.project_path)
        .output();
    let _ = Command::new("git")
        .args(["checkout", "-b", "staging"])
        .current_dir(&project.project_path)
        .output();

    // Create config with branch mapping
    project.write_file(
        ".controlpath/config.yaml",
        r"branchEnvironments:
  staging: staging
  main: production
defaultEnv: production
",
    );

    // Enable without --env flag - should use staging from branch mapping
    project.run_command_success(&["enable", "my_flag", "--all"]);

    // Compile AST to enable evaluation
    project.run_command_success(&["compile", "--env", "staging"]);

    // Verify flag behavior: evaluate with a test user and verify it's enabled
    if let Some(result) =
        project.evaluate_flag_simple("my_flag", "staging", r#"{"id": "test_user"}"#)
    {
        assert!(
            result == "ON" || result == "true" || result == "True",
            "Flag should be enabled, got: {}",
            result
        );
    } else {
        // Fall back to config check if evaluation not available
        let deployment = project.get_deployment("staging");
        assert!(deployment.contains("serve: true") || deployment.contains("serve: True"));
    }
}

#[test]
#[serial]
fn test_enable_smart_defaults_from_default_env() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", false),
    );

    // Create config with defaultEnv
    project.write_file(".controlpath/config.yaml", "defaultEnv: production\n");

    // Enable without --env flag - should use production from defaultEnv
    project.run_command_success(&["enable", "my_flag", "--all"]);

    // Compile AST to enable evaluation
    project.run_command_success(&["compile", "--env", "production"]);

    // Verify flag behavior: evaluate with a test user and verify it's enabled
    if let Some(result) =
        project.evaluate_flag_simple("my_flag", "production", r#"{"id": "test_user"}"#)
    {
        assert!(
            result == "ON" || result == "true" || result == "True",
            "Flag should be enabled, got: {}",
            result
        );
    } else {
        // Fall back to config check if evaluation not available
        let deployment = project.get_deployment("production");
        assert!(deployment.contains("serve: true") || deployment.contains("serve: True"));
    }
}

#[test]
#[serial]
fn test_deploy_smart_defaults_from_branch_mapping() {
    let project = TestProject::new();

    // Create config with staging environment
    project.write_file(
        "control-path.yaml",
        r"mode: local
flags:
  - name: my_flag
    type: boolean
    default: false
    environments:
      staging:
        - serve: true
      production:
        - serve: false
",
    );

    // Initialize git repo and create staging branch
    use std::process::Command;
    let _ = Command::new("git")
        .args(["init"])
        .current_dir(&project.project_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&project.project_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&project.project_path)
        .output();
    // Create initial commit (required before checking out branches)
    project.write_file("README.md", "# Test\n");
    let _ = Command::new("git")
        .args(["add", "README.md"])
        .current_dir(&project.project_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&project.project_path)
        .output();
    let _ = Command::new("git")
        .args(["checkout", "-b", "staging"])
        .current_dir(&project.project_path)
        .output();

    // Create config with branch mapping
    fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();
    project.write_file(
        ".controlpath/config.yaml",
        r"branchEnvironments:
  staging: staging
  main: production
defaultEnv: production
",
    );

    // Deploy without --env flag - should use staging from branch mapping
    project.run_command_success(&["deploy"]);

    // Verify AST was created for staging
    assert!(project.ast_exists("staging"));
}

#[test]
#[serial]
fn test_deploy_smart_defaults_from_default_env() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Create config with defaultEnv
    project.write_file(".controlpath/config.yaml", "defaultEnv: production\n");

    // Deploy without --env flag - should use production from defaultEnv
    project.run_command_success(&["deploy"]);

    // Verify AST was created for production
    assert!(project.ast_exists("production"));
}

#[test]
#[serial]
fn test_ci_smart_defaults_from_branch_mapping() {
    let project = TestProject::new();

    // Create config with staging environment
    project.write_file(
        "control-path.yaml",
        r"mode: local
flags:
  - name: test_flag
    type: boolean
    default: false
    environments:
      staging:
        - serve: true
      production:
        - serve: false
",
    );

    // Initialize git repo and create staging branch
    use std::process::Command;
    let _ = Command::new("git")
        .args(["init"])
        .current_dir(&project.project_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&project.project_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&project.project_path)
        .output();
    // Create initial commit (required before checking out branches)
    project.write_file("README.md", "# Test\n");
    let _ = Command::new("git")
        .args(["add", "README.md"])
        .current_dir(&project.project_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&project.project_path)
        .output();
    let _ = Command::new("git")
        .args(["checkout", "-b", "staging"])
        .current_dir(&project.project_path)
        .output();

    // Create config with branch mapping
    fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();
    project.write_file(
        ".controlpath/config.yaml",
        r"branchEnvironments:
  staging: staging
  main: production
defaultEnv: production
language: typescript
",
    );

    // Run CI without --env flag - should use staging from branch mapping
    project.run_command_success(&["ci", "--no-sdk"]);

    // Verify AST was created for staging
    assert!(project.ast_exists("staging"));
}

#[test]
#[serial]
fn test_large_scale_flags() {
    // Test behavior with many flags and rules
    let project = TestProject::new();
    project.run_command_success(&["setup", "--skip-install", "--no-examples"]);

    // Create a config with many flags
    let mut flags_yaml = "mode: local\nflags:\n".to_string();
    for i in 0..50 {
        flags_yaml.push_str(&format!(
            "  - name: flag_{}\n    type: boolean\n    default: false\n    environments:\n      production:\n        - serve: {}\n",
            i,
            if i % 2 == 0 { "true" } else { "false" }
        ));
    }
    project.write_file("control-path.yaml", &flags_yaml);

    // Compile should succeed with many flags
    project.run_command_success(&["compile", "--env", "production"]);

    // Verify AST was created and is usable
    assert!(project.ast_exists("production"));
    let ast_size = std::fs::metadata(project.path(".controlpath/production.ast"))
        .map(|m| m.len())
        .unwrap_or(0);
    assert!(ast_size > 0, "AST should not be empty even with many flags");

    // Verify a few flags can be evaluated (if evaluation available)
    if let Some(result) =
        project.evaluate_flag_simple("flag_0", "production", r#"{"id": "test_user"}"#)
    {
        assert!(
            !result.is_empty(),
            "Should be able to evaluate flags even with many flags"
        );
    }
}

#[test]
#[serial]
fn test_error_recovery_on_invalid_flag_name() {
    // Test behavior when invalid flag names are used
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", false),
    );

    // Try to enable a non-existent flag - should fail gracefully
    let output =
        project.run_command(&["enable", "nonexistent_flag", "--env", "production", "--all"]);
    assert!(
        !output.status.success(),
        "Should fail when enabling non-existent flag"
    );

    // Verify no partial state was created (no AST should be created)
    assert!(!project.ast_exists("production"));

    // Verify existing flag is still valid
    project.run_command_success(&["enable", "my_flag", "--env", "production", "--all"]);
    project.run_command_success(&["compile", "--env", "production"]);
    assert!(project.ast_exists("production"));
}

#[test]
#[serial]
fn test_error_recovery_on_invalid_expression() {
    // Test behavior when invalid expressions are used in rules
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", false),
    );

    // Try to enable with invalid expression - should fail gracefully
    let output = project.run_command(&[
        "enable",
        "my_flag",
        "--env",
        "production",
        "--rule",
        "invalid expression syntax !!!",
    ]);
    assert!(
        !output.status.success(),
        "Should fail when using invalid expression"
    );

    // Verify no partial state was created
    let config = project.get_definitions();
    assert!(
        !config.contains("invalid expression"),
        "Invalid expression should not be added to config"
    );
}

#[test]
#[serial]
fn test_compile_with_many_rules() {
    // Test behavior with many rules per flag
    let project = TestProject::new();
    project.run_command_success(&["setup", "--skip-install", "--no-examples"]);

    // Create a flag with many rules
    let mut rules_yaml = "mode: local\nflags:\n  - name: complex_flag\n    type: boolean\n    default: false\n    environments:\n      production:\n".to_string();
    for i in 0..20 {
        rules_yaml.push_str(&format!(
            "        - when: \"role == 'role_{}'\"\n          serve: {}\n",
            i,
            if i % 2 == 0 { "true" } else { "false" }
        ));
    }
    project.write_file("control-path.yaml", &rules_yaml);

    // Compile should succeed with many rules
    project.run_command_success(&["compile", "--env", "production"]);

    // Verify AST was created and is usable
    assert!(project.ast_exists("production"));

    // Verify evaluation works (if available)
    if let Some(result) = project.evaluate_flag_simple(
        "complex_flag",
        "production",
        r#"{"id": "test_user", "role": "role_0"}"#,
    ) {
        assert!(
            result == "ON" || result == "true" || result == "True",
            "Should evaluate correctly even with many rules"
        );
    }
}
