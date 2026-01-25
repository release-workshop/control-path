//! Dev command implementation - development workflow with smart defaults

use crate::error::{CliError, CliResult};
use crate::ops::compile as ops_compile;
use crate::ops::compile::CompileOptions;
use crate::ops::generate_sdk as ops_generate_sdk;
use crate::ops::generate_sdk::GenerateOptions;
use crate::utils::environment;
use crate::utils::language;
use crate::utils::unified_config;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct Options {
    /// Language override (if None, uses config/cached language)
    pub lang: Option<String>,
}

/// Validates core files exist, offers to create if missing
fn validate_core_files() -> CliResult<()> {
    use crate::utils::unified_config;

    // Check for config first
    if unified_config::unified_config_exists() {
        let unified = unified_config::read_unified_config()?;
        let envs = unified_config::get_environments(&unified);
        if envs.is_empty() {
            return Err(CliError::Message(
                "No environments found in control-path.yaml\n  Add flags with environment rules first.".to_string(),
            ));
        }

        // Check .controlpath directory exists (for AST output)
        let controlpath_dir = PathBuf::from(".controlpath");
        if !controlpath_dir.exists() {
            return Err(CliError::Message(
                ".controlpath directory not found\n  Run 'controlpath setup' to initialize the project.".to_string(),
            ));
        }
    } else {
        // Legacy: check for old file structure
        let definitions_path = PathBuf::from("flags.definitions.yaml");
        let controlpath_dir = PathBuf::from(".controlpath");

        // Check definitions file
        if !definitions_path.exists() {
            return Err(CliError::Message(
                "Configuration file not found: control-path.yaml or flags.definitions.yaml\n  Run 'controlpath setup' to initialize the project.".to_string(),
            ));
        }

        // Check .controlpath directory
        if !controlpath_dir.exists() {
            return Err(CliError::Message(
                ".controlpath directory not found\n  Run 'controlpath setup' to initialize the project.".to_string(),
            ));
        }

        // Check for at least one deployment file
        let deployment_files = find_deployment_files();
        if deployment_files.is_empty() {
            return Err(CliError::Message(
                "No deployment files found in .controlpath/\n  Run 'controlpath setup' or 'controlpath env add --name <env>' to create one.".to_string(),
            ));
        }
    }

    Ok(())
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

/// Regenerates the SDK when the definitions file changes.
fn regenerate_sdk(options: &Options) -> CliResult<()> {
    // Determine language (priority: CLI flag > Config > Auto-detect > Default)
    let language = language::determine_language(options.lang.clone())?;

    let generate_opts = GenerateOptions {
        lang: Some(language.clone()),
        output: None,
        skip_validation: false,
    };

    ops_generate_sdk::generate_sdk_helper(&generate_opts)?;

    println!("✓ SDK regenerated");
    Ok(())
}

/// Recompiles ASTs for all deployment files when they change.
fn recompile_all_asts() -> CliResult<()> {
    let compile_opts = CompileOptions {
        envs: None, // Compile all environments
        skip_validation: false,
    };

    let compiled = ops_compile::compile_envs(&compile_opts)?;
    println!(
        "✓ Compiled {} environment(s): {}",
        compiled.len(),
        compiled.join(", ")
    );
    Ok(())
}

/// Recompiles a specific deployment file's AST.
fn recompile_ast_for_file(deployment_path: &Path) -> CliResult<()> {
    // Extract environment name from path
    let env_name = deployment_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace(".deployment", ""))
        .ok_or_else(|| CliError::Message("Invalid deployment file path".to_string()))?;

    let compile_opts = CompileOptions {
        envs: Some(vec![env_name]),
        skip_validation: false,
    };

    ops_compile::compile_envs(&compile_opts)?;
    Ok(())
}

/// Runs the dev command, monitoring files for changes and auto-compiling/regenerating.
pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("✗ Dev mode failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    // Validate core files exist
    validate_core_files()?;

    // Determine language (will use config/cached if available)
    let language = language::determine_language(options.lang.clone())?;

    // Determine environment for messaging
    let env_info = if let Ok(Some(env)) = environment::determine_environment() {
        format!(" (env: {env})")
    } else {
        String::new()
    };

    println!("🚀 Starting dev mode...");
    println!("  Language: {language}{env_info}");

    // Initial compilation/generation
    println!("\n📝 Initial generation...");

    // Check for config or legacy files
    if unified_config::unified_config_exists() {
        // Config exists - regenerate SDK and compile ASTs
        if let Err(e) = regenerate_sdk(options) {
            eprintln!("  ⚠ Warning: Initial SDK generation failed: {e}");
        }
        if let Err(e) = recompile_all_asts() {
            eprintln!("  ⚠ Warning: Initial compilation failed: {e}");
        }
    } else {
        // Legacy: check for old file structure
        let definitions_path = PathBuf::from("flags.definitions.yaml");
        let deployment_files = find_deployment_files();

        if definitions_path.exists() {
            if let Err(e) = regenerate_sdk(options) {
                eprintln!("  ⚠ Warning: Initial SDK generation failed: {e}");
            }
        }

        if !deployment_files.is_empty() {
            if let Err(e) = recompile_all_asts() {
                eprintln!("  ⚠ Warning: Initial compilation failed: {e}");
            }
        }
    }

    println!("\n👀 Watching for changes... (Press Ctrl+C to stop)");

    // Set up file watcher
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .map_err(|e| CliError::Message(format!("Failed to create file watcher: {e}")))?;

    // Watch config or legacy files
    if unified_config::unified_config_exists() {
        // Watch config file
        let unified_path = unified_config::get_unified_config_path();
        watcher
            .watch(&unified_path, RecursiveMode::NonRecursive)
            .map_err(|e| CliError::Message(format!("Failed to watch config file: {e}")))?;
    } else {
        // Legacy: watch definitions file
        let definitions_path = PathBuf::from("flags.definitions.yaml");
        if definitions_path.exists() {
            watcher
                .watch(&definitions_path, RecursiveMode::NonRecursive)
                .map_err(|e| CliError::Message(format!("Failed to watch definitions file: {e}")))?;
        }

        // Watch deployment files
        let deployment_files = find_deployment_files();
        for deployment_file in &deployment_files {
            watcher
                .watch(deployment_file, RecursiveMode::NonRecursive)
                .map_err(|e| CliError::Message(format!("Failed to watch deployment file: {e}")))?;
        }
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

    // Store canonical path for file comparison
    let file_path_for_comparison = if unified_config::unified_config_exists() {
        unified_config::get_unified_config_path()
            .canonicalize()
            .unwrap_or_else(|_| unified_config::get_unified_config_path())
    } else {
        let definitions_path = PathBuf::from("flags.definitions.yaml");
        definitions_path
            .canonicalize()
            .unwrap_or_else(|_| definitions_path.clone())
    };

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

                            // Check if it's the config or definitions file
                            if path_canonical == file_path_for_comparison {
                                pending_changes.insert(path.clone());
                                last_change = Instant::now();
                            }

                            // Check if it's a deployment file (legacy only)
                            if !unified_config::unified_config_exists() {
                                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                    if name.ends_with(".deployment.yaml") {
                                        pending_changes.insert(path.clone());
                                        last_change = Instant::now();

                                        // If it's a new file, start watching it
                                        if path.exists() {
                                            std::thread::sleep(Duration::from_millis(50));
                                            if let Err(e) =
                                                watcher.watch(&path, RecursiveMode::NonRecursive)
                                            {
                                                eprintln!(
                                                    "  ⚠ Warning: Failed to watch new deployment file {}: {e}",
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
                eprintln!("  ⚠ Warning: File watcher error: {e}");
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

                        if changed_path_canonical == file_path_for_comparison {
                            if unified_config::unified_config_exists() {
                                println!("\n📝 Config file changed");
                                if let Err(e) = regenerate_sdk(options) {
                                    eprintln!("  ✗ SDK regeneration failed: {e}");
                                }
                                if let Err(e) = recompile_all_asts() {
                                    eprintln!("  ✗ Compilation failed: {e}");
                                }
                            } else {
                                println!("\n📝 Definitions file changed");
                                if let Err(e) = regenerate_sdk(options) {
                                    eprintln!("  ✗ SDK regeneration failed: {e}");
                                }
                            }
                        } else if changed_path.exists() && !unified_config::unified_config_exists()
                        {
                            // Legacy: Check if it's a deployment file
                            if let Some(name) = changed_path.file_name().and_then(|n| n.to_str()) {
                                if name.ends_with(".deployment.yaml") {
                                    println!(
                                        "\n📝 Deployment file changed: {}",
                                        changed_path.display()
                                    );
                                    if let Err(e) = recompile_ast_for_file(changed_path) {
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
    use crate::test_helpers::DirGuard;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_validate_core_files_missing_definitions() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let result = validate_core_files();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Configuration file not found"));
    }

    #[test]
    #[serial]
    fn test_validate_core_files_missing_controlpath() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write("flags.definitions.yaml", "flags: []\n").unwrap();

        let result = validate_core_files();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains(".controlpath directory not found"));
    }

    #[test]
    #[serial]
    fn test_validate_core_files_missing_deployments() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write("flags.definitions.yaml", "flags: []\n").unwrap();
        fs::create_dir_all(".controlpath").unwrap();

        let result = validate_core_files();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No deployment files found"));
    }

    #[test]
    #[serial]
    fn test_validate_core_files_success() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "flags.definitions.yaml",
            r"flags:
  - name: test_flag
    type: boolean
    default: false
",
        )
        .unwrap();
        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/production.deployment.yaml",
            r"environment: production
rules: {}
",
        )
        .unwrap();

        let result = validate_core_files();
        assert!(result.is_ok());
    }
}
