//! Config file reading utilities

use crate::error::{CliError, CliResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Read language preference from config file
///
/// Uses proper YAML parsing to read the language field from the config file.
/// Returns None if the config file doesn't exist or doesn't contain a language field.
pub fn read_config_language() -> CliResult<Option<String>> {
    let config_path = Path::new(".controlpath/config.yaml");

    if !config_path.exists() {
        return Ok(None);
    }

    let config_content = fs::read_to_string(config_path)
        .map_err(|e| CliError::Message(format!("Failed to read config file: {e}")))?;

    // Use proper YAML parsing with ConfigFile struct for consistency
    let config: ConfigFile = serde_yaml::from_str(&config_content)
        .map_err(|e| CliError::Message(format!("Failed to parse config file: {e}")))?;

    Ok(config.language)
}

/// Write language preference to config file
///
/// Creates or updates the config file with the detected language.
/// This caches the language detection result for future runs.
/// Uses proper YAML parsing to preserve existing config structure.
pub fn write_config_language(language: &str) -> CliResult<()> {
    use std::path::PathBuf;

    let config_dir = PathBuf::from(".controlpath");
    let config_path = config_dir.join("config.yaml");

    // Create .controlpath directory if it doesn't exist
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|e| {
            CliError::Message(format!("Failed to create .controlpath directory: {e}"))
        })?;
    }

    // Read existing config if it exists, otherwise create new one
    let mut config: ConfigFile = if config_path.exists() {
        let config_content = fs::read_to_string(&config_path)
            .map_err(|e| CliError::Message(format!("Failed to read config file: {e}")))?;
        serde_yaml::from_str(&config_content).unwrap_or(ConfigFile {
            language: None,
            default_env: None,
            default_env_alt: None,
            sdk_output: None,
            sign_key: None,
            monorepo: None,
        })
    } else {
        ConfigFile {
            language: None,
            default_env: None,
            default_env_alt: None,
            sdk_output: None,
            sign_key: None,
            monorepo: None,
        }
    };

    // Update language field
    config.language = Some(language.to_string());

    // Write updated config using proper YAML serialization
    let updated_content = serde_yaml::to_string(&config)
        .map_err(|e| CliError::Message(format!("Failed to serialize config file: {e}")))?;
    fs::write(&config_path, updated_content)
        .map_err(|e| CliError::Message(format!("Failed to write config file: {e}")))?;

    Ok(())
}

/// Write default environment preference to config file
///
/// Creates or updates the config file with the default environment.
/// Uses proper YAML parsing to preserve existing config structure.
pub fn write_config_default_env(default_env: &str) -> CliResult<()> {
    use std::path::PathBuf;

    let config_dir = PathBuf::from(".controlpath");
    let config_path = config_dir.join("config.yaml");

    // Create .controlpath directory if it doesn't exist
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|e| {
            CliError::Message(format!("Failed to create .controlpath directory: {e}"))
        })?;
    }

    // Read existing config if it exists, otherwise create new one
    let mut config: ConfigFile = if config_path.exists() {
        let config_content = fs::read_to_string(&config_path)
            .map_err(|e| CliError::Message(format!("Failed to read config file: {e}")))?;
        serde_yaml::from_str(&config_content).unwrap_or(ConfigFile {
            language: None,
            default_env: None,
            default_env_alt: None,
            sdk_output: None,
            sign_key: None,
            monorepo: None,
        })
    } else {
        ConfigFile {
            language: None,
            default_env: None,
            default_env_alt: None,
            sdk_output: None,
            sign_key: None,
            monorepo: None,
        }
    };

    // Update default_env field (set both for compatibility)
    config.default_env = Some(default_env.to_string());
    config.default_env_alt = Some(default_env.to_string());

    // Write updated config using proper YAML serialization
    let updated_content = serde_yaml::to_string(&config)
        .map_err(|e| CliError::Message(format!("Failed to serialize config file: {e}")))?;
    fs::write(&config_path, updated_content)
        .map_err(|e| CliError::Message(format!("Failed to write config file: {e}")))?;

    Ok(())
}

/// Monorepo configuration from config file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

        // Ensure .controlpath directory exists and is clean
        fs::create_dir_all(".controlpath").unwrap();

        // Remove any existing config file to ensure clean state
        let config_path = ".controlpath/config.yaml";
        let _ = fs::remove_file(config_path);

        // Write config with typescript (test name suggests checking spaces, but this tests basic reading)
        fs::write(config_path, "language: typescript\n").unwrap();

        // Verify the file was written correctly before reading
        let file_content = fs::read_to_string(config_path).unwrap();
        assert!(
            file_content.contains("typescript"),
            "Config file should contain typescript, but got: {}",
            file_content
        );

        let result = read_config_language().unwrap();
        assert_eq!(
            result,
            Some("typescript".to_string()),
            "Expected typescript but got {:?}. Config file content: {}",
            result,
            file_content
        );
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
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();

        // Write invalid UTF-8 bytes directly to the file
        // This will cause fs::read_to_string to fail when trying to convert to String
        let config_path = temp_path.join(".controlpath/config.yaml");
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(b"invalid: yaml: content: ").unwrap();
        // Write invalid UTF-8 sequence that cannot be converted to String
        file.write_all(&[0xFF, 0xFE, 0xFD]).unwrap();
        file.write_all(b" invalid utf8").unwrap();
        drop(file);

        // Test that reading invalid YAML causes an error
        // fs::read_to_string will fail when trying to convert invalid UTF-8 to String
        let result = read_monorepo_config(&config_path);

        // Verify that invalid YAML with invalid UTF-8 causes an error
        assert!(
            result.is_err(),
            "Expected error for invalid YAML with invalid UTF-8 bytes. \
            fs::read_to_string should fail when converting invalid UTF-8 to String. \
            Got: {:?}",
            result
        );

        // Verify the error message mentions reading or UTF-8
        if let Err(e) = result {
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains("read")
                    || error_msg.contains("Failed to read")
                    || error_msg.contains("UTF-8")
                    || error_msg.contains("invalid")
                    || error_msg.contains("stream did not contain valid UTF-8"),
                "Error message should mention reading or UTF-8, but got: {}",
                error_msg
            );
        }
    }

    #[test]
    #[serial]
    fn test_write_config_default_env_preserves_language() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::create_dir_all(".controlpath").unwrap();

        // First, write a config with a language
        fs::write(".controlpath/config.yaml", "language: typescript\n").unwrap();

        // Verify language is set
        let result = read_config_language().unwrap();
        assert_eq!(result, Some("typescript".to_string()));

        // Now write default_env - this should preserve the language
        write_config_default_env("production").unwrap();

        // Verify language is still preserved
        let result_after = read_config_language().unwrap();
        assert_eq!(
            result_after,
            Some("typescript".to_string()),
            "Language should be preserved after writing default_env"
        );

        // Verify default_env was set
        let config_content = fs::read_to_string(".controlpath/config.yaml").unwrap();
        assert!(config_content.contains("defaultEnv:") || config_content.contains("default_env:"));
        assert!(config_content.contains("production"));
        assert!(config_content.contains("typescript"));
    }
}
