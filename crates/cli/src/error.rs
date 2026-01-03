//! CLI error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Compiler error: {0}")]
    Compiler(#[from] controlpath_compiler::CompilerError),

    #[error("{0}")]
    Message(String),
}

pub type CliResult<T> = Result<T, CliError>;
