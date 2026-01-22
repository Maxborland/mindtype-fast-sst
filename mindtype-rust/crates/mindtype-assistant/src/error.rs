//! Assistant error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AssistantError {
    #[error("LLM error: {0}")]
    LlmError(#[from] mindtype_llm::LlmError),

    #[error("TTS error: {0}")]
    TtsError(#[from] mindtype_tts::TtsError),

    #[error("Transcription error: {0}")]
    TranscriptionError(String),

    #[error("Assistant not ready")]
    NotReady,
}
