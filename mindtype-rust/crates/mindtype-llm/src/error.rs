//! LLM error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("Provider not configured")]
    NotConfigured,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Model not available: {0}")]
    ModelNotAvailable(String),

    #[error("Context too long: {0}")]
    ContextTooLong(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}
