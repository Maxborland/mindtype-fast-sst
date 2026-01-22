//! Platform error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlatformError {
    #[error("Hotkey registration failed: {0}")]
    HotkeyRegistration(String),

    #[error("Hotkey already registered")]
    HotkeyAlreadyRegistered,

    #[error("Invalid hotkey format: {0}")]
    InvalidHotkeyFormat(String),

    #[error("Text insertion failed: {0}")]
    TextInsertionFailed(String),

    #[error("Clipboard error: {0}")]
    ClipboardError(String),

    #[error("Window operation failed: {0}")]
    WindowError(String),

    #[error("Tray icon error: {0}")]
    TrayError(String),

    #[error("Device ID error: {0}")]
    DeviceIdError(String),

    #[error("Platform not supported")]
    NotSupported,

    #[error("Windows API error: {0}")]
    #[cfg(windows)]
    WindowsError(#[from] windows::core::Error),
}
