//! Config file reading utilities

use crate::error::{CliError, CliResult};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Read language preference from config file
pub fn read_config_language() -> CliResult<Option<String>> {
    let config_path = Path::new(".controlpath/config.yaml");

    if !config_path.exists() {
        return Ok(None);
    }

    let config_content = fs::read_to_string(config_path)
        .map_err(|e| CliError::Message(format!("Failed to read config file: {e}")))?;

    // Simple YAML parsing for language field
    // This is a basic implementation - for production, consider using a YAML library
    for line in config_content.lines() {
        let line = line.trim();
        if line.starts_with("language:") {
            let value = line
                .split(':')
                .nth(1)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            return Ok(value);
        }
    }

    Ok(None)
}

/// Monorepo configuration from config file
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MonorepoConfig {
    /// Service directory patterns (searched in order)
    #[serde(default = "default_service_directories")]
    pub service_directories: Vec<String>,
    /// Service discovery mode: "auto" or "explicit"
    #[serde(default = "default_discovery")]
    pub discovery: String,
    /// Explicit service list (used when discovery: explicit)
    pub services: Option<Vec<String>>,
}

fn default_service_directories() -> Vec<String> {
    vec![
        "services".to_string(),
        "packages".to_string(),
        "apps".to_string(),
        "microservices".to_string(),
    ]
}

fn default_discovery() -> String {
    "auto".to_string()
}

/// Full config file structure
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // Fields will be used as config system is extended
pub struct ConfigFile {
    pub language: Option<String>,
    pub default_env: Option<String>,
    #[serde(rename = "defaultEnv")]
    pub default_env_alt: Option<String>,
    pub sdk_output: Option<String>,
    pub sign_key: Option<String>,
    pub monorepo: Option<MonorepoConfig>,
}

/// Read monorepo configuration from config file
pub fn read_monorepo_config(config_path: &Path) -> CliResult<Option<MonorepoConfig>> {
    if !config_path.exists() {
        return Ok(None);
    }

    let config_content = fs::read_to_string(config_path)
        .map_err(|e| CliError::Message(format!("Failed to read config file: {e}")))?;

    let config: ConfigFile = serde_yaml::from_str(&config_content)
        .map_err(|e| CliError::Message(format!("Failed to parse config file: {e}")))?;

    Ok(config.monorepo)
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
            // Ensure directory exists
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
    #[serial]
    fn test_read_config_language_no_config() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        let result = read_config_language().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[serial]
    fn test_read_config_language_with_language() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(".controlpath/config.yaml", "language: python\n").unwrap();

        let result = read_config_language().unwrap();
        assert_eq!(result, Some("python".to_string()));
    }

    #[test]
    #[serial]
    fn test_read_config_language_with_spaces() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(".controlpath/config.yaml", "language: typescript\n").unwrap();

        let result = read_config_language().unwrap();
        assert_eq!(result, Some("typescript".to_string()));
    }

    #[test]
    #[serial]
    fn test_read_config_language_no_language_field() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(".controlpath/config.yaml", "defaultEnv: production\n").unwrap();

        let result = read_config_language().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[serial]
    fn test_read_monorepo_config_with_monorepo_section() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/config.yaml",
            "language: typescript\nmonorepo:\n  serviceDirectories:\n    - services\n    - packages\n  discovery: auto\n",
        ).unwrap();

        let config_path = temp_path.join(".controlpath/config.yaml");
        let result = read_monorepo_config(&config_path).unwrap();
        assert!(result.is_some());
        let monorepo = result.unwrap();
        // Check that we have the expected directories (order may vary)
        assert!(monorepo.service_directories.len() >= 2);
        assert!(monorepo
            .service_directories
            .contains(&"services".to_string()));
        assert!(monorepo
            .service_directories
            .contains(&"packages".to_string()));
        assert_eq!(monorepo.discovery, "auto");
        assert!(monorepo.services.is_none());
    }

    #[test]
    #[serial]
    fn test_read_monorepo_config_with_explicit_services() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/config.yaml",
            "monorepo:\n  serviceDirectories:\n    - services\n  discovery: explicit\n  services:\n    - service-a\n    - service-b\n",
        ).unwrap();

        let config_path = temp_path.join(".controlpath/config.yaml");
        let result = read_monorepo_config(&config_path).unwrap();
        assert!(result.is_some());
        let monorepo = result.unwrap();
        assert_eq!(monorepo.discovery, "explicit");
        assert!(monorepo.services.is_some());
        assert_eq!(
            monorepo.services.as_ref().unwrap(),
            &vec!["service-a".to_string(), "service-b".to_string()]
        );
    }

    #[test]
    #[serial]
    fn test_read_monorepo_config_with_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(".controlpath/config.yaml", "monorepo:\n  discovery: auto\n").unwrap();

        let config_path = temp_path.join(".controlpath/config.yaml");
        let result = read_monorepo_config(&config_path).unwrap();
        assert!(result.is_some());
        let monorepo = result.unwrap();
        // Should use default service directories
        assert!(!monorepo.service_directories.is_empty());
        assert_eq!(monorepo.discovery, "auto");
    }

    #[test]
    #[serial]
    fn test_read_monorepo_config_no_monorepo_section() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/config.yaml",
            "language: typescript\ndefaultEnv: production\n",
        )
        .unwrap();

        let config_path = temp_path.join(".controlpath/config.yaml");
        let result = read_monorepo_config(&config_path).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[serial]
    fn test_read_monorepo_config_no_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        let config_path = temp_path.join(".controlpath/config.yaml");
        let result = read_monorepo_config(&config_path).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[serial]
    fn test_read_monorepo_config_invalid_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/config.yaml",
            "invalid: yaml: content: [unclosed",
        )
        .unwrap();

        let config_path = temp_path.join(".controlpath/config.yaml");
        let result = read_monorepo_config(&config_path);
        assert!(result.is_err());
    }
}
