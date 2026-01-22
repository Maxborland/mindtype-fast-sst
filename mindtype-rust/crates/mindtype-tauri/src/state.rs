//! Application state management

use anyhow::Result;
use mindtype_core::{AudioRecorder, FileProcessor};
use mindtype_licensing::LicenseManager;
use mindtype_platform::Platform;
use mindtype_whisper::{Accelerator, ModelSize, WhisperTranscriber};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// LLM provider configuration for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LlmConfig {
    MindTypeCloud,
    OpenAi {
        api_key: String,
        model: Option<String>,
    },
    Anthropic {
        api_key: String,
        model: Option<String>,
    },
    Gemini {
        api_key: String,
        model: Option<String>,
    },
    OpenRouter {
        api_key: String,
        model: Option<String>,
    },
    Yandex {
        api_key: String,
        folder_id: String,
        model: Option<String>,
    },
    Ollama {
        base_url: Option<String>,
        model: Option<String>,
    },
}

impl Default for LlmConfig {
    fn default() -> Self {
        LlmConfig::MindTypeCloud
    }
}

/// Recording state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordingState {
    Idle,
    Recording,
    Transcribing,
    Inserting,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub language: String,
    pub model_id: String,
    pub hotkey: String,
    pub setup_completed: bool,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub license_key: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            model_id: "small".to_string(),
            hotkey: "Ctrl+Alt+V".to_string(),
            setup_completed: false,
            llm: LlmConfig::default(),
            license_key: None,
        }
    }
}

/// Transcription result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcription {
    pub id: String,
    pub text: String,
    pub language: String,
    pub duration_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Main application state
pub struct AppState {
    pub settings: RwLock<Settings>,
    pub recording_state: RwLock<RecordingState>,
    pub transcriptions: RwLock<Vec<Transcription>>,
    pub audio_recorder: RwLock<Option<AudioRecorder>>,
    pub transcriber: RwLock<Option<WhisperTranscriber>>,
    pub file_processor: RwLock<FileProcessor>,
    pub platform: Arc<Platform>,
    pub license_manager: RwLock<LicenseManager>,
    pub data_dir: PathBuf,
}

impl AppState {
    /// Create a new AppState instance
    pub fn new() -> Result<Self> {
        let settings = Self::load_settings().unwrap_or_default();

        // Get data directory
        let project_dirs = directories::ProjectDirs::from("com", "mindtype", "MindType")
            .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;
        let data_dir = project_dirs.data_dir().to_path_buf();
        let cache_dir = project_dirs.cache_dir().to_path_buf();

        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&cache_dir)?;

        // Initialize platform
        let platform = Arc::new(Platform::new()?);

        // Get device ID for licensing
        let device_id = platform.get_device_id().unwrap_or_else(|_| "unknown".to_string());

        // Initialize license manager
        let license_manager = LicenseManager::new(
            "https://mindtype.app",
            &device_id,
            &cache_dir,
        );

        Ok(Self {
            settings: RwLock::new(settings),
            recording_state: RwLock::new(RecordingState::Idle),
            transcriptions: RwLock::new(Vec::new()),
            audio_recorder: RwLock::new(None),
            transcriber: RwLock::new(None),
            file_processor: RwLock::new(FileProcessor::new()),
            platform,
            license_manager: RwLock::new(license_manager),
            data_dir,
        })
    }

    /// Load settings from disk
    fn load_settings() -> Result<Settings> {
        let config_dir = directories::ProjectDirs::from("com", "mindtype", "MindType")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        let settings_path = config_dir.config_dir().join("settings.json");

        if settings_path.exists() {
            let content = std::fs::read_to_string(&settings_path)?;
            let settings: Settings = serde_json::from_str(&content)?;
            Ok(settings)
        } else {
            Ok(Settings::default())
        }
    }

    /// Save settings to disk
    pub async fn save_settings(&self) -> Result<()> {
        let config_dir = directories::ProjectDirs::from("com", "mindtype", "MindType")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        let config_path = config_dir.config_dir();
        std::fs::create_dir_all(config_path)?;

        let settings_path = config_path.join("settings.json");
        let settings = self.settings.read().await;
        let content = serde_json::to_string_pretty(&*settings)?;
        std::fs::write(settings_path, content)?;
        Ok(())
    }

    /// Get models directory
    pub fn models_dir(&self) -> PathBuf {
        self.data_dir.join("models")
    }

    /// Initialize the Whisper transcriber
    pub async fn init_transcriber(&self, model_id: &str) -> Result<()> {
        let models_dir = self.models_dir();
        std::fs::create_dir_all(&models_dir)?;

        let model_size = match model_id {
            "tiny" => ModelSize::Tiny,
            "small" => ModelSize::Small,
            "medium" => ModelSize::Medium,
            "large" | "large-v3" => ModelSize::LargeV3,
            _ => ModelSize::Small,
        };

        let transcriber = WhisperTranscriber::new(&models_dir, model_size, Accelerator::Auto)?;
        *self.transcriber.write().await = Some(transcriber);
        Ok(())
    }

    /// Get the current recording state
    pub async fn get_recording_state(&self) -> RecordingState {
        *self.recording_state.read().await
    }

    /// Set the recording state
    pub async fn set_recording_state(&self, state: RecordingState) {
        *self.recording_state.write().await = state;
    }

    /// Add a transcription to history
    pub async fn add_transcription(&self, transcription: Transcription) {
        let mut transcriptions = self.transcriptions.write().await;
        transcriptions.insert(0, transcription);
        // Keep only the last 100 transcriptions
        transcriptions.truncate(100);
    }
}
