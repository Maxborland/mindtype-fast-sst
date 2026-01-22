//! Application messages

use mindtype_core::{AcceleratorPref, AppConfig, AudioLevel, LlmConfig, TranscriptionResult};
use std::path::PathBuf;
use uuid::Uuid;

/// Main application messages
#[derive(Debug, Clone)]
pub enum Message {
    // Lifecycle
    Loaded(Result<AppConfig, String>),
    ConfigSaved,
    Tick,

    // Navigation
    SetView(View),
    WizardNext,
    WizardBack,

    // Wizard
    WizardSelectLanguage(String),
    WizardSelectMode(bool), // true = Simple
    WizardSelectProvider(bool), // true = MindType Cloud
    WizardApiKeyChanged(String),
    WizardSelectModel(mindtype_whisper::ModelSize),
    WizardModelDownloadProgress(u8),
    WizardModelDownloaded,
    WizardHotkeyChanged(String),
    WizardMicrophoneTest,
    WizardMicrophoneSelected(Option<String>),
    WizardComplete,

    // Recording
    HotkeyPressed,
    HotkeyReleased,
    AudioLevel(AudioLevel),
    RecordingStarted,
    RecordingStopped(Vec<f32>),
    TranscriptionComplete(Result<TranscriptionResult, String>),
    TranscriptionCopied(Uuid),

    // Files
    FilesDropped(Vec<PathBuf>),
    FileRemove(Uuid),
    FilesClear,
    FileJobUpdate(Uuid, mindtype_core::FileStatus, u8),
    FilesProcessComplete,

    // Settings
    SettingsOpen,
    SettingsClose,
    SettingChanged(SettingChange),

    // License
    LicenseKeyChanged(String),
    LicenseValidate,
    LicenseValidated(Result<mindtype_licensing::LicenseStatus, String>),
    CreditsUpdated(u32),

    // System
    TrayAction(mindtype_platform::TrayAction),
    WindowMinimize,
    WindowRestore,
    Quit,
}

/// Current application view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    /// Setup wizard
    #[default]
    Wizard,
    /// Main recorder view
    Main,
    /// File processing view
    Files,
    /// Settings view
    Settings,
}

/// Wizard steps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WizardStep {
    #[default]
    Welcome,
    Mode,
    Provider,
    ApiKeys,
    Model,
    Microphone,
    Hotkey,
    Complete,
}

impl WizardStep {
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Welcome => Some(Self::Mode),
            Self::Mode => Some(Self::Provider),
            Self::Provider => Some(Self::ApiKeys),
            Self::ApiKeys => Some(Self::Model),
            Self::Model => Some(Self::Microphone),
            Self::Microphone => Some(Self::Hotkey),
            Self::Hotkey => Some(Self::Complete),
            Self::Complete => None,
        }
    }

    pub fn prev(&self) -> Option<Self> {
        match self {
            Self::Welcome => None,
            Self::Mode => Some(Self::Welcome),
            Self::Provider => Some(Self::Mode),
            Self::ApiKeys => Some(Self::Provider),
            Self::Model => Some(Self::ApiKeys),
            Self::Microphone => Some(Self::Model),
            Self::Hotkey => Some(Self::Microphone),
            Self::Complete => Some(Self::Hotkey),
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::Welcome => "Welcome to MindType",
            Self::Mode => "Choose Your Mode",
            Self::Provider => "AI Provider",
            Self::ApiKeys => "API Configuration",
            Self::Model => "Speech Recognition",
            Self::Microphone => "Microphone Setup",
            Self::Hotkey => "Hotkey Setup",
            Self::Complete => "All Set!",
        }
    }
}

/// Settings changes
#[derive(Debug, Clone)]
pub enum SettingChange {
    Language(String),
    Model(mindtype_whisper::ModelSize),
    Accelerator(AcceleratorPref),
    Hotkey(String),
    Microphone(Option<String>),
    LlmProvider(LlmConfig),
}

/// Main tab in advanced mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MainTab {
    #[default]
    Recorder,
    Files,
    Settings,
}
