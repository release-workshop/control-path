//! Reusable compile operations

use crate::error::CliResult;
use crate::utils::catalog;
use std::env;

/// Options for compiling AST artifacts
pub struct CompileOptions {
    /// Environment names to compile (if None, compiles all)
    pub envs: Option<Vec<String>>,
}

/// Compile AST artifacts for environments
pub fn compile_envs(options: &CompileOptions) -> CliResult<Vec<String>> {
    let base_dir = env::current_dir().map_err(|e| {
        crate::error::CliError::Message(format!("Failed to resolve working directory: {e}"))
    })?;
    catalog::compile_catalog_envs(&base_dir, options.envs.clone())
}
