//! Services command implementation
//!
//! Commands for managing and inspecting services in a monorepo.

use crate::error::{CliError, CliResult};
use crate::monorepo::{detect_workspace_root, discover_services, Service};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Options for services commands
pub struct Options {
    /// The services subcommand to execute
    pub subcommand: ServicesSubcommand,
}

/// Services subcommands
#[derive(Debug, Clone)]
pub enum ServicesSubcommand {
    /// List all services in monorepo
    List {
        /// Show detailed information (flag counts, environments, etc.)
        detailed: bool,
        /// Output format (table, json)
        format: OutputFormat,
    },
    /// Show status of services
    Status {
        /// Specific service to check (shows all if not provided)
        service: Option<String>,
        /// Check sync status
        check_sync: bool,
    },
}

/// Output format for services list
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Table,
    Json,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "table" => Some(Self::Table),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

/// Service information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub path: String,
    pub relative_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environments: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_sdk: Option<bool>,
}

/// Service status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub has_definitions: bool,
    pub has_deployments: bool,
    pub flag_count: Option<usize>,
    pub environments: Vec<String>,
    pub sdk_generated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_status: Option<SyncStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub in_sync: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_in_deployments: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_in_deployments: Option<Vec<String>>,
}

/// Run the services command
pub fn run(options: &Options) -> i32 {
    match run_inner(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn run_inner(options: &Options) -> CliResult<()> {
    // Detect workspace root
    let workspace_root = detect_workspace_root(None)?.ok_or_else(|| {
        CliError::Message(
            "Not in a monorepo. Services commands only work in monorepo environments".to_string(),
        )
    })?;

    match &options.subcommand {
        ServicesSubcommand::List { detailed, format } => {
            list_services(&workspace_root, *detailed, *format)
        }
        ServicesSubcommand::Status {
            service,
            check_sync,
        } => status_services(&workspace_root, service.as_deref(), *check_sync),
    }
}

fn list_services(workspace_root: &Path, detailed: bool, format: OutputFormat) -> CliResult<()> {
    let services = discover_services(workspace_root)?;

    if services.is_empty() {
        println!("No services found in monorepo");
        return Ok(());
    }

    let service_infos: Vec<ServiceInfo> = if detailed {
        services.iter().map(get_detailed_service_info).collect()
    } else {
        services
            .iter()
            .map(|s| ServiceInfo {
                name: s.name.clone(),
                path: s.path.display().to_string(),
                relative_path: s.relative_path.display().to_string(),
                flag_count: None,
                environments: None,
                has_sdk: None,
            })
            .collect()
    };

    match format {
        OutputFormat::Table => {
            if detailed {
                print_detailed_table(&service_infos);
            } else {
                print_simple_table(&service_infos);
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&service_infos)
                .map_err(|e| CliError::Message(format!("Failed to serialize to JSON: {e}")))?;
            println!("{json}");
        }
    }

    Ok(())
}

fn status_services(
    workspace_root: &Path,
    service_name: Option<&str>,
    check_sync: bool,
) -> CliResult<()> {
    let services = if let Some(name) = service_name {
        // Validate the service exists and is valid
        vec![Service::from_name_validated(name, workspace_root)?]
    } else {
        discover_services(workspace_root)?
    };

    if services.is_empty() {
        println!("No services found in monorepo");
        return Ok(());
    }

    let mut statuses: Vec<ServiceStatus> = Vec::new();

    for service in &services {
        let status = get_service_status(service, check_sync)?;
        statuses.push(status);
    }

    // Print status
    print_status_table(&statuses);

    Ok(())
}

fn get_detailed_service_info(service: &Service) -> ServiceInfo {
    let flag_count = count_flags(&service.path);
    let environments = list_environments(&service.path);
    let has_sdk = check_sdk_generated(&service.path);

    ServiceInfo {
        name: service.name.clone(),
        path: service.path.display().to_string(),
        relative_path: service.relative_path.display().to_string(),
        flag_count: Some(flag_count),
        environments: Some(environments),
        has_sdk: Some(has_sdk),
    }
}

fn get_service_status(service: &Service, check_sync: bool) -> CliResult<ServiceStatus> {
    let definitions_path = service.path.join("flags.definitions.yaml");
    let controlpath_dir = service.path.join(".controlpath");

    let has_definitions = definitions_path.exists();
    let has_deployments = controlpath_dir.exists();
    let flag_count = if has_definitions {
        Some(count_flags(&service.path))
    } else {
        None
    };
    let environments = list_environments(&service.path);
    let sdk_generated = check_sdk_generated(&service.path);

    let sync_status = if check_sync && has_definitions && has_deployments {
        Some(check_sync_status(&service.path)?)
    } else {
        None
    };

    Ok(ServiceStatus {
        name: service.name.clone(),
        path: service.path.display().to_string(),
        relative_path: service.relative_path.display().to_string(),
        has_definitions,
        has_deployments,
        flag_count,
        environments,
        sdk_generated,
        sync_status,
    })
}

fn count_flags(service_path: &Path) -> usize {
    let definitions_path = service_path.join("flags.definitions.yaml");
    if !definitions_path.exists() {
        return 0;
    }

    if let Ok(content) = fs::read_to_string(&definitions_path) {
        if let Ok(definitions) = controlpath_compiler::parse_definitions(&content) {
            if let Some(flags) = definitions.get("flags").and_then(|f| f.as_object()) {
                return flags.len();
            }
        }
    }
    0
}

fn list_environments(service_path: &Path) -> Vec<String> {
    let controlpath_dir = service_path.join(".controlpath");
    if !controlpath_dir.exists() {
        return Vec::new();
    }

    let mut environments = Vec::new();
    if let Ok(entries) = fs::read_dir(&controlpath_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".deployment.yaml") {
                    let env_name = name.strip_suffix(".deployment.yaml").unwrap_or(name);
                    environments.push(env_name.to_string());
                }
            }
        }
    }
    environments.sort();
    environments
}

fn check_sdk_generated(service_path: &Path) -> bool {
    let flags_dir = service_path.join("flags");
    flags_dir.exists() && flags_dir.is_dir()
}

fn check_sync_status(service_path: &Path) -> CliResult<SyncStatus> {
    use controlpath_compiler::parse_definitions;
    use std::collections::HashSet;

    // Read definitions
    let definitions_path = service_path.join("flags.definitions.yaml");
    let definitions_content = fs::read_to_string(&definitions_path)
        .map_err(|e| CliError::Message(format!("Failed to read definitions: {e}")))?;
    let definitions = parse_definitions(&definitions_content)
        .map_err(|e| CliError::Message(format!("Failed to parse definitions: {e}")))?;

    let definition_flags: HashSet<String> = definitions
        .get("flags")
        .and_then(|f| f.as_object())
        .map(|flags| flags.keys().cloned().collect())
        .unwrap_or_default();

    // Read all deployment files
    let controlpath_dir = service_path.join(".controlpath");
    let mut deployment_flags: HashSet<String> = HashSet::new();

    if controlpath_dir.exists() {
        if let Ok(entries) = fs::read_dir(&controlpath_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".deployment.yaml") {
                        let deployment_path = entry.path();
                        if let Ok(content) = fs::read_to_string(&deployment_path) {
                            if let Ok(deployment) = controlpath_compiler::parse_deployment(&content)
                            {
                                if let Some(flags) =
                                    deployment.get("flags").and_then(|f| f.as_object())
                                {
                                    for flag_name in flags.keys() {
                                        deployment_flags.insert(flag_name.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let missing_in_deployments: Vec<String> = definition_flags
        .difference(&deployment_flags)
        .cloned()
        .collect();
    let extra_in_deployments: Vec<String> = deployment_flags
        .difference(&definition_flags)
        .cloned()
        .collect();

    let in_sync = missing_in_deployments.is_empty() && extra_in_deployments.is_empty();

    Ok(SyncStatus {
        in_sync,
        missing_in_deployments: if missing_in_deployments.is_empty() {
            None
        } else {
            Some(missing_in_deployments)
        },
        extra_in_deployments: if extra_in_deployments.is_empty() {
            None
        } else {
            Some(extra_in_deployments)
        },
    })
}

fn print_simple_table(services: &[ServiceInfo]) {
    println!("Services:");
    println!("{:-<80}", "");
    println!("{:<30} {:<50}", "Name", "Path");
    println!("{:-<80}", "");

    for service in services {
        println!("{:<30} {:<50}", service.name, service.relative_path);
    }
}

fn print_detailed_table(services: &[ServiceInfo]) {
    println!("Services (detailed):");
    println!("{:-<100}", "");
    println!(
        "{:<20} {:<30} {:<10} {:<20} {:<10}",
        "Name", "Path", "Flags", "Environments", "SDK"
    );
    println!("{:-<100}", "");

    for service in services {
        let flag_count = service
            .flag_count
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".to_string());
        let envs = service
            .environments
            .as_ref()
            .map(|e| e.join(", "))
            .unwrap_or_else(|| "-".to_string());
        let sdk = if service.has_sdk.unwrap_or(false) {
            "✓"
        } else {
            "-"
        };

        println!(
            "{:<20} {:<30} {:<10} {:<20} {:<10}",
            service.name, service.relative_path, flag_count, envs, sdk
        );
    }
}

fn print_status_table(statuses: &[ServiceStatus]) {
    for status in statuses {
        println!("\nService: {}", status.name);
        println!("  Path: {}", status.relative_path);
        println!(
            "  Definitions: {}",
            if status.has_definitions { "✓" } else { "✗" }
        );
        println!(
            "  Deployments: {}",
            if status.has_deployments { "✓" } else { "✗" }
        );

        if let Some(count) = status.flag_count {
            println!("  Flags: {}", count);
        }

        if !status.environments.is_empty() {
            println!("  Environments: {}", status.environments.join(", "));
        } else {
            println!("  Environments: (none)");
        }

        println!(
            "  SDK Generated: {}",
            if status.sdk_generated { "✓" } else { "✗" }
        );

        if let Some(ref sync) = status.sync_status {
            println!(
                "  Sync Status: {}",
                if sync.in_sync {
                    "✓ In sync"
                } else {
                    "✗ Out of sync"
                }
            );
            if let Some(ref missing) = sync.missing_in_deployments {
                println!("    Missing in deployments: {}", missing.join(", "));
            }
            if let Some(ref extra) = sync.extra_in_deployments {
                println!("    Extra in deployments: {}", extra.join(", "));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    struct DirGuard {
        original_dir: PathBuf,
    }

    impl DirGuard {
        fn new(temp_path: &std::path::Path) -> Self {
            fs::create_dir_all(temp_path).unwrap();
            let original_dir = std::env::current_dir().unwrap();
            std::env::set_current_dir(temp_path).unwrap();
            DirGuard { original_dir }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original_dir);
        }
    }

    #[test]
    fn test_output_format_from_str() {
        assert!(matches!(
            OutputFormat::from_str("table"),
            Some(OutputFormat::Table)
        ));
        assert!(matches!(
            OutputFormat::from_str("json"),
            Some(OutputFormat::Json)
        ));
        assert!(matches!(
            OutputFormat::from_str("TABLE"),
            Some(OutputFormat::Table)
        ));
        assert!(OutputFormat::from_str("invalid").is_none());
    }

    #[test]
    #[serial]
    fn test_list_services_simple() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create monorepo
        let services_dir = temp_path.join("services");
        let service_a = services_dir.join("service-a");
        let service_b = services_dir.join("service-b");
        fs::create_dir_all(&service_a).unwrap();
        fs::create_dir_all(&service_b).unwrap();
        fs::write(service_a.join("flags.definitions.yaml"), "flags: {}").unwrap();
        fs::write(service_b.join("flags.definitions.yaml"), "flags: {}").unwrap();

        let options = Options {
            subcommand: ServicesSubcommand::List {
                detailed: false,
                format: OutputFormat::Table,
            },
        };

        let result = run_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_list_services_detailed() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create monorepo with service
        let services_dir = temp_path.join("services");
        let service_a = services_dir.join("service-a");
        fs::create_dir_all(&service_a).unwrap();
        fs::write(
            service_a.join("flags.definitions.yaml"),
            "flags:\n  - name: flag1\n    type: boolean\n    defaultValue: false",
        )
        .unwrap();
        fs::create_dir_all(service_a.join(".controlpath")).unwrap();
        fs::write(
            service_a.join(".controlpath/production.deployment.yaml"),
            "environment: production\nrules:\n  flag1:\n    rules:\n      - serve: false",
        )
        .unwrap();

        let options = Options {
            subcommand: ServicesSubcommand::List {
                detailed: true,
                format: OutputFormat::Table,
            },
        };

        let result = run_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_list_services_json() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create monorepo
        let services_dir = temp_path.join("services");
        let service_a = services_dir.join("service-a");
        fs::create_dir_all(&service_a).unwrap();
        fs::write(service_a.join("flags.definitions.yaml"), "flags: {}").unwrap();

        let options = Options {
            subcommand: ServicesSubcommand::List {
                detailed: false,
                format: OutputFormat::Json,
            },
        };

        let result = run_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_status_services_all() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create monorepo
        let services_dir = temp_path.join("services");
        let service_a = services_dir.join("service-a");
        fs::create_dir_all(&service_a).unwrap();
        fs::write(service_a.join("flags.definitions.yaml"), "flags: {}").unwrap();

        let options = Options {
            subcommand: ServicesSubcommand::Status {
                service: None,
                check_sync: false,
            },
        };

        let result = run_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_status_services_specific() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create monorepo
        let services_dir = temp_path.join("services");
        let service_a = services_dir.join("service-a");
        fs::create_dir_all(&service_a).unwrap();
        fs::write(service_a.join("flags.definitions.yaml"), "flags: {}").unwrap();

        let options = Options {
            subcommand: ServicesSubcommand::Status {
                service: Some("service-a".to_string()),
                check_sync: false,
            },
        };

        let result = run_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_status_services_with_sync_check() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create monorepo with synced service
        let services_dir = temp_path.join("services");
        let service_a = services_dir.join("service-a");
        fs::create_dir_all(&service_a).unwrap();
        fs::write(
            service_a.join("flags.definitions.yaml"),
            "flags:\n  - name: flag1\n    type: boolean\n    defaultValue: false",
        )
        .unwrap();
        fs::create_dir_all(service_a.join(".controlpath")).unwrap();
        fs::write(
            service_a.join(".controlpath/production.deployment.yaml"),
            "environment: production\nrules:\n  flag1:\n    rules:\n      - serve: false",
        )
        .unwrap();

        let options = Options {
            subcommand: ServicesSubcommand::Status {
                service: Some("service-a".to_string()),
                check_sync: true,
            },
        };

        let result = run_inner(&options);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_list_services_not_in_monorepo() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Not a monorepo
        fs::create_dir_all(temp_path.join("some-dir")).unwrap();

        let options = Options {
            subcommand: ServicesSubcommand::List {
                detailed: false,
                format: OutputFormat::Table,
            },
        };

        let result = run_inner(&options);
        assert!(result.is_err());
    }
}
