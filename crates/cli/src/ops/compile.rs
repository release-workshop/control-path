//! Reusable compile operations

use crate::error::{CliError, CliResult};
use crate::utils::catalog;
use std::env;

/// Options for compiling environments
pub struct CompileOptions {
    /// Environment names to compile (if None, compiles all found)
    pub envs: Option<Vec<String>>,
    /// Skip validation before compilation
    pub skip_validation: bool,
}

/// Compile ASTs for one or more environments from the v2 catalog.
pub fn compile_envs(options: &CompileOptions) -> CliResult<Vec<String>> {
    let base_dir = env::current_dir()
        .map_err(|e| CliError::Message(format!("Failed to resolve working directory: {e}")))?;

    catalog::compile_catalog_envs(&base_dir, options.envs.clone(), options.skip_validation)
}
