//! Application configuration

use mindtype_llm::ProviderType;
use mindtype_whisper::ModelSize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AppMode {
    /// Simple mode - minimal UI, MindType Cloud by default
    #[default]
    Simple,
    /// Advanced mode - full settings access
    Advanced,
}

impl std::fmt::Display for AppMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppMode::Simple => write!(f, "Simple"),
            AppMode::Advanced => write!(f, "Advanced"),
        }
    }
}

/// Accelerator preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AcceleratorPref {
    #[default]
    Auto,
    DirectML,
    Cuda,
    Cpu,
}

/// LLM provider configuration
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

impl LlmConfig {
    pub fn provider_type(&self) -> ProviderType {
        match self {
            LlmConfig::MindTypeCloud => ProviderType::MindTypeCloud,
            LlmConfig::OpenAi { .. } => ProviderType::OpenAi,
            LlmConfig::Anthropic { .. } => ProviderType::Anthropic,
            LlmConfig::Gemini { .. } => ProviderType::Gemini,
            LlmConfig::OpenRouter { .. } => ProviderType::OpenRouter,
            LlmConfig::Yandex { .. } => ProviderType::Yandex,
            LlmConfig::Ollama { .. } => ProviderType::Ollama,
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    // Setup
    pub setup_completed: bool,
    pub app_mode: AppMode,
    pub use_mindtype_cloud: bool,

    // Transcription
    pub model_size: ModelSize,
    pub language: String,
    pub accelerator: AcceleratorPref,

    // Hotkeys
    pub record_hotkey: String,

    // Audio
    pub microphone: Option<String>,

    // UI
    pub ui_language: String,

    // License
    pub license_key: Option<String>,

    // LLM
    pub llm: LlmConfig,

    // Internal paths
    #[serde(skip)]
    pub config_dir: PathBuf,
    #[serde(skip)]
    pub data_dir: PathBuf,
    #[serde(skip)]
    pub models_dir: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        let dirs = directories::ProjectDirs::from("space", "mindtype", "MindType")
            .expect("Could not determine config directory");

        Self {
            setup_completed: false,
            app_mode: AppMode::Simple,
            use_mindtype_cloud: true,

            model_size: ModelSize::Small,
            language: "auto".to_string(),
            accelerator: AcceleratorPref::Auto,

            record_hotkey: "Ctrl+Alt+V".to_string(),

            microphone: None,

            ui_language: "en".to_string(),

            license_key: None,

            llm: LlmConfig::MindTypeCloud,

            config_dir: dirs.config_dir().to_path_buf(),
            data_dir: dirs.data_dir().to_path_buf(),
            models_dir: dirs.data_dir().join("models"),
        }
    }
}

impl AppConfig {
    /// Load configuration from disk or create default
    pub fn load() -> Result<Self, crate::CoreError> {
        let dirs = directories::ProjectDirs::from("space", "mindtype", "MindType")
            .expect("Could not determine config directory");

        let config_file = dirs.config_dir().join("config.toml");

        let mut config = if config_file.exists() {
            info!("Loading config from {:?}", config_file);
            let content = std::fs::read_to_string(&config_file)?;
            toml::from_str(&content)
                .map_err(|e| crate::CoreError::ConfigError(e.to_string()))?
        } else {
            info!("Creating default config");
            Self::default()
        };

        // Set paths (not serialized)
        config.config_dir = dirs.config_dir().to_path_buf();
        config.data_dir = dirs.data_dir().to_path_buf();
        config.models_dir = dirs.data_dir().join("models");

        // Ensure directories exist
        std::fs::create_dir_all(&config.config_dir)?;
        std::fs::create_dir_all(&config.data_dir)?;
        std::fs::create_dir_all(&config.models_dir)?;

        Ok(config)
    }

    /// Save configuration to disk
    pub fn save(&self) -> Result<(), crate::CoreError> {
        let config_file = self.config_dir.join("config.toml");

        info!("Saving config to {:?}", config_file);

        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::CoreError::ConfigError(e.to_string()))?;

        std::fs::write(config_file, content)?;

        Ok(())
    }

    /// API base URL
    pub fn api_base(&self) -> &'static str {
        "https://mindtype.space"
    }

    /// Check if model is downloaded
    pub fn is_model_downloaded(&self) -> bool {
        mindtype_whisper::WhisperModel::exists_at(&self.models_dir, self.model_size)
    }
}
