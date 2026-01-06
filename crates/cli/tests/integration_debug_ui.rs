//! Integration tests for debug UI

mod integration_test_helpers;

use integration_test_helpers::*;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
fn test_debug_ui_help() {
    let project = TestProject::new();

    // Test that debug command shows help
    let output = project.run_command(&["debug", "--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("debug") || stdout.contains("Debug"));
}

#[test]
fn test_debug_ui_missing_ast() {
    let project = TestProject::new();

    // Debug UI should fail without AST file
    let output = project.run_command_failure(&["debug", "--env", "production"]);
    assert!(!output.status.success());
}

#[test]
fn test_debug_ui_command_structure() {
    let project = TestProject::with_deployment(
        &simple_flag_definition("my_flag"),
        "production",
        &simple_deployment("production", "my_flag", true),
    );

    // Compile first to create AST
    project.run_command_success(&["compile", "--env", "production"]);

    // Test that debug command accepts valid arguments and starts the server
    // We spawn the process, wait briefly to verify it starts, then kill it

    // Test with --env flag
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_controlpath"));
    cmd.current_dir(&project.project_path);
    cmd.args(["debug", "--env", "production"]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let mut child = cmd.spawn().expect("Failed to spawn debug command");
    
    // Wait a short time to verify the server starts successfully
    thread::sleep(Duration::from_millis(500));
    
    // Verify the process is still running (server started successfully)
    match child.try_wait() {
        Ok(Some(status)) => {
            // Process exited early - this is a failure
            panic!("Debug server exited early with status: {:?}", status);
        }
        Ok(None) => {
            // Process is still running - good, server started
        }
        Err(e) => {
            panic!("Error checking process status: {}", e);
        }
    }

    // Kill the process
    child.kill().expect("Failed to kill debug process");
    let _ = child.wait();

    // Test with --ast flag
    let mut cmd2 = Command::new(env!("CARGO_BIN_EXE_controlpath"));
    cmd2.current_dir(&project.project_path);
    cmd2.args(["debug", "--ast", ".controlpath/production.ast"]);
    cmd2.stdout(Stdio::null());
    cmd2.stderr(Stdio::null());

    let mut child2 = cmd2.spawn().expect("Failed to spawn debug command");
    
    // Wait a short time to verify the server starts successfully
    thread::sleep(Duration::from_millis(500));
    
    // Verify the process is still running
    match child2.try_wait() {
        Ok(Some(status)) => {
            panic!("Debug server exited early with status: {:?}", status);
        }
        Ok(None) => {
            // Process is still running - good
        }
        Err(e) => {
            panic!("Error checking process status: {}", e);
        }
    }

    // Kill the process
    child2.kill().expect("Failed to kill debug process");
    let _ = child2.wait();
}

// Note: Full debug UI integration testing is challenging because:
// 1. Debug UI runs indefinitely (web server)
// 2. It requires HTTP client to test endpoints
// 3. It needs time for server to start
// 4. Requires proper cleanup of background processes
//
// For comprehensive debug UI testing, consider:
// - Manual testing during development
// - Using specialized test frameworks (e.g., reqwest + tokio test)
// - Unit tests for the API handlers (already in debug.rs tests)
// - Integration tests that spawn server, make requests, then verify responses
