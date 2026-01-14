//! Compile command implementation

use crate::error::{CliError, CliResult};
use crate::monorepo::{
    detect_workspace_root, discover_services, ServiceContext, ServicePathResolver,
};
use controlpath_compiler::{
    compile, parse_definitions, parse_deployment, serialize, validate_definitions,
    validate_deployment,
};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Options {
    pub deployment: Option<String>,
    pub env: Option<String>,
    pub output: Option<String>,
    pub definitions: Option<String>,
    pub service_context: Option<ServiceContext>,
    pub all_services: bool,
}

fn determine_deployment_path(options: &Options) -> Result<PathBuf, CliError> {
    let resolver = options
        .service_context
        .as_ref()
        .map(|ctx| ServicePathResolver::new(ctx.clone()));

    options.deployment.as_ref().map_or_else(
        || {
            options.env.as_ref().map_or_else(
                || {
                    Err(CliError::Message(
                        "Either --deployment <file> or --env <env> must be provided".to_string(),
                    ))
                },
                |env| {
                    Ok(if let Some(ref r) = resolver {
                        r.deployment_file(env)
                    } else {
                        PathBuf::from(format!(".controlpath/{env}.deployment.yaml"))
                    })
                },
            )
        },
        |deployment| {
            Ok(if let Some(ref r) = resolver {
                r.base_path().join(deployment)
            } else {
                PathBuf::from(deployment)
            })
        },
    )
}

fn determine_output_path(options: &Options, deployment_path: &Path) -> PathBuf {
    let resolver = options
        .service_context
        .as_ref()
        .map(|ctx| ServicePathResolver::new(ctx.clone()));

    options.output.as_ref().map_or_else(
        || {
            options.env.as_ref().map_or_else(
                || {
                    // Infer from deployment path
                    let deployment_dir = deployment_path.parent().unwrap_or_else(|| Path::new("."));
                    let deployment_stem = deployment_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("deployment")
                        .replace(".deployment", "");
                    deployment_dir.join(format!("{deployment_stem}.ast"))
                },
                |env| {
                    if let Some(ref r) = resolver {
                        r.ast_file(env)
                    } else {
                        PathBuf::from(format!(".controlpath/{env}.ast"))
                    }
                },
            )
        },
        |output| {
            if let Some(ref r) = resolver {
                r.base_path().join(output)
            } else {
                PathBuf::from(output)
            }
        },
    )
}

fn determine_definitions_path(options: &Options) -> PathBuf {
    let resolver = options
        .service_context
        .as_ref()
        .map(|ctx| ServicePathResolver::new(ctx.clone()));

    options.definitions.as_ref().map_or_else(
        || {
            if let Some(ref r) = resolver {
                r.definitions_file()
            } else {
                PathBuf::from("flags.definitions.yaml")
            }
        },
        |definitions| {
            if let Some(ref r) = resolver {
                r.base_path().join(definitions)
            } else {
                PathBuf::from(definitions)
            }
        },
    )
}

pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("✗ Compilation failed");
            eprintln!("  Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    // Handle bulk operations for monorepo
    if options.all_services {
        return run_bulk_compile(options);
    }

    // Determine file paths
    let deployment_path = determine_deployment_path(options)?;
    let output_path = determine_output_path(options, &deployment_path);
    let definitions_path = determine_definitions_path(options);

    // Read and parse definitions
    let definitions_content = fs::read_to_string(&definitions_path)
        .map_err(|e| CliError::Message(format!("Failed to read definitions file: {e}")))?;
    let definitions = parse_definitions(&definitions_content)?;

    // Validate definitions
    validate_definitions(&definitions)
        .map_err(|e| CliError::Message(format!("Definitions file is invalid: {e}")))?;

    // Read and parse deployment
    let deployment_content = fs::read_to_string(&deployment_path)
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

    println!("✓ Compiled to {}", output_path.display());
    Ok(())
}

fn run_bulk_compile(_options: &Options) -> CliResult<()> {
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

    println!("Compiling {} service(s)...", services.len());

    let mut total_compiled = 0;
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

        // Find all deployment files
        let controlpath_dir = resolver.base_path().join(".controlpath");
        if !controlpath_dir.exists() {
            println!("  - Deployments: not found, skipping");
            continue;
        }

        let mut service_compiled = 0;
        let mut service_errors = 0;

        if let Ok(entries) = fs::read_dir(&controlpath_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".deployment.yaml") {
                        let env_name = name.strip_suffix(".deployment.yaml").unwrap_or(name);
                        let deployment_path = entry.path();
                        let output_path = resolver.ast_file(env_name);

                        match compile_service_deployment(
                            &deployment_path,
                            &output_path,
                            &definitions,
                        ) {
                            Ok(()) => {
                                println!("  ✓ {}: compiled", env_name);
                                service_compiled += 1;
                                total_compiled += 1;
                            }
                            Err(e) => {
                                println!("  ✗ {}: failed", env_name);
                                eprintln!("    Error: {e}");
                                service_errors += 1;
                                total_errors += 1;
                            }
                        }
                    }
                }
            }
        }

        if service_errors > 0 {
            println!(
                "  Service summary: {} compiled, {} errors",
                service_compiled, service_errors
            );
        }
    }

    println!("\nSummary:");
    println!("  Compiled: {}", total_compiled);
    if total_errors > 0 {
        println!("  Errors: {}", total_errors);
        return Err(CliError::Message(format!(
            "Compilation failed for {} deployment(s)",
            total_errors
        )));
    }

    Ok(())
}

fn compile_service_deployment(
    deployment_path: &Path,
    output_path: &Path,
    definitions: &serde_json::Value,
) -> CliResult<()> {
    // Read and parse deployment
    let deployment_content = fs::read_to_string(deployment_path)
        .map_err(|e| CliError::Message(format!("Failed to read deployment: {e}")))?;
    let deployment = parse_deployment(&deployment_content)?;

    // Validate deployment
    validate_deployment(&deployment)
        .map_err(|e| CliError::Message(format!("Deployment invalid: {e}")))?;

    // Compile to AST
    let artifact = compile(&deployment, definitions)?;

    // Serialize to MessagePack
    let ast_bytes = serialize(&artifact)?;

    // Create output directory if needed
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Write AST file
    fs::write(output_path, ast_bytes)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_determine_deployment_path_with_deployment() {
        let options = Options {
            deployment: Some("test.deployment.yaml".to_string()),
            env: None,
            output: None,
            definitions: None,
            service_context: None,
            all_services: false,
        };
        let path = determine_deployment_path(&options).unwrap();
        assert_eq!(path, PathBuf::from("test.deployment.yaml"));
    }

    #[test]
    fn test_determine_deployment_path_with_env() {
        let options = Options {
            deployment: None,
            env: Some("production".to_string()),
            output: None,
            definitions: None,
            service_context: None,
            all_services: false,
        };
        let path = determine_deployment_path(&options).unwrap();
        assert_eq!(
            path,
            PathBuf::from(".controlpath/production.deployment.yaml")
        );
    }

    #[test]
    fn test_determine_deployment_path_without_options() {
        let options = Options {
            deployment: None,
            env: None,
            output: None,
            definitions: None,
            service_context: None,
            all_services: false,
        };
        assert!(determine_deployment_path(&options).is_err());
    }

    #[test]
    fn test_determine_output_path_with_output() {
        let options = Options {
            deployment: None,
            env: None,
            output: Some("output.ast".to_string()),
            definitions: None,
            service_context: None,
            all_services: false,
        };
        let deployment_path = PathBuf::from("test.deployment.yaml");
        let path = determine_output_path(&options, &deployment_path);
        assert_eq!(path, PathBuf::from("output.ast"));
    }

    #[test]
    fn test_determine_output_path_with_env() {
        let options = Options {
            deployment: None,
            env: Some("production".to_string()),
            output: None,
            definitions: None,
            service_context: None,
            all_services: false,
        };
        let deployment_path = PathBuf::from("test.deployment.yaml");
        let path = determine_output_path(&options, &deployment_path);
        assert_eq!(path, PathBuf::from(".controlpath/production.ast"));
    }

    #[test]
    fn test_determine_output_path_inferred() {
        let options = Options {
            deployment: Some("test.deployment.yaml".to_string()),
            env: None,
            output: None,
            definitions: None,
            service_context: None,
            all_services: false,
        };
        let deployment_path = PathBuf::from("test.deployment.yaml");
        let path = determine_output_path(&options, &deployment_path);
        assert_eq!(path, PathBuf::from("test.ast"));
    }

    #[test]
    fn test_determine_definitions_path_with_option() {
        let options = Options {
            deployment: None,
            env: None,
            output: None,
            definitions: Some("custom.definitions.yaml".to_string()),
            service_context: None,
            all_services: false,
        };
        let path = determine_definitions_path(&options);
        assert_eq!(path, PathBuf::from("custom.definitions.yaml"));
    }

    #[test]
    fn test_determine_definitions_path_default() {
        let options = Options {
            deployment: None,
            env: None,
            output: None,
            definitions: None,
            service_context: None,
            all_services: false,
        };
        let path = determine_definitions_path(&options);
        assert_eq!(path, PathBuf::from("flags.definitions.yaml"));
    }

    #[test]
    fn test_compile_command_success() {
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

        let deployment_path = temp_path.join("test.deployment.yaml");
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

        let output_path = temp_path.join("test.ast");

        let options = Options {
            deployment: Some(deployment_path.to_str().unwrap().to_string()),
            env: None,
            output: Some(output_path.to_str().unwrap().to_string()),
            definitions: Some(definitions_path.to_str().unwrap().to_string()),
            service_context: None,
            all_services: false,
        };

        let exit_code = run(&options);
        assert_eq!(exit_code, 0);
        assert!(output_path.exists());
    }
}
