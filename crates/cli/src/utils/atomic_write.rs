//! Atomic file write utilities.

use crate::error::{CliError, CliResult};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path_for(path: &Path) -> CliResult<PathBuf> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        CliError::Message(format!(
            "Cannot write to path '{}' without a file name",
            path.display()
        ))
    })?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| CliError::Message(format!("System clock error: {e}")))?
        .as_nanos();
    let pid = std::process::id();
    Ok(parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name.to_string_lossy(),
        pid,
        nanos
    )))
}

/// Atomically write bytes to a file path.
///
/// Writes to a temporary file in the same directory, fsyncs it,
/// then renames it over the target file.
pub fn atomic_write(path: &Path, contents: &[u8]) -> CliResult<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    let temp_path = temp_path_for(path)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)?;

    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);

    if let Err(e) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(CliError::Io(e));
    }

    // Best effort: sync directory metadata after rename.
    if let Ok(dir_handle) = OpenOptions::new().read(true).open(parent) {
        let _ = dir_handle.sync_all();
    }

    Ok(())
}

/// Atomically write UTF-8 text to a file path.
pub fn atomic_write_string(path: &Path, contents: &str) -> CliResult<()> {
    atomic_write(path, contents.as_bytes())
}
