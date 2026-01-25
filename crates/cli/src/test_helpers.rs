//! Test helpers for unit tests
//!
//! This module provides shared utilities for unit tests within the CLI crate.
//! For integration tests, see `tests/integration_test_helpers.rs`.

#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::{Path, PathBuf};

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
}

#[cfg(test)]
impl DirGuard {
    /// Create a new DirGuard and change to the specified directory.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The directory doesn't exist and can't be created
    /// - The current directory can't be determined
    /// - The directory can't be changed to
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let path = path.as_ref();
        fs::create_dir_all(path)?;
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(path)?;
        Ok(DirGuard { original_dir })
    }
}

#[cfg(test)]
impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original_dir);
    }
}
