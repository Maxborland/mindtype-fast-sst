//! System tray handling

use anyhow::Result;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Emitter, Manager,
};
use std::sync::OnceLock;

/// Global tray icon reference for updating state
static TRAY_ICON: OnceLock<TrayIcon> = OnceLock::new();

/// Generate a simple colored circle icon as RGBA bytes
fn generate_circle_icon(color: [u8; 3], size: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let center = size as f32 / 2.0;
    let radius = center - 2.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();

            let idx = ((y * size + x) * 4) as usize;
            if distance <= radius {
                // Inside circle
                pixels[idx] = color[0];     // R
                pixels[idx + 1] = color[1]; // G
                pixels[idx + 2] = color[2]; // B
                pixels[idx + 3] = 255;      // A
            } else if distance <= radius + 1.0 {
                // Anti-aliased edge
                let alpha = ((radius + 1.0 - distance) * 255.0) as u8;
                pixels[idx] = color[0];
                pixels[idx + 1] = color[1];
                pixels[idx + 2] = color[2];
                pixels[idx + 3] = alpha;
            }
            // else: transparent (already 0)
        }
    }
    pixels
}

/// Create idle icon (green circle)
fn create_idle_icon() -> Result<Image<'static>> {
    let size = 32u32;
    let pixels = generate_circle_icon([0, 170, 0], size); // Green
    Ok(Image::new_owned(pixels, size, size))
}

/// Create recording icon (red circle)
fn create_recording_icon() -> Result<Image<'static>> {
    let size = 32u32;
    let pixels = generate_circle_icon([220, 50, 50], size); // Red
    Ok(Image::new_owned(pixels, size, size))
}

/// Create processing icon (yellow/orange circle)
fn create_processing_icon() -> Result<Image<'static>> {
    let size = 32u32;
    let pixels = generate_circle_icon([255, 170, 0], size); // Orange
    Ok(Image::new_owned(pixels, size, size))
}

/// Recording state for tray icon
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    Idle,
    Recording,
    Processing,
}

/// Update the tray icon based on recording state
pub fn update_tray_state(state: TrayState) {
    if let Some(tray) = TRAY_ICON.get() {
        let icon = match state {
            TrayState::Idle => create_idle_icon(),
            TrayState::Recording => create_recording_icon(),
            TrayState::Processing => create_processing_icon(),
        };

        if let Ok(icon) = icon {
            let _ = tray.set_icon(Some(icon));
        }

        let tooltip = match state {
            TrayState::Idle => "MindType - Ready (Ctrl+Alt+V to record)",
            TrayState::Recording => "MindType - Recording...",
            TrayState::Processing => "MindType - Processing...",
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

/// Setup system tray icon and menu
pub fn setup_tray(app: &App) -> Result<()> {
    // Create menu items
    let show = MenuItem::with_id(app, "show", "Show MindType", true, None::<&str>)?;
    let start_recording = MenuItem::with_id(app, "start_recording", "Start Recording", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    // Build menu
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &start_recording,
            &settings,
            &quit,
        ],
    )?;

    // Create initial icon (idle state)
    let icon = create_idle_icon()?;

    // Build tray icon
    let tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("MindType - Ready (Ctrl+Alt+V to record)")
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "start_recording" => {
                    let _ = app.emit("tray-start-recording", ());
                }
                "settings" => {
                    let _ = app.emit("tray-open-settings", ());
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    // Show main window on left click
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            }
        })
        .build(app)?;

    // Store tray reference for state updates
    let _ = TRAY_ICON.set(tray);

    tracing::info!("System tray initialized");
    Ok(())
}
