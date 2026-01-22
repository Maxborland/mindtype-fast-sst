//! Core error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Audio error: {0}")]
    AudioError(String),

    #[error("Transcription error: {0}")]
    TranscriptionError(String),

    #[error("File processing error: {0}")]
    FileError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Platform error: {0}")]
    PlatformError(#[from] mindtype_platform::PlatformError),

    #[error("Whisper error: {0}")]
    WhisperError(#[from] mindtype_whisper::WhisperError),

    #[error("License error: {0}")]
    LicenseError(#[from] mindtype_licensing::LicenseError),

    #[error("LLM error: {0}")]
    LlmError(#[from] mindtype_llm::LlmError),
}
