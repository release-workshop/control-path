//! SaaS environment naming rules for `.controlpath/<env>.ast` and CDN path segments.

use std::path::Path;

/// Returns whether `name` is safe for SaaS AST cache files and CDN URL path segments.
#[must_use]
pub fn is_valid_saas_environment_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
}

/// Valid environment name from a `.controlpath/<env>.ast` file path.
///
/// Uses [`Path::file_stem`] (not suffix stripping on the full filename) so odd names
/// like `..ast` resolve to `..` and are rejected rather than `.`.
#[must_use]
pub fn environment_from_ast_path(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("ast") {
        return None;
    }
    let stem = path.file_stem().and_then(|s| s.to_str())?;
    if !is_valid_saas_environment_name(stem) {
        return None;
    }
    Some(stem.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn rejects_dot_and_dotdot_stems() {
        assert!(!is_valid_saas_environment_name("."));
        assert!(!is_valid_saas_environment_name(".."));
    }

    #[test]
    fn environment_from_ast_path_rejects_dotdot_stem() {
        assert!(environment_from_ast_path(Path::new(".controlpath/..ast")).is_none());
    }

    #[test]
    fn environment_from_ast_path_accepts_production() {
        use std::fs;

        let dir =
            std::env::temp_dir().join(format!("controlpath-saas-env-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("production.ast");
        fs::write(&path, b"x").unwrap();
        assert_eq!(
            environment_from_ast_path(&path),
            Some("production".to_string())
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
