//! Audio playback using rodio
//!
//! Uses a dedicated audio thread to handle playback since rodio's
//! OutputStream is not Send+Sync.

use crate::error::TtsError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Commands for the audio player thread
enum PlayerCommand {
    Play(Vec<u8>),
    Stop,
    Shutdown,
}

/// Audio player handle for TTS output
#[derive(Clone)]
pub struct AudioPlayer {
    command_tx: mpsc::UnboundedSender<PlayerCommand>,
    is_playing: Arc<AtomicBool>,
}

impl AudioPlayer {
    /// Create a new audio player
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let is_playing = Arc::new(AtomicBool::new(false));
        let is_playing_clone = is_playing.clone();

        // Spawn audio thread
        std::thread::spawn(move || {
            audio_thread(command_rx, is_playing_clone);
        });

        Self {
            command_tx,
            is_playing,
        }
    }

    /// Play MP3 audio data
    pub fn play_mp3(&self, data: &[u8]) -> Result<(), TtsError> {
        self.command_tx
            .send(PlayerCommand::Play(data.to_vec()))
            .map_err(|_| TtsError::PlaybackError("Audio thread not running".to_string()))?;
        Ok(())
    }

    /// Stop current playback
    pub fn stop(&self) {
        let _ = self.command_tx.send(PlayerCommand::Stop);
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::SeqCst)
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        let _ = self.command_tx.send(PlayerCommand::Shutdown);
    }
}

/// Audio thread function
fn audio_thread(
    mut command_rx: mpsc::UnboundedReceiver<PlayerCommand>,
    is_playing: Arc<AtomicBool>,
) {
    use rodio::{Decoder, OutputStream, Sink};
    use std::io::Cursor;

    // Create audio output (must stay in this thread)
    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(output) => output,
        Err(e) => {
            tracing::error!("Failed to create audio output: {}", e);
            return;
        }
    };

    let mut current_sink: Option<Sink> = None;

    // Process commands
    while let Some(cmd) = command_rx.blocking_recv() {
        match cmd {
            PlayerCommand::Play(data) => {
                // Stop any current playback
                if let Some(sink) = current_sink.take() {
                    sink.stop();
                }
                is_playing.store(false, Ordering::SeqCst);

                // Create new sink
                let sink = match Sink::try_new(&stream_handle) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to create sink: {}", e);
                        continue;
                    }
                };

                // Decode MP3
                let cursor = Cursor::new(data);
                match Decoder::new(cursor) {
                    Ok(source) => {
                        sink.append(source);
                        is_playing.store(true, Ordering::SeqCst);

                        // Track playing state
                        let is_playing_clone = is_playing.clone();
                        current_sink = Some(sink);

                        if let Some(current) = &current_sink {
                            // Non-blocking check
                            while !current.empty() {
                                std::thread::sleep(std::time::Duration::from_millis(50));
                                // Check for new commands without blocking
                                if let Ok(new_cmd) = command_rx.try_recv() {
                                    match new_cmd {
                                        PlayerCommand::Stop => {
                                            current.stop();
                                            is_playing_clone.store(false, Ordering::SeqCst);
                                            break;
                                        }
                                        PlayerCommand::Play(_new_data) => {
                                            // Stop current playback - new data will be played on next iteration
                                            current.stop();
                                            is_playing_clone.store(false, Ordering::SeqCst);
                                            break;
                                        }
                                        PlayerCommand::Shutdown => {
                                            current.stop();
                                            return;
                                        }
                                    }
                                }
                            }
                            is_playing_clone.store(false, Ordering::SeqCst);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to decode MP3: {}", e);
                    }
                }
            }
            PlayerCommand::Stop => {
                if let Some(sink) = current_sink.take() {
                    sink.stop();
                }
                is_playing.store(false, Ordering::SeqCst);
            }
            PlayerCommand::Shutdown => {
                if let Some(sink) = current_sink.take() {
                    sink.stop();
                }
                return;
            }
        }
    }
}
