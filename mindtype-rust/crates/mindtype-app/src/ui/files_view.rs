//! File processing view

use crate::app::MindTypeApp;
use crate::messages::Message;
use crate::theme;
use crate::ui::components::{
    heading, helper, inset_panel, label, panel, primary_button, secondary_button, separator,
};

use iced::widget::{button, column, container, progress_bar, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};
use mindtype_core::FileStatus;

pub fn view(app: &MindTypeApp) -> Element<Message> {
    column![
        // Title bar
        container(
            row![
                text("▓").size(14),
                Space::with_width(8),
                text("MindType - Files").size(14),
                Space::with_width(Length::Fill),
            ]
            .align_y(Alignment::Center)
            .padding([4, 8]),
        )
        .style(|_| theme::title_bar())
        .width(Length::Fill),
        // Content
        container(
            column![
                // Drop zone
                container(
                    column![
                        text("Drop files here").size(14),
                        Space::with_height(8),
                        helper("or"),
                        Space::with_height(8),
                        secondary_button("Browse..."),
                    ]
                    .align_x(Alignment::Center)
                )
                .style(|_| theme::inset_container())
                .padding(24)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
                Space::with_height(16),
                // File list
                label("Processing queue:"),
                Space::with_height(8),
                inset_panel(
                    scrollable(
                        column![
                            // Placeholder - would show actual jobs
                            file_item("meeting.mp4", FileStatus::Transcribing, 45),
                            file_item("interview.mp3", FileStatus::Completed, 100),
                            file_item("lecture.wav", FileStatus::Pending, 0),
                        ]
                        .spacing(8)
                    )
                    .height(200)
                ),
                Space::with_height(16),
                // Actions
                row![
                    secondary_button("Clear All").on_press(Message::FilesClear),
                    Space::with_width(Length::Fill),
                    primary_button("Export Results"),
                ]
                .align_y(Alignment::Center),
                Space::with_height(Length::Fill),
                // Supported formats
                separator(),
                container(
                    helper("Supported: mp3, wav, m4a, flac, mp4, mkv, avi, mov")
                )
                .padding([8, 0]),
            ]
        )
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill),
    ]
    .into()
}

fn file_item(filename: &str, status: FileStatus, progress: u8) -> Element<'static, Message> {
    let filename_owned = filename.to_owned();
    let status_text = status.to_string();
    let is_done = matches!(status, FileStatus::Completed);
    let is_failed = matches!(status, FileStatus::Failed(_));

    row![
        column![
            text(filename_owned).size(14),
            if is_done {
                text("✓ Done".to_owned())
                    .size(12)
                    .color(theme::colors::GREEN)
            } else if is_failed {
                text(status_text.clone())
                    .size(12)
                    .color(theme::colors::RED)
            } else {
                text(status_text.clone())
                    .size(12)
                    .color(theme::colors::GRAY_MID)
            },
        ]
        .spacing(2)
        .width(Length::FillPortion(2)),
        if !is_done && !is_failed {
            container(
                progress_bar(0.0..=100.0, progress as f32)
                    .height(16)
                    .style(theme::progress_bar_style)
            )
            .width(Length::FillPortion(1))
        } else {
            container(Space::new(0, 0)).width(Length::FillPortion(1))
        },
        Space::with_width(8),
        text(format!("{}%", progress))
            .size(12)
            .width(40),
    ]
    .align_y(Alignment::Center)
    .spacing(8)
    .into()
}
