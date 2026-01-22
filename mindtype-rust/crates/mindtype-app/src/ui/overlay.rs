//! Recording overlay window
//!
//! A floating pill-shaped overlay that appears during recording,
//! showing recording status, waveform visualization, and duration.

use crate::messages::Message;
use crate::theme;
use iced::widget::{canvas, column, container, row, text, Canvas, Space};
use iced::{mouse, Element, Length, Point, Rectangle, Renderer, Size, Theme};
use mindtype_core::AudioLevel;
use std::time::Duration;

/// Waveform visualization canvas
pub struct WaveformCanvas {
    levels: Vec<f32>,
    max_levels: usize,
}

impl WaveformCanvas {
    pub fn new() -> Self {
        Self {
            levels: Vec::with_capacity(60),
            max_levels: 60, // 1 second at 60fps
        }
    }

    pub fn push_level(&mut self, level: f32) {
        self.levels.push(level);
        if self.levels.len() > self.max_levels {
            self.levels.remove(0);
        }
    }

    pub fn clear(&mut self) {
        self.levels.clear();
    }
}

impl Default for WaveformCanvas {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message> canvas::Program<Message> for WaveformCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let width = bounds.width;
        let height = bounds.height;
        let center_y = height / 2.0;

        if self.levels.is_empty() {
            // Draw a flat line when no data
            frame.stroke(
                &canvas::Path::line(
                    Point::new(0.0, center_y),
                    Point::new(width, center_y),
                ),
                canvas::Stroke::default()
                    .with_color(theme::colors::TEXT_SECONDARY)
                    .with_width(1.0),
            );
        } else {
            // Draw waveform
            let bar_width = width / self.max_levels as f32;

            for (i, &level) in self.levels.iter().enumerate() {
                let x = i as f32 * bar_width;
                let bar_height = level * height * 0.8;

                // Draw symmetrical bar around center
                let top = center_y - bar_height / 2.0;

                frame.fill_rectangle(
                    Point::new(x, top),
                    Size::new(bar_width - 1.0, bar_height),
                    theme::colors::RECORDING_RED,
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

/// Recording overlay state
pub struct OverlayState {
    pub is_recording: bool,
    pub waveform: WaveformCanvas,
    pub recording_duration: Duration,
}

impl OverlayState {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            waveform: WaveformCanvas::new(),
            recording_duration: Duration::ZERO,
        }
    }

    pub fn start_recording(&mut self) {
        self.is_recording = true;
        self.waveform.clear();
        self.recording_duration = Duration::ZERO;
    }

    pub fn stop_recording(&mut self) {
        self.is_recording = false;
    }

    pub fn update_level(&mut self, level: &AudioLevel) {
        if self.is_recording {
            self.waveform.push_level(level.rms);
        }
    }

    pub fn tick(&mut self, delta: Duration) {
        if self.is_recording {
            self.recording_duration += delta;
        }
    }
}

impl Default for OverlayState {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the overlay view
pub fn overlay_view<'a>(state: &'a OverlayState, audio_level: &'a AudioLevel) -> Element<'a, Message> {
    let duration_secs = state.recording_duration.as_secs_f32();

    // Pulsing recording indicator
    let indicator = container(
        text("●")
            .size(16)
            .color(theme::colors::RECORDING_RED),
    )
    .padding([0, 4]);

    // Duration text
    let duration_text = text(format!("{:.1}s", duration_secs))
        .size(14)
        .color(theme::colors::TEXT_WHITE);

    // Level meter (simple bars)
    let level_bars = create_level_bars(audio_level.rms);

    // Waveform canvas (cloned state for static reference)
    let waveform = Canvas::new(&state.waveform)
        .width(100)
        .height(30);

    let content = row![
        indicator,
        Space::with_width(8),
        waveform,
        Space::with_width(8),
        level_bars,
        Space::with_width(8),
        duration_text,
    ]
    .align_y(iced::Alignment::Center)
    .padding([8, 16]);

    container(content)
        .style(|_| theme::recording_overlay())
        .into()
}

/// Create level meter bars
fn create_level_bars<'a>(level: f32) -> Element<'a, Message> {
    let num_bars = 5;
    let filled = (level * num_bars as f32).round() as usize;

    let bars: Vec<Element<'a, Message>> = (0..num_bars)
        .map(|i| {
            let color = if i < filled {
                if i >= num_bars - 1 {
                    theme::colors::RECORDING_RED
                } else {
                    theme::colors::SUCCESS_GREEN
                }
            } else {
                theme::colors::BUTTON_SHADOW
            };

            container(Space::new(4, 16))
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(color)),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into()
        })
        .collect();

    row(bars).spacing(2).into()
}

/// Overlay window settings for Iced multi-window
pub mod window {
    use iced::window::{self, Position, Level};
    use iced::Size;

    /// Get overlay window settings
    pub fn settings() -> window::Settings {
        window::Settings {
            size: Size::new(280.0, 50.0),
            position: Position::Centered,
            min_size: None,
            max_size: None,
            visible: true,
            resizable: false,
            decorations: false,
            transparent: true,
            level: Level::AlwaysOnTop,
            icon: None,
            platform_specific: Default::default(),
            exit_on_close_request: false,
        }
    }

    /// Position overlay at bottom center of screen
    pub fn position_bottom_center(screen_width: f32, screen_height: f32) -> Position {
        let overlay_width = 280.0;
        let overlay_height = 50.0;
        let margin = 80.0;

        Position::Specific(iced::Point::new(
            (screen_width - overlay_width) / 2.0,
            screen_height - overlay_height - margin,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waveform_canvas() {
        let mut waveform = WaveformCanvas::new();
        assert!(waveform.levels.is_empty());

        waveform.push_level(0.5);
        assert_eq!(waveform.levels.len(), 1);

        for _ in 0..100 {
            waveform.push_level(0.5);
        }
        assert!(waveform.levels.len() <= waveform.max_levels);
    }

    #[test]
    fn test_overlay_state() {
        let mut state = OverlayState::new();
        assert!(!state.is_recording);

        state.start_recording();
        assert!(state.is_recording);

        state.tick(Duration::from_millis(500));
        assert_eq!(state.recording_duration.as_millis(), 500);

        state.stop_recording();
        assert!(!state.is_recording);
    }
}
