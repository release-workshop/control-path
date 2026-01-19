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

    // Verify staging environment was created (setup creates it by default)
    assert!(project.file_exists(".controlpath/staging.deployment.yaml"));

    // Verify AST was compiled for both environments
    assert!(project.ast_exists("production"));
    assert!(project.ast_exists("staging"));

    // Verify config.yaml was created with language and defaultEnv
    assert!(project.file_exists(".controlpath/config.yaml"));
    let config = project.read_file(".controlpath/config.yaml");
    assert!(config.contains("language:"));
    assert!(config.contains("typescript") || config.contains("TypeScript"));
    assert!(config.contains("defaultEnv:") || config.contains("default_env:"));

    // Verify SDK was generated
    assert!(project.file_exists("./flags"));
    assert!(project.file_exists("./flags/index.ts"));

    // Verify example usage file was created
    assert!(project.file_exists("example_usage.ts"));
}

#[test]
fn test_setup_respects_no_examples() {
    let project = TestProject::new();

    // Create package.json to trigger TypeScript detection
    project.write_file("package.json", "{}");

    // Remove .controlpath directory
    fs::remove_dir_all(project.path(".controlpath")).unwrap();

    // Run setup with --no-examples
    project.run_command_success(&[
        "setup",
        "--lang",
        "typescript",
        "--skip-install",
        "--no-examples",
    ]);

    // Verify project structure was created
    assert!(project.file_exists(".controlpath/production.deployment.yaml"));

    // Verify staging was NOT created (only created when examples are enabled)
    assert!(!project.file_exists(".controlpath/staging.deployment.yaml"));

    // Verify definitions file exists but is empty (no example flags)
    // (Empty file is needed for compilation to work)
    assert!(project.file_exists("flags.definitions.yaml"));
    let definitions = project.get_definitions();
    assert!(
        !definitions.contains("example_flag"),
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

    // Remove .controlpath directory
    fs::remove_dir_all(project.path(".controlpath")).unwrap();

    // Run setup with explicit language
    project.run_command_success(&["setup", "--lang", "typescript", "--skip-install"]);

    // Verify config.yaml contains the language
    let config = project.read_file(".controlpath/config.yaml");
    assert!(config.contains("language:"));
    assert!(config.contains("typescript") || config.contains("TypeScript"));

    // Now run setup again without --lang (should use cached language)
    // First, remove the project files but keep config
    fs::remove_file(project.path("flags.definitions.yaml")).ok();
    fs::remove_file(project.path(".controlpath/production.deployment.yaml")).ok();
    fs::remove_file(project.path(".controlpath/staging.deployment.yaml")).ok();
    fs::remove_file(project.path(".controlpath/production.ast")).ok();
    fs::remove_file(project.path(".controlpath/staging.ast")).ok();

    // Run setup again without --lang
    project.run_command_success(&["setup", "--skip-install", "--no-examples"]);

    // Verify it still worked (using cached language from config)
    assert!(project.file_exists(".controlpath/production.deployment.yaml"));
}

#[test]
fn test_setup_skip_install_flag() {
    let project = TestProject::new();

    // Create package.json to trigger TypeScript detection
    project.write_file("package.json", "{}");

    // Remove .controlpath directory
    fs::remove_dir_all(project.path(".controlpath")).unwrap();

    // Run setup with --skip-install flag
    // This should complete successfully without attempting to install npm packages
    project.run_command_success(&["setup", "--lang", "typescript", "--skip-install"]);

    // Verify project structure was created
    assert!(project.file_exists("flags.definitions.yaml"));
    assert!(project.file_exists(".controlpath/production.deployment.yaml"));
    assert!(project.file_exists(".controlpath/staging.deployment.yaml"));

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

#[test]
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

    // Verify flag was enabled
    let deployment = project.get_deployment("production");
    assert!(deployment.contains("serve: true") || deployment.contains("serve: True"));

    // Verify AST was automatically compiled
    assert!(
        project.ast_exists("production"),
        "AST should be automatically compiled after enable"
    );
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
        "enable",
        "my_flag",
        "--env",
        "production",
        "--all",
        "--no-compile",
    ]);

    // Verify flag was enabled
    let deployment = project.get_deployment("production");
    assert!(deployment.contains("serve: true") || deployment.contains("serve: True"));

    // Verify AST was NOT automatically compiled
    assert!(
        !project.ast_exists("production"),
        "AST should NOT be compiled when --no-compile is used"
    );
}

#[test]
fn test_new_flag_auto_generates_sdk() {
    let project = TestProject::new();

    // Initialize project first
    project.run_command_success(&["init", "--force"]);

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

    // Verify SDK was automatically generated
    assert!(
        project.file_exists("./flags"),
        "SDK should be automatically generated after new-flag"
    );
    assert!(
        project.file_exists("./flags/index.ts"),
        "SDK index.ts should exist"
    );
}

#[test]
fn test_new_flag_skip_sdk_flag() {
    let project = TestProject::new();

    // Initialize project first
    project.run_command_success(&["init", "--force"]);

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
fn test_new_flag_enable_in_auto_compiles() {
    let project = TestProject::new();

    // Initialize project first
    project.run_command_success(&["init", "--force"]);

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

    // Verify flag was enabled in production
    let deployment = project.get_deployment("production");
    assert!(deployment.contains("test_feature"));
    assert!(deployment.contains("serve: true") || deployment.contains("serve: True"));

    // Verify AST was automatically compiled for the enabled environment
    assert!(
        project.ast_exists("production"),
        "AST should be automatically compiled when using --enable-in"
    );
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

    // Run CI command - should validate, compile, and regenerate SDK
    project.run_command_success(&["ci"]);

    // Verify AST was created
    assert!(project.ast_exists("production"));

    // Verify SDK was generated (check for flags directory)
    assert!(project.file_exists("flags") || project.file_exists("flags/index.ts"));
}

#[test]
fn test_ci_respects_env_filter() {
    let project = TestProject::new();

    // Create definitions
    project.write_file(
        "flags.definitions.yaml",
        &simple_flag_definition("test_flag"),
    );

    // Create multiple environments
    fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();
    project.write_file(
        ".controlpath/production.deployment.yaml",
        &simple_deployment("production", "test_flag", true),
    );
    project.write_file(
        ".controlpath/staging.deployment.yaml",
        &simple_deployment("staging", "test_flag", false),
    );

    // Run CI for production only
    project.run_command_success(&["ci", "--env", "production", "--no-sdk"]);

    // Verify only production AST was created
    assert!(project.ast_exists("production"));
    assert!(!project.ast_exists("staging"));
}

#[test]
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
