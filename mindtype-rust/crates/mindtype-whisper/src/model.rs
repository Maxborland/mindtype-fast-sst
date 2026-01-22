//! Whisper model definitions and loading

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Whisper model sizes with their approximate VRAM/RAM requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelSize {
    /// ~75MB - Fastest, lowest quality
    Tiny,
    /// ~466MB - Good balance for most uses
    #[default]
    Small,
    /// ~1.5GB - Higher quality
    Medium,
    /// ~3GB - Highest quality
    LargeV3,
}

impl ModelSize {
    /// Returns the model directory name
    pub fn dir_name(&self) -> &'static str {
        match self {
            ModelSize::Tiny => "tiny",
            ModelSize::Small => "small",
            ModelSize::Medium => "medium",
            ModelSize::LargeV3 => "large-v3",
        }
    }

    /// Returns approximate model size in bytes
    pub fn size_bytes(&self) -> u64 {
        match self {
            ModelSize::Tiny => 75 * 1024 * 1024,
            ModelSize::Small => 466 * 1024 * 1024,
            ModelSize::Medium => 1500 * 1024 * 1024,
            ModelSize::LargeV3 => 3000 * 1024 * 1024,
        }
    }

    /// Returns human-readable size
    pub fn size_human(&self) -> &'static str {
        match self {
            ModelSize::Tiny => "75 MB",
            ModelSize::Small => "466 MB",
            ModelSize::Medium => "1.5 GB",
            ModelSize::LargeV3 => "3 GB",
        }
    }

    /// All available model sizes
    pub fn all() -> &'static [ModelSize] {
        &[
            ModelSize::Tiny,
            ModelSize::Small,
            ModelSize::Medium,
            ModelSize::LargeV3,
        ]
    }
}

impl std::fmt::Display for ModelSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelSize::Tiny => write!(f, "Tiny"),
            ModelSize::Small => write!(f, "Small"),
            ModelSize::Medium => write!(f, "Medium"),
            ModelSize::LargeV3 => write!(f, "Large V3"),
        }
    }
}

/// Represents a loaded Whisper model
#[derive(Debug)]
pub struct WhisperModel {
    pub size: ModelSize,
    pub path: PathBuf,
}

impl WhisperModel {
    /// Check if model files exist at the given path
    pub fn exists_at(models_dir: &std::path::Path, size: ModelSize) -> bool {
        let model_dir = models_dir.join(size.dir_name());
        let encoder_path = model_dir.join("encoder.onnx");
        let decoder_path = model_dir.join("decoder.onnx");

        encoder_path.exists() && decoder_path.exists()
    }

    /// Get the expected path for a model
    pub fn expected_path(models_dir: &std::path::Path, size: ModelSize) -> PathBuf {
        models_dir.join(size.dir_name())
    }
}
