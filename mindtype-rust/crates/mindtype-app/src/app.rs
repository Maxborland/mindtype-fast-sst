//! Main application state and logic

use crate::messages::{MainTab, Message, View, WizardStep};
use crate::theme;
use crate::ui::{files_view, main_view, settings_view, wizard_view};

use iced::widget::{container};
use iced::{Element, Length, Subscription, Task, Theme};
use mindtype_core::{AppConfig, AppMode, AudioLevel, AudioRecorder, FileProcessor, TranscriptionManager, TranscriptionResult};
use mindtype_licensing::{LicenseManager, LicenseStatus};
use mindtype_platform::{HotkeyEvent, Platform as WindowsPlatform};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Subscription ID markers
struct HotkeySubscriptionId;
struct AudioLevelSubscriptionId;

/// Main application state
pub struct MindTypeApp {
    // Config
    config: AppConfig,

    // Navigation
    view: View,
    wizard_step: WizardStep,
    main_tab: MainTab,

    // Wizard state
    wizard_api_key: String,
    wizard_model_progress: Option<u8>,

    // Recording state
    is_recording: bool,
    audio_level: AudioLevel,
    recent_transcriptions: Vec<TranscriptionResult>,

    // Services
    audio_recorder: AudioRecorder,
    transcription_manager: Arc<tokio::sync::RwLock<TranscriptionManager>>,
    file_processor: Arc<FileProcessor>,
    license_manager: Option<LicenseManager>,

    // Platform
    platform: Option<WindowsPlatform>,
    hotkey_rx: Option<mpsc::UnboundedReceiver<HotkeyEvent>>,
    audio_level_rx: Option<mpsc::UnboundedReceiver<AudioLevel>>,

    // Status
    license_status: LicenseStatus,
    credits_balance: u32,
    error_message: Option<String>,
}

impl MindTypeApp {
    fn new() -> (Self, Task<Message>) {
        let config = AppConfig::load().unwrap_or_default();

        let view = if config.setup_completed {
            View::Main
        } else {
            View::Wizard
        };

        let mut audio_recorder = AudioRecorder::new().unwrap_or_default();
        let _ = audio_recorder.select_device(config.microphone.as_deref());

        // Set up audio level callback
        let audio_level_rx = audio_recorder.set_level_callback();

        // Initialize platform
        let mut platform = WindowsPlatform::new().ok();
        let mut hotkey_rx = None;

        // Register hotkey if setup is complete
        if config.setup_completed {
            if let Some(ref mut p) = platform {
                match p.register_hotkey(&config.record_hotkey) {
                    Ok(()) => {
                        info!("Hotkey registered: {}", config.record_hotkey);
                        // Get the hotkey receiver - we need to consume it from platform
                        // For now we'll poll via subscription
                    }
                    Err(e) => {
                        error!("Failed to register hotkey: {}", e);
                    }
                }
            }
        }

        let mut transcription_manager = TranscriptionManager::new();

        // Initialize transcriber if model exists
        if config.setup_completed && config.is_model_downloaded() {
            if let Err(e) = transcription_manager.init(&config) {
                error!("Failed to init transcriber: {}", e);
            }
        }

        let app = Self {
            config,
            view,
            wizard_step: WizardStep::default(),
            main_tab: MainTab::default(),
            wizard_api_key: String::new(),
            wizard_model_progress: None,
            is_recording: false,
            audio_level: AudioLevel { rms: 0.0, peak: 0.0 },
            recent_transcriptions: Vec::new(),
            audio_recorder,
            transcription_manager: Arc::new(tokio::sync::RwLock::new(transcription_manager)),
            file_processor: Arc::new(FileProcessor::new()),
            license_manager: None,
            platform,
            hotkey_rx,
            audio_level_rx: Some(audio_level_rx),
            license_status: LicenseStatus::NotConfigured,
            credits_balance: 0,
            error_message: None,
        };

        (app, Task::none())
    }

    fn title(&self) -> String {
        match self.view {
            View::Wizard => format!("MindType Setup - {}", self.wizard_step.title()),
            View::Main | View::Files => "MindType".to_string(),
            View::Settings => "MindType - Settings".to_string(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        // Debug: log all messages except Tick (too noisy)
        if !matches!(message, Message::Tick) {
            info!("update() received: {:?}", message);
        }

        match message {
            // Navigation
            Message::SetView(view) => {
                self.view = view;
            }

            // Wizard navigation
            Message::WizardNext => {
                if let Some(next) = self.wizard_step.next() {
                    // Skip API Keys step if using MindType Cloud
                    let next = if next == WizardStep::ApiKeys && self.config.use_mindtype_cloud {
                        next.next().unwrap_or(next)
                    } else {
                        next
                    };
                    self.wizard_step = next;
                }
            }
            Message::WizardBack => {
                if let Some(prev) = self.wizard_step.prev() {
                    // Skip API Keys step if using MindType Cloud
                    let prev = if prev == WizardStep::ApiKeys && self.config.use_mindtype_cloud {
                        prev.prev().unwrap_or(prev)
                    } else {
                        prev
                    };
                    self.wizard_step = prev;
                }
            }

            // Wizard settings
            Message::WizardSelectLanguage(lang) => {
                self.config.ui_language = lang;
            }
            Message::WizardSelectMode(simple) => {
                self.config.app_mode = if simple {
                    AppMode::Simple
                } else {
                    AppMode::Advanced
                };
            }
            Message::WizardSelectProvider(cloud) => {
                self.config.use_mindtype_cloud = cloud;
                if cloud {
                    self.config.llm = mindtype_core::LlmConfig::MindTypeCloud;
                }
            }
            Message::WizardApiKeyChanged(key) => {
                self.wizard_api_key = key;
            }
            Message::WizardSelectModel(size) => {
                self.config.model_size = size;
            }
            Message::WizardModelDownloadProgress(progress) => {
                self.wizard_model_progress = Some(progress);
            }
            Message::WizardModelDownloaded => {
                self.wizard_model_progress = None;
            }
            Message::WizardHotkeyChanged(hotkey) => {
                self.config.record_hotkey = hotkey;
            }
            Message::WizardMicrophoneSelected(mic) => {
                self.config.microphone = mic.clone();
                let _ = self.audio_recorder.select_device(mic.as_deref());
            }
            Message::WizardMicrophoneTest => {
                // Start a short recording test
                if !self.is_recording {
                    if let Err(e) = self.audio_recorder.start() {
                        error!("Mic test failed: {}", e);
                    } else {
                        self.is_recording = true;
                        // Stop after 2 seconds
                        return Task::perform(
                            async {
                                tokio::time::sleep(Duration::from_secs(2)).await;
                            },
                            |_| Message::HotkeyReleased,
                        );
                    }
                }
            }
            Message::WizardComplete => {
                self.config.setup_completed = true;
                if let Err(e) = self.config.save() {
                    error!("Failed to save config: {}", e);
                }

                // Register hotkey
                if let Some(ref mut platform) = self.platform {
                    if let Err(e) = platform.register_hotkey(&self.config.record_hotkey) {
                        error!("Failed to register hotkey: {}", e);
                    } else {
                        info!("Hotkey registered: {}", self.config.record_hotkey);
                    }
                }

                // Initialize transcription manager
                if self.config.is_model_downloaded() {
                    let mut manager = futures::executor::block_on(self.transcription_manager.write());
                    if let Err(e) = manager.init(&self.config) {
                        error!("Failed to init transcriber: {}", e);
                    }
                }

                self.view = View::Main;
            }

            // Recording
            Message::HotkeyPressed => {
                if !self.is_recording {
                    info!("Hotkey pressed - starting recording");

                    // Save the foreground window so we can restore it later
                    if let Some(ref platform) = self.platform {
                        platform.save_foreground_window();
                    }

                    self.is_recording = true;
                    if let Err(e) = self.audio_recorder.start() {
                        error!("Failed to start recording: {}", e);
                        self.is_recording = false;
                    }
                }
            }
            Message::HotkeyReleased => {
                if self.is_recording {
                    info!("Hotkey released - stopping recording");
                    self.is_recording = false;

                    match self.audio_recorder.stop() {
                        Ok(samples) => {
                            if samples.len() > 1600 {
                                // At least 0.1s of audio
                                return Task::perform(async move { samples }, Message::RecordingStopped);
                            } else {
                                warn!("Recording too short, ignoring");
                            }
                        }
                        Err(e) => {
                            error!("Failed to stop recording: {}", e);
                        }
                    }
                }
            }
            Message::AudioLevel(level) => {
                self.audio_level = level;
            }
            Message::RecordingStopped(samples) => {
                info!("Recording stopped, {} samples ({:.2}s)", samples.len(), samples.len() as f32 / 16000.0);

                // Trigger transcription
                let transcription_manager = Arc::clone(&self.transcription_manager);
                let language = self.config.language.clone();

                return Task::perform(
                    async move {
                        let mut manager = transcription_manager.write().await;
                        manager.transcribe(&samples, &language).await
                    },
                    |result| Message::TranscriptionComplete(result.map_err(|e| e.to_string())),
                );
            }
            Message::TranscriptionComplete(result) => {
                match result {
                    Ok(tr) => {
                        info!("Transcription complete: '{}' ({:.2}s)", tr.text, tr.audio_duration_secs);

                        // Insert text into the previously active window
                        if !tr.text.is_empty() {
                            if let Some(ref platform) = self.platform {
                                // Restore focus to original window
                                if let Err(e) = platform.restore_foreground_window() {
                                    warn!("Failed to restore foreground window: {}", e);
                                }

                                // Small delay for window focus to settle
                                std::thread::sleep(Duration::from_millis(100));

                                // Insert the transcribed text
                                if let Err(e) = platform.insert_text(&tr.text) {
                                    error!("Failed to insert text: {}", e);
                                    self.error_message = Some(format!("Failed to insert text: {}", e));
                                } else {
                                    info!("Text inserted successfully");
                                }
                            }
                        }

                        // Add to history
                        self.recent_transcriptions.insert(0, tr);
                        if self.recent_transcriptions.len() > 10 {
                            self.recent_transcriptions.truncate(10);
                        }
                    }
                    Err(e) => {
                        error!("Transcription failed: {}", e);
                        self.error_message = Some(e);
                    }
                }
            }
            Message::TranscriptionCopied(id) => {
                if let Some(tr) = self.recent_transcriptions.iter().find(|t| t.id == id) {
                    // Copy to clipboard using platform
                    if let Some(ref platform) = self.platform {
                        // We'll just set to clipboard without pasting
                        // For now, reuse the insert_via_clipboard logic but skip the paste
                        info!("Copied transcription to clipboard: {}", id);
                    }
                }
            }

            // Files
            Message::FilesDropped(paths) => {
                // Add files asynchronously
                let file_processor = self.file_processor.clone();
                return Task::perform(
                    async move {
                        file_processor.add_files(paths).await
                    },
                    |ids| {
                        info!("Added {} files", ids.len());
                        Message::Tick // Refresh UI
                    }
                );
            }
            Message::FileRemove(id) => {
                let file_processor = self.file_processor.clone();
                return Task::perform(
                    async move {
                        file_processor.remove_job(id).await;
                    },
                    |_| Message::Tick
                );
            }
            Message::FilesClear => {
                let file_processor = self.file_processor.clone();
                return Task::perform(
                    async move {
                        file_processor.clear().await;
                    },
                    |_| Message::Tick
                );
            }

            // Settings
            Message::SettingsOpen => {
                self.view = View::Settings;
            }
            Message::SettingsClose => {
                self.view = View::Main;
                if let Err(e) = self.config.save() {
                    error!("Failed to save config: {}", e);
                }
            }
            Message::SettingChanged(change) => {
                match change {
                    crate::messages::SettingChange::Language(lang) => {
                        self.config.ui_language = lang;
                    }
                    crate::messages::SettingChange::Model(model) => {
                        self.config.model_size = model;
                    }
                    crate::messages::SettingChange::Accelerator(acc) => {
                        self.config.accelerator = acc;
                    }
                    crate::messages::SettingChange::Hotkey(hk) => {
                        // Unregister old hotkey and register new one
                        if let Some(ref mut platform) = self.platform {
                            let _ = platform.unregister_hotkey();
                            if let Err(e) = platform.register_hotkey(&hk) {
                                error!("Failed to register new hotkey: {}", e);
                            }
                        }
                        self.config.record_hotkey = hk;
                    }
                    crate::messages::SettingChange::Microphone(mic) => {
                        self.config.microphone = mic.clone();
                        let _ = self.audio_recorder.select_device(mic.as_deref());
                    }
                    crate::messages::SettingChange::LlmProvider(llm) => {
                        self.config.llm = llm;
                    }
                }
            }

            // License
            Message::LicenseKeyChanged(key) => {
                self.config.license_key = if key.is_empty() { None } else { Some(key) };
            }
            Message::LicenseValidate => {
                if let Some(ref key) = self.config.license_key {
                    let key = key.clone();
                    let device_id = self.platform.as_ref()
                        .and_then(|p| p.get_device_id().ok())
                        .unwrap_or_else(|| "unknown".to_string());

                    return Task::perform(
                        async move {
                            // Make API call to validate license
                            let client = reqwest::Client::new();
                            let response = client
                                .post("https://mindtype.space/api/license/validate")
                                .json(&serde_json::json!({
                                    "license_key": key,
                                    "device_id": device_id,
                                    "app_version": env!("CARGO_PKG_VERSION"),
                                }))
                                .send()
                                .await?;

                            if response.status().is_success() {
                                let data: serde_json::Value = response.json().await?;
                                // Parse response and return status
                                if data["valid"].as_bool().unwrap_or(false) {
                                    Ok(LicenseStatus::Valid {
                                        plan: mindtype_licensing::Plan::Personal,
                                        expires_at: None,
                                    })
                                } else {
                                    Ok(LicenseStatus::Invalid)
                                }
                            } else {
                                Ok(LicenseStatus::Invalid)
                            }
                        },
                        |result: Result<LicenseStatus, reqwest::Error>| {
                            Message::LicenseValidated(result.map_err(|e| e.to_string()))
                        },
                    );
                }
            }
            Message::LicenseValidated(result) => {
                match result {
                    Ok(status) => {
                        self.license_status = status;
                    }
                    Err(e) => {
                        error!("License validation failed: {}", e);
                        self.license_status = LicenseStatus::Invalid;
                    }
                }
            }
            Message::CreditsUpdated(balance) => {
                self.credits_balance = balance;
            }

            // System
            Message::TrayAction(action) => {
                match action {
                    mindtype_platform::TrayAction::ShowWindow => {
                        // TODO: Show window from tray
                    }
                    mindtype_platform::TrayAction::Settings => {
                        self.view = View::Settings;
                    }
                    mindtype_platform::TrayAction::Quit => {
                        std::process::exit(0);
                    }
                }
            }
            Message::WindowMinimize => {
                // TODO: Minimize to tray
            }
            Message::WindowRestore => {
                // TODO: Restore from tray
            }
            Message::Quit => {
                std::process::exit(0);
            }

            // Misc
            Message::Tick => {
                // Check for hotkey events - polling approach since we can't easily pass the receiver
                // This is triggered by the time subscription
            }
            Message::ConfigSaved => {}
            Message::Loaded(_) => {}
            Message::RecordingStarted => {}
            Message::FileJobUpdate(_, _, _) => {}
            Message::FilesProcessComplete => {}
        }

        Task::none()
    }

    fn view(&self) -> Element<Message> {
        let content: Element<Message> = match self.view {
            View::Wizard => wizard_view::view(self),
            View::Main => main_view::view(self),
            View::Files => files_view::view(self),
            View::Settings => settings_view::view(self),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(0)
            .style(|_| theme::window_container())
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        // Tick subscription for UI updates (60fps for smooth animations)
        // Also used for polling hotkey state from platform layer
        iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
    }

    fn theme(&self) -> Theme {
        Theme::Light
    }
}

// Getters for UI
impl MindTypeApp {
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn wizard_step(&self) -> WizardStep {
        self.wizard_step
    }

    pub fn wizard_api_key(&self) -> &str {
        &self.wizard_api_key
    }

    pub fn wizard_model_progress(&self) -> Option<u8> {
        self.wizard_model_progress
    }

    pub fn main_tab(&self) -> MainTab {
        self.main_tab
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    pub fn audio_level(&self) -> &AudioLevel {
        &self.audio_level
    }

    pub fn recent_transcriptions(&self) -> &[TranscriptionResult] {
        &self.recent_transcriptions
    }

    pub fn license_status(&self) -> &LicenseStatus {
        &self.license_status
    }

    pub fn credits_balance(&self) -> u32 {
        self.credits_balance
    }

    pub fn available_microphones(&self) -> Vec<String> {
        self.audio_recorder.available_devices().unwrap_or_default()
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

/// Run the application
pub fn run() -> iced::Result {
    iced::application(MindTypeApp::title, MindTypeApp::update, MindTypeApp::view)
        .subscription(MindTypeApp::subscription)
        .theme(MindTypeApp::theme)
        .window_size((480.0, 400.0))
        .run_with(MindTypeApp::new)
}
