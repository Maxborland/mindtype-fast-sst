//! MindType Core
//!
//! Business logic and configuration for MindType.

mod audio;
mod config;
mod error;
mod file_processor;
mod transcription;

pub use audio::{AudioRecorder, AudioLevel};
pub use config::{AcceleratorPref, AppConfig, AppMode, LlmConfig};
pub use error::CoreError;
pub use file_processor::{FileProcessor, FileJob, FileJobUpdate, FileStatus};
pub use transcription::{TranscriptionManager, TranscriptionResult};

/// Application state
pub mod state {
    pub use super::config::AppConfig;
}
