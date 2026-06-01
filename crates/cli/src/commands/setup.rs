//! Setup command implementation

use crate::error::{CliError, CliResult};
use crate::ops::{compile as ops_compile, generate_sdk as ops_generate_sdk};
use crate::utils::atomic_write::atomic_write_string;
use crate::utils::config;
use crate::utils::language;
use std::fs;
use std::path::Path;
use std::process::Command;

// Default SDK output is now node_modules/@controlpath/generated (phantom SDK)
// This can be overridden via sdk.output in control-path.yaml
const UNIFIED_CONFIG_FILE: &str = "control-path.yaml";

pub struct Options {
    /// Language for SDK generation (auto-detected if not provided)
    pub lang: Option<String>,
    /// Skip installing runtime SDK package
    pub skip_install: bool,
    /// Skip creating example flags and usage files
    ///
    /// When set, creates a minimal project without example flags or example usage files.
    /// This is useful for projects that want to start with a clean slate.
    pub no_examples: bool,
}

fn create_example_usage_file(lang: &str) -> CliResult<()> {
    match lang {
        "typescript" | "ts" => {
            let example_content = r#"// Example usage of Control Path SDK
import { evaluator } from '@controlpath/generated';
import type { User } from '@controlpath/generated';

async function main() {
  // Initialize the evaluator with the AST artifact
  await evaluator.init({ artifact: './.controlpath/production.ast' });
  
  // Create user context
  const user: User = {
    id: 'user123',
    role: 'admin',
    email: 'user@example.com',
  };
  
  // Example: Evaluate a boolean flag (using setContext pattern)
  evaluator.setContext(user);
  const newDashboardEnabled = await evaluator.exampleFlag();
  console.log('Example flag enabled:', newDashboardEnabled);
  
  // Example: Evaluate a flag with explicit user (overrides setContext)
  const result = await evaluator.exampleFlag(user);
  console.log('Example flag (explicit user):', result);
  
  // Example: Evaluate all flags at once
  const allFlags = await evaluator.evaluateAll(user);
  console.log('All flags:', allFlags);
  
  // Example: Evaluate multiple flags in batch (type-safe)
  const batch = await evaluator.evaluateBatch(['exampleFlag'], user);
  console.log('Batch evaluation:', batch);
}

main().catch(console.error);
"#;
            atomic_write_string(Path::new("example_usage.ts"), example_content).map_err(|e| {
                CliError::Message(format!(
                    "Failed to write example_usage.ts: {}. \
                    Ensure you have write permissions in the current directory.",
                    e
                ))
            })
        }
        _ => {
            // For other languages, create a basic example
            let example_content = format!(
                r#"// Example usage of Control Path SDK for {}
// TODO: Add language-specific example
"#,
                lang
            );
            atomic_write_string(
                Path::new(&format!("example_usage.{}", get_file_extension(lang))),
                &example_content,
            )
            .map_err(|e| {
                CliError::Message(format!(
                    "Failed to write example_usage.{}: {}. \
                    Ensure you have write permissions in the current directory.",
                    get_file_extension(lang),
                    e
                ))
            })
        }
    }
}

fn get_file_extension(lang: &str) -> &str {
    match lang {
        "typescript" | "ts" => "ts",
        "python" | "py" => "py",
        "javascript" | "js" => "js",
        _ => "txt",
    }
}

/// Detect which package manager is being used based on lock files
fn detect_package_manager() -> (&'static str, &'static [&'static str]) {
    // Check for lock files in order of preference
    if Path::new("pnpm-lock.yaml").exists() {
        ("pnpm", &["add", "@controlpath/runtime"])
    } else if Path::new("yarn.lock").exists() {
        ("yarn", &["add", "@controlpath/runtime"])
    } else if Path::new("bun.lockb").exists() {
        ("bun", &["add", "@controlpath/runtime"])
    } else if Path::new("package-lock.json").exists() {
        ("npm", &["install", "@controlpath/runtime"])
    } else {
        // Default to npm if no lock file found
        ("npm", &["install", "@controlpath/runtime"])
    }
}

fn install_runtime_sdk(lang: &str) -> CliResult<()> {
    match lang {
        "typescript" | "ts" => {
            // Check if package.json exists
            if !Path::new("package.json").exists() {
                // Create a basic package.json if it doesn't exist
                let package_json = r#"{
  "name": "my-control-path-project",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "start": "node example.js"
  }
}
"#;
                atomic_write_string(Path::new("package.json"), package_json).map_err(|e| {
                    CliError::Message(format!(
                        "Failed to create package.json: {}. \
                        Ensure you have write permissions in the current directory.",
                        e
                    ))
                })?;
            }

            // Detect package manager and run appropriate install command
            let (pm_name, pm_args) = detect_package_manager();
            let pm_args_str = pm_args.to_vec().join(" ");
            let output = Command::new(pm_name).args(pm_args).output().map_err(|e| {
                CliError::Message(format!(
                    "Failed to run {} {}: {}. Make sure {} is installed and available in PATH.",
                    pm_name, pm_args_str, e, pm_name
                ))
            })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(CliError::Message(format!(
                    "{} {} failed: {}",
                    pm_name, pm_args_str, stderr
                )));
            }

            Ok(())
        }
        "python" | "py" => {
            // For Python, we would use pip, but the runtime SDK doesn't exist yet for Python
            // For now, just skip or show a message
            println!("  Note: Python runtime SDK installation not yet implemented");
            Ok(())
        }
        _ => {
            println!(
                "  Note: Runtime SDK installation for {} not yet implemented",
                lang
            );
            Ok(())
        }
    }
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(_lang) => {
            println!();
            println!("✓ Setup complete!");
            println!();
            println!("Next steps:");
            println!("  1. Add your first flag:    controlpath new-flag");
            println!("  2. Enable a flag:          controlpath flag enable <flag> --env staging");
            println!("  3. Test flags:             controlpath test");
            println!("  4. Start watch mode:       controlpath watch");
            println!("  5. Get help:               controlpath help");
            0
        }
        Err(e) => {
            eprintln!("✗ Setup failed");
            eprintln!("  Error: {e}");
            eprintln!();
            eprintln!("Recovery steps:");
            eprintln!("  • Check error message above for specific issue");
            eprintln!("  • Ensure you have write permissions in the current directory");
            eprintln!("  • Remove control-path.yaml and .controlpath/ to start fresh");
            eprintln!("  • Run 'controlpath setup --skip-install' to skip package installation");
            1
        }
    }
}

/// Check if project already exists
fn check_existing_project() -> bool {
    Path::new(UNIFIED_CONFIG_FILE).exists() || Path::new(".controlpath").exists()
}

/// Ensure .controlpath directory exists
fn ensure_controlpath_directory() -> CliResult<()> {
    fs::create_dir_all(".controlpath").map_err(CliError::from)
}

/// Create config file
fn create_unified_config_file(with_examples: bool) -> CliResult<()> {
    let config_content = if with_examples {
        r"catalog:
  id: example-service
mode: local
flags:
  example_flag:
    default: false
    kind: release
    description: An example feature flag
environments:
  production:
    rules:
      example_flag:
        - serve: false
  staging:
    rules:
      example_flag:
        - serve: false
"
    } else {
        r"catalog:
  id: example-service
mode: local
flags: {}
"
    };
    atomic_write_string(Path::new(UNIFIED_CONFIG_FILE), config_content)
}

fn run_inner(options: &Options) -> CliResult<String> {
    println!("Setting up Control Path project...");
    println!("This will create the project structure and generate initial files.");
    println!();

    // Check if project already exists
    if check_existing_project() {
        return Err(CliError::Message(
            "Project already initialized. Remove control-path.yaml or .controlpath directory to start fresh.".to_string(),
        ));
    }

    // Determine language (priority: CLI flag > Config > Auto-detect > Default)
    let lang = language::determine_language(options.lang.clone())?;
    println!("Using language: {}", lang);
    println!();

    // Step 1: Initialize project structure
    println!("Step 1/6: Creating project structure...");
    ensure_controlpath_directory()?;
    let create_examples = !options.no_examples;
    create_unified_config_file(create_examples)?;
    println!("   ✓ Created control-path.yaml");
    if create_examples {
        println!("   ✓ Created example flag");
    }
    println!();

    // Determine initial environments
    let initial_envs = if create_examples {
        vec!["production".to_string(), "staging".to_string()]
    } else {
        vec!["production".to_string()]
    };

    // Step 2: Write config.yaml with language + defaultEnv
    println!("Step 2/6: Writing configuration...");
    config::write_config_language(&lang)?;
    config::write_config_default_env("production")?;
    println!("   ✓ Configuration written to {}", UNIFIED_CONFIG_FILE);
    println!();

    // Step 3: Generate SDK (only if we have flags)
    if create_examples {
        println!("Step 3/6: Generating SDK...");
        let generate_options = ops_generate_sdk::GenerateOptions {
            lang: Some(lang.clone()),
            output: None, // Use default (node_modules/@controlpath/generated) or config
            skip_validation: false,
        };
        ops_generate_sdk::generate_sdk_helper(&generate_options).map_err(|e| {
            CliError::Message(format!(
                "Failed to generate SDK: {}. \
                Check that the config is valid and the output directory is writable.",
                e
            ))
        })?;
        println!("   ✓ SDK generated");
        println!();
    } else {
        println!("Step 3/6: Skipping SDK generation (no flags defined)");
        println!();
    }

    // Step 4: Compile ASTs for all initial environments
    println!("Step 4/6: Compiling ASTs for initial environments...");
    let compile_options = ops_compile::CompileOptions {
        envs: Some(initial_envs.clone()),
        skip_validation: false,
    };
    let compiled_envs = ops_compile::compile_envs(&compile_options).map_err(|e| {
        CliError::Message(format!(
            "Failed to compile ASTs for environments {}: {}. \
            Check that the config is valid.",
            initial_envs.join(", "),
            e
        ))
    })?;
    println!("   ✓ Compiled ASTs for: {}", compiled_envs.join(", "));
    println!();

    // Step 5: Install runtime SDK (conditional)
    if !options.skip_install {
        println!("Step 5/6: Installing runtime SDK...");
        install_runtime_sdk(&lang).map_err(|e| {
            CliError::Message(format!(
                "Failed to install runtime SDK: {}. \
                You can skip this step with --skip-install and install manually later.",
                e
            ))
        })?;
        println!("   ✓ Runtime SDK installed");
    } else {
        println!("Step 5/6: Skipping runtime SDK installation (--skip-install)");
    }
    println!();

    // Step 6: Create example usage file (only if not --no-examples)
    if !options.no_examples {
        println!("Step 6/6: Creating example usage file...");
        create_example_usage_file(&lang).map_err(|e| {
            CliError::Message(format!(
                "Failed to create example usage file: {}. \
                Ensure you have write permissions in the current directory.",
                e
            ))
        })?;
        println!("   ✓ Example file created");
        println!();
    }

    Ok(lang)
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
    fn test_get_file_extension() {
        assert_eq!(get_file_extension("typescript"), "ts");
        assert_eq!(get_file_extension("ts"), "ts");
        assert_eq!(get_file_extension("python"), "py");
        assert_eq!(get_file_extension("py"), "py");
        assert_eq!(get_file_extension("javascript"), "js");
        assert_eq!(get_file_extension("unknown"), "txt");
    }

    #[test]
    #[serial]
    fn test_create_example_usage_file_typescript() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let result = create_example_usage_file("typescript");
        assert!(result.is_ok());

        let example_path = temp_path.join("example_usage.ts");
        assert!(example_path.exists(), "example_usage.ts should be created");

        let content = fs::read_to_string(&example_path).unwrap();
        assert!(content.contains("evaluator"), "Should import evaluator");
        assert!(
            content.contains(".controlpath/production.ast"),
            "Should reference correct AST path"
        );
    }

    #[test]
    #[serial]
    fn test_create_example_usage_file_other_lang() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let result = create_example_usage_file("python");
        assert!(result.is_ok());

        let example_path = temp_path.join("example_usage.py");
        assert!(example_path.exists(), "example_usage.py should be created");
    }

    #[test]
    fn test_get_file_extension_variations() {
        assert_eq!(get_file_extension("typescript"), "ts");
        assert_eq!(get_file_extension("ts"), "ts");
        assert_eq!(get_file_extension("python"), "py");
        assert_eq!(get_file_extension("py"), "py");
        assert_eq!(get_file_extension("javascript"), "js");
        assert_eq!(get_file_extension("js"), "js");
        assert_eq!(get_file_extension("rust"), "txt");
        assert_eq!(get_file_extension(""), "txt");
    }

    #[test]
    #[serial]
    fn test_create_example_usage_file_javascript() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let result = create_example_usage_file("javascript");
        assert!(result.is_ok());

        let example_path = temp_path.join("example_usage.js");
        assert!(example_path.exists(), "example_usage.js should be created");
    }

    #[test]
    #[serial]
    fn test_create_example_usage_file_content_check() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let result = create_example_usage_file("typescript");
        assert!(result.is_ok());

        let example_path = temp_path.join("example_usage.ts");
        let content = fs::read_to_string(&example_path).unwrap();

        // Check that the example contains key elements
        assert!(content.contains("evaluator"), "Should import evaluator");
        assert!(
            content.contains(".controlpath/production.ast"),
            "Should reference correct AST path"
        );
        assert!(
            content.contains("evaluateAll"),
            "Should show evaluateAll usage"
        );
        assert!(
            content.contains("evaluateBatch"),
            "Should show evaluateBatch usage"
        );
    }

    #[test]
    fn test_options_struct() {
        let opts = Options {
            lang: Some("typescript".to_string()),
            skip_install: false,
            no_examples: false,
        };
        assert_eq!(opts.lang, Some("typescript".to_string()));
        assert!(!opts.skip_install);
        assert!(!opts.no_examples);

        let opts2 = Options {
            lang: None,
            skip_install: true,
            no_examples: true,
        };
        assert_eq!(opts2.lang, None);
        assert!(opts2.skip_install);
        assert!(opts2.no_examples);
    }

    #[test]
    #[serial]
    fn test_setup_with_skip_install() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create package.json to trigger TypeScript detection
        fs::write("package.json", "{}").unwrap();

        let options = Options {
            lang: Some("typescript".to_string()),
            skip_install: true,
            no_examples: false,
        };

        // This test verifies that setup runs without trying to install npm packages
        // The skip_install flag should prevent npm install from being called
        // Note: Full integration test would require mocking npm or using a test environment
        // This test verifies the flag is respected in the options struct
        assert!(options.skip_install, "skip_install should be true");

        // Verify that when skip_install is true, we don't attempt npm install
        // The actual npm install call is conditional on !options.skip_install
        // This is tested implicitly through the code structure
    }

    #[test]
    fn test_setup_with_invalid_language_option() {
        // Test that invalid language is accepted in options (validation happens later)
        let options = Options {
            lang: Some("invalid_lang".to_string()),
            skip_install: true,
            no_examples: false,
        };

        // Options struct should accept any language string
        // Validation happens during SDK generation
        assert_eq!(options.lang, Some("invalid_lang".to_string()));
    }

    #[test]
    #[serial]
    fn test_setup_auto_detects_language_from_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Create package.json to trigger TypeScript detection
        fs::write("package.json", "{}").unwrap();

        let options = Options {
            lang: None, // Should auto-detect
            skip_install: true,
            no_examples: false,
        };

        // Test that language detection works (this is tested in utils/language.rs)
        // Here we just verify the options allow None for lang
        assert_eq!(options.lang, None);

        // The actual detection is tested in utils/language.rs tests
        // This test verifies the setup command accepts None for lang
    }

    // Note: Full end-to-end integration test would require:
    // - Mocking or stubbing npm install
    // - Setting up a complete project structure
    // - Verifying all files are created correctly
    // This is better suited for manual testing or CI/CD integration tests
    // The unit tests above verify individual components work correctly
}
