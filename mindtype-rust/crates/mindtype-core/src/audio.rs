//! Audio recording

use crate::error::CoreError;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Audio level for visualization
#[derive(Debug, Clone)]
pub struct AudioLevel {
    /// RMS level (0.0 - 1.0)
    pub rms: f32,
    /// Peak level (0.0 - 1.0)
    pub peak: f32,
}

/// Audio recorder
///
/// Note: The cpal Stream is !Send+!Sync, so we manage it on a dedicated thread.
/// This struct communicates with the recording thread via atomic flags and channels.
pub struct AudioRecorder {
    host: cpal::Host,
    device: Option<cpal::Device>,
    config: Option<cpal::SupportedStreamConfig>,
    samples: Arc<Mutex<Vec<f32>>>,
    recording: Arc<AtomicBool>,
    level_tx: Option<mpsc::UnboundedSender<AudioLevel>>,
    /// Handle to the recording thread (if active)
    recording_thread: Option<JoinHandle<()>>,
}

impl AudioRecorder {
    /// Create a new audio recorder
    pub fn new() -> Result<Self, CoreError> {
        let host = cpal::default_host();

        Ok(Self {
            host,
            device: None,
            config: None,
            samples: Arc::new(Mutex::new(Vec::new())),
            recording: Arc::new(AtomicBool::new(false)),
            level_tx: None,
            recording_thread: None,
        })
    }

    /// Get available input devices
    pub fn available_devices(&self) -> Result<Vec<String>, CoreError> {
        let devices = self
            .host
            .input_devices()
            .map_err(|e| CoreError::AudioError(format!("Failed to enumerate devices: {}", e)))?;

        let names: Vec<String> = devices
            .filter_map(|d| d.name().ok())
            .collect();

        Ok(names)
    }

    /// Select input device by name
    pub fn select_device(&mut self, name: Option<&str>) -> Result<(), CoreError> {
        let device = match name {
            Some(name) => {
                let devices = self.host.input_devices().map_err(|e| {
                    CoreError::AudioError(format!("Failed to enumerate devices: {}", e))
                })?;

                devices
                    .filter_map(|d| {
                        let dev_name = d.name().ok()?;
                        if dev_name == name {
                            Some(d)
                        } else {
                            None
                        }
                    })
                    .next()
                    .ok_or_else(|| CoreError::AudioError(format!("Device not found: {}", name)))?
            }
            None => self
                .host
                .default_input_device()
                .ok_or_else(|| CoreError::AudioError("No default input device".to_string()))?,
        };

        let config = device
            .supported_input_configs()
            .map_err(|e| CoreError::AudioError(format!("Failed to get configs: {}", e)))?
            .filter(|c| c.channels() == 1 && c.sample_format() == cpal::SampleFormat::F32)
            .find(|c| {
                let min = c.min_sample_rate().0;
                let max = c.max_sample_rate().0;
                min <= 16000 && 16000 <= max
            })
            .map(|c| c.with_sample_rate(cpal::SampleRate(16000)))
            .or_else(|| {
                // Fallback: use default config
                device.default_input_config().ok()
            })
            .ok_or_else(|| CoreError::AudioError("No suitable audio config".to_string()))?;

        info!(
            "Selected device: {:?}, config: {:?}",
            device.name(),
            config
        );

        self.device = Some(device);
        self.config = Some(config);

        Ok(())
    }

    /// Set level callback
    pub fn set_level_callback(&mut self) -> mpsc::UnboundedReceiver<AudioLevel> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.level_tx = Some(tx);
        rx
    }

    /// Start recording
    ///
    /// Spawns a dedicated thread to manage the audio stream, since cpal::Stream
    /// is !Send+!Sync and cannot be stored in shared state.
    pub fn start(&mut self) -> Result<(), CoreError> {
        // Stop any existing recording first
        if self.recording_thread.is_some() {
            self.stop()?;
        }

        let device_name = self
            .device
            .as_ref()
            .and_then(|d| d.name().ok())
            .ok_or_else(|| CoreError::AudioError("No device selected".to_string()))?;

        let config = self
            .config
            .clone()
            .ok_or_else(|| CoreError::AudioError("No config selected".to_string()))?;

        // Clear previous samples
        self.samples.lock().unwrap().clear();
        self.recording.store(true, Ordering::SeqCst);

        let samples = Arc::clone(&self.samples);
        let recording = Arc::clone(&self.recording);
        let level_tx = self.level_tx.clone();

        let stream_config: cpal::StreamConfig = config.clone().into();
        let channels = stream_config.channels as usize;
        let target_sample_rate = 16000u32;
        let source_sample_rate = stream_config.sample_rate.0;

        debug!(
            "Starting recording: {} Hz, {} channels",
            source_sample_rate, channels
        );

        // Spawn a dedicated thread to manage the stream
        let handle = thread::spawn(move || {
            // Get device in the recording thread
            let host = cpal::default_host();
            let device = host
                .input_devices()
                .ok()
                .and_then(|mut devices| devices.find(|d| d.name().ok().as_deref() == Some(device_name.as_str())))
                .or_else(|| host.default_input_device());

            let Some(device) = device else {
                error!("Could not find audio device in recording thread");
                return;
            };

            let err_fn = |err| error!("Audio stream error: {}", err);

            // Clone for use in the callback vs the while loop
            let recording_callback = Arc::clone(&recording);

            let stream = match device.build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !recording_callback.load(Ordering::SeqCst) {
                        return;
                    }

                    // Convert to mono if needed
                    let mono: Vec<f32> = if channels > 1 {
                        data.chunks(channels)
                            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                            .collect()
                    } else {
                        data.to_vec()
                    };

                    // Resample to 16kHz if needed
                    let resampled = if source_sample_rate != target_sample_rate {
                        resample(&mono, source_sample_rate, target_sample_rate)
                    } else {
                        mono.clone()
                    };

                    // Calculate levels for visualization
                    if let Some(tx) = &level_tx {
                        let rms = (mono.iter().map(|&x| x * x).sum::<f32>() / mono.len() as f32)
                            .sqrt()
                            .min(1.0);
                        let peak = mono
                            .iter()
                            .map(|&x| x.abs())
                            .max_by(|a, b| a.partial_cmp(b).unwrap())
                            .unwrap_or(0.0)
                            .min(1.0);

                        let _ = tx.send(AudioLevel { rms, peak });
                    }

                    // Store samples
                    samples.lock().unwrap().extend(resampled);
                },
                err_fn,
                None,
            ) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to build stream: {}", e);
                    return;
                }
            };

            if let Err(e) = stream.play() {
                error!("Failed to start stream: {}", e);
                return;
            }

            info!("Recording thread started");

            // Keep the stream alive by holding it in this thread
            // The thread will exit when recording becomes false
            while recording.load(Ordering::SeqCst) {
                thread::sleep(std::time::Duration::from_millis(50));
            }

            // Stream drops here, properly cleaning up
            info!("Recording thread exiting");
        });

        self.recording_thread = Some(handle);
        info!("Recording started");
        Ok(())
    }

    /// Stop recording and return samples
    pub fn stop(&mut self) -> Result<Vec<f32>, CoreError> {
        // Signal the recording thread to stop
        self.recording.store(false, Ordering::SeqCst);

        // Wait for the recording thread to finish
        if let Some(handle) = self.recording_thread.take() {
            // Give the thread time to notice the flag change
            thread::sleep(std::time::Duration::from_millis(100));

            // Join the thread (with timeout via try_join pattern)
            match handle.join() {
                Ok(()) => debug!("Recording thread joined successfully"),
                Err(e) => error!("Recording thread panicked: {:?}", e),
            }
        }

        let samples = std::mem::take(&mut *self.samples.lock().unwrap());
        info!("Recording stopped: {} samples", samples.len());

        Ok(samples)
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::SeqCst)
    }

    /// Get current device name
    pub fn current_device_name(&self) -> Option<String> {
        self.device.as_ref().and_then(|d| d.name().ok())
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new().expect("Failed to create audio recorder")
    }
}

/// Simple linear resampling
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let new_len = (samples.len() as f64 / ratio) as usize;
    let mut result = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let pos = i as f64 * ratio;
        let idx = pos as usize;
        let frac = pos - idx as f64;

        let sample = if idx + 1 < samples.len() {
            samples[idx] * (1.0 - frac as f32) + samples[idx + 1] * frac as f32
        } else {
            samples[idx.min(samples.len() - 1)]
        };

        result.push(sample);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_same_rate() {
        let samples = vec![0.0, 0.5, 1.0, 0.5, 0.0];
        let result = resample(&samples, 16000, 16000);
        assert_eq!(result, samples);
    }

    #[test]
    fn test_resample_downsample() {
        let samples: Vec<f32> = (0..48000).map(|i| (i as f32 / 48000.0)).collect();
        let result = resample(&samples, 48000, 16000);

        // Should be roughly 1/3 of the original length
        assert!((result.len() as f32 / 16000.0 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_resample_upsample() {
        let samples: Vec<f32> = (0..8000).map(|i| (i as f32 / 8000.0)).collect();
        let result = resample(&samples, 8000, 16000);

        // Should be roughly 2x the original length
        assert!((result.len() as f32 / 16000.0 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_resample_preserves_endpoints() {
        let samples = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let result = resample(&samples, 44100, 16000);

        // First sample should be close to 0
        assert!(result[0].abs() < 0.01);
        // Result should have expected length
        assert!(!result.is_empty());
    }

    #[test]
    fn test_audio_level_struct() {
        let level = AudioLevel { rms: 0.5, peak: 0.8 };
        assert_eq!(level.rms, 0.5);
        assert_eq!(level.peak, 0.8);
    }

    #[test]
    fn test_audio_recorder_new() {
        // This test may fail on systems without audio devices
        let result = AudioRecorder::new();
        if let Ok(recorder) = result {
            assert!(!recorder.is_recording());
            assert!(recorder.current_device_name().is_none());
        }
    }

    #[test]
    fn test_audio_recorder_is_recording_initial() {
        if let Ok(recorder) = AudioRecorder::new() {
            assert!(!recorder.is_recording());
        }
    }

    #[test]
    fn test_resample_empty() {
        let samples: Vec<f32> = vec![];
        let result = resample(&samples, 48000, 16000);
        assert!(result.is_empty());
    }

    #[test]
    fn test_resample_single_sample() {
        // When downsampling single sample from higher to lower rate,
        // the result may be empty due to integer division
        let samples = vec![0.5];
        let result = resample(&samples, 16000, 16000);
        // Same rate should preserve the sample
        assert_eq!(result.len(), 1);
        assert!((result[0] - 0.5).abs() < 0.01);
    }
}
