//! Recording state machine and audio handling

use crate::state::{AppState, RecordingState, Transcription};
use anyhow::Result;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// Recording flow controller
///
/// State machine:
/// ```text
/// Idle -> [Press] -> Recording -> [Release] -> Transcribing -> [Done] -> Inserting -> Idle
/// ```
pub struct RecordingFlow {
    app: AppHandle,
    state: Arc<AppState>,
}

impl RecordingFlow {
    pub fn new(app: AppHandle, state: Arc<AppState>) -> Self {
        Self { app, state }
    }

    /// Handle hotkey press - start recording
    pub async fn on_hotkey_pressed(&self) -> Result<()> {
        let current_state = self.state.get_recording_state().await;

        if current_state != RecordingState::Idle {
            tracing::warn!("Cannot start recording: current state is {:?}", current_state);
            return Ok(());
        }

        tracing::info!("Starting recording...");

        // Save the currently focused window before we take focus
        self.state.platform.save_foreground_window();

        // Start recording
        let mut recorder = mindtype_core::AudioRecorder::new()?;
        recorder.select_device(None)?;
        recorder.start()?;
        *self.state.audio_recorder.write().await = Some(recorder);

        // Update state
        self.state.set_recording_state(RecordingState::Recording).await;

        // Emit events to frontend
        self.app.emit("recording-state-changed", RecordingState::Recording)?;
        self.app.emit("show-overlay", ())?;

        Ok(())
    }

    /// Handle hotkey release - stop recording and transcribe
    pub async fn on_hotkey_released(&self) -> Result<()> {
        let current_state = self.state.get_recording_state().await;

        if current_state != RecordingState::Recording {
            tracing::warn!("Cannot stop recording: current state is {:?}", current_state);
            return Ok(());
        }

        tracing::info!("Stopping recording, starting transcription...");

        // Update state
        self.state.set_recording_state(RecordingState::Transcribing).await;
        self.app.emit("recording-state-changed", RecordingState::Transcribing)?;

        // Stop recorder and get audio
        let audio_data = {
            let mut recorder_guard = self.state.audio_recorder.write().await;
            let mut recorder = recorder_guard.take()
                .ok_or_else(|| anyhow::anyhow!("No recorder active"))?;
            recorder.stop()?
        };

        // Check if we have enough audio
        if audio_data.len() < 1600 {  // Less than 0.1 seconds at 16kHz
            tracing::warn!("Audio too short, canceling transcription");
            self.state.set_recording_state(RecordingState::Idle).await;
            self.app.emit("recording-state-changed", RecordingState::Idle)?;
            self.app.emit("hide-overlay", ())?;
            return Ok(());
        }

        // Transcribe
        let transcription = self.transcribe(audio_data).await?;

        // Insert text
        self.insert_text(&transcription.text).await?;

        // Hide overlay
        self.app.emit("hide-overlay", ())?;

        Ok(())
    }

    /// Transcribe audio data
    async fn transcribe(&self, audio_data: Vec<f32>) -> Result<Transcription> {
        let mut transcriber_guard = self.state.transcriber.write().await;
        let transcriber = transcriber_guard.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Transcriber not initialized"))?;

        let start_time = std::time::Instant::now();
        let result = transcriber.transcribe(&audio_data, "auto").await?;
        let duration_ms = start_time.elapsed().as_millis() as u64;

        let transcription = Transcription {
            id: uuid::Uuid::new_v4().to_string(),
            text: result.text.clone(),
            language: result.language.clone(),
            duration_ms,
            timestamp: chrono::Utc::now(),
        };

        // Add to history
        self.state.add_transcription(transcription.clone()).await;

        // Emit transcription result
        self.app.emit("transcription-complete", &transcription)?;

        Ok(transcription)
    }

    /// Insert text at cursor position
    async fn insert_text(&self, text: &str) -> Result<()> {
        self.state.set_recording_state(RecordingState::Inserting).await;
        self.app.emit("recording-state-changed", RecordingState::Inserting)?;

        // Small delay to ensure UI updates
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Restore focus to original window
        self.state.platform.restore_foreground_window()?;

        // Small delay after focus restoration
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Insert text
        self.state.platform.insert_text(text)?;

        // Back to idle
        self.state.set_recording_state(RecordingState::Idle).await;
        self.app.emit("recording-state-changed", RecordingState::Idle)?;

        Ok(())
    }

    /// Cancel current recording
    pub async fn cancel(&self) -> Result<()> {
        let current_state = self.state.get_recording_state().await;

        if current_state == RecordingState::Recording {
            // Stop recorder without transcribing
            let mut recorder_guard = self.state.audio_recorder.write().await;
            if let Some(mut recorder) = recorder_guard.take() {
                let _ = recorder.stop();
            }
        }

        self.state.set_recording_state(RecordingState::Idle).await;
        self.app.emit("recording-state-changed", RecordingState::Idle)?;
        self.app.emit("hide-overlay", ())?;

        Ok(())
    }
}
