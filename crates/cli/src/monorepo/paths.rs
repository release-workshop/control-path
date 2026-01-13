//! Service-scoped file path resolution

use crate::monorepo::service::ServiceContext;
use std::path::{Path, PathBuf};

/// Resolve a file path relative to service context
///
/// If in monorepo mode with a service, paths are resolved relative to service directory.
/// Otherwise, paths are resolved relative to current directory (backward compatible).
///
/// Note: This function is exported for use in other commands that need service-scoped path resolution.
#[allow(dead_code)] // Exported for use in other commands
pub fn resolve_service_path(path: &str, context: &ServiceContext) -> PathBuf {
    let base = context.base_path();

    // If path is absolute, use as-is
    if Path::new(path).is_absolute() {
        return PathBuf::from(path);
    }

    // Resolve relative to service base path
    base.join(path)
}

/// Helper for resolving common Control Path file paths
pub struct ServicePathResolver {
    context: ServiceContext,
}

impl ServicePathResolver {
    pub fn new(context: ServiceContext) -> Self {
        Self { context }
    }

    /// Get the base path for this resolver
    pub fn base_path(&self) -> PathBuf {
        self.context.base_path()
    }

    /// Resolve flags.definitions.yaml path
    pub fn definitions_file(&self) -> PathBuf {
        resolve_service_path("flags.definitions.yaml", &self.context)
    }

    /// Resolve deployment file path for environment
    pub fn deployment_file(&self, env: &str) -> PathBuf {
        resolve_service_path(
            &format!(".controlpath/{env}.deployment.yaml"),
            &self.context,
        )
    }

    /// Resolve AST file path for environment
    pub fn ast_file(&self, env: &str) -> PathBuf {
        resolve_service_path(&format!(".controlpath/{env}.ast"), &self.context)
    }

    /// Resolve .controlpath directory
    #[allow(dead_code)] // Will be used when more commands are integrated
    pub fn controlpath_dir(&self) -> PathBuf {
        resolve_service_path(".controlpath", &self.context)
    }

    /// Resolve SDK output directory (default: ./flags)
    #[allow(dead_code)] // Will be used when generate-sdk is integrated
    pub fn sdk_output(&self, default: &str) -> PathBuf {
        resolve_service_path(default, &self.context)
    }

    /// Resolve config file path
    #[allow(dead_code)] // Will be used when config reading is integrated
    pub fn config_file(&self) -> PathBuf {
        resolve_service_path(".controlpath/config.yaml", &self.context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monorepo::service::{Service, ServiceContext};
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_service_path_relative() {
        let context = ServiceContext::single_project();
        let path = resolve_service_path("flags.definitions.yaml", &context);
        assert_eq!(path, PathBuf::from(".").join("flags.definitions.yaml"));
    }

    #[test]
    fn test_resolve_service_path_absolute() {
        let context = ServiceContext::single_project();
        let absolute_path = if cfg!(windows) {
            "C:\\absolute\\path"
        } else {
            "/absolute/path"
        };
        let path = resolve_service_path(absolute_path, &context);
        assert_eq!(path, PathBuf::from(absolute_path));
    }

    #[test]
    fn test_resolve_service_path_with_service() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let service_path = temp_path.join("services").join("service-a");
        let service = Service::from_path(&service_path, temp_path);

        let context = ServiceContext {
            service: Some(service),
            workspace_root: Some(temp_path.to_path_buf()),
            is_monorepo: true,
        };

        let path = resolve_service_path("flags.definitions.yaml", &context);
        assert_eq!(path, service_path.join("flags.definitions.yaml"));
    }

    #[test]
    fn test_service_path_resolver_definitions_file() {
        let context = ServiceContext::single_project();
        let resolver = ServicePathResolver::new(context);

        let path = resolver.definitions_file();
        assert_eq!(path, PathBuf::from(".").join("flags.definitions.yaml"));
    }

    #[test]
    fn test_service_path_resolver_deployment_file() {
        let context = ServiceContext::single_project();
        let resolver = ServicePathResolver::new(context);

        let path = resolver.deployment_file("production");
        assert_eq!(
            path,
            PathBuf::from(".").join(".controlpath/production.deployment.yaml")
        );
    }

    #[test]
    fn test_service_path_resolver_ast_file() {
        let context = ServiceContext::single_project();
        let resolver = ServicePathResolver::new(context);

        let path = resolver.ast_file("production");
        assert_eq!(path, PathBuf::from(".").join(".controlpath/production.ast"));
    }

    #[test]
    fn test_service_path_resolver_controlpath_dir() {
        let context = ServiceContext::single_project();
        let resolver = ServicePathResolver::new(context);

        let path = resolver.controlpath_dir();
        assert_eq!(path, PathBuf::from(".").join(".controlpath"));
    }

    #[test]
    fn test_service_path_resolver_sdk_output() {
        let context = ServiceContext::single_project();
        let resolver = ServicePathResolver::new(context);

        let path = resolver.sdk_output("./flags");
        assert_eq!(path, PathBuf::from(".").join("./flags"));
    }

    #[test]
    fn test_service_path_resolver_config_file() {
        let context = ServiceContext::single_project();
        let resolver = ServicePathResolver::new(context);

        let path = resolver.config_file();
        assert_eq!(path, PathBuf::from(".").join(".controlpath/config.yaml"));
    }

    #[test]
    fn test_service_path_resolver_with_service() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let service_path = temp_path.join("services").join("service-a");
        let service = Service::from_path(&service_path, temp_path);

        let context = ServiceContext {
            service: Some(service.clone()),
            workspace_root: Some(temp_path.to_path_buf()),
            is_monorepo: true,
        };

        let resolver = ServicePathResolver::new(context);

        assert_eq!(
            resolver.definitions_file(),
            service_path.join("flags.definitions.yaml")
        );
        assert_eq!(
            resolver.deployment_file("staging"),
            service_path.join(".controlpath/staging.deployment.yaml")
        );
        assert_eq!(
            resolver.ast_file("staging"),
            service_path.join(".controlpath/staging.ast")
        );
        assert_eq!(
            resolver.controlpath_dir(),
            service_path.join(".controlpath")
        );
        assert_eq!(
            resolver.config_file(),
            service_path.join(".controlpath/config.yaml")
        );
    }

    #[test]
    fn test_service_path_resolver_base_path() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let service_path = temp_path.join("services").join("service-a");
        let service = Service::from_path(&service_path, temp_path);

        let context = ServiceContext {
            service: Some(service),
            workspace_root: Some(temp_path.to_path_buf()),
            is_monorepo: true,
        };

        let resolver = ServicePathResolver::new(context);
        assert_eq!(resolver.base_path(), service_path);
    }
}
