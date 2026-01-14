//! Generate SDK command implementation

use crate::error::{CliError, CliResult};
use crate::generator::generate_sdk;
use crate::monorepo::{
    detect_workspace_root, discover_services, ServiceContext, ServicePathResolver,
};
use crate::utils::language;
use controlpath_compiler::{parse_definitions, validate_definitions};
use std::fs;
use std::path::PathBuf;

pub struct Options {
    pub lang: Option<String>,
    pub output: Option<String>,
    pub definitions: Option<String>,
    pub all_services: bool,
    #[allow(dead_code)]
    pub service_context: Option<ServiceContext>,
}

fn determine_definitions_path(options: &Options) -> PathBuf {
    PathBuf::from(
        options
            .definitions
            .as_deref()
            .unwrap_or("flags.definitions.yaml"),
    )
}

fn determine_output_path(options: &Options) -> PathBuf {
    PathBuf::from(options.output.as_deref().unwrap_or("./flags"))
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => {
            println!("✓ SDK generated successfully");
            0
        }
        Err(e) => {
            eprintln!("✗ SDK generation failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    // Handle bulk operations for monorepo
    if options.all_services {
        return run_bulk_generate_sdk(options);
    }

    // Determine file paths
    let definitions_path = determine_definitions_path(options);
    let output_path = determine_output_path(options);

    // Read and parse definitions
    let definitions_content = fs::read_to_string(&definitions_path)
        .map_err(|e| CliError::Message(format!("Failed to read definitions file: {e}")))?;
    let definitions = parse_definitions(&definitions_content)?;

    // Validate definitions
    validate_definitions(&definitions)?;

    // Determine language (priority: CLI flag > Config > Auto-detect > Default)
    let language = language::determine_language(options.lang.clone())?.to_lowercase();

    // Generate SDK
    generate_sdk(&language, &definitions, &output_path)?;

    println!("  Generated SDK to {}", output_path.display());
    Ok(())
}

fn run_bulk_generate_sdk(options: &Options) -> CliResult<()> {
    // Detect workspace root
    let workspace_root = detect_workspace_root(None)?.ok_or_else(|| {
        CliError::Message(
            "Not in a monorepo. --all-services flag only works in monorepo environments"
                .to_string(),
        )
    })?;

    // Discover all services
    let services = discover_services(&workspace_root)?;

    if services.is_empty() {
        return Err(CliError::Message(
            "No services found in monorepo".to_string(),
        ));
    }

    println!("Generating SDKs for {} service(s)...", services.len());

    // Determine language (priority: CLI flag > Config > Auto-detect > Default)
    let language = language::determine_language(options.lang.clone())?.to_lowercase();

    let mut total_generated = 0;
    let mut total_errors = 0;

    for service in &services {
        println!("\nService: {}", service.name);
        println!("  Path: {}", service.relative_path.display());

        let resolver = ServicePathResolver::new(ServiceContext {
            service: Some(service.clone()),
            workspace_root: Some(workspace_root.clone()),
            is_monorepo: true,
        });

        // Read definitions
        let definitions_path = resolver.definitions_file();
        if !definitions_path.exists() {
            println!("  - Definitions: not found, skipping");
            continue;
        }

        let definitions_content = fs::read_to_string(&definitions_path)
            .map_err(|e| CliError::Message(format!("Failed to read definitions: {e}")))?;
        let definitions = parse_definitions(&definitions_content)?;
        validate_definitions(&definitions)
            .map_err(|e| CliError::Message(format!("Definitions invalid: {e}")))?;

        // Determine output path
        let output_path = if let Some(ref output) = options.output {
            resolver.base_path().join(output)
        } else {
            resolver.sdk_output("flags")
        };

        match generate_sdk(&language, &definitions, &output_path) {
            Ok(()) => {
                println!("  ✓ SDK generated to {}", output_path.display());
                total_generated += 1;
            }
            Err(e) => {
                println!("  ✗ SDK generation failed");
                eprintln!("    Error: {e}");
                total_errors += 1;
            }
        }
    }

    println!("\nSummary:");
    println!("  Generated: {}", total_generated);
    if total_errors > 0 {
        println!("  Errors: {}", total_errors);
        return Err(CliError::Message(format!(
            "SDK generation failed for {} service(s)",
            total_errors
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::generate_sdk;
    use tempfile::TempDir;

    #[test]
    fn test_generate_sdk() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("generated");

        let definitions_content = r#"flags:
  - name: test_flag
    type: boolean
    defaultValue: false
"#;

        let definitions = parse_definitions(definitions_content).unwrap();
        let result = generate_sdk("typescript", &definitions, &output_path);
        assert!(result.is_ok());

        // Verify files were created
        assert!(output_path.join("index.ts").exists());
        assert!(output_path.join("types.ts").exists());
        assert!(output_path.join("package.json").exists());
    }
}
