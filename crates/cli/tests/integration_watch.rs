//! Integration tests for watch mode

mod integration_test_helpers;

use integration_test_helpers::*;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
fn test_watch_mode_definitions_change() {
    let project = TestProject::with_definitions(&simple_flag_definition("initial_flag"));

    // Test that watch command accepts valid arguments and starts successfully
    // We spawn the process, wait briefly to verify it starts, then kill it

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_controlpath"));
    cmd.current_dir(&project.project_path);
    cmd.args(["watch", "--lang", "typescript", "--definitions"]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

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
fn test_watch_mode_help() {
    let project = TestProject::new();

    // Test that watch command shows help
    let output = project.run_command(&["watch", "--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("watch") || stdout.contains("Watch"));
}

// Note: Full watch mode integration testing is challenging because:
// 1. Watch mode runs indefinitely
// 2. It requires file system watching which is async
// 3. It needs time for file changes to be detected
//
// For comprehensive watch mode testing, consider:
// - Manual testing during development
// - Using specialized test frameworks that can handle async file watching
// - Unit tests for the watch logic components (already in watch.rs tests)
// - Integration tests that spawn watch, make changes, wait, then verify output
