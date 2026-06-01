//! Disk-only SaaS environment discovery from `.controlpath/<env>.ast` files.

use std::fs;
use std::path::Path;

use controlpath_compiler::environment_from_ast_path;

use crate::error::{CliError, CliResult};

/// Discovers SaaS environments from compiled artifact files on disk.
#[derive(Debug, Clone, Copy, Default)]
pub struct FilesystemAstCache;

impl FilesystemAstCache {
    /// Environment names for every valid `.controlpath/<env>.ast` file.
    ///
    /// Returns an actionable error when `.controlpath` is missing or contains no valid `*.ast`
    /// files. Uses [`environment_from_ast_path`] so discovery matches sync write and prune rules.
    pub fn discover_environments(base_dir: &Path) -> CliResult<Vec<String>> {
        let cache_dir = base_dir.join(".controlpath");
        if !cache_dir.is_dir() {
            return Err(no_saas_sync_cache_error());
        }

        let environments = discover_environments_in_dir(&cache_dir)?;
        if environments.is_empty() {
            return Err(no_saas_sync_cache_error());
        }

        Ok(environments)
    }
}

/// Valid environment names from every `*.ast` file in `cache_dir` (sorted).
///
/// Returns an empty vec when no entries pass [`environment_from_ast_path`]. Does not require
/// `cache_dir` to exist; callers that need SDK-generate errors should use
/// [`FilesystemAstCache::discover_environments`].
pub fn discover_environments_in_dir(cache_dir: &Path) -> CliResult<Vec<String>> {
    let mut environments = Vec::new();
    for entry in fs::read_dir(cache_dir)
        .map_err(|e| CliError::Message(format!("Failed to read {}: {e}", cache_dir.display())))?
    {
        let entry = entry.map_err(|e| {
            CliError::Message(format!("Failed to read {} entry: {e}", cache_dir.display()))
        })?;
        if let Some(environment) = environment_from_ast_path(&entry.path()) {
            environments.push(environment);
        }
    }

    environments.sort();
    Ok(environments)
}

/// Actionable error when SaaS SDK generation has no on-disk compiled artifacts.
fn no_saas_sync_cache_error() -> CliError {
    CliError::Message(
        "SaaS mode: no compiled artifacts in .controlpath/*.ast. \
         Run `controlpath ci` (or sync with the SaaS client) before `generate-sdk`. \
         Remove stray *.ast files you did not intend to embed (sync prunes only on download)."
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn discover_environments_in_dir_returns_sorted_env_names() {
        let temp_dir = TempDir::new().unwrap();
        let cache = temp_dir.path().join(".controlpath");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("staging.ast"), b"staging").unwrap();
        fs::write(cache.join("production.ast"), b"production").unwrap();
        fs::write(cache.join("saas-fake-state.json"), b"{}").unwrap();

        let envs = discover_environments_in_dir(&cache).unwrap();
        assert_eq!(envs, vec!["production".to_string(), "staging".to_string()]);
    }

    #[test]
    fn discover_environments_returns_sorted_env_names_from_ast_files() {
        let temp_dir = TempDir::new().unwrap();
        let cache = temp_dir.path().join(".controlpath");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("staging.ast"), b"staging").unwrap();
        fs::write(cache.join("production.ast"), b"production").unwrap();
        fs::write(cache.join("saas-fake-state.json"), b"{}").unwrap();

        let envs = FilesystemAstCache::discover_environments(temp_dir.path()).unwrap();
        assert_eq!(envs, vec!["production".to_string(), "staging".to_string()]);
    }

    #[test]
    fn discover_environments_fails_when_no_ast_files() {
        let temp_dir = TempDir::new().unwrap();
        let err = FilesystemAstCache::discover_environments(temp_dir.path()).unwrap_err();
        assert!(err.to_string().contains("no compiled artifacts"));
    }

    #[test]
    fn discover_environments_ignores_invalid_environment_names() {
        let temp_dir = TempDir::new().unwrap();
        let cache = temp_dir.path().join(".controlpath");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("..ast"), b"x").unwrap();
        fs::write(cache.join(".ast"), b"x").unwrap();

        let err = FilesystemAstCache::discover_environments(temp_dir.path()).unwrap_err();
        assert!(err.to_string().contains("no compiled artifacts"));
    }
}
