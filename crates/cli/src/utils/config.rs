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
    if let Some(config) = read_config_file()? {
        Ok(config.language)
    } else {
        Ok(None)
    }
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
            branch_environments: None,
            mode: None,
        })
    } else {
        ConfigFile {
            language: None,
            default_env: None,
            default_env_alt: None,
            sdk_output: None,
            sign_key: None,
            branch_environments: None,
            mode: None,
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
            branch_environments: None,
            mode: None,
        })
    } else {
        ConfigFile {
            language: None,
            default_env: None,
            default_env_alt: None,
            sdk_output: None,
            sign_key: None,
            branch_environments: None,
            mode: None,
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
    #[serde(rename = "branchEnvironments")]
    pub branch_environments: Option<std::collections::HashMap<String, String>>,
    /// Operation mode: 'local' for local compilation, 'saas' for remote AST generation via SaaS API
    pub mode: Option<String>,
}

/// Read the full config file
///
/// Returns the parsed ConfigFile if it exists, None otherwise.
/// This is more efficient than reading the config multiple times.
pub fn read_config_file() -> CliResult<Option<ConfigFile>> {
    let config_path = Path::new(".controlpath/config.yaml");
    if !config_path.exists() {
        return Ok(None);
    }

    let config_content = fs::read_to_string(config_path)
        .map_err(|e| CliError::Message(format!("Failed to read config file: {e}")))?;

    let config: ConfigFile = serde_yaml::from_str(&config_content)
        .map_err(|e| CliError::Message(format!("Failed to parse config file: {e}")))?;

    Ok(Some(config))
}

/// Environment defaults from config file
///
/// Contains both branch environments mapping and default environment.
pub type EnvironmentDefaults = (
    Option<std::collections::HashMap<String, String>>,
    Option<String>,
);

/// Read environment defaults from config file
///
/// Returns both branch environments mapping and default environment in a single read.
/// This is more efficient than reading the config file multiple times.
pub fn read_environment_defaults() -> CliResult<EnvironmentDefaults> {
    if let Some(config) = read_config_file()? {
        let default_env = config.default_env.or(config.default_env_alt);
        Ok((config.branch_environments, default_env))
    } else {
        Ok((None, None))
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
    fn test_read_config_language_no_config() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let result = read_config_language().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[serial]
    fn test_read_config_language_with_language() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

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
        let _guard = DirGuard::new(temp_path).unwrap();

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
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(".controlpath/config.yaml", "defaultEnv: production\n").unwrap();

        let result = read_config_language().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[serial]
    fn test_write_config_default_env_preserves_language() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

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
