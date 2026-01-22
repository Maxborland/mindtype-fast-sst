//! Transcription management

use crate::config::AppConfig;
use crate::error::CoreError;
use chrono::{DateTime, Utc};
use mindtype_whisper::{Transcription, WhisperTranscriber, Accelerator};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// Result of a transcription operation
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub id: Uuid,
    pub text: String,
    pub language: String,
    pub confidence: f32,
    pub duration_ms: u64,
    pub audio_duration_secs: f32,
    pub created_at: DateTime<Utc>,
}

/// Manages transcription history
pub struct TranscriptionManager {
    transcriber: Option<WhisperTranscriber>,
    history: Arc<RwLock<Vec<TranscriptionResult>>>,
    max_history: usize,
}

impl TranscriptionManager {
    /// Create new manager without transcriber (lazy load)
    pub fn new() -> Self {
        Self {
            transcriber: None,
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
        }
    }

    /// Initialize the transcriber with config
    pub fn init(&mut self, config: &AppConfig) -> Result<(), CoreError> {
        let accelerator = match config.accelerator {
            crate::config::AcceleratorPref::Auto => Accelerator::Auto,
            crate::config::AcceleratorPref::DirectML => Accelerator::DirectML,
            crate::config::AcceleratorPref::Cuda => Accelerator::Cuda,
            crate::config::AcceleratorPref::Cpu => Accelerator::Cpu,
        };

        let transcriber = WhisperTranscriber::new(
            &config.models_dir,
            config.model_size,
            accelerator,
        )?;

        self.transcriber = Some(transcriber);
        info!("Transcriber initialized with model: {}", config.model_size);

        Ok(())
    }

    /// Check if transcriber is initialized
    pub fn is_initialized(&self) -> bool {
        self.transcriber.is_some()
    }

    /// Transcribe audio samples
    pub async fn transcribe(
        &mut self,
        samples: &[f32],
        language: &str,
    ) -> Result<TranscriptionResult, CoreError> {
        let transcriber = self
            .transcriber
            .as_mut()
            .ok_or_else(|| CoreError::TranscriptionError("Transcriber not initialized".into()))?;

        let audio_duration_secs = samples.len() as f32 / 16000.0;
        debug!(
            "Transcribing {} samples ({:.2}s)",
            samples.len(),
            audio_duration_secs
        );

        let transcription = transcriber
            .transcribe(samples, language)
            .await?;

        let result = TranscriptionResult {
            id: Uuid::new_v4(),
            text: transcription.text,
            language: transcription.language,
            confidence: transcription.confidence,
            duration_ms: transcription.duration_ms,
            audio_duration_secs,
            created_at: Utc::now(),
        };

        // Add to history
        let mut history = self.history.write().await;
        history.insert(0, result.clone());
        if history.len() > self.max_history {
            history.truncate(self.max_history);
        }

        info!(
            "Transcription complete: {} chars in {}ms",
            result.text.len(),
            result.duration_ms
        );

        Ok(result)
    }

    /// Get transcription history
    pub async fn history(&self) -> Vec<TranscriptionResult> {
        self.history.read().await.clone()
    }

    /// Clear history
    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }
}

impl Default for TranscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}
