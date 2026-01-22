//! Licensing error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LicenseError {
    #[error("Invalid license key format")]
    InvalidKeyFormat,

    #[error("License key not found")]
    KeyNotFound,

    #[error("License expired")]
    Expired,

    #[error("Device limit reached")]
    DeviceLimitReached,

    #[error("Trial expired")]
    TrialExpired,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Insufficient credits")]
    InsufficientCredits,

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
