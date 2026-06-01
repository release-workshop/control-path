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
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct Options {
    /// Language override (if None, uses config/cached language)
    pub lang: Option<String>,
}

fn validate_core_files() -> CliResult<()> {
    if !unified_config::unified_config_exists() {
        return Err(CliError::Message(
            "Configuration file not found: control-path.yaml\n  Run 'controlpath setup' or 'controlpath init' to initialize the project.".to_string(),
        ));
    }

    let unified = unified_config::read_unified_config()?;
    let envs = unified_config::get_environments(&unified);
    if envs.is_empty() {
        return Err(CliError::Message(
            "No environments found in control-path.yaml\n  Run 'controlpath env add --name <env>' first.".to_string(),
        ));
    }

    let controlpath_dir = PathBuf::from(".controlpath");
    if !controlpath_dir.exists() {
        return Err(CliError::Message(
            ".controlpath directory not found\n  Run 'controlpath setup' to initialize the project.".to_string(),
        ));
    }

    Ok(())
}

fn regenerate_sdk(options: &Options) -> CliResult<()> {
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

fn recompile_all_asts() -> CliResult<()> {
    let compile_opts = CompileOptions {
        envs: None,
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
    validate_core_files()?;

    let language = language::determine_language(options.lang.clone())?;

    let env_info = if let Ok(Some(env)) = environment::determine_environment() {
        format!(" (env: {env})")
    } else {
        String::new()
    };

    println!("🚀 Starting dev mode...");
    println!("  Language: {language}{env_info}");

    println!("\n📝 Initial generation...");
    if let Err(e) = regenerate_sdk(options) {
        eprintln!("  ⚠ Warning: Initial SDK generation failed: {e}");
    }
    if let Err(e) = recompile_all_asts() {
        eprintln!("  ⚠ Warning: Initial compilation failed: {e}");
    }

    println!("\n👀 Watching for changes... (Press Ctrl+C to stop)");

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .map_err(|e| CliError::Message(format!("Failed to create file watcher: {e}")))?;

    let unified_path = unified_config::get_unified_config_path();
    watcher
        .watch(&unified_path, RecursiveMode::NonRecursive)
        .map_err(|e| CliError::Message(format!("Failed to watch config file: {e}")))?;

    let controlpath_dir = PathBuf::from(".controlpath");
    if controlpath_dir.exists() {
        watcher
            .watch(&controlpath_dir, RecursiveMode::NonRecursive)
            .map_err(|e| {
                CliError::Message(format!("Failed to watch .controlpath directory: {e}"))
            })?;
    }

    let file_path_for_comparison = unified_path.canonicalize().unwrap_or(unified_path);

    let debounce_duration = Duration::from_millis(300);
    let mut last_change = Instant::now();
    let mut pending_changes: HashSet<PathBuf> = HashSet::new();

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => match event.kind {
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                    for path in event.paths {
                        let path_canonical = path.canonicalize().unwrap_or_else(|_| path.clone());

                        if path_canonical == file_path_for_comparison {
                            pending_changes.insert(path.clone());
                            last_change = Instant::now();
                        }
                    }
                }
                _ => {}
            },
            Ok(Err(e)) => {
                eprintln!("  ⚠ Warning: File watcher error: {e}");
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if !pending_changes.is_empty() && last_change.elapsed() >= debounce_duration {
                    for changed_path in &pending_changes {
                        let changed_path_canonical = changed_path
                            .canonicalize()
                            .unwrap_or_else(|_| changed_path.clone());

                        if changed_path_canonical == file_path_for_comparison {
                            println!("\n📝 Config file changed");
                            if let Err(e) = regenerate_sdk(options) {
                                eprintln!("  ✗ SDK regeneration failed: {e}");
                            }
                            if let Err(e) = recompile_all_asts() {
                                eprintln!("  ✗ Compilation failed: {e}");
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
    fn test_validate_core_files_missing_config() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let result = validate_core_files();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("control-path.yaml"));
    }

    #[test]
    #[serial]
    fn test_validate_core_files_missing_controlpath() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test
mode: local
flags:
  my_flag:
    default: false
    kind: release
environments:
  production:
    rules: {}
",
        )
        .unwrap();

        let result = validate_core_files();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains(".controlpath directory not found"));
    }

    #[test]
    #[serial]
    fn test_validate_core_files_success() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::write(
            "control-path.yaml",
            r"catalog:
  id: test
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
",
        )
        .unwrap();
        fs::create_dir_all(".controlpath").unwrap();

        let result = validate_core_files();
        assert!(result.is_ok());
    }
}
