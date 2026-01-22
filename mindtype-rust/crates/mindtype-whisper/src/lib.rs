//! MindType Whisper ONNX Integration
//!
//! Provides speech-to-text transcription using Whisper models via ONNX Runtime.
//!
//! ## Architecture
//!
//! The transcriber follows Whisper's architecture:
//! 1. Audio → Mel spectrogram (80 bins, 16kHz)
//! 2. Mel → Encoder → Hidden states
//! 3. Hidden states + Tokens → Decoder → Logits
//! 4. Autoregressive decoding until EOT token
//!
//! ## Models
//!
//! Whisper models come in several sizes:
//! - Tiny: ~75MB, fastest, lower quality
//! - Small: ~466MB, good balance (default)
//! - Medium: ~1.5GB, higher quality
//! - Large-V3: ~3GB, highest quality
//!
//! ## Accelerators
//!
//! Supports multiple execution providers:
//! - Auto: Tries DirectML (Windows) or CUDA, falls back to CPU
//! - DirectML: Windows GPU/NPU acceleration
//! - CUDA: NVIDIA GPU acceleration
//! - CPU: Pure CPU execution

mod error;
mod mel;
mod model;
mod tokenizer;
mod transcriber;

pub use error::WhisperError;
pub use mel::{compute_mel_spectrogram, N_FRAMES, N_MELS, SAMPLE_RATE};
pub use model::{ModelSize, WhisperModel};
pub use tokenizer::WhisperTokenizer;
pub use transcriber::{Accelerator, Transcription, WhisperCliTranscriber, WhisperTranscriber};
