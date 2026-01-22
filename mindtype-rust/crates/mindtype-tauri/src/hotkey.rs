//! Global hotkey handling

use anyhow::Result;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Setup global hotkeys for the application
pub fn setup_hotkeys(app: &AppHandle) -> Result<()> {
    // Default hotkey: Ctrl+Alt+V
    let shortcut: Shortcut = "Ctrl+Alt+V".parse()?;

    app.global_shortcut().on_shortcut(shortcut, |app, _shortcut, event| {
        match event.state {
            ShortcutState::Pressed => {
                tracing::debug!("Hotkey pressed - starting recording");
                let _ = app.emit("hotkey-pressed", ());
            }
            ShortcutState::Released => {
                tracing::debug!("Hotkey released - stopping recording");
                let _ = app.emit("hotkey-released", ());
            }
        }
    })?;

    tracing::info!("Global hotkey registered: Ctrl+Alt+V");
    Ok(())
}

/// Update the global hotkey
pub fn update_hotkey(app: &AppHandle, new_shortcut: &str) -> Result<()> {
    // Unregister all existing shortcuts
    app.global_shortcut().unregister_all()?;

    // Register new shortcut
    let shortcut: Shortcut = new_shortcut.parse()?;

    app.global_shortcut().on_shortcut(shortcut, |app, _shortcut, event| {
        match event.state {
            ShortcutState::Pressed => {
                let _ = app.emit("hotkey-pressed", ());
            }
            ShortcutState::Released => {
                let _ = app.emit("hotkey-released", ());
            }
        }
    })?;

    tracing::info!("Global hotkey updated: {}", new_shortcut);
    Ok(())
}
