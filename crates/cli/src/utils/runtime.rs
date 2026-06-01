//! Runtime CLI behavior flags shared across commands.

use crate::error::{CliError, CliResult};
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub struct RuntimeOptions {
    pub json_output: bool,
    pub non_interactive: bool,
    pub verbose: u8,
    pub quiet: bool,
}

static RUNTIME_OPTIONS: OnceLock<RuntimeOptions> = OnceLock::new();

pub fn init_runtime_options(options: RuntimeOptions) -> CliResult<()> {
    RUNTIME_OPTIONS.set(options).map_err(|_| {
        CliError::Message("Runtime options were already initialized for this process".to_string())
    })
}

fn current() -> RuntimeOptions {
    RUNTIME_OPTIONS.get().cloned().unwrap_or(RuntimeOptions {
        json_output: false,
        non_interactive: false,
        verbose: 0,
        quiet: false,
    })
}

pub fn is_json_output() -> bool {
    current().json_output
}

pub fn is_non_interactive() -> bool {
    current().non_interactive
}

#[allow(dead_code)]
pub fn is_quiet() -> bool {
    current().quiet
}

#[allow(dead_code)]
pub fn verbose_level() -> u8 {
    current().verbose
}

pub fn require_interactive(action: &str) -> CliResult<()> {
    if is_non_interactive() {
        return Err(CliError::Message(format!(
            "Cannot {action} in non-interactive mode. Pass required flags or remove --non-interactive."
        )));
    }
    Ok(())
}
