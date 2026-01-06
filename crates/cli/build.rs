use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the Cargo.toml version as fallback
    let cargo_version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let mut version = cargo_version.clone();

    // Try to read version from VERSION file in repo root
    // This file is maintained by release-please
    let repo_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent() // crates/
        .and_then(|p| p.parent()) // repo root
        .map(|p| p.join("VERSION"));

    if let Some(version_path) = repo_root {
        if version_path.exists() {
            if let Ok(file_version) = fs::read_to_string(&version_path) {
                let file_version = file_version.trim();
                // Validate it looks like a version
                if !file_version.is_empty()
                    && file_version.chars().next().unwrap_or(' ').is_ascii_digit()
                {
                    version = file_version.to_string();
                    println!("cargo:rerun-if-changed={}", version_path.display());
                }
            }
        }
    }

    // Fallback: try .release-please-manifest.json
    if version == cargo_version {
        let manifest_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .parent() // crates/
            .and_then(|p| p.parent()) // repo root
            .map(|p| p.join(".release-please-manifest.json"));

        if let Some(manifest_path) = manifest_path {
            if manifest_path.exists() {
                if let Ok(contents) = fs::read_to_string(&manifest_path) {
                    // Parse JSON: look for ".": "version" pattern
                    // The manifest has format: { ".": "0.6.0" }
                    // Find the pattern: ".": "
                    if let Some(pattern_start) = contents.find(r#"".": ""#) {
                        // Skip past the pattern to the version value
                        let value_start = pattern_start + r#"".": ""#.len();
                        // Find the closing quote
                        if let Some(value_end) = contents[value_start..].find('"') {
                            let manifest_version =
                                contents[value_start..value_start + value_end].trim();
                            // Basic validation: should start with a digit
                            if !manifest_version.is_empty()
                                && manifest_version
                                    .chars()
                                    .next()
                                    .unwrap_or(' ')
                                    .is_ascii_digit()
                            {
                                version = manifest_version.to_string();
                                println!("cargo:rerun-if-changed={}", manifest_path.display());
                            }
                        }
                    }
                }
            }
        }
    }

    // Always set CONTROLPATH_VERSION (either from file or Cargo.toml)
    println!("cargo:rustc-env=CONTROLPATH_VERSION={}", version);

    if version != cargo_version {
        println!(
            "cargo:warning=Using version {} from VERSION file or manifest (Cargo.toml has {})",
            version, cargo_version
        );
    }
}
