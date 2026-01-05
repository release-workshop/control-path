//! Watch command implementation

use crate::error::{CliError, CliResult};
use crate::generator::generate_sdk;
use controlpath_compiler::{
    compile, parse_definitions, parse_deployment, serialize, validate_definitions,
    validate_deployment,
};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct Options {
    pub lang: Option<String>,
    pub definitions: bool,
    pub deployments: bool,
}

/// Determines the path to the flag definitions file.
fn determine_definitions_path() -> PathBuf {
    PathBuf::from("flags.definitions.yaml")
}

/// Finds all deployment files in the .controlpath directory.
fn find_deployment_files() -> Vec<PathBuf> {
    let controlpath_dir = PathBuf::from(".controlpath");
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(&controlpath_dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".deployment.yaml") {
                        files.push(entry.path());
                    }
                }
            }
        }
    }

    files
}

/// Determines the output path for SDK generation.
fn determine_output_path_for_sdk() -> PathBuf {
    PathBuf::from("./flags")
}

/// Determines the output path for AST compilation based on deployment path.
fn determine_output_path_for_ast(deployment_path: &Path) -> PathBuf {
    let deployment_dir = deployment_path.parent().unwrap_or_else(|| Path::new("."));
    let deployment_stem = deployment_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("deployment")
        .replace(".deployment", "");
    deployment_dir.join(format!("{deployment_stem}.ast"))
}

/// Regenerates the SDK when the definitions file changes.
fn regenerate_sdk(options: &Options) -> CliResult<()> {
    let definitions_path = determine_definitions_path();
    let output_path = determine_output_path_for_sdk();

    if !definitions_path.exists() {
        return Err(CliError::Message(format!(
            "Definitions file not found: {}",
            definitions_path.display()
        )));
    }

    // Read and parse definitions
    let definitions_content = fs::read_to_string(&definitions_path)
        .map_err(|e| CliError::Message(format!("Failed to read definitions file: {e}")))?;
    let definitions = parse_definitions(&definitions_content)?;

    // Validate definitions
    validate_definitions(&definitions)
        .map_err(|e| CliError::Message(format!("Definitions file is invalid: {e}")))?;

    // Determine language (default to typescript)
    let language = options
        .lang
        .as_deref()
        .unwrap_or("typescript")
        .to_lowercase();

    // Generate SDK
    generate_sdk(&language, &definitions, &output_path)?;

    println!("✓ SDK regenerated to {}", output_path.display());
    Ok(())
}

/// Recompiles the AST when a deployment file changes.
fn recompile_ast(deployment_path: &Path) -> CliResult<()> {
    let definitions_path = determine_definitions_path();
    let output_path = determine_output_path_for_ast(deployment_path);

    // Read and parse definitions
    let definitions_content = fs::read_to_string(&definitions_path)
        .map_err(|e| CliError::Message(format!("Failed to read definitions file: {e}")))?;
    let definitions = parse_definitions(&definitions_content)?;

    // Validate definitions
    validate_definitions(&definitions)
        .map_err(|e| CliError::Message(format!("Definitions file is invalid: {e}")))?;

    // Read and parse deployment
    let deployment_content = fs::read_to_string(deployment_path)
        .map_err(|e| CliError::Message(format!("Failed to read deployment file: {e}")))?;
    let deployment = parse_deployment(&deployment_content)?;

    // Validate deployment
    validate_deployment(&deployment)
        .map_err(|e| CliError::Message(format!("Deployment file is invalid: {e}")))?;

    // Compile to AST
    let artifact = compile(&deployment, &definitions)?;

    // Serialize to MessagePack
    let ast_bytes = serialize(&artifact)?;

    // Create output directory if needed
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Write AST file
    fs::write(&output_path, ast_bytes)?;

    println!(
        "✓ Compiled {} to {}",
        deployment_path.display(),
        output_path.display()
    );
    Ok(())
}

/// Runs the watch command, monitoring files for changes and auto-compiling/regenerating.
pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("✗ Watch mode failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    // Determine what to watch
    // If neither flag is set, watch both (default behavior)
    // Otherwise, watch only what's specified
    let watch_definitions = if options.definitions || options.deployments {
        options.definitions
    } else {
        true // Default: watch both
    };
    let watch_deployments = if options.definitions || options.deployments {
        options.deployments
    } else {
        true // Default: watch both
    };

    // Verify files exist
    let definitions_path = determine_definitions_path();
    if watch_definitions && !definitions_path.exists() {
        return Err(CliError::Message(format!(
            "Definitions file not found: {}",
            definitions_path.display()
        )));
    }

    let deployment_files = if watch_deployments {
        let files = find_deployment_files();
        if files.is_empty() {
            return Err(CliError::Message(
                "No deployment files found in .controlpath/".to_string(),
            ));
        }
        files
    } else {
        Vec::new()
    };

    // Initial compilation/generation
    println!("Starting watch mode...");
    if watch_definitions {
        println!("Watching definitions file: {}", definitions_path.display());
        if let Err(e) = regenerate_sdk(options) {
            eprintln!("  Warning: Initial SDK generation failed: {e}");
        }
    }

    // Store canonical path for definitions comparison
    // This is computed once and reused for efficient path comparison in the event loop
    let definitions_path_for_comparison = if watch_definitions {
        definitions_path
            .canonicalize()
            .unwrap_or_else(|_| definitions_path.clone())
    } else {
        PathBuf::new()
    };

    if watch_deployments {
        println!("Watching deployment files:");
        for deployment_file in &deployment_files {
            println!("  - {}", deployment_file.display());
            if let Err(e) = recompile_ast(deployment_file) {
                eprintln!("    Warning: Initial compilation failed: {e}");
            }
        }
    }

    println!("\nWatching for changes... (Press Ctrl+C to stop)");

    // Set up file watcher
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .map_err(|e| CliError::Message(format!("Failed to create file watcher: {e}")))?;

    // Watch definitions file
    if watch_definitions {
        watcher
            .watch(&definitions_path, RecursiveMode::NonRecursive)
            .map_err(|e| CliError::Message(format!("Failed to watch definitions file: {e}")))?;
    }

    // Watch deployment files
    if watch_deployments {
        for deployment_file in &deployment_files {
            watcher
                .watch(deployment_file, RecursiveMode::NonRecursive)
                .map_err(|e| CliError::Message(format!("Failed to watch deployment file: {e}")))?;
        }

        // Also watch the .controlpath directory for new deployment files
        let controlpath_dir = PathBuf::from(".controlpath");
        if controlpath_dir.exists() {
            watcher
                .watch(&controlpath_dir, RecursiveMode::NonRecursive)
                .map_err(|e| {
                    CliError::Message(format!("Failed to watch .controlpath directory: {e}"))
                })?;
        }
    }

    // Debounce timer
    let debounce_duration = Duration::from_millis(300);
    let mut last_change = Instant::now();
    let mut pending_changes: HashSet<PathBuf> = HashSet::new();

    // Event loop
    loop {
        // Check for events with timeout
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                        // Collect changed paths
                        for path in event.paths {
                            // Normalize path for comparison
                            let path_canonical =
                                path.canonicalize().unwrap_or_else(|_| path.clone());

                            if watch_definitions {
                                // Compare canonical paths
                                if path_canonical == definitions_path_for_comparison {
                                    pending_changes.insert(path.clone());
                                    last_change = Instant::now();
                                }
                            }

                            if watch_deployments {
                                // Check if it's a deployment file
                                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                    if name.ends_with(".deployment.yaml") {
                                        pending_changes.insert(path.clone());
                                        last_change = Instant::now();

                                        // If it's a new file, start watching it
                                        // Note: We use a blocking sleep here to ensure the file is fully written
                                        // before we attempt to watch it. This is acceptable because:
                                        // 1. New file creation is infrequent
                                        // 2. The delay is short (50ms)
                                        // 3. It prevents race conditions with file writing
                                        // Alternative: Could use async/non-blocking approach, but adds complexity
                                        if path.exists() {
                                            std::thread::sleep(Duration::from_millis(50));
                                            if let Err(e) =
                                                watcher.watch(&path, RecursiveMode::NonRecursive)
                                            {
                                                eprintln!(
                                                    "  Warning: Failed to watch new deployment file {}: {e}",
                                                    path.display()
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Err(e)) => {
                eprintln!("  Warning: File watcher error: {e}");
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Check if debounce period has passed
                if !pending_changes.is_empty() && last_change.elapsed() >= debounce_duration {
                    // Process pending changes
                    for changed_path in &pending_changes {
                        // Normalize path for comparison
                        let changed_path_canonical = changed_path
                            .canonicalize()
                            .unwrap_or_else(|_| changed_path.clone());

                        if watch_definitions
                            && changed_path_canonical == definitions_path_for_comparison
                        {
                            println!("\n📝 Definitions file changed");
                            if let Err(e) = regenerate_sdk(options) {
                                eprintln!("  ✗ SDK regeneration failed: {e}");
                            }
                        } else if watch_deployments && changed_path.exists() {
                            // Defensive check: verify it's actually a deployment file
                            // (files are filtered when added to pending_changes, but this adds safety)
                            if let Some(name) = changed_path.file_name().and_then(|n| n.to_str()) {
                                if name.ends_with(".deployment.yaml") {
                                    println!(
                                        "\n📝 Deployment file changed: {}",
                                        changed_path.display()
                                    );
                                    if let Err(e) = recompile_ast(changed_path) {
                                        eprintln!("  ✗ Compilation failed: {e}");
                                    }
                                }
                            }
                        }
                    }
                    pending_changes.clear();
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(CliError::Message("File watcher disconnected".to_string()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_determine_definitions_path() {
        let path = determine_definitions_path();
        assert_eq!(path, PathBuf::from("flags.definitions.yaml"));
    }

    #[test]
    fn test_determine_output_path_for_sdk() {
        let path = determine_output_path_for_sdk();
        assert_eq!(path, PathBuf::from("./flags"));
    }

    #[test]
    fn test_determine_output_path_for_ast() {
        let deployment_path = PathBuf::from(".controlpath/production.deployment.yaml");
        let output_path = determine_output_path_for_ast(&deployment_path);
        assert_eq!(output_path, PathBuf::from(".controlpath/production.ast"));
    }

    #[test]
    #[serial]
    fn test_regenerate_sdk_success() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test definitions file
        let definitions_path = temp_path.join("flags.definitions.yaml");
        fs::write(
            &definitions_path,
            r"flags:
  - name: test_flag
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        let options = Options {
            lang: Some("typescript".to_string()),
            definitions: true,
            deployments: false,
        };

        let result = regenerate_sdk(&options);
        assert!(result.is_ok());

        // Restore directory (ignore errors if directory no longer exists)
        let _ = std::env::set_current_dir(&original_dir);
    }

    #[test]
    #[serial]
    fn test_recompile_ast_success() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test files
        let definitions_path = temp_path.join("flags.definitions.yaml");
        fs::write(
            &definitions_path,
            r"flags:
  - name: test_flag
    type: boolean
    defaultValue: false
",
        )
        .unwrap();

        let controlpath_dir = temp_path.join(".controlpath");
        fs::create_dir_all(&controlpath_dir).unwrap();

        let deployment_path = controlpath_dir.join("test.deployment.yaml");
        fs::write(
            &deployment_path,
            r"environment: test
rules:
  test_flag:
    rules:
      - serve: true
",
        )
        .unwrap();

        // Change to temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();

        let result = recompile_ast(&deployment_path);
        assert!(result.is_ok());

        let output_path = controlpath_dir.join("test.ast");
        assert!(output_path.exists());

        // Restore directory (ignore errors if directory no longer exists)
        let _ = std::env::set_current_dir(&original_dir);
    }
}
