//! Whisper error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WhisperError {
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Failed to load model: {0}")]
    ModelLoadError(String),

    #[error("ONNX Runtime error: {0}")]
    OrtError(#[from] ort::Error),

    #[error("Invalid audio format: {0}")]
    InvalidAudio(String),

    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
