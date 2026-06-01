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

/// Resolve environment from branch name and config defaults.
///
/// Priority:
/// 1. Branch mapping when `branch` matches `branchEnvironments`
/// 2. `defaultEnv`
pub fn resolve_environment_from_branch(
    branch: Option<&str>,
    branch_envs: &Option<std::collections::HashMap<String, String>>,
    default_env: &Option<String>,
) -> Option<String> {
    if let Some(branch) = branch {
        if let Some(branch_envs) = branch_envs {
            if let Some(env) = branch_envs.get(branch) {
                return Some(env.clone());
            }
        }
    }
    default_env.clone()
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
    let (branch_envs, default_env) = config::read_environment_defaults()?;
    Ok(resolve_environment_from_branch(
        get_git_branch().as_deref(),
        &branch_envs,
        &default_env,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    use crate::test_helpers::DirGuard;

    #[test]
    fn resolve_environment_from_branch_mapping_takes_precedence_over_default() {
        let branch_envs = Some(HashMap::from([
            ("staging".to_string(), "staging".to_string()),
            ("main".to_string(), "production".to_string()),
        ]));
        let default_env = Some("production".to_string());

        assert_eq!(
            resolve_environment_from_branch(Some("staging"), &branch_envs, &default_env,),
            Some("staging".to_string())
        );
        assert_eq!(
            resolve_environment_from_branch(Some("main"), &branch_envs, &default_env),
            Some("production".to_string())
        );
    }

    #[test]
    fn resolve_environment_from_branch_falls_back_to_default_env() {
        let branch_envs = Some(HashMap::from([(
            "main".to_string(),
            "production".to_string(),
        )]));
        let default_env = Some("staging".to_string());

        assert_eq!(
            resolve_environment_from_branch(Some("feature-x"), &branch_envs, &default_env),
            Some("staging".to_string())
        );
        assert_eq!(
            resolve_environment_from_branch(None, &branch_envs, &default_env),
            Some("staging".to_string())
        );
    }

    #[test]
    #[serial]
    fn test_determine_environment_reads_config_from_disk() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = DirGuard::new(temp_dir.path()).unwrap();

        // defaultEnv only (no branchEnvironments): result must not depend on git branch.
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
