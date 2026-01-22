//! TTS error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TtsError {
    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Audio playback error: {0}")]
    PlaybackError(String),

    #[error("Voice not found: {0}")]
    VoiceNotFound(String),

    #[error("No audio data received")]
    NoAudioData,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
