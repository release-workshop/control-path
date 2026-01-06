//! Config file reading utilities

use crate::error::{CliError, CliResult};
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
}
