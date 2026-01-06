//! Language detection utilities

use crate::error::CliResult;
use crate::utils::config;
use std::path::Path;

/// Detect language from config file, project files, or default
/// Priority: CLI flag > Config file > Auto-detection > Default
pub fn detect_language() -> CliResult<String> {
    // 1. Check config file first
    if let Some(config_lang) = config::read_config_language()? {
        return Ok(config_lang);
    }

    // 2. Auto-detect from project files
    if Path::new("package.json").exists() {
        return Ok("typescript".to_string());
    }
    if Path::new("requirements.txt").exists() || Path::new("pyproject.toml").exists() {
        return Ok("python".to_string());
    }
    if Path::new("go.mod").exists() {
        return Ok("go".to_string());
    }
    if Path::new("Cargo.toml").exists() {
        return Ok("rust".to_string());
    }

    // 3. Default to typescript
    Ok("typescript".to_string())
}

/// Determine language with priority: CLI flag > Config > Auto-detect > Default
pub fn determine_language(cli_lang: Option<String>) -> CliResult<String> {
    // CLI flag has highest priority
    if let Some(lang) = cli_lang {
        return Ok(lang);
    }

    // Fall back to detection
    detect_language()
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
    fn test_detect_language_from_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write("package.json", "{}").unwrap();
        let lang = detect_language().unwrap();
        assert_eq!(lang, "typescript");
    }

    #[test]
    #[serial]
    fn test_detect_language_from_requirements_txt() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write("requirements.txt", "").unwrap();
        let lang = detect_language().unwrap();
        assert_eq!(lang, "python");
    }

    #[test]
    #[serial]
    fn test_detect_language_from_pyproject_toml() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write("pyproject.toml", "").unwrap();
        let lang = detect_language().unwrap();
        assert_eq!(lang, "python");
    }

    #[test]
    #[serial]
    fn test_detect_language_from_go_mod() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write("go.mod", "").unwrap();
        let lang = detect_language().unwrap();
        assert_eq!(lang, "go");
    }

    #[test]
    #[serial]
    fn test_detect_language_from_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write("Cargo.toml", "").unwrap();
        let lang = detect_language().unwrap();
        assert_eq!(lang, "rust");
    }

    #[test]
    #[serial]
    fn test_detect_language_default() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        // No project files
        let lang = detect_language().unwrap();
        assert_eq!(lang, "typescript");
    }

    #[test]
    fn test_determine_language_with_cli_flag() {
        let lang = determine_language(Some("python".to_string())).unwrap();
        assert_eq!(lang, "python");
    }

    #[test]
    #[serial]
    fn test_determine_language_without_cli_flag() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path);

        fs::write("package.json", "{}").unwrap();
        let lang = determine_language(None).unwrap();
        assert_eq!(lang, "typescript");
    }
}
