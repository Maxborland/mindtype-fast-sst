//! Mac OS 7/8 Platinum Theme
//! Based on Figma design "Setup Wizard Screens Design"

use iced::widget::{button, container, pick_list, progress_bar, scrollable, text_input, checkbox, radio, slider};
use iced::{Background, Border, Color, Shadow, Vector};

/// Color palette from Figma design
pub mod colors {
    use iced::Color;

    // Window backgrounds
    pub const WIZARD_BG: Color = Color::from_rgb(0.96, 0.96, 0.96);      // #F5F5F5 - wizard/dialog light bg
    pub const MAIN_BG: Color = Color::from_rgb(0.62, 0.62, 0.62);        // #9E9E9E - main window dark bg
    pub const CONTENT_BG: Color = Color::WHITE;                          // White content areas

    // Borders and frames
    pub const FRAME_OUTER: Color = Color::BLACK;                         // Outer window frame
    pub const FRAME_INNER: Color = Color::from_rgb(0.85, 0.85, 0.85);   // #D9D9D9 - inner frame
    pub const FRAME_HIGHLIGHT: Color = Color::WHITE;                     // Highlight edge
    pub const FRAME_SHADOW: Color = Color::from_rgb(0.5, 0.5, 0.5);     // #808080 - shadow edge

    // Title bar
    pub const TITLE_BAR_BG: Color = Color::from_rgb(0.75, 0.75, 0.75);  // #C0C0C0
    pub const TITLE_BAR_LINES: Color = Color::BLACK;                     // Horizontal lines

    // Buttons
    pub const BUTTON_FACE: Color = Color::from_rgb(0.85, 0.85, 0.85);   // #D9D9D9
    pub const BUTTON_HIGHLIGHT: Color = Color::WHITE;                    // Top-left edge
    pub const BUTTON_SHADOW: Color = Color::from_rgb(0.5, 0.5, 0.5);    // #808080 bottom-right
    pub const BUTTON_DARK_SHADOW: Color = Color::BLACK;                  // Outer shadow
    pub const BUTTON_PRESSED: Color = Color::from_rgb(0.75, 0.75, 0.75); // Pressed state
    pub const BUTTON_HOVER: Color = Color::from_rgb(0.9, 0.9, 0.9);     // Hover state

    // Text
    pub const TEXT_PRIMARY: Color = Color::BLACK;
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.4, 0.4, 0.4);   // #666666
    pub const TEXT_DISABLED: Color = Color::from_rgb(0.6, 0.6, 0.6);    // #999999
    pub const TEXT_WHITE: Color = Color::WHITE;
    pub const TEXT_LINK: Color = Color::from_rgb(0.0, 0.0, 0.8);        // Blue link

    // Input fields
    pub const INPUT_BG: Color = Color::WHITE;
    pub const INPUT_BORDER_DARK: Color = Color::from_rgb(0.4, 0.4, 0.4); // Top/left inset
    pub const INPUT_BORDER_LIGHT: Color = Color::WHITE;                   // Bottom/right highlight

    // Panels and groups
    pub const PANEL_BG: Color = Color::from_rgb(0.88, 0.88, 0.88);      // #E0E0E0
    pub const GROUP_BORDER: Color = Color::from_rgb(0.5, 0.5, 0.5);     // #808080

    // Status colors
    pub const SUCCESS: Color = Color::from_rgb(0.0, 0.6, 0.0);          // Green checkmark
    pub const WARNING: Color = Color::from_rgb(0.8, 0.6, 0.0);          // Orange/yellow
    pub const ERROR: Color = Color::from_rgb(0.8, 0.0, 0.0);            // Red
    pub const RECORDING_RED: Color = Color::from_rgb(0.8, 0.15, 0.15);  // Recording indicator

    // Progress bar
    pub const PROGRESS_BG: Color = Color::WHITE;
    pub const PROGRESS_FILL: Color = Color::BLACK;

    // Tabs
    pub const TAB_ACTIVE: Color = Color::from_rgb(0.85, 0.85, 0.85);    // Selected tab
    pub const TAB_INACTIVE: Color = Color::from_rgb(0.7, 0.7, 0.7);     // Unselected tab

    // Log area
    pub const LOG_BG: Color = Color::from_rgb(0.95, 0.95, 0.95);        // #F2F2F2
    pub const LOG_BORDER: Color = Color::from_rgb(0.6, 0.6, 0.6);       // #999999

    // Aliases for compatibility
    pub const BLACK: Color = Color::BLACK;
    pub const WHITE: Color = Color::WHITE;
    pub const RED: Color = ERROR;
    pub const GREEN: Color = SUCCESS;
    pub const GRAY_MID: Color = TEXT_SECONDARY;

    // Legacy aliases
    pub const WINDOW_BG: Color = WIZARD_BG;
    pub const BORDER_LIGHT: Color = FRAME_INNER;
    pub const BORDER_DARK: Color = FRAME_SHADOW;
    pub const HIGHLIGHT_BLUE: Color = TEXT_LINK;
    pub const ACCENT_BLUE: Color = TEXT_LINK;
    pub const SUCCESS_GREEN: Color = SUCCESS;
    pub const ERROR_RED: Color = ERROR;
    pub const WARNING_YELLOW: Color = WARNING;
}

/// Border radius constants
pub mod radius {
    pub const NONE: f32 = 0.0;
    pub const SMALL: f32 = 2.0;
    pub const MEDIUM: f32 = 4.0;
    pub const LARGE: f32 = 6.0;
    pub const BUTTON: f32 = 3.0;
}

// ============================================================================
// WINDOW STYLES
// ============================================================================

/// Wizard window container (light background)
pub fn wizard_window() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::WIZARD_BG)),
        border: Border {
            color: colors::FRAME_OUTER,
            width: 2.0,
            radius: radius::NONE.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

/// Main window container (dark background)
pub fn main_window() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::MAIN_BG)),
        border: Border {
            color: colors::FRAME_OUTER,
            width: 2.0,
            radius: radius::NONE.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

/// Window container (general - uses wizard style)
pub fn window_container() -> container::Style {
    wizard_window()
}

// ============================================================================
// TITLE BAR
// ============================================================================

/// Title bar style with gray background
pub fn title_bar() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::TITLE_BAR_BG)),
        border: Border {
            color: colors::FRAME_OUTER,
            width: 1.0,
            radius: radius::NONE.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

/// Close button box in title bar
pub fn close_button_box() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::WIZARD_BG)),
        border: Border {
            color: colors::FRAME_OUTER,
            width: 1.0,
            radius: radius::NONE.into(),
        },
        shadow: Shadow::default(),
        text_color: None,
    }
}

// ============================================================================
// BUTTONS
// ============================================================================

/// Primary button (3D raised style like Mac OS 7)
pub fn primary_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let (background, text_color, border_width) = match status {
        button::Status::Active => (colors::BUTTON_FACE, colors::TEXT_PRIMARY, 1.0),
        button::Status::Hovered => (colors::BUTTON_HOVER, colors::TEXT_PRIMARY, 1.0),
        button::Status::Pressed => (colors::BUTTON_PRESSED, colors::TEXT_PRIMARY, 2.0),
        button::Status::Disabled => (colors::BUTTON_FACE, colors::TEXT_DISABLED, 1.0),
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: colors::BUTTON_DARK_SHADOW,
            width: border_width,
            radius: radius::BUTTON.into(),
        },
        shadow: Shadow {
            color: colors::BUTTON_SHADOW,
            offset: Vector::new(1.0, 1.0),
            blur_radius: 0.0,
        },
    }
}

/// Secondary button (same style, used for "< Back")
pub fn secondary_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    primary_button(_theme, status)
}

/// Disabled button style
pub fn disabled_button(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(colors::BUTTON_FACE)),
        text_color: colors::TEXT_DISABLED,
        border: Border {
            color: colors::FRAME_SHADOW,
            width: 1.0,
            radius: radius::BUTTON.into(),
        },
        shadow: Shadow::default(),
    }
}

/// Tab button (active)
pub fn tab_button_active(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(colors::TAB_ACTIVE)),
        text_color: colors::TEXT_PRIMARY,
        border: Border {
            color: colors::FRAME_OUTER,
            width: 1.0,
            radius: radius::NONE.into(),
        },
        shadow: Shadow::default(),
    }
}

/// Tab button (inactive)
pub fn tab_button_inactive(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => colors::TAB_ACTIVE,
        _ => colors::TAB_INACTIVE,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: colors::TEXT_PRIMARY,
        border: Border {
            color: colors::FRAME_SHADOW,
            width: 1.0,
            radius: radius::NONE.into(),
        },
        shadow: Shadow::default(),
    }
}

/// Tab button style factory
pub fn tab_button(active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme, status| {
        if active {
            tab_button_active(theme, status)
        } else {
            tab_button_inactive(theme, status)
        }
    }
}

/// Danger button (not used in current design but kept for compatibility)
pub fn danger_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Active => (colors::ERROR, colors::TEXT_WHITE),
        button::Status::Hovered => (Color::from_rgb(0.9, 0.1, 0.1), colors::TEXT_WHITE),
        button::Status::Pressed => (Color::from_rgb(0.6, 0.0, 0.0), colors::TEXT_WHITE),
        button::Status::Disabled => (colors::BUTTON_FACE, colors::TEXT_DISABLED),
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: colors::FRAME_OUTER,
            width: 1.0,
            radius: radius::BUTTON.into(),
        },
        shadow: Shadow::default(),
    }
}

/// Accent button (blue)
pub fn accent_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Active => (colors::TEXT_LINK, colors::TEXT_WHITE),
        button::Status::Hovered => (Color::from_rgb(0.1, 0.1, 0.9), colors::TEXT_WHITE),
        button::Status::Pressed => (Color::from_rgb(0.0, 0.0, 0.5), colors::TEXT_WHITE),
        button::Status::Disabled => (colors::BUTTON_FACE, colors::TEXT_DISABLED),
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: colors::FRAME_OUTER,
            width: 1.0,
            radius: radius::BUTTON.into(),
        },
        shadow: Shadow::default(),
    }
}

// ============================================================================
// INPUT FIELDS
// ============================================================================

/// Text input style (inset appearance like Mac OS 7)
pub fn text_input_style(_theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Active => colors::INPUT_BORDER_DARK,
        text_input::Status::Hovered => colors::INPUT_BORDER_DARK,
        text_input::Status::Focused => colors::TEXT_PRIMARY,
        text_input::Status::Disabled => colors::TEXT_DISABLED,
    };

    text_input::Style {
        background: Background::Color(colors::INPUT_BG),
        border: Border {
            color: border_color,
            width: 2.0,
            radius: radius::SMALL.into(),
        },
        icon: colors::TEXT_SECONDARY,
        placeholder: colors::TEXT_DISABLED,
        value: colors::TEXT_PRIMARY,
        selection: colors::TEXT_LINK,
    }
}

// ============================================================================
// PICK LIST (DROPDOWNS)
// ============================================================================

/// Pick list style
pub fn pick_list_style(_theme: &iced::Theme, status: pick_list::Status) -> pick_list::Style {
    let border_color = match status {
        pick_list::Status::Active => colors::INPUT_BORDER_DARK,
        pick_list::Status::Hovered => colors::TEXT_PRIMARY,
        pick_list::Status::Opened => colors::TEXT_PRIMARY,
    };

    pick_list::Style {
        background: Background::Color(colors::INPUT_BG),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius::SMALL.into(),
        },
        text_color: colors::TEXT_PRIMARY,
        placeholder_color: colors::TEXT_DISABLED,
        handle_color: colors::TEXT_PRIMARY,
    }
}

// Note: pick_list::Menu is private in Iced 0.13, menu styling is done through the pick_list itself

// ============================================================================
// PANELS AND CONTAINERS
// ============================================================================

/// Raised panel (for grouped options in wizard)
pub fn panel_container() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::PANEL_BG)),
        border: Border {
            color: colors::GROUP_BORDER,
            width: 1.0,
            radius: radius::SMALL.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

/// Inset container (for content wells, text areas)
pub fn inset_container() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::CONTENT_BG)),
        border: Border {
            color: colors::INPUT_BORDER_DARK,
            width: 2.0,
            radius: radius::SMALL.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

/// Log area container
pub fn log_container() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::LOG_BG)),
        border: Border {
            color: colors::LOG_BORDER,
            width: 1.0,
            radius: radius::SMALL.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

/// Group box container (for Settings groups like "AI Provider", "Performance")
pub fn group_box() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::PANEL_BG)),
        border: Border {
            color: colors::GROUP_BORDER,
            width: 1.0,
            radius: radius::SMALL.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

/// Status panel (for "Status: Trial" etc)
pub fn status_panel() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::PANEL_BG)),
        border: Border {
            color: colors::GROUP_BORDER,
            width: 1.0,
            radius: radius::MEDIUM.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

/// Card container (legacy)
pub fn card_container() -> container::Style {
    panel_container()
}

// ============================================================================
// PROGRESS BAR
// ============================================================================

/// Progress bar style (black fill on white - Mac OS 7 style)
pub fn progress_bar_style(_theme: &iced::Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: Background::Color(colors::PROGRESS_BG),
        bar: Background::Color(colors::PROGRESS_FILL),
        border: Border {
            color: colors::INPUT_BORDER_DARK,
            width: 1.0,
            radius: radius::SMALL.into(),
        },
    }
}

/// Recording progress bar (red)
pub fn recording_progress_style(_theme: &iced::Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: Background::Color(colors::PROGRESS_BG),
        bar: Background::Color(colors::RECORDING_RED),
        border: Border {
            color: colors::INPUT_BORDER_DARK,
            width: 1.0,
            radius: radius::SMALL.into(),
        },
    }
}

// ============================================================================
// SCROLLABLE
// ============================================================================

/// Scrollable style
pub fn scrollable_style(_theme: &iced::Theme, status: scrollable::Status) -> scrollable::Style {
    let scrollbar_color = match status {
        scrollable::Status::Active => colors::BUTTON_FACE,
        scrollable::Status::Hovered { .. } => colors::BUTTON_FACE,
        scrollable::Status::Dragged { .. } => colors::BUTTON_PRESSED,
    };

    let rail = scrollable::Rail {
        background: Some(Background::Color(colors::PANEL_BG)),
        border: Border {
            color: colors::GROUP_BORDER,
            width: 1.0,
            radius: radius::NONE.into(),
        },
        scroller: scrollable::Scroller {
            color: scrollbar_color,
            border: Border {
                color: colors::FRAME_OUTER,
                width: 1.0,
                radius: radius::SMALL.into(),
            },
        },
    };

    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail.clone(),
        horizontal_rail: rail,
        gap: None,
    }
}

// ============================================================================
// CHECKBOX
// ============================================================================

/// Checkbox style
pub fn checkbox_style(_theme: &iced::Theme, status: checkbox::Status) -> checkbox::Style {
    let (background, icon_color, border_color) = match status {
        checkbox::Status::Active { is_checked } => {
            if is_checked {
                (colors::CONTENT_BG, colors::TEXT_PRIMARY, colors::TEXT_PRIMARY)
            } else {
                (colors::CONTENT_BG, colors::TEXT_PRIMARY, colors::INPUT_BORDER_DARK)
            }
        }
        checkbox::Status::Hovered { is_checked } => {
            if is_checked {
                (colors::CONTENT_BG, colors::TEXT_PRIMARY, colors::TEXT_PRIMARY)
            } else {
                (colors::CONTENT_BG, colors::TEXT_PRIMARY, colors::TEXT_PRIMARY)
            }
        }
        checkbox::Status::Disabled { is_checked: _ } => {
            (colors::PANEL_BG, colors::TEXT_DISABLED, colors::TEXT_DISABLED)
        }
    };

    checkbox::Style {
        background: Background::Color(background),
        icon_color,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius::SMALL.into(),
        },
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

// ============================================================================
// RADIO BUTTON
// ============================================================================

/// Radio button style
pub fn radio_style(_theme: &iced::Theme, status: radio::Status) -> radio::Style {
    let (dot_color, background, border_color) = match status {
        radio::Status::Active { is_selected } => {
            if is_selected {
                (colors::TEXT_PRIMARY, colors::CONTENT_BG, colors::TEXT_PRIMARY)
            } else {
                (colors::CONTENT_BG, colors::CONTENT_BG, colors::INPUT_BORDER_DARK)
            }
        }
        radio::Status::Hovered { is_selected } => {
            if is_selected {
                (colors::TEXT_PRIMARY, colors::CONTENT_BG, colors::TEXT_PRIMARY)
            } else {
                (colors::CONTENT_BG, colors::CONTENT_BG, colors::TEXT_PRIMARY)
            }
        }
    };

    radio::Style {
        dot_color,
        background: Background::Color(background),
        border_color,
        border_width: 2.0,
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

// ============================================================================
// SLIDER
// ============================================================================

/// Slider style
pub fn slider_style(_theme: &iced::Theme, status: slider::Status) -> slider::Style {
    let handle_color = match status {
        slider::Status::Active => colors::BUTTON_FACE,
        slider::Status::Hovered => colors::BUTTON_FACE,
        slider::Status::Dragged => colors::BUTTON_PRESSED,
    };

    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(colors::PROGRESS_FILL),
                Background::Color(colors::PANEL_BG),
            ),
            width: 4.0,
            border: Border {
                color: colors::INPUT_BORDER_DARK,
                width: 1.0,
                radius: radius::SMALL.into(),
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Rectangle {
                width: 12,
                border_radius: radius::SMALL.into(),
            },
            background: Background::Color(handle_color),
            border_color: colors::FRAME_OUTER,
            border_width: 1.0,
        },
    }
}

// ============================================================================
// RECORDING OVERLAY
// ============================================================================

/// Recording overlay panel (floating at bottom)
pub fn recording_overlay() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::PANEL_BG)),
        border: Border {
            color: colors::FRAME_OUTER,
            width: 2.0,
            radius: radius::MEDIUM.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
            offset: Vector::new(2.0, 2.0),
            blur_radius: 4.0,
        },
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

/// Transcribing overlay panel
pub fn transcribing_overlay() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::PANEL_BG)),
        border: Border {
            color: colors::FRAME_OUTER,
            width: 1.0,
            radius: radius::MEDIUM.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

// ============================================================================
// DROP ZONE
// ============================================================================

/// File drop zone style
pub fn drop_zone() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::CONTENT_BG)),
        border: Border {
            color: colors::GROUP_BORDER,
            width: 2.0,
            radius: radius::MEDIUM.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_SECONDARY),
    }
}

/// File drop zone hover style
pub fn drop_zone_hover() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::PANEL_BG)),
        border: Border {
            color: colors::TEXT_PRIMARY,
            width: 2.0,
            radius: radius::MEDIUM.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

// ============================================================================
// STATUS INDICATORS
// ============================================================================

/// Recording indicator (pulsing)
pub fn recording_indicator() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::RECORDING_RED)),
        border: Border {
            color: Color::from_rgb(0.6, 0.1, 0.1),
            width: 2.0,
            radius: 50.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.8, 0.0, 0.0, 0.4),
            offset: Vector::new(0.0, 0.0),
            blur_radius: 8.0,
        },
        text_color: Some(colors::TEXT_WHITE),
    }
}

/// Idle indicator
pub fn idle_indicator() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::BORDER_LIGHT)),
        border: Border {
            color: colors::BORDER_DARK,
            width: 1.0,
            radius: 50.0.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_SECONDARY),
    }
}

/// Success badge
pub fn success_badge() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::SUCCESS)),
        border: Border {
            color: Color::from_rgb(0.0, 0.4, 0.0),
            width: 1.0,
            radius: radius::SMALL.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_WHITE),
    }
}

/// Warning badge
pub fn warning_badge() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::WARNING)),
        border: Border {
            color: Color::from_rgb(0.6, 0.4, 0.0),
            width: 1.0,
            radius: radius::SMALL.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_PRIMARY),
    }
}

/// Error badge
pub fn error_badge() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::ERROR)),
        border: Border {
            color: Color::from_rgb(0.5, 0.0, 0.0),
            width: 1.0,
            radius: radius::SMALL.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(colors::TEXT_WHITE),
    }
}

// ============================================================================
// MODAL / OVERLAY
// ============================================================================

/// Semi-transparent overlay for modals
pub fn modal_overlay() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
        border: Border::default(),
        shadow: Shadow::default(),
        text_color: None,
    }
}

/// Title bar active state
pub fn title_bar_active() -> container::Style {
    title_bar()
}

// ============================================================================
// LEVEL METER
// ============================================================================

/// Audio level meter segment (filled)
pub fn level_meter_filled() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::SUCCESS)),
        border: Border {
            color: Color::from_rgb(0.0, 0.4, 0.0),
            width: 1.0,
            radius: 2.0.into(),
        },
        shadow: Shadow::default(),
        text_color: None,
    }
}

/// Audio level meter segment (empty)
pub fn level_meter_empty() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::PANEL_BG)),
        border: Border {
            color: colors::BORDER_LIGHT,
            width: 1.0,
            radius: 2.0.into(),
        },
        shadow: Shadow::default(),
        text_color: None,
    }
}

/// High level meter segment (red/orange)
pub fn level_meter_high() -> container::Style {
    container::Style {
        background: Some(Background::Color(colors::RECORDING_RED)),
        border: Border {
            color: Color::from_rgb(0.6, 0.1, 0.1),
            width: 1.0,
            radius: 2.0.into(),
        },
        shadow: Shadow::default(),
        text_color: None,
    }
}
