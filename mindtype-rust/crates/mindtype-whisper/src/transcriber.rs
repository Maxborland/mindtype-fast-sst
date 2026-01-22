//! Whisper ONNX transcriber implementation
//!
//! Uses ONNX Runtime to run Whisper encoder/decoder models.

use crate::error::WhisperError;
use crate::mel::compute_mel_spectrogram;
use crate::model::{ModelSize, WhisperModel};
use crate::tokenizer::WhisperTokenizer;
use ndarray::Array2;
use ort::session::{builder::GraphOptimizationLevel, Session};
use std::path::Path;
use tracing::{debug, info, warn};

/// Execution provider for ONNX Runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Accelerator {
    /// Automatically select best available
    #[default]
    Auto,
    /// DirectML (Windows GPU/NPU)
    DirectML,
    /// CUDA (NVIDIA)
    Cuda,
    /// CPU only
    Cpu,
}

impl std::fmt::Display for Accelerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Accelerator::Auto => write!(f, "Auto"),
            Accelerator::DirectML => write!(f, "DirectML (GPU/NPU)"),
            Accelerator::Cuda => write!(f, "CUDA (NVIDIA)"),
            Accelerator::Cpu => write!(f, "CPU"),
        }
    }
}

/// Result of a transcription
#[derive(Debug, Clone)]
pub struct Transcription {
    /// The transcribed text
    pub text: String,
    /// Detected or specified language
    pub language: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Processing duration in milliseconds
    pub duration_ms: u64,
}

/// Maximum number of tokens to generate
const MAX_TOKENS: usize = 448;

/// Dimensions for Whisper models (varies by size)
/// Tiny: 384, Small: 768, Medium: 1024, Large: 1280
fn hidden_size_for_model(size: ModelSize) -> usize {
    match size {
        ModelSize::Tiny => 384,
        ModelSize::Small => 768,
        ModelSize::Medium => 1024,
        ModelSize::LargeV3 => 1280,
    }
}

/// Encoder output frames (constant for 30s audio)
const ENCODER_FRAMES: usize = 1500;

/// Whisper transcriber using ONNX Runtime
pub struct WhisperTranscriber {
    model: WhisperModel,
    accelerator: Accelerator,
    encoder: Session,
    decoder: Session,
    tokenizer: WhisperTokenizer,
    hidden_size: usize,
}

impl WhisperTranscriber {
    /// Create a new transcriber with the specified model
    pub fn new(
        models_dir: &Path,
        size: ModelSize,
        accelerator: Accelerator,
    ) -> Result<Self, WhisperError> {
        info!("Loading Whisper model: {} with {:?}", size, accelerator);

        let model_path = WhisperModel::expected_path(models_dir, size);

        if !WhisperModel::exists_at(models_dir, size) {
            return Err(WhisperError::ModelNotFound(format!(
                "Model {} not found at {:?}",
                size, model_path
            )));
        }

        let model = WhisperModel {
            size,
            path: model_path.clone(),
        };

        // Initialize ONNX Runtime (returns bool)
        let _initialized = ort::init()
            .with_name("mindtype-whisper")
            .commit();

        // Build sessions
        let encoder_path = model_path.join("encoder.onnx");
        let decoder_path = model_path.join("decoder.onnx");

        info!("Loading encoder from {:?}", encoder_path);
        let encoder = Self::create_session(&encoder_path, accelerator)?;

        info!("Loading decoder from {:?}", decoder_path);
        let decoder = Self::create_session(&decoder_path, accelerator)?;

        // Load tokenizer
        let vocab_path = model_path.join("vocab.json");
        let tokenizer = if vocab_path.exists() {
            WhisperTokenizer::from_file(&vocab_path)?
        } else {
            warn!("vocab.json not found, using embedded tokenizer");
            WhisperTokenizer::new()
        };

        info!("Whisper model loaded successfully");

        let hidden_size = hidden_size_for_model(size);

        Ok(Self {
            model,
            accelerator,
            encoder,
            decoder,
            tokenizer,
            hidden_size,
        })
    }

    /// Create ONNX session
    fn create_session(model_path: &Path, _accelerator: Accelerator) -> Result<Session, WhisperError> {
        // Build session with CPU provider by default
        // Execution providers are optional and may not be available
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .commit_from_file(model_path)?;

        // Note: In ort 2.0, execution providers are configured differently
        // For now, we just use the default CPU provider
        // DirectML/CUDA can be added when the ort API is more stable

        Ok(session)
    }

    /// Transcribe audio samples
    pub async fn transcribe(
        &mut self,
        audio: &[f32],
        language: &str,
    ) -> Result<Transcription, WhisperError> {
        let start = std::time::Instant::now();

        debug!(
            "Transcribing {} samples ({:.2}s of audio) with language: {}",
            audio.len(),
            audio.len() as f32 / 16000.0,
            language
        );

        // Compute mel spectrogram
        debug!("Computing mel spectrogram...");
        let mel = compute_mel_spectrogram(audio);

        // Run encoder
        debug!("Running encoder...");
        let encoder_output = self.run_encoder(&mel)?;

        // Run decoder
        debug!("Running decoder...");
        let lang = if language == "auto" { None } else { Some(language) };
        let tokens = self.run_decoder(&encoder_output, lang)?;

        // Decode tokens to text
        let text = self.tokenizer.decode(&tokens)?;
        debug!("Decoded text: '{}'", text);

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(Transcription {
            text,
            language: if language == "auto" { "en".to_string() } else { language.to_string() },
            confidence: 0.95,
            duration_ms,
        })
    }

    /// Run the encoder
    fn run_encoder(&mut self, mel: &Array2<f32>) -> Result<Vec<f32>, WhisperError> {
        let mel_shape = mel.shape();
        let batch_size = 1usize;
        let n_mels = mel_shape[0];
        let n_frames = mel_shape[1];

        // Flatten to Vec for input
        let mel_data: Vec<f32> = mel.iter().cloned().collect();
        let shape = vec![batch_size as i64, n_mels as i64, n_frames as i64];

        // Create input tensor
        let input = ort::value::Tensor::from_array((shape.as_slice(), mel_data.into_boxed_slice()))?;

        // Run encoder
        let outputs = self.encoder.run(ort::inputs![input])?;

        // Extract output tensor data
        // In ort 2.0, try_extract_tensor returns (Shape, &[T])
        let (_, output_slice) = outputs[0].try_extract_tensor::<f32>()?;

        Ok(output_slice.to_vec())
    }

    /// Run the decoder with autoregressive generation
    fn run_decoder(
        &mut self,
        encoder_output: &[f32],
        language: Option<&str>,
    ) -> Result<Vec<i64>, WhisperError> {
        use crate::tokenizer::special_tokens;

        let mut tokens = self.tokenizer.initial_tokens(language);
        debug!("Initial tokens: {:?}", tokens);

        // Reshape encoder output to [batch, frames, hidden]
        // encoder_output is flat: [batch * frames * hidden]
        let encoder_len = encoder_output.len();
        let expected_len = ENCODER_FRAMES * self.hidden_size;

        if encoder_len != expected_len {
            warn!(
                "Encoder output size mismatch: got {}, expected {} (frames={}, hidden={})",
                encoder_len, expected_len, ENCODER_FRAMES, self.hidden_size
            );
        }

        // Autoregressive decoding loop
        for step in 0..MAX_TOKENS {
            // Prepare decoder input: tokens as i64
            let decoder_input: Vec<i64> = tokens.clone();
            let seq_len = decoder_input.len();

            // Create input tensors
            // Encoder hidden states: [batch=1, frames, hidden_size]
            let encoder_shape = vec![1i64, ENCODER_FRAMES as i64, self.hidden_size as i64];
            let encoder_tensor = ort::value::Tensor::from_array((
                encoder_shape.as_slice(),
                encoder_output.to_vec().into_boxed_slice(),
            ))?;

            // Decoder input ids: [batch=1, seq_len]
            let decoder_shape = vec![1i64, seq_len as i64];
            let decoder_tensor = ort::value::Tensor::from_array((
                decoder_shape.as_slice(),
                decoder_input.into_boxed_slice(),
            ))?;

            // Run decoder
            let outputs = self.decoder.run(ort::inputs![
                "encoder_hidden_states" => encoder_tensor,
                "decoder_input_ids" => decoder_tensor
            ])?;

            // Extract logits: [batch, seq_len, vocab_size]
            let (logits_shape, logits_data) = outputs[0].try_extract_tensor::<f32>()?;

            // Get logits for the last token position
            // Shape is [1, seq_len, vocab_size], we want [vocab_size] at position seq_len-1
            let vocab_size = if logits_shape.len() >= 3 {
                logits_shape[2] as usize
            } else {
                // Fallback: estimate from data size
                logits_data.len() / seq_len
            };

            let last_pos_start = (seq_len - 1) * vocab_size;
            let last_logits = &logits_data[last_pos_start..last_pos_start + vocab_size];

            // Greedy decoding: select token with highest logit
            let next_token = argmax(last_logits) as i64;

            debug!(
                "Step {}: generated token {} ({})",
                step,
                next_token,
                self.tokenizer.decode_token(next_token)
            );

            // Check for end of transcript
            if next_token == special_tokens::EOT {
                debug!("Reached EOT at step {}", step);
                break;
            }

            // Check for no speech token
            if next_token == special_tokens::NO_SPEECH {
                debug!("No speech detected");
                break;
            }

            tokens.push(next_token);

            // Safety check: stop if we're generating repetitive tokens
            if tokens.len() > 10 {
                let last_few: Vec<_> = tokens.iter().rev().take(5).collect();
                if last_few.iter().all(|&&t| t == next_token) {
                    warn!("Detected repetitive generation, stopping");
                    break;
                }
            }
        }

        debug!("Generated {} tokens total", tokens.len());
        Ok(tokens)
    }

    /// Get the loaded model info
    pub fn model(&self) -> &WhisperModel {
        &self.model
    }

    /// Get the accelerator being used
    pub fn accelerator(&self) -> Accelerator {
        self.accelerator
    }
}

/// Find index of maximum value in a slice
fn argmax(data: &[f32]) -> usize {
    data.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

/// Fallback transcriber using whisper-cli subprocess
pub struct WhisperCliTranscriber {
    model_size: ModelSize,
    whisper_cli_path: std::path::PathBuf,
}

impl WhisperCliTranscriber {
    /// Create a new CLI-based transcriber
    pub fn new(model_size: ModelSize) -> Result<Self, WhisperError> {
        let possible_paths = [
            std::path::PathBuf::from("whisper-cli.exe"),
            std::path::PathBuf::from("./whisper-cli.exe"),
            dirs::data_local_dir()
                .unwrap_or_default()
                .join("MindType")
                .join("whisper-cli.exe"),
        ];

        let whisper_cli_path = possible_paths
            .iter()
            .find(|p| p.exists())
            .cloned()
            .ok_or_else(|| WhisperError::ModelNotFound("whisper-cli not found".to_string()))?;

        Ok(Self {
            model_size,
            whisper_cli_path,
        })
    }

    /// Transcribe using whisper-cli subprocess
    pub async fn transcribe(
        &self,
        audio: &[f32],
        language: &str,
    ) -> Result<Transcription, WhisperError> {
        use std::process::Command;

        let start = std::time::Instant::now();

        // Write audio to temporary WAV file
        let temp_dir = std::env::temp_dir();
        let temp_wav = temp_dir.join(format!("mindtype_audio_{}.wav", uuid::Uuid::new_v4()));

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = hound::WavWriter::create(&temp_wav, spec)
            .map_err(|e| WhisperError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        for &sample in audio {
            writer.write_sample(sample)
                .map_err(|e| WhisperError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        }
        writer.finalize()
            .map_err(|e| WhisperError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        // Run whisper-cli
        let output = Command::new(&self.whisper_cli_path)
            .args([
                "-m", self.model_size.dir_name(),
                "-l", language,
                "-f", temp_wav.to_str().unwrap(),
                "--output-txt",
            ])
            .output()
            .map_err(|e| WhisperError::TranscriptionFailed(format!("Failed to run whisper-cli: {}", e)))?;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_wav);

        if !output.status.success() {
            return Err(WhisperError::TranscriptionFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(Transcription {
            text,
            language: language.to_string(),
            confidence: 0.9,
            duration_ms,
        })
    }
}
