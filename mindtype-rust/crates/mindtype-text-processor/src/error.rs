//! Text processor error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessorError {
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("Invalid language: {0}")]
    InvalidLanguage(String),
}
