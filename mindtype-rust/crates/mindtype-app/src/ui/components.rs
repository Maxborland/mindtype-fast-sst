//! Reusable UI components (System 7 Platinum style)

use crate::messages::Message;
use crate::theme;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length};

/// System 7 style title bar
pub fn title_bar(title: &str) -> Element<'static, Message> {
    let title_owned = title.to_owned();
    container(
        row![
            // Close button placeholder (System 7 style)
            container(Space::new(12, 12))
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(theme::colors::CONTENT_BG)),
                    border: iced::Border {
                        color: theme::colors::BORDER_DARK,
                        width: 1.0,
                        radius: 2.0.into(),
                    },
                    ..Default::default()
                }),
            Space::with_width(12),
            text(title_owned).size(14),
            Space::with_width(Length::Fill),
        ]
        .align_y(Alignment::Center)
        .padding([6, 10]),
    )
    .style(|_| theme::title_bar())
    .width(Length::Fill)
    .into()
}

/// Primary button with System 7 3D style
pub fn primary_button(label: &str) -> button::Button<'static, Message, iced::Theme, iced::Renderer> {
    let label_owned = label.to_owned();
    button(
        text(label_owned)
            .size(14)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .padding([8, 16])
    .style(theme::primary_button)
}

/// Secondary button (less prominent)
pub fn secondary_button(label: &str) -> button::Button<'static, Message, iced::Theme, iced::Renderer> {
    let label_owned = label.to_owned();
    button(
        text(label_owned)
            .size(14)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .padding([6, 12])
    .style(theme::secondary_button)
}

/// Accent button (blue, for primary actions like "Continue", "Finish")
pub fn accent_button(label: &str) -> button::Button<'static, Message, iced::Theme, iced::Renderer> {
    let label_owned = label.to_owned();
    button(
        text(label_owned)
            .size(14)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .padding([8, 20])
    .style(theme::accent_button)
}

/// Danger button (red, for destructive actions)
pub fn danger_button(label: &str) -> button::Button<'static, Message, iced::Theme, iced::Renderer> {
    let label_owned = label.to_owned();
    button(
        text(label_owned)
            .size(14)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .padding([8, 16])
    .style(theme::danger_button)
}

/// Panel with raised appearance
pub fn panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .style(|_| theme::panel_container())
        .padding(12)
        .into()
}

/// Inset panel (recessed appearance)
pub fn inset_panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .style(|_| theme::inset_container())
        .padding(8)
        .into()
}

/// Card (for list items, transcriptions, etc.)
pub fn card<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .style(|_| theme::card_container())
        .padding(10)
        .into()
}

/// Label text (primary color)
pub fn label(txt: &str) -> Element<'static, Message> {
    text(txt.to_owned())
        .size(14)
        .color(theme::colors::TEXT_PRIMARY)
        .into()
}

/// Secondary label text (lighter color)
pub fn secondary_label(txt: &str) -> Element<'static, Message> {
    text(txt.to_owned())
        .size(14)
        .color(theme::colors::TEXT_SECONDARY)
        .into()
}

/// Heading text (larger, bold-like)
pub fn heading(txt: &str) -> Element<'static, Message> {
    text(txt.to_owned())
        .size(18)
        .color(theme::colors::TEXT_PRIMARY)
        .into()
}

/// Small helper text (muted)
pub fn helper(txt: &str) -> Element<'static, Message> {
    text(txt.to_owned())
        .size(12)
        .color(theme::colors::TEXT_SECONDARY)
        .into()
}

/// Horizontal separator line
pub fn separator() -> Element<'static, Message> {
    container(Space::new(Length::Fill, 1))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(theme::colors::BORDER_LIGHT)),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(1)
        .into()
}

/// Audio level meter (visual bar display)
pub fn level_meter(level: f32) -> Element<'static, Message> {
    let num_bars = 12;
    let filled = (level * num_bars as f32).round() as usize;

    let bars: Vec<Element<'static, Message>> = (0..num_bars)
        .map(|i| {
            let style = if i < filled {
                if i >= num_bars - 2 {
                    // Last 2 bars are red (clipping indicator)
                    theme::level_meter_high()
                } else {
                    theme::level_meter_filled()
                }
            } else {
                theme::level_meter_empty()
            };

            container(Space::new(8, 16))
                .style(move |_| style)
                .into()
        })
        .collect();

    row(bars).spacing(2).into()
}

/// Simple text-based level meter (fallback)
pub fn level_meter_text(level: f32) -> Element<'static, Message> {
    let bars = 10;
    let filled = (level * bars as f32).round() as usize;

    let bar_chars: String = (0..bars)
        .map(|i| if i < filled { '█' } else { '░' })
        .collect();

    text(format!("[{}]", bar_chars))
        .size(14)
        .font(iced::Font::MONOSPACE)
        .color(if level > 0.8 {
            theme::colors::RECORDING_RED
        } else {
            theme::colors::SUCCESS_GREEN
        })
        .into()
}

/// Recording indicator (shows when recording is active)
pub fn recording_indicator(recording: bool) -> Element<'static, Message> {
    if recording {
        container(
            row![
                text("●").size(14).color(theme::colors::TEXT_WHITE),
                Space::with_width(4),
                text("REC").size(12).color(theme::colors::TEXT_WHITE),
            ]
            .align_y(Alignment::Center),
        )
        .style(|_| theme::recording_indicator())
        .padding([4, 10])
        .into()
    } else {
        container(
            row![
                text("○").size(14).color(theme::colors::TEXT_SECONDARY),
                Space::with_width(4),
                text("Ready").size(12).color(theme::colors::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
        )
        .style(|_| theme::idle_indicator())
        .padding([4, 10])
        .into()
    }
}

/// Credits display badge
pub fn credits_display(credits: u32) -> Element<'static, Message> {
    container(
        row![
            text("Credits: ").size(12).color(theme::colors::TEXT_SECONDARY),
            text(credits.to_string()).size(12).color(theme::colors::TEXT_PRIMARY),
        ]
        .align_y(Alignment::Center),
    )
    .style(|_| theme::panel_container())
    .padding([4, 8])
    .into()
}

/// Status badge (success)
pub fn success_badge(txt: &str) -> Element<'static, Message> {
    container(text(txt.to_owned()).size(11).color(theme::colors::TEXT_WHITE))
        .style(|_| theme::success_badge())
        .padding([2, 8])
        .into()
}

/// Status badge (warning)
pub fn warning_badge(txt: &str) -> Element<'static, Message> {
    container(text(txt.to_owned()).size(11).color(theme::colors::TEXT_PRIMARY))
        .style(|_| theme::warning_badge())
        .padding([2, 8])
        .into()
}

/// Status badge (error)
pub fn error_badge(txt: &str) -> Element<'static, Message> {
    container(text(txt.to_owned()).size(11).color(theme::colors::TEXT_WHITE))
        .style(|_| theme::error_badge())
        .padding([2, 8])
        .into()
}

/// Empty state placeholder
pub fn empty_state(title: &str, description: &str) -> Element<'static, Message> {
    let title = title.to_owned();
    let description = description.to_owned();

    container(
        column![
            text(title).size(16).color(theme::colors::TEXT_SECONDARY),
            Space::with_height(8),
            text(description).size(13).color(theme::colors::TEXT_DISABLED),
        ]
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(24)
    .center_x(Length::Fill)
    .into()
}
