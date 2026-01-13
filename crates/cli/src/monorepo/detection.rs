//! Monorepo detection utilities

use crate::error::{CliError, CliResult};
use crate::utils::config;
use std::fs;
use std::path::{Path, PathBuf};

/// Common monorepo directory patterns
pub const SERVICE_DIRECTORY_PATTERNS: &[&str] = &["services", "packages", "apps", "microservices"];

/// Check if a directory is a service (contains flags.definitions.yaml or .controlpath/)
pub fn is_service(path: &Path) -> bool {
    path.join("flags.definitions.yaml").exists() || path.join(".controlpath").is_dir()
}

/// Detect workspace root by walking up directory tree
///
/// Looks for:
/// 1. Common monorepo patterns (services/, packages/, etc.)
/// 2. .controlpath/config.yaml with monorepo section
pub fn detect_workspace_root(start_path: Option<&Path>) -> CliResult<Option<PathBuf>> {
    let start = start_path.unwrap_or_else(|| Path::new("."));
    let current_dir = std::env::current_dir()
        .map_err(|e| CliError::Message(format!("Failed to get current directory: {e}")))?;

    let mut path = if start.is_absolute() {
        start.to_path_buf()
    } else {
        current_dir.join(start)
    };

    // Walk up directory tree
    loop {
        // Check for common monorepo patterns
        for pattern in SERVICE_DIRECTORY_PATTERNS {
            let service_dir = path.join(pattern);
            if service_dir.exists() && service_dir.is_dir() {
                // Check if it contains any services
                if let Ok(entries) = fs::read_dir(&service_dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() && is_service(&entry.path()) {
                            return Ok(Some(path));
                        }
                    }
                }
            }
        }

        // Check for .controlpath/config.yaml with monorepo section
        let config_path = path.join(".controlpath/config.yaml");
        if config_path.exists() {
            if let Ok(Some(_)) = config::read_monorepo_config(&config_path) {
                return Ok(Some(path));
            }
        }

        // Move up one directory
        if let Some(parent) = path.parent() {
            if parent == path {
                // Reached filesystem root
                break;
            }
            path = parent.to_path_buf();
        } else {
            break;
        }
    }

    Ok(None)
}

/// Check if current directory structure is a monorepo
pub fn is_monorepo(workspace_root: Option<&Path>) -> bool {
    let root = workspace_root.unwrap_or_else(|| Path::new("."));

    // Check for common monorepo patterns
    for pattern in SERVICE_DIRECTORY_PATTERNS {
        let service_dir = root.join(pattern);
        if service_dir.exists() && service_dir.is_dir() {
            // Check if it contains any services
            if let Ok(entries) = fs::read_dir(&service_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() && is_service(&entry.path()) {
                        return true;
                    }
                }
            }
        }
    }

    // Check for config file with monorepo section
    let config_path = root.join(".controlpath/config.yaml");
    if config_path.exists() {
        if let Ok(Some(_)) = config::read_monorepo_config(&config_path) {
            return true;
        }
    }

    false
}

/// Discover all services in a monorepo
///
/// Uses workspace config if available, otherwise scans service directories.
///
/// Note: This function is exported for future use in bulk operations (section 1.2.2).
#[allow(dead_code)] // Will be used in section 1.2.2 for bulk operations
pub fn discover_services(workspace_root: &Path) -> CliResult<Vec<super::service::Service>> {
    use super::service::Service;

    // Read workspace config
    let config_path = workspace_root.join(".controlpath/config.yaml");
    let monorepo_config = if config_path.exists() {
        config::read_monorepo_config(&config_path)?
    } else {
        None
    };

    // If discovery mode is explicit and services list provided, use that
    if let Some(ref config) = monorepo_config {
        if config.discovery == "explicit" {
            if let Some(ref services) = config.services {
                return Ok(services
                    .iter()
                    .map(|name| Service::from_name(name, workspace_root))
                    .collect());
            }
        }
    }

    // Otherwise, scan service directories
    let service_dirs = monorepo_config
        .as_ref()
        .map(|c| c.service_directories.clone())
        .unwrap_or_else(|| {
            SERVICE_DIRECTORY_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect()
        });

    let mut services = Vec::new();
    for dir in service_dirs {
        let dir_path = workspace_root.join(&dir);
        if dir_path.exists() && dir_path.is_dir() {
            if let Ok(entries) = fs::read_dir(&dir_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && is_service(&path) {
                        services.push(Service::from_path(&path, workspace_root));
                    }
                }
            }
        }
    }

    Ok(services)
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
    fn test_is_service_with_definitions() {
        let temp_dir = TempDir::new().unwrap();
        let service_path = temp_dir.path().join("service-a");
        fs::create_dir_all(&service_path).unwrap();
        fs::write(service_path.join("flags.definitions.yaml"), "").unwrap();

        assert!(is_service(&service_path));
    }

    #[test]
    fn test_is_service_with_controlpath() {
        let temp_dir = TempDir::new().unwrap();
        let service_path = temp_dir.path().join("service-b");
        fs::create_dir_all(&service_path).unwrap();
        fs::create_dir_all(service_path.join(".controlpath")).unwrap();

        assert!(is_service(&service_path));
    }

    #[test]
    fn test_is_service_without_markers() {
        let temp_dir = TempDir::new().unwrap();
        let service_path = temp_dir.path().join("not-a-service");
        fs::create_dir_all(&service_path).unwrap();

        assert!(!is_service(&service_path));
    }

    #[test]
    #[serial]
    fn test_detect_workspace_root_with_services() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create monorepo structure
        let services_dir = temp_path.join("services");
        let service_a = services_dir.join("service-a");
        fs::create_dir_all(&service_a).unwrap();
        fs::write(service_a.join("flags.definitions.yaml"), "").unwrap();

        let result = detect_workspace_root(None).unwrap();
        assert_eq!(
            result.map(|p| p.canonicalize().unwrap()),
            Some(temp_path.canonicalize().unwrap())
        );
    }

    #[test]
    #[serial]
    fn test_detect_workspace_root_with_packages() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create monorepo structure with packages
        let packages_dir = temp_path.join("packages");
        let service_a = packages_dir.join("service-a");
        fs::create_dir_all(&service_a).unwrap();
        fs::create_dir_all(service_a.join(".controlpath")).unwrap();

        let result = detect_workspace_root(None).unwrap();
        assert_eq!(
            result.map(|p| p.canonicalize().unwrap()),
            Some(temp_path.canonicalize().unwrap())
        );
    }

    #[test]
    #[serial]
    fn test_detect_workspace_root_with_config() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create config with monorepo section
        fs::create_dir_all(temp_path.join(".controlpath")).unwrap();
        fs::write(
            temp_path.join(".controlpath/config.yaml"),
            "monorepo:\n  serviceDirectories:\n    - services\n  discovery: auto\n",
        )
        .unwrap();

        let result = detect_workspace_root(None).unwrap();
        assert_eq!(
            result.map(|p| p.canonicalize().unwrap()),
            Some(temp_path.canonicalize().unwrap())
        );
    }

    #[test]
    #[serial]
    fn test_detect_workspace_root_no_monorepo() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create regular directory (not a monorepo)
        fs::create_dir_all(temp_path.join("some-dir")).unwrap();

        let result = detect_workspace_root(None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[serial]
    fn test_detect_workspace_root_nested() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // Create nested structure
        let nested = temp_path.join("nested").join("deep");
        fs::create_dir_all(&nested).unwrap();
        std::env::set_current_dir(&nested).unwrap();

        // Monorepo root is at temp_path
        let services_dir = temp_path.join("services");
        let service_a = services_dir.join("service-a");
        fs::create_dir_all(&service_a).unwrap();
        fs::write(service_a.join("flags.definitions.yaml"), "").unwrap();

        let result = detect_workspace_root(None).unwrap();
        assert_eq!(
            result.map(|p| p.canonicalize().unwrap()),
            Some(temp_path.canonicalize().unwrap())
        );
    }

    #[test]
    fn test_is_monorepo_with_services() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let services_dir = temp_path.join("services");
        let service_a = services_dir.join("service-a");
        fs::create_dir_all(&service_a).unwrap();
        fs::write(service_a.join("flags.definitions.yaml"), "").unwrap();

        assert!(is_monorepo(Some(temp_path)));
    }

    #[test]
    fn test_is_monorepo_with_config() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join(".controlpath")).unwrap();
        fs::write(
            temp_path.join(".controlpath/config.yaml"),
            "monorepo:\n  serviceDirectories:\n    - services\n  discovery: auto\n",
        )
        .unwrap();

        assert!(is_monorepo(Some(temp_path)));
    }

    #[test]
    fn test_is_monorepo_no_monorepo() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join("some-dir")).unwrap();

        assert!(!is_monorepo(Some(temp_path)));
    }

    #[test]
    fn test_discover_services_auto() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create services
        let services_dir = temp_path.join("services");
        let service_a = services_dir.join("service-a");
        let service_b = services_dir.join("service-b");
        fs::create_dir_all(&service_a).unwrap();
        fs::create_dir_all(&service_b).unwrap();
        fs::write(service_a.join("flags.definitions.yaml"), "").unwrap();
        fs::create_dir_all(service_b.join(".controlpath")).unwrap();

        let services = discover_services(temp_path).unwrap();
        assert_eq!(services.len(), 2);
        assert!(services.iter().any(|s| s.name == "service-a"));
        assert!(services.iter().any(|s| s.name == "service-b"));
    }

    #[test]
    fn test_discover_services_explicit() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create config with explicit services
        fs::create_dir_all(temp_path.join(".controlpath")).unwrap();
        fs::write(
            temp_path.join(".controlpath/config.yaml"),
            "monorepo:\n  serviceDirectories:\n    - services\n  discovery: explicit\n  services:\n    - service-a\n    - service-c\n",
        ).unwrap();

        // Create services (including one not in the list)
        let services_dir = temp_path.join("services");
        let service_a = services_dir.join("service-a");
        let service_b = services_dir.join("service-b");
        let service_c = services_dir.join("service-c");
        fs::create_dir_all(&service_a).unwrap();
        fs::create_dir_all(&service_b).unwrap();
        fs::create_dir_all(&service_c).unwrap();
        fs::write(service_a.join("flags.definitions.yaml"), "").unwrap();
        fs::write(service_b.join("flags.definitions.yaml"), "").unwrap();
        fs::write(service_c.join("flags.definitions.yaml"), "").unwrap();

        let services = discover_services(temp_path).unwrap();
        assert_eq!(services.len(), 2);
        assert!(services.iter().any(|s| s.name == "service-a"));
        assert!(services.iter().any(|s| s.name == "service-c"));
        assert!(!services.iter().any(|s| s.name == "service-b"));
    }

    #[test]
    fn test_discover_services_multiple_directories() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create services in multiple directories
        let services_dir = temp_path.join("services");
        let packages_dir = temp_path.join("packages");
        let service_a = services_dir.join("service-a");
        let package_b = packages_dir.join("package-b");
        fs::create_dir_all(&service_a).unwrap();
        fs::create_dir_all(&package_b).unwrap();
        fs::write(service_a.join("flags.definitions.yaml"), "").unwrap();
        fs::write(package_b.join("flags.definitions.yaml"), "").unwrap();

        let services = discover_services(temp_path).unwrap();
        assert_eq!(services.len(), 2);
        assert!(services.iter().any(|s| s.name == "service-a"));
        assert!(services.iter().any(|s| s.name == "package-b"));
    }

    #[test]
    fn test_discover_services_custom_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create config with custom service directory
        fs::create_dir_all(temp_path.join(".controlpath")).unwrap();
        fs::write(
            temp_path.join(".controlpath/config.yaml"),
            "monorepo:\n  serviceDirectories:\n    - microservices\n  discovery: auto\n",
        )
        .unwrap();

        // Create services in custom directory
        let microservices_dir = temp_path.join("microservices");
        let service_a = microservices_dir.join("service-a");
        fs::create_dir_all(&service_a).unwrap();
        fs::write(service_a.join("flags.definitions.yaml"), "").unwrap();

        let services = discover_services(temp_path).unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "service-a");
    }

    #[test]
    fn test_discover_services_empty() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create services directory but no services
        fs::create_dir_all(temp_path.join("services")).unwrap();

        let services = discover_services(temp_path).unwrap();
        assert_eq!(services.len(), 0);
    }
}
