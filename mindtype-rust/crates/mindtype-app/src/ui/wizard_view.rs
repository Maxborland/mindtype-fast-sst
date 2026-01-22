//! Setup wizard views - Mac OS 7/8 style
//! Based on Figma design "Setup Wizard Screens Design"

use crate::app::MindTypeApp;
use crate::messages::{Message, WizardStep};
use crate::theme;

use iced::widget::{button, checkbox, column, container, progress_bar, radio, row, text, text_input, Space};
use iced::{Alignment, Element, Length};
use mindtype_whisper::ModelSize;

/// Main wizard view
pub fn view(app: &MindTypeApp) -> Element<Message> {
    let step = app.wizard_step();

    // Build content based on step
    let content = match step {
        WizardStep::Welcome => welcome_step(app),
        WizardStep::Mode => mode_step(app),
        WizardStep::Provider => provider_step(app),
        WizardStep::ApiKeys => api_keys_step(app),
        WizardStep::Model => model_step(app),
        WizardStep::Microphone => microphone_step(app),
        WizardStep::Hotkey => hotkey_step(app),
        WizardStep::Complete => complete_step(app),
    };

    // Navigation buttons
    let can_back = step.prev().is_some();
    let can_next = step.next().is_some();
    let is_complete = step == WizardStep::Complete;

    let nav_buttons = if is_complete {
        // Only Finish button for complete step
        row![
            Space::with_width(Length::Fill),
            button(text("Finish").size(14))
                .padding([8, 24])
                .style(theme::primary_button)
                .on_press(Message::WizardComplete),
        ]
    } else {
        row![
            if can_back {
                button(text("< Back").size(14))
                    .padding([8, 16])
                    .style(theme::primary_button)
                    .on_press(Message::WizardBack)
            } else {
                button(text("< Back").size(14))
                    .padding([8, 16])
                    .style(theme::disabled_button)
            },
            Space::with_width(Length::Fill),
            if can_next {
                button(text("Next >").size(14))
                    .padding([8, 16])
                    .style(theme::primary_button)
                    .on_press(Message::WizardNext)
            } else {
                button(text("Next >").size(14))
                    .padding([8, 16])
                    .style(theme::disabled_button)
            },
        ]
    };

    // Main window structure
    column![
        // Title bar with stripes
        mac_title_bar("MindType Setup"),
        // Content area
        container(
            column![
                content,
                Space::with_height(Length::Fill),
                nav_buttons,
                Space::with_height(8),
            ]
            .height(Length::Fill)
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| theme::wizard_window()),
    ]
    .into()
}

/// Mac OS 7 style title bar with horizontal stripes
fn mac_title_bar(title: &str) -> Element<'static, Message> {
    let title_owned = title.to_owned();

    // Close button box
    let close_box = container(
        text("×").size(12)
    )
    .width(14)
    .height(14)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(|_| theme::close_button_box());

    // Title with stripes (using text characters for stripes)
    let stripes = "══════════";

    container(
        row![
            close_box,
            Space::with_width(8),
            text(stripes).size(10),
            Space::with_width(8),
            text(title_owned).size(14),
            Space::with_width(8),
            text(stripes).size(10),
            Space::with_width(Length::Fill),
        ]
        .align_y(Alignment::Center)
        .padding([6, 8]),
    )
    .style(|_| theme::title_bar())
    .width(Length::Fill)
    .into()
}

/// Welcome step - Language selection with flags and radio buttons
fn welcome_step(app: &MindTypeApp) -> Element<Message> {
    let current_lang = &app.config().ui_language;

    // Language options with flags
    let languages = [
        ("en", "🇺🇸", "English"),
        ("ru", "🇷🇺", "Русский"),
        ("es", "🇪🇸", "Español"),
        ("de", "🇩🇪", "Deutsch"),
        ("fr", "🇫🇷", "Français"),
        ("zh", "🇨🇳", "中文"),
    ];

    let lang_radios = column(
        languages
            .into_iter()
            .map(|(code, flag, name)| {
                let is_selected = current_lang == code;
                let code_owned = code.to_string();

                row![
                    radio("", code, Some(current_lang.as_str()), move |_| {
                        Message::WizardSelectLanguage(code_owned.clone())
                    })
                    .size(18)
                    .style(theme::radio_style),
                    Space::with_width(8),
                    text(flag).size(18),
                    Space::with_width(8),
                    text(name).size(16),
                ]
                .align_y(Alignment::Center)
                .spacing(4)
                .into()
            })
            .collect::<Vec<_>>()
    )
    .spacing(12);

    // Wrap in inset panel
    let lang_panel = container(lang_radios)
        .padding(16)
        .style(|_| theme::inset_container());

    column![
        // Big title
        text("Welcome to MindType")
            .size(28),
        Space::with_height(8),
        text("Choose your language").size(16),
        Space::with_height(24),
        lang_panel,
    ]
    .spacing(0)
    .into()
}

/// Mode selection step (Simple vs Advanced)
fn mode_step(app: &MindTypeApp) -> Element<Message> {
    let is_simple = app.config().app_mode == mindtype_core::AppMode::Simple;

    column![
        text("Choose Your Mode").size(24),
        Space::with_height(24),
        // Simple Mode panel
        container(
            column![
                radio(
                    "Simple Mode (Recommended)",
                    true,
                    Some(is_simple),
                    Message::WizardSelectMode,
                )
                .size(16)
                .spacing(8)
                .style(theme::radio_style),
                Space::with_height(4),
                text("  Just works. Minimal settings.").size(13).color(theme::colors::TEXT_SECONDARY),
            ]
            .spacing(0)
        )
        .padding(16)
        .style(|_| theme::panel_container()),
        Space::with_height(12),
        // Advanced Mode panel
        container(
            column![
                radio(
                    "Advanced Mode",
                    false,
                    Some(is_simple),
                    Message::WizardSelectMode,
                )
                .size(16)
                .spacing(8)
                .style(theme::radio_style),
                Space::with_height(4),
                text("  Full control. Custom AI providers.").size(13).color(theme::colors::TEXT_SECONDARY),
            ]
            .spacing(0)
        )
        .padding(16)
        .style(|_| theme::panel_container()),
    ]
    .spacing(0)
    .into()
}

/// AI Provider choice step
fn provider_step(app: &MindTypeApp) -> Element<Message> {
    let use_cloud = app.config().use_mindtype_cloud;

    column![
        text("AI Features Setup").size(24),
        Space::with_height(8),
        text("MindType uses AI for summarization").size(14).color(theme::colors::TEXT_SECONDARY),
        Space::with_height(24),
        // MindType Cloud panel
        container(
            column![
                radio(
                    "MindType Cloud (Recommended)",
                    true,
                    Some(use_cloud),
                    Message::WizardSelectProvider,
                )
                .size(16)
                .spacing(8)
                .style(theme::radio_style),
                Space::with_height(8),
                row![
                    Space::with_width(24),
                    checkbox("No API keys needed", true)
                        .size(14)
                        .style(theme::checkbox_style),
                ],
                row![
                    Space::with_width(24),
                    checkbox("Buy credits - use when needed", true)
                        .size(14)
                        .style(theme::checkbox_style),
                ],
                row![
                    Space::with_width(24),
                    checkbox("Credits never expire", true)
                        .size(14)
                        .style(theme::checkbox_style),
                ],
            ]
            .spacing(4)
        )
        .padding(16)
        .style(|_| theme::panel_container()),
        Space::with_height(12),
        // Own API Key panel
        container(
            column![
                radio(
                    "Own API Key (free)",
                    false,
                    Some(use_cloud),
                    Message::WizardSelectProvider,
                )
                .size(16)
                .spacing(8)
                .style(theme::radio_style),
                Space::with_height(4),
                text("  Need key from OpenAI, Claude, etc.").size(13).color(theme::colors::TEXT_SECONDARY),
            ]
            .spacing(0)
        )
        .padding(16)
        .style(|_| theme::panel_container()),
    ]
    .spacing(0)
    .into()
}

/// API Keys entry step
fn api_keys_step(app: &MindTypeApp) -> Element<Message> {
    column![
        text("API Configuration").size(24),
        Space::with_height(24),
        text("Enter your API key:").size(14),
        Space::with_height(8),
        text_input("sk-...", app.wizard_api_key())
            .on_input(Message::WizardApiKeyChanged)
            .padding(10)
            .size(14)
            .style(theme::text_input_style),
        Space::with_height(8),
        text("Your API key is stored locally and never shared.")
            .size(12)
            .color(theme::colors::TEXT_SECONDARY),
        Space::with_height(24),
        text("Supported providers:").size(14),
        Space::with_height(8),
        text("• OpenAI (GPT-4o, GPT-4o-mini)").size(13).color(theme::colors::TEXT_SECONDARY),
        text("• Anthropic (Claude 3.5 Sonnet/Haiku)").size(13).color(theme::colors::TEXT_SECONDARY),
        text("• Google Gemini").size(13).color(theme::colors::TEXT_SECONDARY),
        text("• OpenRouter (200+ models)").size(13).color(theme::colors::TEXT_SECONDARY),
        text("• Ollama (local, no key needed)").size(13).color(theme::colors::TEXT_SECONDARY),
    ]
    .spacing(2)
    .into()
}

/// Model download step
fn model_step(app: &MindTypeApp) -> Element<Message> {
    let current = app.config().model_size;

    column![
        text("Download Speech Model").size(24),
        Space::with_height(8),
        text("Required for voice recognition").size(14).color(theme::colors::TEXT_SECONDARY),
        Space::with_height(24),
        // Whisper Small panel (recommended)
        container(
            column![
                radio(
                    "Whisper Small (Recommended)",
                    ModelSize::Small,
                    Some(current),
                    Message::WizardSelectModel,
                )
                .size(16)
                .spacing(8)
                .style(theme::radio_style),
                text("  244 MB • Fast • Good quality").size(13).color(theme::colors::TEXT_SECONDARY),
            ]
            .spacing(4)
        )
        .padding(16)
        .style(|_| theme::panel_container()),
        Space::with_height(12),
        // Whisper Large panel
        container(
            column![
                radio(
                    "Whisper Large",
                    ModelSize::LargeV3,
                    Some(current),
                    Message::WizardSelectModel,
                )
                .size(16)
                .spacing(8)
                .style(theme::radio_style),
                text("  1.5 GB • Slower • Best quality").size(13).color(theme::colors::TEXT_SECONDARY),
            ]
            .spacing(4)
        )
        .padding(16)
        .style(|_| theme::panel_container()),
        Space::with_height(16),
        // Progress bar section
        model_progress_section(app),
    ]
    .spacing(0)
    .into()
}

/// Model download progress section
fn model_progress_section(app: &MindTypeApp) -> Element<Message> {
    if let Some(progress) = app.wizard_model_progress() {
        let downloaded_mb = (progress as f32 / 100.0 * 244.0) as u32;
        container(
            column![
                text("Downloading...").size(14),
                Space::with_height(8),
                progress_bar(0.0..=100.0, progress as f32)
                    .height(16)
                    .style(theme::progress_bar_style),
                Space::with_height(4),
                text(format!("{} MB / 244 MB", downloaded_mb))
                    .size(12)
                    .color(theme::colors::TEXT_SECONDARY),
            ]
            .spacing(0)
        )
        .padding(12)
        .style(|_| theme::inset_container())
        .into()
    } else if app.config().is_model_downloaded() {
        container(
            row![
                text("✓").size(16).color(theme::colors::SUCCESS),
                Space::with_width(8),
                text("Model ready").size(14),
            ]
            .align_y(Alignment::Center)
        )
        .padding(12)
        .style(|_| theme::panel_container())
        .into()
    } else {
        Space::with_height(0).into()
    }
}

/// Microphone setup step
fn microphone_step(app: &MindTypeApp) -> Element<Message> {
    let mics = app.available_microphones();
    let current = app.config().microphone.clone();
    let selected = current.unwrap_or_else(|| "Default".to_string());

    let mic_options: Vec<String> = std::iter::once("Default".to_string())
        .chain(mics.into_iter())
        .collect();

    column![
        text("Microphone Setup").size(24),
        Space::with_height(24),
        text("Select your microphone:").size(14),
        Space::with_height(8),
        iced::widget::pick_list(
            mic_options,
            Some(selected),
            |s| {
                if s == "Default" {
                    Message::WizardMicrophoneSelected(None)
                } else {
                    Message::WizardMicrophoneSelected(Some(s))
                }
            },
        )
        .padding(8)
        .width(Length::Fill)
        .style(theme::pick_list_style),
        Space::with_height(16),
        row![
            button(text("Test").size(14))
                .padding([8, 16])
                .style(theme::primary_button)
                .on_press(Message::WizardMicrophoneTest),
            Space::with_width(16),
            // Level meter placeholder
            container(
                text("░░░░░░░░░░").font(iced::Font::MONOSPACE).size(14)
            )
            .padding([4, 8])
            .style(|_| theme::inset_container()),
        ]
        .align_y(Alignment::Center),
        Space::with_height(16),
        text("Speak to test. The meter should move.")
            .size(13)
            .color(theme::colors::TEXT_SECONDARY),
    ]
    .spacing(0)
    .into()
}

/// Hotkey setup step
fn hotkey_step(app: &MindTypeApp) -> Element<Message> {
    let current = app.config().record_hotkey.clone();

    column![
        text("Hotkey Setup").size(24),
        Space::with_height(24),
        text("Press and hold this key combo to record:").size(14),
        Space::with_height(8),
        text_input("Ctrl+Alt+V", &current)
            .on_input(Message::WizardHotkeyChanged)
            .padding(10)
            .size(14)
            .style(theme::text_input_style),
        Space::with_height(16),
        text("Common choices:").size(14),
        Space::with_height(8),
        text("• Ctrl+Alt+V - Default").size(13).color(theme::colors::TEXT_SECONDARY),
        text("• Ctrl+Shift+Space - Alternative").size(13).color(theme::colors::TEXT_SECONDARY),
        text("• F9 - Simple function key").size(13).color(theme::colors::TEXT_SECONDARY),
        Space::with_height(24),
        container(
            column![
                text("How it works:").size(14),
                Space::with_height(8),
                text("1. Press and HOLD the hotkey").size(13),
                text("2. Speak your text").size(13),
                text("3. Release the hotkey").size(13),
                text("4. Text appears at cursor").size(13),
            ]
            .spacing(4)
        )
        .padding(16)
        .style(|_| theme::panel_container()),
    ]
    .spacing(0)
    .into()
}

/// Setup complete step
fn complete_step(_app: &MindTypeApp) -> Element<Message> {
    let credits = 750; // Default starting credits

    column![
        // Big checkmark
        container(
            text("✓").size(80)
        )
        .width(Length::Fill)
        .center_x(Length::Fill),
        Space::with_height(16),
        // Title
        container(
            text("Setup Complete!").size(28)
        )
        .width(Length::Fill)
        .center_x(Length::Fill),
        Space::with_height(8),
        container(
            text("MindType is ready to use").size(16).color(theme::colors::TEXT_SECONDARY)
        )
        .width(Length::Fill)
        .center_x(Length::Fill),
        Space::with_height(32),
        // Info panel
        container(
            column![
                text(format!("Your balance: {} credits", credits)).size(16),
                Space::with_height(8),
                text("Tip: MindType runs in system tray.").size(13).color(theme::colors::TEXT_SECONDARY),
                text("Click the icon to open.").size(13).color(theme::colors::TEXT_SECONDARY),
            ]
            .width(Length::Fill)
        )
        .padding(16)
        .width(Length::Fill)
        .style(|_| theme::panel_container()),
    ]
    .spacing(0)
    .align_x(Alignment::Center)
    .into()
}
