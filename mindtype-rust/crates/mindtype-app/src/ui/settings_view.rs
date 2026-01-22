//! Settings view

use crate::app::MindTypeApp;
use crate::messages::{Message, SettingChange};
use crate::theme;
use crate::ui::components::{helper, secondary_button, separator};

use iced::widget::{column, container, pick_list, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};
use mindtype_core::{AcceleratorPref, LlmConfig};
use mindtype_whisper::ModelSize;

pub fn view(app: &MindTypeApp) -> Element<Message> {
    let config = app.config();

    column![
        // Title bar
        container(
            row![
                text("▓").size(14),
                Space::with_width(8),
                text("Settings").size(14),
                Space::with_width(Length::Fill),
                secondary_button("×")
                    .on_press(Message::SettingsClose)
                    .padding([2, 8]),
            ]
            .align_y(Alignment::Center)
            .padding([4, 8]),
        )
        .style(|_| theme::title_bar())
        .width(Length::Fill),
        // Content
        scrollable(
            container(
                column![
                    // General section
                    section_header("General"),
                    setting_row(
                        "Interface Language",
                        pick_list(
                            vec!["English", "Русский", "Español", "Deutsch", "Français", "中文"],
                            Some(lang_display(&config.ui_language)),
                            |s| Message::SettingChanged(SettingChange::Language(
                                lang_code(s).to_string()
                            )),
                        )
                        .padding(6)
                        .width(150),
                    ),
                    setting_row(
                        "Mode",
                        text(config.app_mode.to_string()).size(14),
                    ),
                    Space::with_height(16),
                    // Transcription section
                    section_header("Transcription"),
                    setting_row(
                        "Whisper Model",
                        pick_list(
                            vec![
                                "Tiny (75 MB)",
                                "Small (466 MB)",
                                "Medium (1.5 GB)",
                                "Large V3 (3 GB)",
                            ],
                            Some(model_display(config.model_size)),
                            |s| Message::SettingChanged(SettingChange::Model(model_from_display(s))),
                        )
                        .padding(6)
                        .width(150),
                    ),
                    setting_row(
                        "Accelerator",
                        pick_list(
                            vec!["Auto", "DirectML (GPU/NPU)", "CUDA (NVIDIA)", "CPU"],
                            Some(accel_display(config.accelerator)),
                            |s| Message::SettingChanged(SettingChange::Accelerator(
                                accel_from_display(s)
                            )),
                        )
                        .padding(6)
                        .width(150),
                    ),
                    Space::with_height(16),
                    // Audio section
                    section_header("Audio"),
                    setting_row(
                        "Microphone",
                        pick_list(
                            std::iter::once("Default".to_string())
                                .chain(app.available_microphones())
                                .collect::<Vec<_>>(),
                            Some(config.microphone.clone().unwrap_or_else(|| "Default".to_string())),
                            |s| {
                                if s == "Default" {
                                    Message::SettingChanged(SettingChange::Microphone(None))
                                } else {
                                    Message::SettingChanged(SettingChange::Microphone(Some(s)))
                                }
                            },
                        )
                        .padding(6)
                        .width(200),
                    ),
                    setting_row(
                        "Record Hotkey",
                        text_input("Ctrl+Alt+V", &config.record_hotkey)
                            .on_input(|s| Message::SettingChanged(SettingChange::Hotkey(s)))
                            .padding(6)
                            .width(150)
                            .style(theme::text_input_style),
                    ),
                    Space::with_height(16),
                    // AI section
                    section_header("AI Summary"),
                    setting_row(
                        "Provider",
                        text(provider_display(&config.llm)).size(14),
                    ),
                    if config.use_mindtype_cloud {
                        column![
                            helper("Using MindType Cloud with credits."),
                            row![
                                text("Credits: ").size(14),
                                text(app.credits_balance().to_string()).size(14),
                            ],
                        ]
                        .spacing(4)
                    } else {
                        column![helper("Using custom API provider.")]
                    },
                    Space::with_height(16),
                    // License section
                    section_header("License"),
                    setting_row(
                        "License Key",
                        text_input(
                            "XXXX-XXXX-XXXX-XXXX",
                            config.license_key.as_deref().unwrap_or(""),
                        )
                        .on_input(Message::LicenseKeyChanged)
                        .padding(6)
                        .width(200)
                        .style(theme::text_input_style),
                    ),
                    row![
                        secondary_button("Validate").on_press(Message::LicenseValidate),
                        Space::with_width(16),
                        text(license_status_display(app.license_status())).size(12),
                    ]
                    .align_y(Alignment::Center)
                    .spacing(8),
                    Space::with_height(24),
                    // About
                    separator(),
                    Space::with_height(8),
                    helper(&format!("MindType v{}", env!("CARGO_PKG_VERSION"))),
                    helper("© 2024 MindType. All rights reserved."),
                ]
                .spacing(8)
            )
            .padding(16)
        )
        .height(Length::Fill),
    ]
    .into()
}

fn section_header(title: &str) -> Element<'static, Message> {
    let title_owned = title.to_owned();
    column![
        text(title_owned).size(16),
        container(Space::new(Length::Fill, 1))
            .style(|_| container::Style {
                background: Some(iced::Background::Color(theme::colors::BLACK)),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(1),
    ]
    .spacing(4)
    .into()
}

fn setting_row<'a>(
    label_text: &str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let label_owned = label_text.to_owned();
    row![
        text(label_owned).size(14).width(150),
        control.into(),
    ]
    .align_y(Alignment::Center)
    .spacing(16)
    .into()
}

fn lang_display(code: &str) -> &str {
    match code {
        "en" => "English",
        "ru" => "Русский",
        "es" => "Español",
        "de" => "Deutsch",
        "fr" => "Français",
        "zh" => "中文",
        _ => "English",
    }
}

fn lang_code(display: &str) -> &str {
    match display {
        "English" => "en",
        "Русский" => "ru",
        "Español" => "es",
        "Deutsch" => "de",
        "Français" => "fr",
        "中文" => "zh",
        _ => "en",
    }
}

fn model_display(size: ModelSize) -> &'static str {
    match size {
        ModelSize::Tiny => "Tiny (75 MB)",
        ModelSize::Small => "Small (466 MB)",
        ModelSize::Medium => "Medium (1.5 GB)",
        ModelSize::LargeV3 => "Large V3 (3 GB)",
    }
}

fn model_from_display(s: &str) -> ModelSize {
    if s.starts_with("Tiny") {
        ModelSize::Tiny
    } else if s.starts_with("Small") {
        ModelSize::Small
    } else if s.starts_with("Medium") {
        ModelSize::Medium
    } else {
        ModelSize::LargeV3
    }
}

fn accel_display(accel: AcceleratorPref) -> &'static str {
    match accel {
        AcceleratorPref::Auto => "Auto",
        AcceleratorPref::DirectML => "DirectML (GPU/NPU)",
        AcceleratorPref::Cuda => "CUDA (NVIDIA)",
        AcceleratorPref::Cpu => "CPU",
    }
}

fn accel_from_display(s: &str) -> AcceleratorPref {
    if s.starts_with("DirectML") {
        AcceleratorPref::DirectML
    } else if s.starts_with("CUDA") {
        AcceleratorPref::Cuda
    } else if s == "CPU" {
        AcceleratorPref::Cpu
    } else {
        AcceleratorPref::Auto
    }
}

fn provider_display(llm: &LlmConfig) -> &'static str {
    match llm {
        LlmConfig::MindTypeCloud => "MindType Cloud",
        LlmConfig::OpenAi { .. } => "OpenAI",
        LlmConfig::Anthropic { .. } => "Anthropic",
        LlmConfig::Gemini { .. } => "Google Gemini",
        LlmConfig::OpenRouter { .. } => "OpenRouter",
        LlmConfig::Yandex { .. } => "Yandex GPT",
        LlmConfig::Ollama { .. } => "Ollama (Local)",
    }
}

fn license_status_display(status: &mindtype_licensing::LicenseStatus) -> String {
    match status {
        mindtype_licensing::LicenseStatus::Valid { plan, .. } => {
            format!("✓ Valid ({})", plan)
        }
        mindtype_licensing::LicenseStatus::Trial { days_left, .. } => {
            format!("Trial ({} days left)", days_left)
        }
        mindtype_licensing::LicenseStatus::TrialExpired => "Trial expired".to_string(),
        mindtype_licensing::LicenseStatus::Invalid => "Invalid key".to_string(),
        mindtype_licensing::LicenseStatus::Expired => "Expired".to_string(),
        mindtype_licensing::LicenseStatus::DeviceLimitReached => "Device limit reached".to_string(),
        mindtype_licensing::LicenseStatus::NotConfigured => "Not configured".to_string(),
    }
}
