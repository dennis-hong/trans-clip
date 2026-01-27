use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter, Manager};

static LAST_CMD_C_TIME: AtomicU64 = AtomicU64::new(0);
static LAST_CMD_D_TIME: AtomicU64 = AtomicU64::new(0);
static HOTKEY_ENABLED: AtomicBool = AtomicBool::new(false);
static DOUBLE_PRESS_INTERVAL_MS: AtomicU64 = AtomicU64::new(500);
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

const DEFAULT_DOUBLE_PRESS_INTERVAL_MS: u64 = 500;

/// Set the popup position from settings (deprecated - position is now always bottom center)
pub fn set_popup_position(_position: &str) {
    // No-op: popup position is now always bottom center, managed by set_drawer_mode
    log::info!("set_popup_position called but ignored - position is always bottom center");
}

/// Show the window and set focus
/// Before showing, update the monitor index based on cursor position
/// The window position is managed by set_drawer_mode which keeps it at bottom center
pub fn show_window_at_position(window: &tauri::WebviewWindow) {
    // Update monitor index based on cursor position before showing
    crate::commands::window::update_monitor_from_cursor(window.app_handle());

    // Just show and focus the window - position is handled by set_drawer_mode
    let _ = window.show();
    let _ = window.set_focus();

    log::info!("Window shown and focused (monitor updated from cursor)");
}

#[derive(Clone, serde::Serialize)]
pub struct DoubleCopyPayload {
    pub text: String,
    pub timestamp: String,
}

#[derive(Clone, serde::Serialize)]
pub struct PolishPayload {
    pub text: String,
    pub timestamp: String,
}

#[derive(Clone, serde::Serialize)]
pub struct ShowHistoryPayload {
    pub timestamp: String,
}

// ============================================
// macOS Implementation using CGEventTap (raw FFI)
// ============================================

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use core_graphics::event::CGEventTapLocation;
    use std::ffi::c_void;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Key codes on macOS
    const KEY_C: i64 = 8;
    const KEY_D: i64 = 2;
    const KEY_V: i64 = 9;

    // CGEventFlags
    const K_CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x00100000;
    const K_CG_EVENT_FLAG_MASK_SHIFT: u64 = 0x00020000;

    // CGEventTap types
    type CGEventRef = *mut c_void;
    type CGEventTapProxy = *mut c_void;

    type CGEventTapCallBack = extern "C" fn(
        proxy: CGEventTapProxy,
        event_type: u32,
        event: CGEventRef,
        user_info: *mut c_void,
    ) -> CGEventRef;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventTapCreate(
            tap: u32,            // CGEventTapLocation
            place: u32,          // CGEventTapPlacement
            options: u32,        // CGEventTapOptions
            events_of_interest: u64,
            callback: CGEventTapCallBack,
            user_info: *mut c_void,
        ) -> *mut c_void; // CFMachPortRef

        fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
        fn CGEventGetFlags(event: CGEventRef) -> u64;
        fn CFMachPortCreateRunLoopSource(
            allocator: *const c_void,
            port: *mut c_void,
            order: i64,
        ) -> *mut c_void;
        fn CFRunLoopAddSource(
            run_loop: *mut c_void,
            source: *mut c_void,
            mode: *const c_void,
        );
        fn CFRunLoopGetCurrent() -> *mut c_void;
        fn CFRunLoopRun();
        fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    }

    // Event field constants
    const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

    pub fn start_event_tap(app_handle: AppHandle, interval_ms: u64) -> Result<(), String> {
        if HOTKEY_ENABLED.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Store app handle for callback use
        APP_HANDLE.get_or_init(|| app_handle);
        DOUBLE_PRESS_INTERVAL_MS.store(interval_ms, Ordering::SeqCst);
        HOTKEY_ENABLED.store(true, Ordering::SeqCst);

        // Spawn a thread for the event tap
        thread::spawn(move || {
            run_event_tap();
        });

        log::info!("Hotkey event tap started (interval: {}ms)", interval_ms);
        Ok(())
    }

    extern "C" fn event_callback(
        _proxy: CGEventTapProxy,
        event_type: u32,
        event: CGEventRef,
        _user_info: *mut c_void,
    ) -> CGEventRef {
        if !HOTKEY_ENABLED.load(Ordering::SeqCst) {
            return event;
        }

        // CGEventType::KeyDown is 10
        const CG_EVENT_KEY_DOWN: u32 = 10;

        if event_type != CG_EVENT_KEY_DOWN {
            return event;
        }

        unsafe {
            let keycode = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE);
            let flags = CGEventGetFlags(event);

            let is_cmd_pressed = (flags & K_CG_EVENT_FLAG_MASK_COMMAND) != 0;
            let is_shift_pressed = (flags & K_CG_EVENT_FLAG_MASK_SHIFT) != 0;
            let is_c_key = keycode == KEY_C;
            let is_d_key = keycode == KEY_D;
            let is_v_key = keycode == KEY_V;

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            let interval = DOUBLE_PRESS_INTERVAL_MS.load(Ordering::SeqCst);

            // Cmd+Shift+V detected - show clipboard history
            if is_cmd_pressed && is_shift_pressed && is_v_key {
                log::info!("Cmd+Shift+V detected! Showing clipboard history");
                
                // Reset timers to prevent interference
                LAST_CMD_C_TIME.store(0, Ordering::SeqCst);
                LAST_CMD_D_TIME.store(0, Ordering::SeqCst);
                
                thread::spawn(|| {
                    trigger_show_history();
                });
                
                return event;
            }

            // Cmd+C detected - check for double-press for Translation
            if is_cmd_pressed && !is_shift_pressed && is_c_key {
                let last_time = LAST_CMD_C_TIME.swap(now, Ordering::SeqCst);
                
                // Reset D timer to prevent cross-triggering
                LAST_CMD_D_TIME.store(0, Ordering::SeqCst);

                if last_time > 0 && (now - last_time) < interval {
                    log::info!("Double Cmd+C detected! Interval: {}ms", now - last_time);

                    // Reset the timer to prevent triple-press triggers
                    LAST_CMD_C_TIME.store(0, Ordering::SeqCst);

                    // Small delay to let the clipboard update from the copy action
                    thread::spawn(|| {
                        thread::sleep(std::time::Duration::from_millis(100));
                        trigger_translation_or_history();
                    });
                }
            }

            // Cmd+D detected - check for double-press for Polish
            if is_cmd_pressed && !is_shift_pressed && is_d_key {
                let last_time = LAST_CMD_D_TIME.swap(now, Ordering::SeqCst);
                
                // Reset C timer to prevent cross-triggering
                LAST_CMD_C_TIME.store(0, Ordering::SeqCst);

                if last_time > 0 && (now - last_time) < interval {
                    log::info!("Double Cmd+D detected! Interval: {}ms", now - last_time);

                    // Reset the timer to prevent triple-press triggers
                    LAST_CMD_D_TIME.store(0, Ordering::SeqCst);

                    // Small delay then trigger polish
                    thread::spawn(|| {
                        thread::sleep(std::time::Duration::from_millis(100));
                        trigger_polish();
                    });
                }
            }
        }

        event
    }

    fn run_event_tap() {
        unsafe {
            // CGEventMaskBit for KeyDown (10) = 1 << 10 = 1024
            let event_mask: u64 = 1 << 10;

            let tap = CGEventTapCreate(
                CGEventTapLocation::HID as u32,
                0, // kCGHeadInsertEventTap
                1, // kCGEventTapOptionListenOnly
                event_mask,
                event_callback,
                std::ptr::null_mut(),
            );

            if tap.is_null() {
                log::error!("Failed to create CGEventTap. Accessibility permission may be required.");
                return;
            }

            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            if source.is_null() {
                log::error!("Failed to create run loop source");
                return;
            }

            let run_loop = CFRunLoopGetCurrent();

            // kCFRunLoopCommonModes
            #[link(name = "CoreFoundation", kind = "framework")]
            extern "C" {
                static kCFRunLoopCommonModes: *const c_void;
            }

            CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);

            log::info!("CGEventTap is running");
            CFRunLoopRun();
        }
    }

    fn trigger_translation_or_history() {
        if let Some(app_handle) = APP_HANDLE.get() {
            // Get clipboard text
            match get_clipboard_text() {
                Ok(text) if !text.trim().is_empty() => {
                    let payload = DoubleCopyPayload {
                        text,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };

                    if let Err(e) = app_handle.emit("double_copy_detected", payload) {
                        log::error!("Failed to emit double_copy_detected event: {}", e);
                    } else {
                        log::info!("Emitted double_copy_detected event");

                        // Show and focus the window at the configured position
                        if let Some(window) = app_handle.get_webview_window("main") {
                            show_window_at_position(&window);
                        }
                    }
                }
                Ok(_) => {
                    // Clipboard is empty - show history instead
                    log::info!("Clipboard is empty, showing history panel");
                    trigger_show_history();
                }
                Err(e) => {
                    log::error!("Failed to get clipboard text: {}", e);
                    // On error, also show history as fallback
                    trigger_show_history();
                }
            }
        }
    }

    fn trigger_show_history() {
        if let Some(app_handle) = APP_HANDLE.get() {
            if let Some(window) = app_handle.get_webview_window("main") {
                // Check if window is visible - if so, hide it (toggle behavior)
                if let Ok(is_visible) = window.is_visible() {
                    if is_visible {
                        log::info!("Window is visible, hiding it (toggle)");
                        let _ = window.hide();
                        return;
                    }
                }
            }

            let payload = ShowHistoryPayload {
                timestamp: chrono::Utc::now().to_rfc3339(),
            };

            if let Err(e) = app_handle.emit("show_history", payload) {
                log::error!("Failed to emit show_history event: {}", e);
            } else {
                log::info!("Emitted show_history event");

                // Show and focus the window at the configured position
                if let Some(window) = app_handle.get_webview_window("main") {
                    show_window_at_position(&window);
                }
            }
        }
    }

    fn trigger_polish() {
        if let Some(app_handle) = APP_HANDLE.get() {
            // First, simulate Cmd+C to copy the selected text
            // since Cmd+D doesn't copy anything
            log::info!("trigger_polish: Simulating Cmd+C to copy selected text");
            if let Err(e) = simulate_copy() {
                log::error!("Failed to simulate copy: {}", e);
                return;
            }
            
            // Wait for clipboard to update (need more time for AppleScript + clipboard sync)
            thread::sleep(std::time::Duration::from_millis(200));
            
            // Get clipboard text
            match get_clipboard_text() {
                Ok(text) if !text.trim().is_empty() => {
                    let payload = PolishPayload {
                        text,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };

                    if let Err(e) = app_handle.emit("polish_detected", payload) {
                        log::error!("Failed to emit polish_detected event: {}", e);
                    } else {
                        log::info!("Emitted polish_detected event");

                        // Show and focus the window at the configured position
                        if let Some(window) = app_handle.get_webview_window("main") {
                            show_window_at_position(&window);
                        }
                    }
                }
                Ok(_) => {
                    log::info!("Clipboard is empty, skipping polish");
                }
                Err(e) => {
                    log::error!("Failed to get clipboard text: {}", e);
                }
            }
        }
    }
    
    /// Simulate Cmd+C to copy selected text to clipboard
    fn simulate_copy() -> Result<(), String> {
        use std::process::Command;
        
        // Use key code 8 (c key) with command down - more reliable than keystroke
        let script = r#"
            tell application "System Events"
                key code 8 using command down
            end tell
        "#;
        
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| format!("Failed to execute osascript: {}", e))?;
        
        if output.status.success() {
            log::info!("Simulated Cmd+C (key code 8) successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("osascript failed: {}", stderr))
        }
    }

}

#[cfg(target_os = "macos")]
fn get_clipboard_text() -> Result<String, String> {
    use std::process::Command;

    let output = Command::new("pbpaste")
        .output()
        .map_err(|e| e.to_string())?;

    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
fn get_clipboard_text() -> Result<String, String> {
    Err("Clipboard not supported on this platform".to_string())
}

// ============================================
// HotkeyManager - Public API
// ============================================

pub struct HotkeyManager {
    app_handle: AppHandle,
    double_press_interval_ms: u64,
}

impl HotkeyManager {
    pub fn new(app_handle: AppHandle, interval_ms: u64) -> Self {
        Self {
            app_handle,
            double_press_interval_ms: if interval_ms > 0 {
                interval_ms
            } else {
                DEFAULT_DOUBLE_PRESS_INTERVAL_MS
            },
        }
    }

    #[cfg(target_os = "macos")]
    pub fn start(&self) -> Result<(), String> {
        macos::start_event_tap(self.app_handle.clone(), self.double_press_interval_ms)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn start(&self) -> Result<(), String> {
        log::warn!("Hotkey monitoring is only supported on macOS");
        Ok(())
    }

}

// ============================================
// Accessibility Permission Check
// ============================================

#[cfg(target_os = "macos")]
pub fn check_accessibility_permission() -> bool {
    use std::ffi::c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    }

    unsafe {
        // Check without prompting
        AXIsProcessTrustedWithOptions(std::ptr::null())
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility_permission() -> bool {
    true
}

#[cfg(target_os = "macos")]
pub fn request_accessibility_permission() {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;
    use std::ffi::c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    }

    unsafe {
        // Create options dictionary with kAXTrustedCheckOptionPrompt = true
        // This will show the system prompt to add the app to accessibility
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

        // This call will trigger the system prompt
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef() as *const c_void);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_accessibility_permission() {
    // No-op on other platforms
}
