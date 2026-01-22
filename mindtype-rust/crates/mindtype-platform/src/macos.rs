//! macOS platform implementation

use crate::error::PlatformError;
use crate::{HotkeyEvent, TrayAction};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use core_foundation::base::TCFType;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType,
};
use objc::runtime::{Class, Object};
use objc::{msg_send, sel, sel_impl};
use std::ffi::c_void;

/// Key codes for macOS (virtual key codes)
mod keycode {
    pub const V: u16 = 0x09;
    pub const SPACE: u16 = 0x31;
    pub const RETURN: u16 = 0x24;
    pub const TAB: u16 = 0x30;
    pub const ESCAPE: u16 = 0x35;
    pub const DELETE: u16 = 0x33;
    pub const FORWARD_DELETE: u16 = 0x75;
    pub const HOME: u16 = 0x73;
    pub const END: u16 = 0x77;
    pub const PAGE_UP: u16 = 0x74;
    pub const PAGE_DOWN: u16 = 0x79;
    pub const UP: u16 = 0x7E;
    pub const DOWN: u16 = 0x7D;
    pub const LEFT: u16 = 0x7B;
    pub const RIGHT: u16 = 0x7C;
    pub const F1: u16 = 0x7A;
    pub const F2: u16 = 0x78;
    pub const F3: u16 = 0x63;
    pub const F4: u16 = 0x76;
    pub const F5: u16 = 0x60;
    pub const F6: u16 = 0x61;
    pub const F7: u16 = 0x62;
    pub const F8: u16 = 0x64;
    pub const F9: u16 = 0x65;
    pub const F10: u16 = 0x6D;
    pub const F11: u16 = 0x67;
    pub const F12: u16 = 0x6F;
}

/// macOS platform implementation
pub struct MacOSPlatform {
    hotkey_tx: mpsc::UnboundedSender<HotkeyEvent>,
    hotkey_rx: mpsc::UnboundedReceiver<HotkeyEvent>,
    tray_tx: Option<mpsc::UnboundedSender<TrayAction>>,
    tray_rx: Option<mpsc::UnboundedReceiver<TrayAction>>,
    /// Store the saved window's process ID
    saved_pid: Arc<AtomicU64>,
    hotkey_registered: Arc<AtomicBool>,
    hotkey_thread: Option<std::thread::JoinHandle<()>>,
    /// Target key code for hotkey
    target_keycode: Arc<AtomicU64>,
    /// Target modifiers for hotkey
    target_modifiers: Arc<AtomicU64>,
}

impl MacOSPlatform {
    /// Create a new macOS platform instance
    pub fn new() -> Result<Self, PlatformError> {
        let (hotkey_tx, hotkey_rx) = mpsc::unbounded_channel();

        Ok(Self {
            hotkey_tx,
            hotkey_rx,
            tray_tx: None,
            tray_rx: None,
            saved_pid: Arc::new(AtomicU64::new(0)),
            hotkey_registered: Arc::new(AtomicBool::new(false)),
            hotkey_thread: None,
            target_keycode: Arc::new(AtomicU64::new(0)),
            target_modifiers: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Parse hotkey string into modifiers and key code
    fn parse_hotkey(combo: &str) -> Result<(CGEventFlags, u16), PlatformError> {
        let parts: Vec<String> = combo.split('+').map(|s| s.trim().to_uppercase()).collect();

        if parts.is_empty() {
            return Err(PlatformError::InvalidHotkeyFormat(
                "Empty hotkey".to_string(),
            ));
        }

        let mut modifiers = CGEventFlags::empty();
        let mut key = None;

        for (i, part) in parts.iter().enumerate() {
            match part.as_str() {
                "CTRL" | "CONTROL" => modifiers |= CGEventFlags::CGEventFlagControl,
                "ALT" | "OPTION" => modifiers |= CGEventFlags::CGEventFlagAlternate,
                "SHIFT" => modifiers |= CGEventFlags::CGEventFlagShift,
                "CMD" | "COMMAND" | "WIN" | "SUPER" => {
                    modifiers |= CGEventFlags::CGEventFlagCommand
                }
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
    fn parse_key(key: &str) -> Result<u16, PlatformError> {
        // Handle single character keys (letters)
        if key.len() == 1 {
            let c = key.chars().next().unwrap().to_ascii_uppercase();
            if c.is_ascii_alphabetic() {
                // macOS key codes for A-Z
                let keycode = match c {
                    'A' => 0x00,
                    'B' => 0x0B,
                    'C' => 0x08,
                    'D' => 0x02,
                    'E' => 0x0E,
                    'F' => 0x03,
                    'G' => 0x05,
                    'H' => 0x04,
                    'I' => 0x22,
                    'J' => 0x26,
                    'K' => 0x28,
                    'L' => 0x25,
                    'M' => 0x2E,
                    'N' => 0x2D,
                    'O' => 0x1F,
                    'P' => 0x23,
                    'Q' => 0x0C,
                    'R' => 0x0F,
                    'S' => 0x01,
                    'T' => 0x11,
                    'U' => 0x20,
                    'V' => 0x09,
                    'W' => 0x0D,
                    'X' => 0x07,
                    'Y' => 0x10,
                    'Z' => 0x06,
                    _ => {
                        return Err(PlatformError::InvalidHotkeyFormat(format!(
                            "Unknown key: {}",
                            key
                        )))
                    }
                };
                return Ok(keycode);
            }
            // Handle digits
            if c.is_ascii_digit() {
                let keycode = match c {
                    '0' => 0x1D,
                    '1' => 0x12,
                    '2' => 0x13,
                    '3' => 0x14,
                    '4' => 0x15,
                    '5' => 0x17,
                    '6' => 0x16,
                    '7' => 0x1A,
                    '8' => 0x1C,
                    '9' => 0x19,
                    _ => unreachable!(),
                };
                return Ok(keycode);
            }
        }

        // Handle special keys
        match key {
            "SPACE" => Ok(keycode::SPACE),
            "ENTER" | "RETURN" => Ok(keycode::RETURN),
            "TAB" => Ok(keycode::TAB),
            "ESCAPE" | "ESC" => Ok(keycode::ESCAPE),
            "BACKSPACE" | "DELETE" => Ok(keycode::DELETE),
            "FORWARDDELETE" | "DEL" => Ok(keycode::FORWARD_DELETE),
            "HOME" => Ok(keycode::HOME),
            "END" => Ok(keycode::END),
            "PAGEUP" | "PGUP" => Ok(keycode::PAGE_UP),
            "PAGEDOWN" | "PGDN" => Ok(keycode::PAGE_DOWN),
            "UP" => Ok(keycode::UP),
            "DOWN" => Ok(keycode::DOWN),
            "LEFT" => Ok(keycode::LEFT),
            "RIGHT" => Ok(keycode::RIGHT),
            "F1" => Ok(keycode::F1),
            "F2" => Ok(keycode::F2),
            "F3" => Ok(keycode::F3),
            "F4" => Ok(keycode::F4),
            "F5" => Ok(keycode::F5),
            "F6" => Ok(keycode::F6),
            "F7" => Ok(keycode::F7),
            "F8" => Ok(keycode::F8),
            "F9" => Ok(keycode::F9),
            "F10" => Ok(keycode::F10),
            "F11" => Ok(keycode::F11),
            "F12" => Ok(keycode::F12),
            _ => Err(PlatformError::InvalidHotkeyFormat(format!(
                "Unknown key: {}",
                key
            ))),
        }
    }

    /// Save current clipboard contents
    fn save_clipboard(&self) -> Option<String> {
        unsafe {
            let pasteboard_class = Class::get("NSPasteboard")?;
            let pasteboard: *mut Object = msg_send![pasteboard_class, generalPasteboard];
            if pasteboard.is_null() {
                return None;
            }

            let nsstring_class = Class::get("NSString")?;
            let nsstring_pboard_type: *mut Object =
                msg_send![nsstring_class, stringWithUTF8String:b"public.utf8-plain-text\0".as_ptr()];

            let content: *mut Object = msg_send![pasteboard, stringForType:nsstring_pboard_type];
            if content.is_null() {
                return None;
            }

            let utf8: *const i8 = msg_send![content, UTF8String];
            if utf8.is_null() {
                return None;
            }

            let cstr = std::ffi::CStr::from_ptr(utf8);
            cstr.to_str().ok().map(|s| s.to_string())
        }
    }

    /// Restore clipboard contents
    fn restore_clipboard(&self, text: &str) -> Result<(), PlatformError> {
        unsafe {
            let pasteboard_class = Class::get("NSPasteboard")
                .ok_or_else(|| PlatformError::ClipboardError("NSPasteboard not found".into()))?;
            let pasteboard: *mut Object = msg_send![pasteboard_class, generalPasteboard];
            if pasteboard.is_null() {
                return Err(PlatformError::ClipboardError("Failed to get pasteboard".into()));
            }

            let _: i64 = msg_send![pasteboard, clearContents];

            let nsstring_class = Class::get("NSString")
                .ok_or_else(|| PlatformError::ClipboardError("NSString not found".into()))?;
            let nsstring: *mut Object = msg_send![nsstring_class, alloc];
            let nsstring: *mut Object =
                msg_send![nsstring, initWithBytes:text.as_ptr() length:text.len() encoding:4u64];
            if nsstring.is_null() {
                return Err(PlatformError::ClipboardError("Failed to create NSString".into()));
            }

            let nsstring_pboard_type: *mut Object =
                msg_send![nsstring_class, stringWithUTF8String:b"public.utf8-plain-text\0".as_ptr()];

            let _: bool = msg_send![pasteboard, setString:nsstring forType:nsstring_pboard_type];
            let _: () = msg_send![nsstring, release];
        }
        Ok(())
    }

    /// Release any stuck modifier keys
    fn release_modifier_keys(&self) {
        let modifiers = [
            CGEventFlags::CGEventFlagCommand,
            CGEventFlags::CGEventFlagShift,
            CGEventFlags::CGEventFlagControl,
            CGEventFlags::CGEventFlagAlternate,
        ];

        for modifier in modifiers {
            if let Ok(source) = core_graphics::event_source::CGEventSource::new(
                core_graphics::event_source::CGEventSourceStateID::HIDSystemState,
            ) {
                // Create a key up event to release modifier
                if let Ok(event) = CGEvent::new_keyboard_event(source, 0, false) {
                    event.set_flags(modifier);
                    event.post(CGEventTapLocation::HID);
                }
            }
        }
    }

    /// Insert text using clipboard and Cmd+V with clipboard preservation
    fn insert_via_clipboard(&self, text: &str) -> Result<(), PlatformError> {
        // Save current clipboard contents
        let saved_clipboard = self.save_clipboard();
        debug!("Saved clipboard: {} chars", saved_clipboard.as_ref().map(|s| s.len()).unwrap_or(0));

        // Release any stuck modifier keys
        self.release_modifier_keys();

        unsafe {
            // Get NSPasteboard
            let pasteboard_class = Class::get("NSPasteboard")
                .ok_or_else(|| PlatformError::ClipboardError("NSPasteboard not found".into()))?;
            let pasteboard: *mut Object = msg_send![pasteboard_class, generalPasteboard];
            if pasteboard.is_null() {
                return Err(PlatformError::ClipboardError(
                    "Failed to get pasteboard".into(),
                ));
            }

            // Clear the pasteboard
            let _: i64 = msg_send![pasteboard, clearContents];

            // Create NSString from text
            let nsstring_class = Class::get("NSString")
                .ok_or_else(|| PlatformError::ClipboardError("NSString not found".into()))?;
            let nsstring: *mut Object = msg_send![nsstring_class, alloc];
            let nsstring: *mut Object =
                msg_send![nsstring, initWithBytes:text.as_ptr() length:text.len() encoding:4u64]; // NSUTF8StringEncoding = 4
            if nsstring.is_null() {
                return Err(PlatformError::ClipboardError(
                    "Failed to create NSString".into(),
                ));
            }

            // Get NSPasteboardTypeString
            let nsstring_pboard_type: *mut Object =
                msg_send![nsstring_class, stringWithUTF8String:b"public.utf8-plain-text\0".as_ptr()];

            // Set string to pasteboard
            let result: bool = msg_send![pasteboard, setString:nsstring forType:nsstring_pboard_type];
            if !result {
                let _: () = msg_send![nsstring, release];
                return Err(PlatformError::ClipboardError(
                    "Failed to set clipboard content".into(),
                ));
            }

            // Release NSString
            let _: () = msg_send![nsstring, release];
        }

        // Small delay before paste
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Send Cmd+V
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

    /// Send Cmd+V key combination
    fn send_paste_keys(&self) -> Result<(), PlatformError> {
        // Create Cmd+V key event
        let source = core_graphics::event_source::CGEventSource::new(
            core_graphics::event_source::CGEventSourceStateID::HIDSystemState,
        )
        .map_err(|_| PlatformError::TextInsertionFailed("Failed to create event source".into()))?;

        // Cmd down + V down
        let v_down = CGEvent::new_keyboard_event(source.clone(), keycode::V, true)
            .map_err(|_| PlatformError::TextInsertionFailed("Failed to create key event".into()))?;
        v_down.set_flags(CGEventFlags::CGEventFlagCommand);
        v_down.post(CGEventTapLocation::HID);

        // V up + Cmd up
        let v_up = CGEvent::new_keyboard_event(source, keycode::V, false)
            .map_err(|_| PlatformError::TextInsertionFailed("Failed to create key event".into()))?;
        v_up.set_flags(CGEventFlags::CGEventFlagCommand);
        v_up.post(CGEventTapLocation::HID);

        Ok(())
    }

    /// Register a global hotkey
    pub fn register_hotkey(&mut self, combo: &str) -> Result<(), PlatformError> {
        if self.hotkey_registered.load(Ordering::SeqCst) {
            return Err(PlatformError::HotkeyAlreadyRegistered);
        }

        let (modifiers, keycode) = Self::parse_hotkey(combo)?;
        let tx = self.hotkey_tx.clone();
        let registered = self.hotkey_registered.clone();
        let target_keycode = self.target_keycode.clone();
        let target_modifiers = self.target_modifiers.clone();

        // Store target values
        target_keycode.store(keycode as u64, Ordering::SeqCst);
        target_modifiers.store(modifiers.bits(), Ordering::SeqCst);

        info!("Registering hotkey: {}", combo);

        // Spawn thread to handle hotkey events via CGEventTap
        let handle = std::thread::spawn(move || {
            let callback = move |_proxy: core_graphics::event::CGEventTapProxy,
                                 event_type: CGEventType,
                                 event: &CGEvent|
                  -> Option<CGEvent> {
                if event_type == CGEventType::KeyDown {
                    let code = event.get_integer_value_field(
                        core_graphics::event::EventField::KEYBOARD_EVENT_KEYCODE,
                    ) as u16;
                    let flags = event.get_flags();

                    let target_code = target_keycode.load(Ordering::SeqCst) as u16;
                    let target_mods = CGEventFlags::from_bits_truncate(
                        target_modifiers.load(Ordering::SeqCst),
                    );

                    // Check if modifier flags match (ignoring non-modifier flags)
                    let modifier_mask = CGEventFlags::CGEventFlagCommand
                        | CGEventFlags::CGEventFlagShift
                        | CGEventFlags::CGEventFlagControl
                        | CGEventFlags::CGEventFlagAlternate;

                    if code == target_code && (flags & modifier_mask) == target_mods {
                        debug!("Hotkey pressed");
                        let _ = tx.send(HotkeyEvent::Pressed);
                        return None; // Consume the event
                    }
                }
                Some(event.clone())
            };

            // Create event tap
            let tap = CGEventTap::new(
                CGEventTapLocation::HID,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::Default,
                vec![CGEventType::KeyDown, CGEventType::KeyUp],
                callback,
            );

            match tap {
                Ok(tap) => {
                    registered.store(true, Ordering::SeqCst);
                    debug!("Event tap created, starting run loop");

                    // Enable the tap
                    tap.enable();

                    // Add to run loop
                    let source = tap.mach_port_run_loop_source(0).unwrap();
                    let run_loop = CFRunLoop::get_current();
                    run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });

                    // Run the loop
                    CFRunLoop::run_current();

                    registered.store(false, Ordering::SeqCst);
                }
                Err(e) => {
                    error!("Failed to create event tap: {:?}", e);
                }
            }
        });

        self.hotkey_thread = Some(handle);

        // Wait a bit for registration
        std::thread::sleep(std::time::Duration::from_millis(100));

        if !self.hotkey_registered.load(Ordering::SeqCst) {
            return Err(PlatformError::HotkeyRegistration(
                "Failed to register hotkey. Accessibility permission may be required.".to_string(),
            ));
        }

        Ok(())
    }

    /// Unregister the current hotkey
    pub fn unregister_hotkey(&mut self) -> Result<(), PlatformError> {
        self.hotkey_registered.store(false, Ordering::SeqCst);
        // Stop the run loop
        CFRunLoop::get_main().stop();
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

    /// Save the currently focused window (by PID)
    pub fn save_foreground_window(&self) {
        unsafe {
            let workspace_class = Class::get("NSWorkspace");
            if let Some(workspace_class) = workspace_class {
                let workspace: *mut Object = msg_send![workspace_class, sharedWorkspace];
                let app: *mut Object = msg_send![workspace, frontmostApplication];
                if !app.is_null() {
                    let pid: i32 = msg_send![app, processIdentifier];
                    self.saved_pid.store(pid as u64, Ordering::SeqCst);
                    debug!("Saved foreground app PID: {}", pid);
                }
            }
        }
    }

    /// Restore focus to the saved window
    pub fn restore_foreground_window(&self) -> Result<(), PlatformError> {
        let pid = self.saved_pid.load(Ordering::SeqCst);
        if pid != 0 {
            unsafe {
                // Get running application by PID
                let app_class = Class::get("NSRunningApplication")
                    .ok_or_else(|| PlatformError::WindowError("NSRunningApplication not found".into()))?;
                let app: *mut Object =
                    msg_send![app_class, runningApplicationWithProcessIdentifier: pid as i32];

                if !app.is_null() {
                    // Small delay to allow our window to minimize
                    std::thread::sleep(std::time::Duration::from_millis(50));

                    // Activate the application
                    let result: bool =
                        msg_send![app, activateWithOptions: 1u64]; // NSApplicationActivateIgnoringOtherApps

                    if result {
                        debug!("Restored foreground app");
                        return Ok(());
                    } else {
                        warn!("Failed to activate application");
                        return Err(PlatformError::WindowError(
                            "Failed to activate application".to_string(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Create system tray icon
    pub fn create_tray(&mut self, _tooltip: &str) -> Result<(), PlatformError> {
        let (tray_tx, tray_rx) = mpsc::unbounded_channel();
        self.tray_tx = Some(tray_tx);
        self.tray_rx = Some(tray_rx);

        // Note: Actual tray implementation would use NSStatusItem
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
        use std::process::Command;

        // Get hardware UUID using system_profiler
        let output = Command::new("system_profiler")
            .args(["SPHardwareDataType"])
            .output()
            .map_err(|e| PlatformError::DeviceIdError(format!("Failed to run system_profiler: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse Hardware UUID from output
        let uuid = stdout
            .lines()
            .find(|line| line.contains("Hardware UUID"))
            .and_then(|line| line.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .ok_or_else(|| PlatformError::DeviceIdError("Hardware UUID not found".to_string()))?;

        if uuid.is_empty() {
            return Err(PlatformError::DeviceIdError(
                "Empty Hardware UUID".to_string(),
            ));
        }

        // Hash it for privacy
        let mut hasher = Sha256::new();
        hasher.update(uuid.as_bytes());
        hasher.update(b"mindtype-v1");
        let result = hasher.finalize();

        Ok(format!("{:x}", result)[..32].to_string())
    }
}

impl Default for MacOSPlatform {
    fn default() -> Self {
        Self::new().expect("Failed to create macOS platform")
    }
}
