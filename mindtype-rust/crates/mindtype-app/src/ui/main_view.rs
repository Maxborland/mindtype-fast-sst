//! Main recording view

use crate::app::MindTypeApp;
use crate::messages::Message;
use crate::theme;
use crate::ui::components::{
    credits_display, heading, helper, inset_panel, label, level_meter, panel,
    primary_button, recording_indicator, secondary_button, separator,
};

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};
use mindtype_core::AppMode;

pub fn view(app: &MindTypeApp) -> Element<Message> {
    let is_simple = app.config().app_mode == AppMode::Simple;

    if is_simple {
        simple_view(app)
    } else {
        advanced_view(app)
    }
}

fn simple_view(app: &MindTypeApp) -> Element<'static, Message> {
    let hotkey = app.config().record_hotkey.clone();
    let is_recording = app.is_recording();
    let level = app.audio_level().rms;

    let status_text = if is_recording {
        format!("Recording... {}", level_indicator(level))
    } else {
        format!("Press {} to record", hotkey)
    };

    let recent = app.recent_transcriptions();
    let history_content = if recent.is_empty() {
        column![helper("Your recent transcriptions will appear here.")]
    } else {
        column(
            recent
                .iter()
                .take(5)
                .map(|tr| {
                    let preview = if tr.text.len() > 40 {
                        format!("{}...", &tr.text[..40])
                    } else {
                        tr.text.clone()
                    };

                    row![
                        text("•").size(14),
                        Space::with_width(8),
                        text(preview).size(14),
                        Space::with_width(Length::Fill),
                        secondary_button("copy")
                            .on_press(Message::TranscriptionCopied(tr.id))
                            .padding([4, 8]),
                    ]
                    .align_y(Alignment::Center)
                    .spacing(4)
                    .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(8)
    };

    column![
        // Title bar
        container(
            row![
                text("▓").size(14),
                Space::with_width(8),
                text("MindType").size(14),
                Space::with_width(Length::Fill),
                recording_indicator(is_recording),
            ]
            .align_y(Alignment::Center)
            .padding([4, 8]),
        )
        .style(|_| theme::title_bar())
        .width(Length::Fill),
        // Main content
        container(
            column![
                Space::with_height(20),
                // Status
                container(
                    text(status_text)
                        .size(16)
                        .align_x(iced::alignment::Horizontal::Center)
                )
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
                Space::with_height(20),
                // Recording level
                if is_recording {
                    container(level_meter(level))
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center)
                } else {
                    container(Space::new(0, 0))
                },
                Space::with_height(20),
                // History panel
                inset_panel(
                    column![
                        label("Recent transcriptions:"),
                        Space::with_height(8),
                        scrollable(history_content).height(120),
                    ]
                ),
                Space::with_height(Length::Fill),
                // Bottom bar
                separator(),
                row![
                    credits_display(app.credits_balance()),
                    Space::with_width(Length::Fill),
                    secondary_button("Settings")
                        .on_press(Message::SettingsOpen),
                ]
                .padding([8, 12])
                .align_y(Alignment::Center),
            ]
        )
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill),
    ]
    .into()
}

fn advanced_view(app: &MindTypeApp) -> Element<Message> {
    let tab = app.main_tab();

    // Tab buttons
    let tabs = row![
        tab_button("Recorder", tab == crate::messages::MainTab::Recorder)
            .on_press(Message::SetView(crate::messages::View::Main)),
        tab_button("Files", tab == crate::messages::MainTab::Files)
            .on_press(Message::SetView(crate::messages::View::Files)),
        tab_button("Settings", tab == crate::messages::MainTab::Settings)
            .on_press(Message::SettingsOpen),
    ]
    .spacing(0);

    let content = match tab {
        crate::messages::MainTab::Recorder => recorder_tab(app),
        crate::messages::MainTab::Files => {
            // Redirect to files view
            column![text("Files").size(14)].into()
        }
        crate::messages::MainTab::Settings => {
            // Redirect to settings view
            column![text("Settings").size(14)].into()
        }
    };

    column![
        // Title bar
        container(
            row![
                text("▓").size(14),
                Space::with_width(8),
                text("MindType").size(14),
                Space::with_width(Length::Fill),
                recording_indicator(app.is_recording()),
            ]
            .align_y(Alignment::Center)
            .padding([4, 8]),
        )
        .style(|_| theme::title_bar())
        .width(Length::Fill),
        // Tabs
        tabs,
        // Content
        container(content)
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .into()
}

fn recorder_tab(app: &MindTypeApp) -> Element<Message> {
    let hotkey = &app.config().record_hotkey;
    let is_recording = app.is_recording();
    let level = app.audio_level().rms;

    let recent = app.recent_transcriptions();

    column![
        // Instructions
        panel(
            column![
                if is_recording {
                    text(format!("Recording... {}", level_indicator(level))).size(14)
                } else {
                    text(format!("Press {} to record", hotkey)).size(14)
                },
                Space::with_height(8),
                level_meter(level),
            ]
            .align_x(Alignment::Center)
        ),
        Space::with_height(16),
        // History
        label("History:"),
        Space::with_height(8),
        inset_panel(
            scrollable(
                column(
                    recent
                        .iter()
                        .map(|tr| {
                            column![
                                row![
                                    text(tr.text.clone()).size(13),
                                    Space::with_width(Length::Fill),
                                    secondary_button("copy")
                                        .on_press(Message::TranscriptionCopied(tr.id))
                                        .padding([2, 6]),
                                ]
                                .align_y(Alignment::Start),
                                helper(&format!(
                                    "{} • {:.1}s",
                                    tr.language,
                                    tr.audio_duration_secs
                                )),
                            ]
                            .spacing(2)
                            .into()
                        })
                        .collect::<Vec<_>>(),
                )
                .spacing(12)
            )
            .height(Length::Fill)
        ),
    ]
    .spacing(4)
    .into()
}

fn tab_button(label: &str, active: bool) -> button::Button<'static, Message, iced::Theme, iced::Renderer> {
    button(
        text(label.to_owned())
            .size(14)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .padding([8, 16])
    .style(theme::tab_button(active))
}

fn level_indicator(level: f32) -> String {
    let bars = 10;
    let filled = (level * bars as f32).round() as usize;
    (0..bars)
        .map(|i| if i < filled { '█' } else { '░' })
        .collect()
}
