//! Linux platform implementation

use crate::error::PlatformError;
use crate::{HotkeyEvent, TrayAction};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    ConnectionExt, GrabMode, KeyPressEvent, ModMask, Window, KEY_PRESS_EVENT,
};
use x11rb::protocol::Event;

/// Linux platform implementation
pub struct LinuxPlatform {
    hotkey_tx: mpsc::UnboundedSender<HotkeyEvent>,
    hotkey_rx: mpsc::UnboundedReceiver<HotkeyEvent>,
    tray_tx: Option<mpsc::UnboundedSender<TrayAction>>,
    tray_rx: Option<mpsc::UnboundedReceiver<TrayAction>>,
    /// Store the saved window ID
    saved_window: Arc<AtomicU32>,
    hotkey_registered: Arc<AtomicBool>,
    hotkey_thread: Option<std::thread::JoinHandle<()>>,
}

impl LinuxPlatform {
    /// Create a new Linux platform instance
    pub fn new() -> Result<Self, PlatformError> {
        let (hotkey_tx, hotkey_rx) = mpsc::unbounded_channel();

        Ok(Self {
            hotkey_tx,
            hotkey_rx,
            tray_tx: None,
            tray_rx: None,
            saved_window: Arc::new(AtomicU32::new(0)),
            hotkey_registered: Arc::new(AtomicBool::new(false)),
            hotkey_thread: None,
        })
    }

    /// Parse hotkey string into modifiers and keysym
    fn parse_hotkey(combo: &str) -> Result<(u16, u32), PlatformError> {
        let parts: Vec<String> = combo.split('+').map(|s| s.trim().to_uppercase()).collect();

        if parts.is_empty() {
            return Err(PlatformError::InvalidHotkeyFormat(
                "Empty hotkey".to_string(),
            ));
        }

        let mut modifiers: u16 = 0;
        let mut key = None;

        for (i, part) in parts.iter().enumerate() {
            match part.as_str() {
                "CTRL" | "CONTROL" => modifiers |= ModMask::CONTROL.into(),
                "ALT" => modifiers |= ModMask::M1.into(), // Mod1 is typically Alt
                "SHIFT" => modifiers |= ModMask::SHIFT.into(),
                "WIN" | "SUPER" => modifiers |= ModMask::M4.into(), // Mod4 is typically Super
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

    /// Parse a key name to X11 keysym
    fn parse_key(key: &str) -> Result<u32, PlatformError> {
        // X11 keysyms for common keys
        // Handle single character keys
        if key.len() == 1 {
            let c = key.chars().next().unwrap();
            if c.is_ascii_lowercase() {
                // Lowercase letter keysyms start at 0x61
                return Ok(c as u32);
            }
            if c.is_ascii_uppercase() {
                // Uppercase letter keysyms start at 0x41
                return Ok(c as u32);
            }
            if c.is_ascii_digit() {
                // Digit keysyms are same as ASCII
                return Ok(c as u32);
            }
        }

        // Handle special keys (XK_ values)
        match key {
            "SPACE" => Ok(0x0020),
            "ENTER" | "RETURN" => Ok(0xFF0D),
            "TAB" => Ok(0xFF09),
            "ESCAPE" | "ESC" => Ok(0xFF1B),
            "BACKSPACE" => Ok(0xFF08),
            "DELETE" | "DEL" => Ok(0xFFFF),
            "INSERT" | "INS" => Ok(0xFF63),
            "HOME" => Ok(0xFF50),
            "END" => Ok(0xFF57),
            "PAGEUP" | "PGUP" => Ok(0xFF55),
            "PAGEDOWN" | "PGDN" => Ok(0xFF56),
            "UP" => Ok(0xFF52),
            "DOWN" => Ok(0xFF54),
            "LEFT" => Ok(0xFF51),
            "RIGHT" => Ok(0xFF53),
            "F1" => Ok(0xFFBE),
            "F2" => Ok(0xFFBF),
            "F3" => Ok(0xFFC0),
            "F4" => Ok(0xFFC1),
            "F5" => Ok(0xFFC2),
            "F6" => Ok(0xFFC3),
            "F7" => Ok(0xFFC4),
            "F8" => Ok(0xFFC5),
            "F9" => Ok(0xFFC6),
            "F10" => Ok(0xFFC7),
            "F11" => Ok(0xFFC8),
            "F12" => Ok(0xFFC9),
            _ => Err(PlatformError::InvalidHotkeyFormat(format!(
                "Unknown key: {}",
                key
            ))),
        }
    }

    /// Get keycode from keysym using X11 connection
    fn keysym_to_keycode(
        conn: &impl Connection,
        keysym: u32,
    ) -> Result<u8, PlatformError> {
        // Get the keyboard mapping
        let setup = conn.setup();
        let min_keycode = setup.min_keycode;
        let max_keycode = setup.max_keycode;

        let reply = conn
            .get_keyboard_mapping(min_keycode, max_keycode - min_keycode + 1)
            .map_err(|e| PlatformError::HotkeyRegistration(format!("X11 error: {}", e)))?
            .reply()
            .map_err(|e| PlatformError::HotkeyRegistration(format!("X11 reply error: {}", e)))?;

        let keysyms_per_keycode = reply.keysyms_per_keycode as usize;
        let keysyms = reply.keysyms;

        for keycode in min_keycode..=max_keycode {
            let offset = (keycode - min_keycode) as usize * keysyms_per_keycode;
            for i in 0..keysyms_per_keycode {
                if keysyms.get(offset + i) == Some(&keysym) {
                    return Ok(keycode);
                }
            }
        }

        Err(PlatformError::InvalidHotkeyFormat(format!(
            "Keysym 0x{:X} not found in keyboard mapping",
            keysym
        )))
    }

    /// Save current clipboard contents
    fn save_clipboard(&self) -> Option<String> {
        // Try xclip first
        if let Ok(output) = Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .output()
        {
            if output.status.success() {
                return String::from_utf8(output.stdout).ok();
            }
        }

        // Fallback to xsel
        if let Ok(output) = Command::new("xsel")
            .args(["--clipboard", "--output"])
            .output()
        {
            if output.status.success() {
                return String::from_utf8(output.stdout).ok();
            }
        }

        None
    }

    /// Restore clipboard contents
    fn restore_clipboard(&self, text: &str) -> Result<(), PlatformError> {
        self.set_clipboard(text)
    }

    /// Set clipboard contents
    fn set_clipboard(&self, text: &str) -> Result<(), PlatformError> {
        // Try xclip first for clipboard
        let xclip_result = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn();

        match xclip_result {
            Ok(mut child) => {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    stdin
                        .write_all(text.as_bytes())
                        .map_err(|e| PlatformError::ClipboardError(format!("Write failed: {}", e)))?;
                }
                child.wait().map_err(|e| {
                    PlatformError::ClipboardError(format!("xclip failed: {}", e))
                })?;
                Ok(())
            }
            Err(_) => {
                // Fallback to xsel
                let xsel_result = Command::new("xsel")
                    .args(["--clipboard", "--input"])
                    .stdin(std::process::Stdio::piped())
                    .spawn();

                match xsel_result {
                    Ok(mut child) => {
                        use std::io::Write;
                        if let Some(ref mut stdin) = child.stdin {
                            stdin.write_all(text.as_bytes()).map_err(|e| {
                                PlatformError::ClipboardError(format!("Write failed: {}", e))
                            })?;
                        }
                        child.wait().map_err(|e| {
                            PlatformError::ClipboardError(format!("xsel failed: {}", e))
                        })?;
                        Ok(())
                    }
                    Err(_) => Err(PlatformError::ClipboardError(
                        "Neither xclip nor xsel found. Install one of them.".to_string(),
                    )),
                }
            }
        }
    }

    /// Release any stuck modifier keys using xdotool
    fn release_modifier_keys(&self) {
        // Release common modifier keys
        let modifiers = ["ctrl", "alt", "shift", "super"];
        for modifier in modifiers {
            let _ = Command::new("xdotool")
                .args(["keyup", modifier])
                .output();
        }
    }

    /// Insert text using clipboard with clipboard preservation
    fn insert_via_clipboard(&self, text: &str) -> Result<(), PlatformError> {
        // Save current clipboard contents
        let saved_clipboard = self.save_clipboard();
        debug!("Saved clipboard: {} chars", saved_clipboard.as_ref().map(|s| s.len()).unwrap_or(0));

        // Release any stuck modifier keys
        self.release_modifier_keys();

        // Set clipboard to new text
        self.set_clipboard(text)?;

        // Small delay before paste
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Send Ctrl+V using xdotool
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

    /// Send Ctrl+V key combination using xdotool
    fn send_paste_keys(&self) -> Result<(), PlatformError> {
        // Small delay for clipboard to be ready
        std::thread::sleep(std::time::Duration::from_millis(50));

        let output = Command::new("xdotool")
            .args(["key", "ctrl+v"])
            .output()
            .map_err(|e| {
                PlatformError::TextInsertionFailed(format!("xdotool failed: {}", e))
            })?;

        if !output.status.success() {
            return Err(PlatformError::TextInsertionFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Register a global hotkey
    pub fn register_hotkey(&mut self, combo: &str) -> Result<(), PlatformError> {
        if self.hotkey_registered.load(Ordering::SeqCst) {
            return Err(PlatformError::HotkeyAlreadyRegistered);
        }

        let (modifiers, keysym) = Self::parse_hotkey(combo)?;
        let tx = self.hotkey_tx.clone();
        let registered = self.hotkey_registered.clone();
        let combo_str = combo.to_string();

        info!("Registering hotkey: {}", combo);

        // Spawn thread to handle X11 key events
        let handle = std::thread::spawn(move || {
            let conn = match x11rb::connect(None) {
                Ok((conn, _)) => conn,
                Err(e) => {
                    error!("Failed to connect to X11: {}", e);
                    return;
                }
            };

            let root = conn.setup().roots[0].root;

            // Get keycode from keysym
            let keycode = match Self::keysym_to_keycode(&conn, keysym) {
                Ok(kc) => kc,
                Err(e) => {
                    error!("Failed to get keycode: {}", e);
                    return;
                }
            };

            // Grab the key globally
            // We need to grab for all possible combinations of Caps/Num lock
            let lock_masks: [u16; 4] = [
                0,
                ModMask::LOCK.into(),          // Caps Lock
                ModMask::M2.into(),             // Num Lock (usually Mod2)
                ModMask::LOCK.bits() | ModMask::M2.bits(),
            ];

            for lock_mask in lock_masks {
                if conn
                    .grab_key(
                        false,
                        root,
                        (modifiers | lock_mask).into(),
                        keycode,
                        GrabMode::ASYNC,
                        GrabMode::ASYNC,
                    )
                    .is_err()
                {
                    error!("Failed to grab key for hotkey: {}", combo_str);
                    return;
                }
            }

            registered.store(true, Ordering::SeqCst);
            debug!("Hotkey registered, entering event loop");

            // Event loop
            loop {
                if !registered.load(Ordering::SeqCst) {
                    break;
                }

                match conn.poll_for_event() {
                    Ok(Some(event)) => {
                        if let Event::KeyPress(key_event) = event {
                            if key_event.detail == keycode {
                                debug!("Hotkey pressed");
                                let _ = tx.send(HotkeyEvent::Pressed);
                            }
                        }
                    }
                    Ok(None) => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => {
                        error!("X11 event error: {}", e);
                        break;
                    }
                }
            }

            // Ungrab keys
            for lock_mask in lock_masks {
                let _ = conn.ungrab_key(keycode, root, (modifiers | lock_mask).into());
            }
            registered.store(false, Ordering::SeqCst);
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
        // Thread will clean up on next iteration
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
        // Get active window using xdotool
        if let Ok(output) = Command::new("xdotool").args(["getactivewindow"]).output() {
            if output.status.success() {
                if let Ok(window_id) = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse::<u32>()
                {
                    self.saved_window.store(window_id, Ordering::SeqCst);
                    debug!("Saved foreground window: {}", window_id);
                }
            }
        }
    }

    /// Restore focus to the saved window
    pub fn restore_foreground_window(&self) -> Result<(), PlatformError> {
        let window_id = self.saved_window.load(Ordering::SeqCst);
        if window_id != 0 {
            // Small delay to allow our window to minimize
            std::thread::sleep(std::time::Duration::from_millis(50));

            let output = Command::new("xdotool")
                .args(["windowactivate", &window_id.to_string()])
                .output()
                .map_err(|e| {
                    PlatformError::WindowError(format!("xdotool failed: {}", e))
                })?;

            if output.status.success() {
                debug!("Restored foreground window");
                Ok(())
            } else {
                warn!("Failed to restore foreground window");
                Err(PlatformError::WindowError(
                    "windowactivate failed".to_string(),
                ))
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

        // Note: Actual tray implementation would use libappindicator or similar
        // For Tauri apps, the tray is handled by Tauri's tray API
        info!("System tray created (handled by Tauri)");

        Ok(())
    }

    /// Get tray action receiver
    pub fn tray_receiver(&self) -> Option<&mpsc::UnboundedReceiver<TrayAction>> {
        self.tray_rx.as_ref()
    }

    /// Get device fingerprint for licensing
    pub fn get_device_id(&self) -> Result<String, PlatformError> {
        use sha2::{Digest, Sha256};

        // Read machine-id (standard on most Linux systems)
        let machine_id = std::fs::read_to_string("/etc/machine-id")
            .or_else(|_| std::fs::read_to_string("/var/lib/dbus/machine-id"))
            .map_err(|e| {
                PlatformError::DeviceIdError(format!("Could not read machine-id: {}", e))
            })?;

        let machine_id = machine_id.trim();
        if machine_id.is_empty() {
            return Err(PlatformError::DeviceIdError(
                "Empty machine-id".to_string(),
            ));
        }

        // Hash it for privacy
        let mut hasher = Sha256::new();
        hasher.update(machine_id.as_bytes());
        hasher.update(b"mindtype-v1");
        let result = hasher.finalize();

        Ok(format!("{:x}", result)[..32].to_string())
    }
}

impl Default for LinuxPlatform {
    fn default() -> Self {
        Self::new().expect("Failed to create Linux platform")
    }
}
