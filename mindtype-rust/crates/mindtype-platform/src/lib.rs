//! MindType Platform Abstraction Layer
//!
//! Provides platform-specific functionality for Windows, macOS, and Linux.

mod error;

#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

pub use error::PlatformError;

#[cfg(windows)]
pub use windows::WindowsPlatform as Platform;

#[cfg(target_os = "macos")]
pub use macos::MacOSPlatform as Platform;

#[cfg(target_os = "linux")]
pub use linux::LinuxPlatform as Platform;

/// Hotkey event
#[derive(Debug, Clone)]
pub enum HotkeyEvent {
    /// Hotkey was pressed
    Pressed,
    /// Hotkey was released
    Released,
}

/// Tray menu action
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayAction {
    ShowWindow,
    Settings,
    Quit,
}
