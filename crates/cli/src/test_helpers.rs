//! Test helpers for unit tests
//!
//! This module provides shared utilities for unit tests within the CLI crate.
//! For integration tests, see `tests/integration_test_helpers.rs`.

#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::Mutex;

/// Serializes all tests that change the process working directory.
#[cfg(test)]
static CWD_TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Guard for changing the current working directory in tests.
/// Automatically restores the original directory when dropped.
///
/// This is useful for tests that need to run in a temporary directory
/// but want to ensure cleanup happens even if the test panics.
///
/// # Example
///
/// ```rust,no_run
/// use tempfile::TempDir;
/// use crate::test_helpers::DirGuard;
///
/// let temp_dir = TempDir::new().unwrap();
/// let _guard = DirGuard::new(temp_dir.path()).unwrap();
/// // Now we're in temp_dir, and will be restored when _guard drops
/// ```
#[cfg(test)]
pub struct DirGuard {
    original_dir: PathBuf,
    _lock: std::sync::MutexGuard<'static, ()>,
}

#[cfg(test)]
impl DirGuard {
    /// Create a new DirGuard and change to the specified directory.
    ///
    /// Holds a process-wide lock so parallel tests cannot race on `set_current_dir`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The directory doesn't exist and can't be created
    /// - The current directory can't be determined
    /// - The directory can't be changed to
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        // If a prior cwd test panicked while holding the lock, recover so the suite can finish.
        // Tradeoff: the panicking test may have left the process cwd wrong; this test then
        // captures that cwd as `original_dir` and restores to it on drop. Fix the panicking test;
        // do not rely on poison recovery to hide cwd bugs.
        let _lock = CWD_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let path = path.as_ref();
        fs::create_dir_all(path)?;
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(path)?;
        Ok(DirGuard {
            original_dir,
            _lock,
        })
    }
}

#[cfg(test)]
impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original_dir);
    }
}

/// Minimal v2 catalog YAML for unit tests.
#[cfg(test)]
pub fn v2_test_catalog(flag_name: &str, serve: bool) -> String {
    format!(
        r"catalog:
  id: test-service
mode: local
flags:
  {flag_name}:
    default: false
    kind: release
environments:
  production:
    rules:
      {flag_name}:
        - serve: {serve}
"
    )
}

/// Write a v2 catalog to `control-path.yaml` in the current directory.
#[cfg(test)]
pub fn write_v2_test_catalog(flag_name: &str, serve: bool) {
    fs::write("control-path.yaml", v2_test_catalog(flag_name, serve)).unwrap();
}
