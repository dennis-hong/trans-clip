use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

static LAST_CMD_C_TIME: AtomicU64 = AtomicU64::new(0);
static HOTKEY_ENABLED: AtomicBool = AtomicBool::new(false);
static DOUBLE_PRESS_INTERVAL_MS: AtomicU64 = AtomicU64::new(500);
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
// Popup position: 0 = cursor, 1 = center, 2 = top-right
static POPUP_POSITION: AtomicU8 = AtomicU8::new(0);

const DEFAULT_DOUBLE_PRESS_INTERVAL_MS: u64 = 500;

/// Set the popup position from settings
pub fn set_popup_position(position: &str) {
    let value = match position {
        "cursor" => 0,
        "center" => 1,
        "top-right" => 2,
        _ => 0,
    };
    POPUP_POSITION.store(value, Ordering::SeqCst);
    log::info!("Popup position set to: {} ({})", position, value);
}

/// Get current popup position
fn get_popup_position() -> u8 {
    POPUP_POSITION.load(Ordering::SeqCst)
}

/// Get mouse cursor position (macOS only)
#[cfg(target_os = "macos")]
fn get_mouse_position() -> Option<(f64, f64)> {
    use std::ffi::c_void;
    
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventCreate(source: *const c_void) -> *mut c_void;
        fn CGEventGetLocation(event: *const c_void) -> CGPoint;
        fn CFRelease(cf: *mut c_void);
    }
    
    #[repr(C)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    
    unsafe {
        let event = CGEventCreate(std::ptr::null());
        if event.is_null() {
            return None;
        }
        let point = CGEventGetLocation(event);
        CFRelease(event);
        Some((point.x, point.y))
    }
}

#[cfg(not(target_os = "macos"))]
fn get_mouse_position() -> Option<(f64, f64)> {
    None
}

/// Position and show the window based on the current popup position setting
pub fn show_window_at_position(window: &tauri::WebviewWindow) {
    let position_type = get_popup_position();
    
    // Get window size
    let window_size = window.outer_size().unwrap_or(tauri::PhysicalSize { width: 400, height: 500 });
    
    // Get monitor info
    let monitor = window.current_monitor().ok().flatten();
    let (screen_width, screen_height, screen_x, screen_y) = if let Some(m) = monitor {
        let size = m.size();
        let pos = m.position();
        (size.width as f64, size.height as f64, pos.x as f64, pos.y as f64)
    } else {
        (1920.0, 1080.0, 0.0, 0.0)
    };
    
    let (x, y) = match position_type {
        0 => {
            // Cursor position
            if let Some((mx, my)) = get_mouse_position() {
                // Offset slightly from cursor
                let offset_x = 20.0;
                let offset_y = 20.0;
                
                // Ensure window stays within screen bounds
                let mut x = mx + offset_x;
                let mut y = my + offset_y;
                
                if x + window_size.width as f64 > screen_x + screen_width {
                    x = mx - window_size.width as f64 - offset_x;
                }
                if y + window_size.height as f64 > screen_y + screen_height {
                    y = my - window_size.height as f64 - offset_y;
                }
                
                (x as i32, y as i32)
            } else {
                // Fallback to center if can't get mouse position
                let x = screen_x + (screen_width - window_size.width as f64) / 2.0;
                let y = screen_y + (screen_height - window_size.height as f64) / 2.0;
                (x as i32, y as i32)
            }
        }
        1 => {
            // Center
            let x = screen_x + (screen_width - window_size.width as f64) / 2.0;
            let y = screen_y + (screen_height - window_size.height as f64) / 2.0;
            (x as i32, y as i32)
        }
        2 => {
            // Top-right (with padding)
            let padding = 20.0;
            let x = screen_x + screen_width - window_size.width as f64 - padding;
            let y = screen_y + padding + 30.0; // Account for menu bar
            (x as i32, y as i32)
        }
        _ => {
            // Default to center
            let x = screen_x + (screen_width - window_size.width as f64) / 2.0;
            let y = screen_y + (screen_height - window_size.height as f64) / 2.0;
            (x as i32, y as i32)
        }
    };
    
    // Set position and show
    let _ = window.set_position(PhysicalPosition::new(x, y));
    let _ = window.show();
    let _ = window.set_focus();
    
    log::info!("Window positioned at ({}, {}) with position_type={}", x, y, position_type);
}

#[derive(Clone, serde::Serialize)]
pub struct DoubleCopyPayload {
    pub text: String,
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

    // Key code for 'C' on macOS
    const KEY_C: i64 = 8;

    // CGEventFlags
    const K_CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x00100000;

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

            // Check if it's Cmd+C
            let is_cmd_pressed = (flags & K_CG_EVENT_FLAG_MASK_COMMAND) != 0;
            let is_c_key = keycode == KEY_C;

            if is_cmd_pressed && is_c_key {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                let last_time = LAST_CMD_C_TIME.swap(now, Ordering::SeqCst);
                let interval = DOUBLE_PRESS_INTERVAL_MS.load(Ordering::SeqCst);

                // Check if this is a double-press
                if last_time > 0 && (now - last_time) < interval {
                    log::info!("Double Cmd+C detected! Interval: {}ms", now - last_time);

                    // Reset the timer to prevent triple-press triggers
                    LAST_CMD_C_TIME.store(0, Ordering::SeqCst);

                    // Small delay to let the clipboard update from the copy action
                    thread::spawn(|| {
                        thread::sleep(std::time::Duration::from_millis(100));
                        trigger_translation();
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

    fn trigger_translation() {
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
                    log::info!("Clipboard is empty, skipping translation");
                }
                Err(e) => {
                    log::error!("Failed to get clipboard text: {}", e);
                }
            }
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
