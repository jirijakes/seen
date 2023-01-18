use miette::Diagnostic;
use thiserror::Error;

pub mod md;

/// Errors that happened during format conversions.
#[derive(Debug, Error, Diagnostic)]
pub enum ConvertError {
    #[error(
        "Command '{0}' not found. For proper functionality, it has to be installed on the system."
    )]
    CommandNotFound(String),

    #[error("Command resulted in an error.")]
    CommandError(#[from] std::io::Error),

    #[error("Command output produced an error.")]
    CommandOutput(Box<dyn std::error::Error>),
}
