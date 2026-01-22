//! File processing for batch transcription

use crate::error::CoreError;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, error};
use uuid::Uuid;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Status of a file processing job
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    /// Waiting in queue
    Pending,
    /// Extracting audio from video
    ExtractingAudio,
    /// Transcribing audio
    Transcribing,
    /// Generating AI summary
    Summarizing,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed(String),
}

impl std::fmt::Display for FileStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileStatus::Pending => write!(f, "Waiting..."),
            FileStatus::ExtractingAudio => write!(f, "Extracting audio..."),
            FileStatus::Transcribing => write!(f, "Transcribing..."),
            FileStatus::Summarizing => write!(f, "Generating summary..."),
            FileStatus::Completed => write!(f, "Done"),
            FileStatus::Failed(e) => write!(f, "Failed: {}", e),
        }
    }
}

/// A file processing job
#[derive(Debug, Clone)]
pub struct FileJob {
    pub id: Uuid,
    pub path: PathBuf,
    pub filename: String,
    pub status: FileStatus,
    pub progress: u8,
    pub transcription: Option<String>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl FileJob {
    /// Create a new job
    pub fn new(path: PathBuf) -> Self {
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        Self {
            id: Uuid::new_v4(),
            path,
            filename,
            status: FileStatus::Pending,
            progress: 0,
            transcription: None,
            summary: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    /// Check if file type is supported
    pub fn is_supported(path: &std::path::Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        matches!(
            ext.as_deref(),
            Some("mp3") | Some("wav") | Some("m4a") | Some("flac") | Some("ogg")
                | Some("mp4") | Some("mkv") | Some("avi") | Some("mov") | Some("webm")
        )
    }

    /// Check if this is a video file (needs audio extraction)
    pub fn is_video(&self) -> bool {
        let ext = self
            .path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        matches!(
            ext.as_deref(),
            Some("mp4") | Some("mkv") | Some("avi") | Some("mov") | Some("webm")
        )
    }
}

/// File job update event
#[derive(Debug, Clone)]
pub struct FileJobUpdate {
    pub id: Uuid,
    pub status: FileStatus,
    pub progress: u8,
}

/// File processor manages batch transcription jobs
pub struct FileProcessor {
    jobs: Arc<RwLock<HashMap<Uuid, FileJob>>>,
    job_order: Arc<RwLock<Vec<Uuid>>>,
    update_tx: Option<mpsc::UnboundedSender<FileJobUpdate>>,
    processing: Arc<RwLock<bool>>,
}

impl FileProcessor {
    /// Create a new file processor
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            job_order: Arc::new(RwLock::new(Vec::new())),
            update_tx: None,
            processing: Arc::new(RwLock::new(false)),
        }
    }

    /// Set update callback
    pub fn set_update_callback(&mut self) -> mpsc::UnboundedReceiver<FileJobUpdate> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.update_tx = Some(tx);
        rx
    }

    /// Add files to the queue
    pub async fn add_files(&self, paths: Vec<PathBuf>) -> Vec<Uuid> {
        let mut jobs = self.jobs.write().await;
        let mut order = self.job_order.write().await;
        let mut ids = Vec::new();

        for path in paths {
            if !FileJob::is_supported(&path) {
                warn!("Unsupported file type: {:?}", path);
                continue;
            }

            let job = FileJob::new(path);
            let id = job.id;
            jobs.insert(id, job);
            order.push(id);
            ids.push(id);

            info!("Added file to queue: {}", id);
        }

        ids
    }

    /// Get all jobs in order
    pub async fn get_jobs(&self) -> Vec<FileJob> {
        let jobs = self.jobs.read().await;
        let order = self.job_order.read().await;

        order
            .iter()
            .filter_map(|id| jobs.get(id).cloned())
            .collect()
    }

    /// Get a specific job
    pub async fn get_job(&self, id: Uuid) -> Option<FileJob> {
        self.jobs.read().await.get(&id).cloned()
    }

    /// Remove a job
    pub async fn remove_job(&self, id: Uuid) {
        self.jobs.write().await.remove(&id);
        self.job_order.write().await.retain(|&x| x != id);
    }

    /// Clear all jobs
    pub async fn clear(&self) {
        self.jobs.write().await.clear();
        self.job_order.write().await.clear();
    }

    /// Update job status
    async fn update_job(&self, id: Uuid, status: FileStatus, progress: u8) {
        if let Some(job) = self.jobs.write().await.get_mut(&id) {
            job.status = status.clone();
            job.progress = progress;

            if matches!(status, FileStatus::Completed | FileStatus::Failed(_)) {
                job.completed_at = Some(Utc::now());
            }
        }

        if let Some(tx) = &self.update_tx {
            let _ = tx.send(FileJobUpdate {
                id,
                status,
                progress,
            });
        }
    }

    /// Set job transcription result
    async fn set_transcription(&self, id: Uuid, text: String) {
        if let Some(job) = self.jobs.write().await.get_mut(&id) {
            job.transcription = Some(text);
        }
    }

    /// Set job summary result
    pub async fn update_summary(&self, id: Uuid, text: String) {
        if let Some(job) = self.jobs.write().await.get_mut(&id) {
            job.summary = Some(text);
        }
    }

    /// Load audio from file, returning samples at 16kHz mono (Whisper format)
    fn load_audio_file(path: &Path) -> Result<Vec<f32>, CoreError> {
        let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase());

        // Try WAV with hound first (simpler and faster)
        if ext.as_deref() == Some("wav") {
            return Self::load_wav_file(path);
        }

        // Use symphonia for other formats
        Self::load_audio_with_symphonia(path)
    }

    /// Load WAV file using hound
    fn load_wav_file(path: &Path) -> Result<Vec<f32>, CoreError> {
        let reader = hound::WavReader::open(path)
            .map_err(|e| CoreError::FileError(format!("Failed to open WAV: {}", e)))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate;
        let channels = spec.channels as usize;

        // Read all samples
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader
                .into_samples::<f32>()
                .filter_map(Result::ok)
                .collect(),
            hound::SampleFormat::Int => {
                let bits = spec.bits_per_sample;
                let max_val = (1 << (bits - 1)) as f32;
                reader
                    .into_samples::<i32>()
                    .filter_map(Result::ok)
                    .map(|s| s as f32 / max_val)
                    .collect()
            }
        };

        // Convert to mono if stereo
        let mono_samples = if channels > 1 {
            samples
                .chunks(channels)
                .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                .collect()
        } else {
            samples
        };

        // Resample to 16kHz if needed
        if sample_rate != 16000 {
            Self::resample(&mono_samples, sample_rate, 16000)
        } else {
            Ok(mono_samples)
        }
    }

    /// Load audio using symphonia (supports MP3, M4A, OGG, FLAC)
    fn load_audio_with_symphonia(path: &Path) -> Result<Vec<f32>, CoreError> {
        let file = std::fs::File::open(path)
            .map_err(|e| CoreError::FileError(format!("Failed to open file: {}", e)))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Provide hint based on extension
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        // Probe the media source
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| CoreError::FileError(format!("Failed to probe format: {}", e)))?;

        let mut format = probed.format;

        // Find the first audio track
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| CoreError::FileError("No audio track found".to_string()))?;

        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

        // Create decoder
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| CoreError::FileError(format!("Failed to create decoder: {}", e)))?;

        let mut all_samples: Vec<f32> = Vec::new();

        // Decode all packets
        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => {
                    warn!("Error reading packet: {}", e);
                    break;
                }
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    let duration = decoded.capacity() as u64;

                    let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);
                    sample_buf.copy_interleaved_ref(decoded);

                    all_samples.extend(sample_buf.samples());
                }
                Err(e) => {
                    warn!("Decode error: {}", e);
                    continue;
                }
            }
        }

        // Convert to mono
        let mono_samples = if channels > 1 {
            all_samples
                .chunks(channels)
                .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                .collect()
        } else {
            all_samples
        };

        // Resample to 16kHz if needed
        if sample_rate != 16000 {
            Self::resample(&mono_samples, sample_rate, 16000)
        } else {
            Ok(mono_samples)
        }
    }

    /// Resample audio to target sample rate using linear interpolation
    fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>, CoreError> {
        if from_rate == to_rate {
            return Ok(samples.to_vec());
        }

        let ratio = to_rate as f64 / from_rate as f64;
        let new_len = (samples.len() as f64 * ratio) as usize;
        let mut resampled = Vec::with_capacity(new_len);

        for i in 0..new_len {
            let src_idx = i as f64 / ratio;
            let idx_floor = src_idx.floor() as usize;
            let idx_ceil = (idx_floor + 1).min(samples.len() - 1);
            let frac = src_idx - idx_floor as f64;

            let sample = samples[idx_floor] as f64 * (1.0 - frac) + samples[idx_ceil] as f64 * frac;
            resampled.push(sample as f32);
        }

        Ok(resampled)
    }

    /// Extract audio from video file using ffmpeg
    async fn extract_audio_from_video(video_path: &Path) -> Result<PathBuf, CoreError> {
        use tokio::process::Command;

        let temp_dir = std::env::temp_dir();
        let audio_path = temp_dir.join(format!(
            "mindtype_audio_{}.wav",
            uuid::Uuid::new_v4()
        ));

        let output = Command::new("ffmpeg")
            .args([
                "-i",
                video_path.to_str().unwrap_or(""),
                "-vn",           // No video
                "-acodec",
                "pcm_s16le",     // 16-bit PCM
                "-ar",
                "16000",         // 16kHz
                "-ac",
                "1",             // Mono
                "-y",            // Overwrite
                audio_path.to_str().unwrap_or(""),
            ])
            .output()
            .await
            .map_err(|e| CoreError::FileError(format!("Failed to run ffmpeg: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::FileError(format!(
                "ffmpeg failed: {}",
                stderr
            )));
        }

        Ok(audio_path)
    }

    /// Process all pending jobs
    pub async fn process_all(&self) -> Result<(), CoreError> {
        if *self.processing.read().await {
            return Ok(());
        }

        *self.processing.write().await = true;

        let order = self.job_order.read().await.clone();

        for id in order {
            let job = match self.get_job(id).await {
                Some(j) if j.status == FileStatus::Pending => j,
                _ => continue,
            };

            info!("Processing file: {}", job.filename);

            let audio_path = if job.is_video() {
                // Extract audio from video
                self.update_job(id, FileStatus::ExtractingAudio, 10).await;
                match Self::extract_audio_from_video(&job.path).await {
                    Ok(path) => path,
                    Err(e) => {
                        error!("Failed to extract audio: {}", e);
                        self.update_job(id, FileStatus::Failed(e.to_string()), 0).await;
                        continue;
                    }
                }
            } else {
                job.path.clone()
            };

            // Load audio
            self.update_job(id, FileStatus::Transcribing, 20).await;
            let audio_data = match Self::load_audio_file(&audio_path) {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to load audio: {}", e);
                    self.update_job(id, FileStatus::Failed(e.to_string()), 0).await;
                    // Clean up temp file if it was extracted
                    if job.is_video() {
                        let _ = std::fs::remove_file(&audio_path);
                    }
                    continue;
                }
            };

            // Clean up temp file if it was extracted
            if job.is_video() {
                let _ = std::fs::remove_file(&audio_path);
            }

            self.update_job(id, FileStatus::Transcribing, 40).await;

            // Note: Actual transcription would be called from the Tauri command
            // which has access to the transcriber. Here we just store the audio
            // and the Tauri layer handles transcription.

            // For now, mark as complete with placeholder
            // In production, this would integrate with the transcriber
            self.set_transcription(
                id,
                format!("Audio loaded: {} samples at 16kHz", audio_data.len()),
            )
            .await;

            self.update_job(id, FileStatus::Completed, 100).await;
        }

        *self.processing.write().await = false;

        Ok(())
    }

    /// Process a single job with the provided transcriber
    /// This is called from the Tauri layer which has access to WhisperTranscriber
    pub async fn process_job_with_transcriber<T, F>(
        &self,
        id: Uuid,
        transcribe_fn: F,
    ) -> Result<(), CoreError>
    where
        F: FnOnce(Vec<f32>) -> T,
        T: std::future::Future<Output = Result<String, String>>,
    {
        let job = match self.get_job(id).await {
            Some(j) => j,
            None => return Err(CoreError::FileError("Job not found".to_string())),
        };

        let audio_path = if job.is_video() {
            self.update_job(id, FileStatus::ExtractingAudio, 10).await;
            Self::extract_audio_from_video(&job.path).await?
        } else {
            job.path.clone()
        };

        self.update_job(id, FileStatus::Transcribing, 20).await;
        let audio_data = Self::load_audio_file(&audio_path)?;

        if job.is_video() {
            let _ = std::fs::remove_file(&audio_path);
        }

        self.update_job(id, FileStatus::Transcribing, 50).await;

        let transcription = transcribe_fn(audio_data)
            .await
            .map_err(|e| CoreError::TranscriptionError(e))?;

        self.set_transcription(id, transcription).await;
        self.update_job(id, FileStatus::Completed, 100).await;

        Ok(())
    }
}

impl Default for FileProcessor {
    fn default() -> Self {
        Self::new()
    }
}
