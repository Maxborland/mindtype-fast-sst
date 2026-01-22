//! Windows platform implementation

use crate::error::PlatformError;
use crate::{HotkeyEvent, TrayAction};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, SendInput, UnregisterHotKey, HOT_KEY_MODIFIERS, INPUT, INPUT_KEYBOARD,
    KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, MOD_ALT, MOD_CONTROL,
    MOD_SHIFT, MOD_WIN, VIRTUAL_KEY, VK_CONTROL, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetMessageW, SetForegroundWindow, MSG, WM_HOTKEY,
};

const HOTKEY_ID: i32 = 1;

/// Windows platform implementation
pub struct WindowsPlatform {
    hotkey_tx: mpsc::UnboundedSender<HotkeyEvent>,
    hotkey_rx: mpsc::UnboundedReceiver<HotkeyEvent>,
    tray_tx: Option<mpsc::UnboundedSender<TrayAction>>,
    tray_rx: Option<mpsc::UnboundedReceiver<TrayAction>>,
    /// Store window handle as raw pointer for thread safety
    saved_window: Arc<AtomicIsize>,
    hotkey_registered: Arc<AtomicBool>,
    hotkey_thread: Option<std::thread::JoinHandle<()>>,
}

impl WindowsPlatform {
    /// Create a new Windows platform instance
    pub fn new() -> Result<Self, PlatformError> {
        let (hotkey_tx, hotkey_rx) = mpsc::unbounded_channel();

        Ok(Self {
            hotkey_tx,
            hotkey_rx,
            tray_tx: None,
            tray_rx: None,
            saved_window: Arc::new(AtomicIsize::new(0)),
            hotkey_registered: Arc::new(AtomicBool::new(false)),
            hotkey_thread: None,
        })
    }

    /// Parse hotkey string into modifiers and key
    fn parse_hotkey(combo: &str) -> Result<(HOT_KEY_MODIFIERS, VIRTUAL_KEY), PlatformError> {
        let parts: Vec<String> = combo.split('+').map(|s| s.trim().to_uppercase()).collect();

        if parts.is_empty() {
            return Err(PlatformError::InvalidHotkeyFormat(
                "Empty hotkey".to_string(),
            ));
        }

        let mut modifiers = HOT_KEY_MODIFIERS(0);
        let mut key = None;

        for (i, part) in parts.iter().enumerate() {
            match part.as_str() {
                "CTRL" | "CONTROL" => modifiers |= MOD_CONTROL,
                "ALT" => modifiers |= MOD_ALT,
                "SHIFT" => modifiers |= MOD_SHIFT,
                "WIN" | "SUPER" => modifiers |= MOD_WIN,
                _ => {
                    // Last part should be the key
                    if i == parts.len() - 1 {
                        key = Some(Self::parse_key(part)?);
                    } else {
                        return Err(PlatformError::InvalidHotkeyFormat(format!(
                            "Unknown modifier: {}",
                            part
                        )));
                    }
                }
            }
        }

        let key = key.ok_or_else(|| {
            PlatformError::InvalidHotkeyFormat("No key specified in hotkey".to_string())
        })?;

        Ok((modifiers, key))
    }

    /// Parse a key name to virtual key code
    fn parse_key(key: &str) -> Result<VIRTUAL_KEY, PlatformError> {
        // Handle single character keys
        if key.len() == 1 {
            let c = key.chars().next().unwrap();
            if c.is_ascii_alphanumeric() {
                return Ok(VIRTUAL_KEY(c.to_ascii_uppercase() as u16));
            }
        }

        // Handle special keys
        match key {
            "SPACE" => Ok(VIRTUAL_KEY(0x20)),
            "ENTER" | "RETURN" => Ok(VIRTUAL_KEY(0x0D)),
            "TAB" => Ok(VIRTUAL_KEY(0x09)),
            "ESCAPE" | "ESC" => Ok(VIRTUAL_KEY(0x1B)),
            "BACKSPACE" => Ok(VIRTUAL_KEY(0x08)),
            "DELETE" | "DEL" => Ok(VIRTUAL_KEY(0x2E)),
            "INSERT" | "INS" => Ok(VIRTUAL_KEY(0x2D)),
            "HOME" => Ok(VIRTUAL_KEY(0x24)),
            "END" => Ok(VIRTUAL_KEY(0x23)),
            "PAGEUP" | "PGUP" => Ok(VIRTUAL_KEY(0x21)),
            "PAGEDOWN" | "PGDN" => Ok(VIRTUAL_KEY(0x22)),
            "UP" => Ok(VIRTUAL_KEY(0x26)),
            "DOWN" => Ok(VIRTUAL_KEY(0x28)),
            "LEFT" => Ok(VIRTUAL_KEY(0x25)),
            "RIGHT" => Ok(VIRTUAL_KEY(0x27)),
            "F1" => Ok(VIRTUAL_KEY(0x70)),
            "F2" => Ok(VIRTUAL_KEY(0x71)),
            "F3" => Ok(VIRTUAL_KEY(0x72)),
            "F4" => Ok(VIRTUAL_KEY(0x73)),
            "F5" => Ok(VIRTUAL_KEY(0x74)),
            "F6" => Ok(VIRTUAL_KEY(0x75)),
            "F7" => Ok(VIRTUAL_KEY(0x76)),
            "F8" => Ok(VIRTUAL_KEY(0x77)),
            "F9" => Ok(VIRTUAL_KEY(0x78)),
            "F10" => Ok(VIRTUAL_KEY(0x79)),
            "F11" => Ok(VIRTUAL_KEY(0x7A)),
            "F12" => Ok(VIRTUAL_KEY(0x7B)),
            _ => Err(PlatformError::InvalidHotkeyFormat(format!(
                "Unknown key: {}",
                key
            ))),
        }
    }

    /// Save current clipboard contents (returns None if clipboard is empty or has unsupported format)
    fn save_clipboard(&self) -> Option<Vec<u16>> {
        use windows::Win32::System::DataExchange::{
            CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
        };
        use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

        const CF_UNICODETEXT: u32 = 13;

        unsafe {
            // Check if clipboard has text
            if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() {
                return None;
            }

            if OpenClipboard(HWND::default()).is_err() {
                return None;
            }

            let result = (|| -> Option<Vec<u16>> {
                let handle = GetClipboardData(CF_UNICODETEXT).ok()?;
                if handle.0.is_null() {
                    return None;
                }

                let hmem = windows::Win32::Foundation::HGLOBAL(handle.0);
                let size = GlobalSize(hmem);
                if size == 0 {
                    return None;
                }

                let ptr = GlobalLock(hmem);
                if ptr.is_null() {
                    return None;
                }

                // Copy the data
                let len = size / 2; // u16 elements
                let mut data = vec![0u16; len];
                std::ptr::copy_nonoverlapping(ptr as *const u16, data.as_mut_ptr(), len);
                let _ = GlobalUnlock(hmem);

                // Find null terminator and truncate
                if let Some(null_pos) = data.iter().position(|&c| c == 0) {
                    data.truncate(null_pos + 1);
                }

                Some(data)
            })();

            let _ = CloseClipboard();
            result
        }
    }

    /// Restore clipboard contents from saved data
    fn restore_clipboard(&self, data: &[u16]) -> Result<(), PlatformError> {
        use windows::Win32::System::DataExchange::{
            CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
        };
        use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

        const CF_UNICODETEXT: u32 = 13;

        unsafe {
            OpenClipboard(HWND::default())
                .map_err(|e| PlatformError::ClipboardError(format!("OpenClipboard failed: {}", e)))?;

            if let Err(e) = EmptyClipboard() {
                let _ = CloseClipboard();
                return Err(PlatformError::ClipboardError(format!(
                    "EmptyClipboard failed: {}",
                    e
                )));
            }

            let bytes_needed = data.len() * 2;
            let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes_needed).map_err(|e| {
                let _ = CloseClipboard();
                PlatformError::ClipboardError(format!("GlobalAlloc failed: {}", e))
            })?;

            let ptr = GlobalLock(hmem);
            if ptr.is_null() {
                let _ = CloseClipboard();
                return Err(PlatformError::ClipboardError("GlobalLock failed".into()));
            }

            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u16, data.len());
            let _ = GlobalUnlock(hmem);

            if SetClipboardData(CF_UNICODETEXT, windows::Win32::Foundation::HANDLE(hmem.0)).is_err() {
                let _ = CloseClipboard();
                return Err(PlatformError::ClipboardError("SetClipboardData failed".into()));
            }

            let _ = CloseClipboard();
        }

        Ok(())
    }

    /// Release any stuck modifier keys (Ctrl, Alt, Shift, Win)
    fn release_modifier_keys(&self) {
        use windows::Win32::UI::Input::KeyboardAndMouse::{VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN};

        unsafe {
            let modifiers = [
                VK_LCONTROL, VK_RCONTROL,
                VK_LMENU, VK_RMENU,  // Alt
                VK_LSHIFT, VK_RSHIFT,
                VK_LWIN, VK_RWIN,
            ];

            let mut inputs: Vec<INPUT> = Vec::new();

            for vk in modifiers {
                let mut input: INPUT = std::mem::zeroed();
                input.r#type = INPUT_KEYBOARD;
                input.Anonymous.ki = KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                };
                inputs.push(input);
            }

            if !inputs.is_empty() {
                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            }
        }
    }

    /// Insert text using clipboard and Ctrl+V with clipboard preservation
    fn insert_via_clipboard(&self, text: &str) -> Result<(), PlatformError> {
        use windows::Win32::System::DataExchange::{
            CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
        };
        use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

        // Save current clipboard contents
        let saved_clipboard = self.save_clipboard();
        debug!("Saved clipboard: {} chars", saved_clipboard.as_ref().map(|v| v.len()).unwrap_or(0));

        // Release any stuck modifier keys before paste
        self.release_modifier_keys();

        unsafe {
            // Open clipboard
            OpenClipboard(HWND::default())
                .map_err(|e| PlatformError::ClipboardError(format!("OpenClipboard failed: {}", e)))?;

            // Empty clipboard
            if let Err(e) = EmptyClipboard() {
                let _ = CloseClipboard();
                return Err(PlatformError::ClipboardError(format!(
                    "EmptyClipboard failed: {}",
                    e
                )));
            }

            // Convert text to wide string
            let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes_needed = wide.len() * 2;

            // Allocate global memory
            let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes_needed).map_err(|e| {
                let _ = CloseClipboard();
                PlatformError::ClipboardError(format!("GlobalAlloc failed: {}", e))
            })?;

            // Lock and copy
            let ptr = GlobalLock(hmem);
            if ptr.is_null() {
                let _ = CloseClipboard();
                return Err(PlatformError::ClipboardError(
                    "GlobalLock failed".to_string(),
                ));
            }

            std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr as *mut u16, wide.len());
            let _ = GlobalUnlock(hmem);

            // Set clipboard data (CF_UNICODETEXT = 13)
            const CF_UNICODETEXT: u32 = 13;
            if SetClipboardData(CF_UNICODETEXT, windows::Win32::Foundation::HANDLE(hmem.0)).is_err()
            {
                let _ = CloseClipboard();
                return Err(PlatformError::ClipboardError(
                    "SetClipboardData failed".to_string(),
                ));
            }

            let _ = CloseClipboard();
        }

        // Small delay before paste (100ms like Python implementation)
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Send Ctrl+V
        self.send_paste_keys()?;

        // Wait for paste to complete
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Restore original clipboard contents
        if let Some(saved) = saved_clipboard {
            if let Err(e) = self.restore_clipboard(&saved) {
                warn!("Failed to restore clipboard: {}", e);
            } else {
                debug!("Restored clipboard");
            }
        }

        Ok(())
    }

    /// Send Ctrl+V key combination
    fn send_paste_keys(&self) -> Result<(), PlatformError> {
        unsafe {
            let mut inputs: [INPUT; 4] = std::mem::zeroed();

            // Ctrl down
            inputs[0].r#type = INPUT_KEYBOARD;
            inputs[0].Anonymous.ki = KEYBDINPUT {
                wVk: VK_CONTROL,
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            };

            // V down
            inputs[1].r#type = INPUT_KEYBOARD;
            inputs[1].Anonymous.ki = KEYBDINPUT {
                wVk: VK_V,
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            };

            // V up
            inputs[2].r#type = INPUT_KEYBOARD;
            inputs[2].Anonymous.ki = KEYBDINPUT {
                wVk: VK_V,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };

            // Ctrl up
            inputs[3].r#type = INPUT_KEYBOARD;
            inputs[3].Anonymous.ki = KEYBDINPUT {
                wVk: VK_CONTROL,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };

            let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            if sent != 4 {
                return Err(PlatformError::TextInsertionFailed(
                    "SendInput failed".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Register a global hotkey
    pub fn register_hotkey(&mut self, combo: &str) -> Result<(), PlatformError> {
        if self.hotkey_registered.load(Ordering::SeqCst) {
            return Err(PlatformError::HotkeyAlreadyRegistered);
        }

        let (modifiers, key) = Self::parse_hotkey(combo)?;
        let tx = self.hotkey_tx.clone();
        let registered = self.hotkey_registered.clone();

        info!("Registering hotkey: {}", combo);

        // Spawn thread to handle hotkey messages
        let handle = std::thread::spawn(move || {
            unsafe {
                if RegisterHotKey(HWND::default(), HOTKEY_ID, modifiers, key.0 as u32).is_err() {
                    error!("Failed to register hotkey");
                    return;
                }

                registered.store(true, Ordering::SeqCst);
                debug!("Hotkey registered, entering message loop");

                let mut msg = MSG::default();
                while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                    if msg.message == WM_HOTKEY && msg.wParam.0 == HOTKEY_ID as usize {
                        debug!("Hotkey pressed");
                        let _ = tx.send(HotkeyEvent::Pressed);
                    }
                }

                let _ = UnregisterHotKey(HWND::default(), HOTKEY_ID);
                registered.store(false, Ordering::SeqCst);
            }
        });

        self.hotkey_thread = Some(handle);

        // Wait a bit for registration
        std::thread::sleep(std::time::Duration::from_millis(100));

        if !self.hotkey_registered.load(Ordering::SeqCst) {
            return Err(PlatformError::HotkeyRegistration(
                "Failed to register hotkey".to_string(),
            ));
        }

        Ok(())
    }

    /// Unregister the current hotkey
    pub fn unregister_hotkey(&mut self) -> Result<(), PlatformError> {
        self.hotkey_registered.store(false, Ordering::SeqCst);
        // Thread will clean up on next message
        Ok(())
    }

    /// Get hotkey events receiver
    pub fn hotkey_receiver(&self) -> &mpsc::UnboundedReceiver<HotkeyEvent> {
        &self.hotkey_rx
    }

    /// Insert text at the current cursor position
    pub fn insert_text(&self, text: &str) -> Result<(), PlatformError> {
        debug!("Inserting text: {} chars", text.len());
        self.insert_via_clipboard(text)
    }

    /// Save the currently focused window
    pub fn save_foreground_window(&self) {
        unsafe {
            let hwnd = GetForegroundWindow();
            self.saved_window.store(hwnd.0 as isize, Ordering::SeqCst);
            debug!("Saved foreground window: {:?}", hwnd);
        }
    }

    /// Restore focus to the saved window
    pub fn restore_foreground_window(&self) -> Result<(), PlatformError> {
        let hwnd_raw = self.saved_window.load(Ordering::SeqCst);
        if hwnd_raw != 0 {
            unsafe {
                let hwnd = HWND(hwnd_raw as *mut std::ffi::c_void);
                // Small delay to allow our window to minimize
                std::thread::sleep(std::time::Duration::from_millis(50));

                if SetForegroundWindow(hwnd).as_bool() {
                    debug!("Restored foreground window");
                    Ok(())
                } else {
                    warn!("Failed to restore foreground window");
                    Err(PlatformError::WindowError(
                        "SetForegroundWindow failed".to_string(),
                    ))
                }
            }
        } else {
            Ok(())
        }
    }

    /// Create system tray icon
    pub fn create_tray(&mut self, _tooltip: &str) -> Result<(), PlatformError> {
        let (tray_tx, tray_rx) = mpsc::unbounded_channel();
        self.tray_tx = Some(tray_tx);
        self.tray_rx = Some(tray_rx);

        // TODO: Implement actual tray icon using Shell_NotifyIconW
        info!("System tray created (placeholder)");

        Ok(())
    }

    /// Get tray action receiver
    pub fn tray_receiver(&self) -> Option<&mpsc::UnboundedReceiver<TrayAction>> {
        self.tray_rx.as_ref()
    }

    /// Get device fingerprint for licensing
    pub fn get_device_id(&self) -> Result<String, PlatformError> {
        // Generate device ID from hardware info
        // For now, use machine GUID from registry
        use sha2::{Digest, Sha256};
        use windows::Win32::System::Registry::{
            RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, KEY_READ,
        };

        let mut machine_guid = String::new();

        unsafe {
            let key_path: Vec<u16> = "SOFTWARE\\Microsoft\\Cryptography"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let value_name: Vec<u16> = "MachineGuid"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let mut hkey = windows::Win32::System::Registry::HKEY::default();
            if RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                windows::core::PCWSTR(key_path.as_ptr()),
                0,
                KEY_READ,
                &mut hkey,
            )
            .is_ok()
            {
                let mut buffer = [0u16; 128];
                let mut size = (buffer.len() * 2) as u32;

                if RegQueryValueExW(
                    hkey,
                    windows::core::PCWSTR(value_name.as_ptr()),
                    None,
                    None,
                    Some(buffer.as_mut_ptr() as *mut u8),
                    Some(&mut size),
                )
                .is_ok()
                {
                    let len = (size as usize / 2).saturating_sub(1);
                    machine_guid = String::from_utf16_lossy(&buffer[..len]);
                }
            }
        }

        if machine_guid.is_empty() {
            return Err(PlatformError::DeviceIdError(
                "Could not read MachineGuid".to_string(),
            ));
        }

        // Hash it for privacy
        let mut hasher = Sha256::new();
        hasher.update(machine_guid.as_bytes());
        hasher.update(b"mindtype-v1");
        let result = hasher.finalize();

        Ok(format!("{:x}", result)[..32].to_string())
    }
}

impl Default for WindowsPlatform {
    fn default() -> Self {
        Self::new().expect("Failed to create Windows platform")
    }
}
