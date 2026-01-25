//! Environment detection utilities
//!
//! Provides smart defaults for environment selection based on:
//! - Git branch mapping (from branchEnvironments config)
//! - Default environment (from defaultEnv config)

use crate::error::CliResult;
use crate::utils::config;
use std::process::Command;

/// Get current git branch name
pub fn get_git_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    }
}

/// Determine environment from git branch or default
///
/// Priority:
/// 1. Git branch mapping (if branchEnvironments config exists and branch matches)
/// 2. defaultEnv from config
/// 3. None (no default found)
///
/// This function reads the config file only once for efficiency.
pub fn determine_environment() -> CliResult<Option<String>> {
    // Read config once to get both branch mappings and defaultEnv
    let (branch_envs, default_env) = config::read_environment_defaults()?;

    // Try to get git branch and check branch mapping
    if let Some(branch) = get_git_branch() {
        if let Some(ref branch_envs) = branch_envs {
            if let Some(env) = branch_envs.get(&branch) {
                return Ok(Some(env.clone()));
            }
        }
    }

    // Fallback to defaultEnv
    Ok(default_env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    use crate::test_helpers::DirGuard;

    #[test]
    #[serial]
    fn test_determine_environment_from_branch_mapping() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        // Initialize git repo
        let _ = Command::new("git").args(["init"]).output();

        // Configure git user (required for commits)
        let _ = Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .output();
        let _ = Command::new("git")
            .args(["config", "user.name", "Test User"])
            .output();

        // Create an initial commit (required before checking out branches)
        fs::write("README.md", "# Test\n").unwrap();
        let _ = Command::new("git").args(["add", "README.md"]).output();
        let _ = Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .output();

        // Create and checkout staging branch
        let _ = Command::new("git")
            .args(["checkout", "-b", "staging"])
            .output();

        // Create config with branch mapping
        fs::create_dir_all(".controlpath").unwrap();
        fs::write(
            ".controlpath/config.yaml",
            r"branchEnvironments:
  staging: staging
  main: production
defaultEnv: production
",
        )
        .unwrap();

        let result = determine_environment().unwrap();
        assert_eq!(result, Some("staging".to_string()));
    }

    #[test]
    #[serial]
    fn test_determine_environment_from_default_env() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        fs::create_dir_all(".controlpath").unwrap();
        fs::write(".controlpath/config.yaml", "defaultEnv: staging\n").unwrap();

        let result = determine_environment().unwrap();
        assert_eq!(result, Some("staging".to_string()));
    }

    #[test]
    #[serial]
    fn test_determine_environment_no_config() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let _guard = DirGuard::new(temp_path).unwrap();

        let result = determine_environment().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_git_branch_no_git() {
        // This test may or may not have git available
        // Just verify the function doesn't panic
        let _ = get_git_branch();
    }
}
