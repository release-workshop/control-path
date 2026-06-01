//! Watch command implementation

use crate::error::{CliError, CliResult};
use crate::generator::generate_sdk;
use crate::utils::catalog;
use crate::utils::unified_config;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct Options {
    pub lang: Option<String>,
    pub definitions: bool,
    pub deployments: bool,
}

fn determine_catalog_path() -> PathBuf {
    unified_config::get_unified_config_path()
}

fn determine_output_path_for_sdk() -> PathBuf {
    if unified_config::unified_config_exists() {
        if let Ok(unified) = unified_config::read_unified_config() {
            if let Some(config_output) = unified_config::get_sdk_output_path(&unified) {
                return PathBuf::from(config_output);
            }
        }
    }
    PathBuf::from("node_modules/@controlpath/generated")
}

fn regenerate_sdk(options: &Options) -> CliResult<()> {
    let output_path = determine_output_path_for_sdk();
    let base_dir = std::env::current_dir()
        .map_err(|e| CliError::Message(format!("Failed to resolve working directory: {e}")))?;
    // SdkGenerate validation only; no SaaS CDN embedding (unlike load_for_sdk_generate).
    // Follow-up: delegate to ops::generate_sdk_helper (issue 04 watch slice / issue 05).
    let sdk_catalog = catalog::load_for_explain(&base_dir)?.sdk;

    let language = options
        .lang
        .as_deref()
        .unwrap_or("typescript")
        .to_lowercase();

    generate_sdk(&language, &sdk_catalog, &output_path)?;

    println!("✓ SDK regenerated to {}", output_path.display());
    Ok(())
}

fn recompile_all_catalog() -> CliResult<()> {
    let base_dir = std::env::current_dir()
        .map_err(|e| CliError::Message(format!("Failed to resolve working directory: {e}")))?;
    let compiled = catalog::compile_catalog_envs(&base_dir, None)?;
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
            eprintln!("✗ Watch mode failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

/// Resolve which watch actions to run from CLI flags.
///
/// v2 catalogs use a single `control-path.yaml`; `--definitions` limits to SDK
/// regeneration and `--deployments` limits to AST recompilation.
fn watch_actions(options: &Options) -> (bool, bool) {
    match (options.definitions, options.deployments) {
        (false, false) => (true, true),
        (true, false) => (true, false),
        (false, true) => (false, true),
        (true, true) => (true, true),
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    let (regen_sdk, recompile_asts) = watch_actions(options);

    let catalog_path = determine_catalog_path();
    if !catalog_path.exists() {
        return Err(CliError::Message(format!(
            "Catalog file not found: {}. Run 'controlpath setup' or 'controlpath init'.",
            catalog_path.display()
        )));
    }

    println!("Starting watch mode...");
    println!("Watching catalog file: {}", catalog_path.display());
    if regen_sdk {
        if let Err(e) = regenerate_sdk(options) {
            eprintln!("  Warning: Initial SDK generation failed: {e}");
        }
    }
    if recompile_asts {
        if let Err(e) = recompile_all_catalog() {
            eprintln!("  Warning: Initial compilation failed: {e}");
        }
    }

    let catalog_path_for_comparison = catalog_path
        .canonicalize()
        .unwrap_or_else(|_| catalog_path.clone());

    println!("\nWatching for changes... (Press Ctrl+C to stop)");

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .map_err(|e| CliError::Message(format!("Failed to create file watcher: {e}")))?;

    watcher
        .watch(&catalog_path, RecursiveMode::NonRecursive)
        .map_err(|e| CliError::Message(format!("Failed to watch catalog file: {e}")))?;

    let debounce_duration = Duration::from_millis(300);
    let mut last_change = Instant::now();
    let mut pending_changes: HashSet<PathBuf> = HashSet::new();

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => match event.kind {
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                    for path in event.paths {
                        let path_canonical = path.canonicalize().unwrap_or_else(|_| path.clone());

                        if path_canonical == catalog_path_for_comparison {
                            pending_changes.insert(path.clone());
                            last_change = Instant::now();
                        }
                    }
                }
                _ => {}
            },
            Ok(Err(e)) => {
                eprintln!("  Warning: File watcher error: {e}");
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if !pending_changes.is_empty() && last_change.elapsed() >= debounce_duration {
                    for changed_path in &pending_changes {
                        let changed_path_canonical = changed_path
                            .canonicalize()
                            .unwrap_or_else(|_| changed_path.clone());

                        if changed_path_canonical == catalog_path_for_comparison {
                            println!("\n📝 Catalog file changed");
                            if regen_sdk {
                                if let Err(e) = regenerate_sdk(options) {
                                    eprintln!("  ✗ SDK regeneration failed: {e}");
                                }
                            }
                            if recompile_asts {
                                if let Err(e) = recompile_all_catalog() {
                                    eprintln!("  ✗ Compilation failed: {e}");
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

    const TEST_CATALOG: &str = r"catalog:
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
";

    #[test]
    fn test_watch_actions_defaults() {
        let options = Options {
            lang: None,
            definitions: false,
            deployments: false,
        };
        assert_eq!(watch_actions(&options), (true, true));
    }

    #[test]
    fn test_watch_actions_deployments_only() {
        let options = Options {
            lang: None,
            definitions: false,
            deployments: true,
        };
        assert_eq!(watch_actions(&options), (false, true));
    }

    #[test]
    fn test_determine_catalog_path() {
        let path = determine_catalog_path();
        assert_eq!(path, PathBuf::from("control-path.yaml"));
    }

    #[test]
    fn test_determine_output_path_for_sdk() {
        let path = determine_output_path_for_sdk();
        assert_eq!(path, PathBuf::from("node_modules/@controlpath/generated"));
    }

    #[test]
    #[serial]
    fn test_regenerate_sdk_success() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        fs::write(temp_path.join("control-path.yaml"), TEST_CATALOG).unwrap();

        let _guard = DirGuard::new(temp_path).unwrap();

        let options = Options {
            lang: Some("typescript".to_string()),
            definitions: true,
            deployments: false,
        };

        let result = regenerate_sdk(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_run_inner_catalog_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();

        let options = Options {
            lang: None,
            definitions: true,
            deployments: false,
        };

        let result = run_inner(&options);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Catalog file not found"));
    }
}
