//! Integration tests for watch mode

mod integration_test_helpers;

use integration_test_helpers::*;
use serial_test::serial;
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
#[serial]
fn test_watch_mode_definitions_change() {
    let project = TestProject::with_definitions(&simple_flag_definition("initial_flag"));

    fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_controlpath"));
    cmd.current_dir(&project.project_path);
    cmd.args(["watch", "--lang", "typescript"]);
    // Don't suppress stderr so we can see errors
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("Failed to spawn watch command");

    // Wait a short time to verify the watch process starts successfully
    thread::sleep(Duration::from_millis(500));

    // Verify the process is still running (watch started successfully)
    match child.try_wait() {
        Ok(Some(status)) => {
            // Process exited early - this is a failure
            panic!("Watch process exited early with status: {:?}", status);
        }
        Ok(None) => {
            // Process is still running - good, watch started
        }
        Err(e) => {
            panic!("Error checking process status: {}", e);
        }
    }

    // Kill the process
    child.kill().expect("Failed to kill watch process");
    let _ = child.wait();
}

#[test]
#[serial]
fn test_watch_mode_help() {
    let project = TestProject::new();

    // Test that watch command shows help
    let output = project.run_command(&["watch", "--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("watch") || stdout.contains("Watch"));
}

/// v2 project: changing control-path.yaml while watch runs should regenerate the SDK.
#[test]
#[serial]
fn test_watch_v2_regenerates_sdk_on_catalog_change() {
    let project = TestProject::with_definitions(&simple_flag_definition("initial_flag"));
    fs::create_dir_all(project.project_path.join(".controlpath")).unwrap();
    project.write_file(
        ".controlpath/config.yaml",
        "language: typescript\ndefaultEnv: production\n",
    );

    project.run_command_success(&["generate-sdk", "--lang", "typescript"]);
    let types_path = project.path("node_modules/@controlpath/generated/types.ts");
    let before = fs::read_to_string(&types_path).unwrap();
    assert!(!before.contains("addedFlag"));

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_controlpath"));
    cmd.current_dir(&project.project_path);
    cmd.args(["watch", "--definitions", "--lang", "typescript"]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("Failed to spawn watch command");
    thread::sleep(Duration::from_millis(800));

    fs::write(
        project.path("control-path.yaml"),
        r"catalog:
  id: test-service
mode: local
flags:
  initial_flag:
    default: false
    kind: release
  added_flag:
    default: true
    kind: release
environments:
  production:
    rules:
      initial_flag:
        - serve: true
",
    )
    .unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    let mut regen_seen = false;
    while std::time::Instant::now() < deadline {
        if let Ok(after) = fs::read_to_string(&types_path) {
            if after.contains("addedFlag") {
                regen_seen = true;
                break;
            }
        }
        thread::sleep(Duration::from_millis(200));
    }

    let _ = child.kill();
    let _ = child.wait();

    assert!(
        regen_seen,
        "watch should regenerate SDK when control-path.yaml changes (v2)"
    );
}
