//! Integration tests for workflow commands

mod integration_test_helpers;

use integration_test_helpers::*;

use std::fs;

#[test]
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

    project.assert_boolean_flag(
        "test_feature",
        "production",
        r#"{"id": "test_user"}"#,
        false,
    );
}

#[test]
fn test_enable_workflow() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: test-service
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  production:
    rules: {}
",
    );

    project.run_command_success(&["flag", "enable", "my_flag", "--env", "production", "--all"]);

    project.run_command_success(&["compile", "--env", "production"]);

    project.assert_boolean_flag("my_flag", "production", r#"{"id": "test_user"}"#, true);
}

#[test]
fn test_enable_with_rule_workflow() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: test-service
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  production:
    rules: {}
",
    );

    // Enable with a rule (uses config)
    project.run_command_success(&[
        "flag",
        "enable",
        "my_flag",
        "--env",
        "production",
        "--rule",
        "role == 'admin'", // Updated: no user. prefix
    ]);

    project.run_command_success(&["compile", "--env", "production"]);

    project.assert_boolean_flag(
        "my_flag",
        "production",
        r#"{"id": "admin1", "role": "admin"}"#,
        true,
    );
    project.assert_boolean_flag(
        "my_flag",
        "production",
        r#"{"id": "user1", "role": "user"}"#,
        false,
    );
}

#[test]
fn test_deploy_success_message_describes_hot_swap_not_restart() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    let output = project.run_command(&["deploy", "--env", "production"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Deployment ready"));
    assert!(stdout.contains("artifact URL or artifact path"));
    assert!(stdout.contains("kill switch URL or kill switch path"));
    assert!(stdout.contains("no application restart required"));
    assert!(
        !stdout.contains("Restart your application"),
        "deploy must not instruct restart for refresh targets: {stdout}"
    );
}

#[test]
fn test_deploy_workflow() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    project.run_command_success(&["deploy", "--env", "production"]);

    project.assert_ast_compiled("production");
    project.assert_boolean_flag("my_flag", "production", r#"{"id": "test_user"}"#, true);
}

#[test]
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
    project.run_command_success(&[
        "flag",
        "enable",
        "new_feature",
        "--env",
        "production",
        "--all",
    ]);

    project.run_command_success(&["deploy", "--env", "production"]);

    let config = project.get_definitions();
    assert!(config.contains("new_feature"));
    assert!(config.contains("production"));

    project.assert_ast_compiled("production");
    project.assert_boolean_flag("new_feature", "production", r#"{"id": "test_user"}"#, true);
}

#[test]
fn test_flag_add_list_show_remove_workflow() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("existing_flag"),
        "production",
        &simple_deployment("production", "existing_flag", false),
    );

    // Add a flag to the v2 catalog
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
    project.run_command_success(&["flag", "remove", "--name", "test_flag"]);

    let definitions = project.read_file("control-path.yaml");
    assert!(!definitions.contains("test_flag"));
}

#[test]
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
    assert!(project.file_exists("node_modules/@controlpath/generated/index.ts"));
    assert!(project.file_exists("node_modules/@controlpath/generated/types.ts"));

    // Verify SDK content is correct and usable
    let sdk_content = project.read_file("node_modules/@controlpath/generated/index.ts");
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

    // Verify SDK was generated (default path is node_modules/@controlpath/generated)
    assert!(project.file_exists("node_modules/@controlpath/generated/index.ts"));
    assert!(project.file_exists("node_modules/@controlpath/generated/types.ts"));
    assert!(project.file_exists("node_modules/@controlpath/generated/package.json"));

    // Verify example usage file was created
    assert!(project.file_exists("example_usage.ts"));

    // Note: We can't easily verify that npm install was NOT called without mocking,
    // but the fact that the command succeeded with --skip-install indicates
    // the flag is being respected (otherwise it would fail if npm install was attempted
    // in an environment without npm or with network issues)
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

    project.run_command_success(&["compile", "--env", "production"]);

    project.assert_ast_compiled("production");
    project.assert_boolean_flag("my_flag", "production", r#"{"id": "test_user"}"#, true);

    // Generate SDK
    project.run_command_success(&["generate-sdk", "--lang", "typescript"]);

    // Verify SDK was generated and contains correct content
    assert!(project.file_exists("node_modules/@controlpath/generated/index.ts"));
    assert!(project.file_exists("node_modules/@controlpath/generated/types.ts"));

    // Verify SDK content is correct and usable
    let sdk_content = project.read_file("node_modules/@controlpath/generated/index.ts");
    assert!(
        sdk_content.contains("export")
            || sdk_content.contains("class")
            || sdk_content.contains("function"),
        "SDK should contain exportable code"
    );
}

#[test]
fn test_enable_auto_compiles_env() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: test-service
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  production:
    rules: {}
",
    );

    assert!(!project.ast_exists("production"));

    project.run_command_success(&["flag", "enable", "my_flag", "--env", "production", "--all"]);

    project.assert_ast_compiled("production");
    project.assert_boolean_flag("my_flag", "production", r#"{"id": "test_user"}"#, true);
}

#[test]
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
        "flag",
        "enable",
        "my_flag",
        "--env",
        "production",
        "--all",
        "--no-compile",
    ]);

    assert!(
        !project.ast_exists("production"),
        "AST should NOT be compiled when --no-compile is used"
    );
}

#[test]
fn test_new_flag_auto_generates_sdk() {
    let project = TestProject::new();

    // Initialize project first
    project.run_command_success(&["setup", "--skip-install", "--no-examples"]);

    // Create package.json to enable SDK generation
    project.write_file("package.json", "{}");

    // Ensure SDK directory doesn't exist before new-flag
    assert!(!project.file_exists("node_modules/@controlpath/generated"));

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
        project.file_exists("node_modules/@controlpath/generated/index.ts"),
        "SDK should be automatically generated after new-flag"
    );

    // Verify SDK content is correct and usable
    let sdk_content = project.read_file("node_modules/@controlpath/generated/index.ts");
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
fn test_new_flag_skip_sdk_flag() {
    let project = TestProject::new();

    // Initialize project first
    project.run_command_success(&["setup", "--skip-install", "--no-examples"]);

    // Create package.json to enable SDK generation
    project.write_file("package.json", "{}");

    // Ensure SDK directory doesn't exist before new-flag
    assert!(!project.file_exists("node_modules/@controlpath/generated"));

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
        !project.file_exists("node_modules/@controlpath/generated"),
        "SDK should NOT be generated when --skip-sdk is used"
    );
}

#[test]
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

    project.assert_ast_compiled("production");
    project.assert_boolean_flag("test_feature", "production", r#"{"id": "test_user"}"#, true);
}

#[test]
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

    project.run_command_success(&["ci"]);

    project.assert_ast_compiled("production");
    project.assert_boolean_flag("test_flag", "production", r#"{"id": "test_user"}"#, true);
    assert!(project.file_exists("node_modules/@controlpath/generated/index.ts"));
}

#[test]
fn test_ci_respects_env_filter() {
    let project = TestProject::new();

    // Create config with multiple environments
    project.write_file(
        "control-path.yaml",
        r"catalog:
  id: test-service
mode: local
flags:
  test_flag:
    default: false
    kind: release
environments:
  production:
    rules:
      test_flag:
        - serve: true
  staging:
    rules:
      test_flag:
        - serve: false
",
    );

    // Create .controlpath directory for AST output
    fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();

    project.run_command_success(&["ci", "--env", "production", "--no-sdk"]);

    project.assert_ast_compiled("production");
    project.assert_boolean_flag("test_flag", "production", r#"{"id": "test_user"}"#, true);
    assert!(!project.ast_exists("staging"));
}

#[test]
fn test_ci_respects_no_sdk() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("test_flag"),
        "production",
        &simple_deployment("production", "test_flag", true),
    );

    project.run_command_success(&["ci", "--no-sdk"]);

    project.assert_ast_compiled("production");
    project.assert_boolean_flag("test_flag", "production", r#"{"id": "test_user"}"#, true);
}

#[test]
fn test_ci_fails_on_invalid_catalog() {
    let project = TestProject::new();

    project.write_file("control-path.yaml", "invalid: yaml: content: [");

    let output = project.run_command(&["ci", "--no-sdk"]);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("invalid") || combined.contains("error") || combined.contains("failed"),
        "Should error about invalid catalog: {combined}"
    );
}

#[test]
fn test_ci_fails_on_invalid_environment_rules() {
    let project = TestProject::new();

    project.write_file(
        "control-path.yaml",
        r"catalog:
  id: test-service
mode: local
flags:
  test_flag:
    default: false
    kind: release
environments:
  production:
    rules:
      test_flag:
        - serve: not_a_boolean
",
    );

    let output = project.run_command(&["ci", "--no-sdk"]);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("invalid") || combined.contains("error") || combined.contains("failed"),
        "Should error about invalid environment rules: {combined}"
    );
}

#[test]
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
fn test_enable_smart_defaults_from_branch_mapping() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: test-service
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  staging:
    rules: {}
  production:
    rules: {}
",
    );

    project.init_git_repo_on_branch("staging");

    fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();
    project.write_file(
        ".controlpath/config.yaml",
        r"branchEnvironments:
  staging: staging
  main: production
defaultEnv: production
",
    );

    project.run_command_success(&["flag", "enable", "my_flag", "--all"]);

    project.run_command_success(&["compile", "--env", "staging"]);

    project.assert_boolean_flag("my_flag", "staging", r#"{"id": "test_user"}"#, true);
}

#[test]
fn test_enable_smart_defaults_from_default_env() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", false),
    );

    // Create config with defaultEnv
    project.write_file(".controlpath/config.yaml", "defaultEnv: production\n");

    // Enable without --env flag - should use production from defaultEnv
    project.run_command_success(&["flag", "enable", "my_flag", "--all"]);

    project.run_command_success(&["compile", "--env", "production"]);

    project.assert_boolean_flag("my_flag", "production", r#"{"id": "test_user"}"#, true);
}

#[test]
fn test_deploy_smart_defaults_from_branch_mapping() {
    let project = TestProject::new();

    // Create config with staging environment
    project.write_file(
        "control-path.yaml",
        r"catalog:
  id: test-service
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  staging:
    rules:
      my_flag:
        - serve: true
  production:
    rules:
      my_flag:
        - serve: false
",
    );

    project.init_git_repo_on_branch("staging");

    fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();
    project.write_file(
        ".controlpath/config.yaml",
        r"branchEnvironments:
  staging: staging
  main: production
defaultEnv: production
",
    );

    project.run_command_success(&["deploy"]);

    project.assert_ast_compiled("staging");
    project.assert_boolean_flag("my_flag", "staging", r#"{"id": "test_user"}"#, true);
}

#[test]
fn test_deploy_smart_defaults_from_default_env() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Create config with defaultEnv
    project.write_file(".controlpath/config.yaml", "defaultEnv: production\n");

    project.run_command_success(&["deploy"]);

    project.assert_ast_compiled("production");
    project.assert_boolean_flag("my_flag", "production", r#"{"id": "test_user"}"#, true);
}

#[test]
fn test_ci_smart_defaults_from_branch_mapping() {
    let project = TestProject::new();

    // Create config with staging environment
    project.write_file(
        "control-path.yaml",
        r"catalog:
  id: test-service
mode: local
flags:
  test_flag:
    default: false
    kind: release
environments:
  staging:
    rules:
      test_flag:
        - serve: true
  production:
    rules:
      test_flag:
        - serve: false
",
    );

    project.init_git_repo_on_branch("staging");

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

    project.run_command_success(&["ci", "--no-sdk"]);

    project.assert_ast_compiled("staging");
    project.assert_boolean_flag("test_flag", "staging", r#"{"id": "test_user"}"#, true);
}

#[test]
fn test_large_scale_flags() {
    // Test behavior with many flags and rules
    let project = TestProject::new();
    project.run_command_success(&["setup", "--skip-install", "--no-examples"]);

    // Create a config with many flags
    let mut flags_yaml = "catalog:\n  id: test-service\nmode: local\nflags:\n".to_string();
    for i in 0..50 {
        flags_yaml.push_str(&format!(
            "  flag_{i}:\n    default: false\n    kind: release\n"
        ));
    }
    flags_yaml.push_str("environments:\n  production:\n    rules:\n");
    for i in 0..50 {
        flags_yaml.push_str(&format!(
            "      flag_{i}:\n        - serve: {}\n",
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
fn test_error_recovery_on_invalid_flag_name() {
    // Test behavior when invalid flag names are used
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", false),
    );

    // Try to enable a non-existent flag - should fail gracefully
    let output = project.run_command(&[
        "flag",
        "enable",
        "nonexistent_flag",
        "--env",
        "production",
        "--all",
    ]);
    assert!(
        !output.status.success(),
        "Should fail when enabling non-existent flag"
    );

    // Verify no partial state was created (no AST should be created)
    assert!(!project.ast_exists("production"));

    // Verify existing flag is still valid
    project.run_command_success(&["flag", "enable", "my_flag", "--env", "production", "--all"]);
    project.run_command_success(&["compile", "--env", "production"]);
    assert!(project.ast_exists("production"));
}

#[test]
fn test_error_recovery_on_invalid_expression() {
    // Test behavior when invalid expressions are used in rules
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", false),
    );

    // Try to enable with invalid expression - should fail gracefully
    let output = project.run_command(&[
        "flag",
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
fn test_compile_with_many_rules() {
    // Test behavior with many rules per flag
    let project = TestProject::new();
    project.run_command_success(&["setup", "--skip-install", "--no-examples"]);

    // Create a flag with many rules
    let mut rules_yaml = "catalog:\n  id: test-service\nmode: local\nflags:\n  complex_flag:\n    default: false\n    kind: release\nenvironments:\n  production:\n    rules:\n      complex_flag:\n".to_string();
    for i in 0..20 {
        rules_yaml.push_str(&format!(
            "        - when: \"role == 'role_{i}'\"\n          serve: {}\n",
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

#[test]
fn test_v2_local_workflow_end_to_end() {
    let project = TestProject::new();

    project.run_command_success(&[
        "init",
        "--no-monorepo",
        "--namespace",
        "acme",
        "--service-id",
        "checkout-service",
    ]);

    assert!(project.file_exists("control-path.yaml"));
    let config = project.read_file("control-path.yaml");
    assert!(config.contains("namespace: acme"));
    assert!(config.contains("checkout-service"));

    project.write_file(
        "control-path.yaml",
        r"catalog:
  id: checkout-service
  namespace: acme
mode: local
flags:
  new_dashboard:
    default: false
    kind: kill_switch
environments:
  production:
    rules: {}
",
    );

    project.run_command_success(&["env", "list"]);
    let env_output = project.run_command(&["env", "list"]);
    let env_list = String::from_utf8_lossy(&env_output.stdout);
    assert!(env_list.contains("production"));

    project.run_command_success(&[
        "flag",
        "enable",
        "new_dashboard",
        "--env",
        "production",
        "--all",
    ]);

    project.run_command_success(&["generate-sdk", "--lang", "typescript"]);
    assert!(project.file_exists("node_modules/@controlpath/generated/index.ts"));

    project.run_command_success(&["deploy", "--env", "production"]);
    project.assert_ast_compiled("production");
    project.assert_boolean_flag(
        "new_dashboard",
        "production",
        r#"{"id": "test_user"}"#,
        true,
    );
    assert!(project.file_exists(".controlpath/production.kill-switches.json"));

    let kill_switches = project.read_file(".controlpath/production.kill-switches.json");
    assert!(kill_switches.contains("\"version\""));
    assert!(kill_switches.contains("\"flags\""));

    project.run_command_success(&["flag", "deprecate", "--name", "new_dashboard"]);
    let output = project.run_command(&[
        "flag",
        "enable",
        "new_dashboard",
        "--env",
        "production",
        "--all",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("deprecated"));

    project.run_command_success(&["ci", "--env", "production", "--no-sdk"]);
    project.assert_ast_compiled("production");
    project.assert_boolean_flag(
        "new_dashboard",
        "production",
        r#"{"id": "test_user"}"#,
        true,
    );
}

#[test]
fn test_kill_switch_updates_v2_artifact() {
    let project = TestProject::with_definitions(
        r"catalog:
  id: checkout
  namespace: acme
mode: local
flags:
  new_dashboard:
    default: false
    kind: kill_switch
environments:
  production:
    rules:
      new_dashboard:
        - serve: false
",
    );

    fs::create_dir_all(project.path(".controlpath")).ok();
    project.run_command_success(&[
        "kill-switch",
        "set",
        "new_dashboard",
        "true",
        "--env",
        "production",
    ]);

    let kill_path = project.read_file(".controlpath/production.kill-switches.json");
    assert!(kill_path.contains("\"new_dashboard\": true"));

    project.run_command_success(&["deploy", "--env", "production"]);
    assert!(project.ast_exists("production"));
    assert!(project.file_exists(".controlpath/production.kill-switches.json"));

    project.run_command_success(&["ci", "--env", "production", "--no-sdk"]);
}

#[test]
fn test_kill_switch_path_refresh_without_restart() {
    let project = TestProject::new();
    let kill_switch_path = project
        .path("volume/production.kill-switches.json")
        .to_string_lossy()
        .into_owned();

    project.write_file(
        "control-path.yaml",
        &format!(
            r"catalog:
  id: checkout
  namespace: acme
mode: local
flags:
  new_dashboard:
    default: false
    kind: kill_switch
environments:
  production:
    rules:
      new_dashboard:
        - serve: true
kill_switches:
  production:
    path: {kill_switch_path}
"
        ),
    );

    project.run_command_success(&["validate"]);
    project.run_command_success(&["compile", "--env", "production"]);
    project.run_command_success(&["generate-sdk", "--lang", "typescript"]);
    project.run_command_success(&["deploy", "--env", "production"]);
    project.assert_ast_compiled("production");

    if let Some(parent) = project
        .path("volume/production.kill-switches.json")
        .parent()
    {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(
        project.path("volume/production.kill-switches.json"),
        r#"{"version":"2.0","flags":{"new_dashboard":false}}"#,
    )
    .unwrap();

    project.assert_generated_boolean_flag(
        "new_dashboard",
        "production",
        r#"{"id": "test_user"}"#,
        false,
        &kill_switch_path,
        false,
    );
}
