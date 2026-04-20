use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter, Manager};

static LAST_CMD_C_TIME: AtomicU64 = AtomicU64::new(0);
static LAST_CMD_E_TIME: AtomicU64 = AtomicU64::new(0);
static LAST_OPTION_TIME: AtomicU64 = AtomicU64::new(0);
static HOTKEY_ENABLED: AtomicBool = AtomicBool::new(false);
static DOUBLE_PRESS_INTERVAL_MS: AtomicU64 = AtomicU64::new(500);
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
#[cfg(target_os = "macos")]
static EVENT_TAP_PTR: AtomicUsize = AtomicUsize::new(0);
#[cfg(target_os = "macos")]
static EVENT_RUN_LOOP_PTR: AtomicUsize = AtomicUsize::new(0);

const DEFAULT_DOUBLE_PRESS_INTERVAL_MS: u64 = 500;
const KEY_DOWN_EVENT_TYPE: u32 = 10;
const FLAGS_CHANGED_EVENT_TYPE: u32 = 12;
const KEY_C: i64 = 8;
const KEY_E: i64 = 14;
const KEY_LEFT_OPTION: i64 = 58;
const KEY_RIGHT_OPTION: i64 = 61;
const K_CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x00100000;
const K_CG_EVENT_FLAG_MASK_CONTROL: u64 = 0x00040000;
const K_CG_EVENT_FLAG_MASK_SHIFT: u64 = 0x00020000;
const K_CG_EVENT_FLAG_MASK_OPTION: u64 = 0x00080000;

fn is_option_modifier_keycode(keycode: i64) -> bool {
    matches!(keycode, KEY_LEFT_OPTION | KEY_RIGHT_OPTION)
}

fn is_option_press_event(event_type: u32, keycode: i64, flags: u64) -> bool {
    let has_disallowed_modifiers = flags
        & (K_CG_EVENT_FLAG_MASK_COMMAND
            | K_CG_EVENT_FLAG_MASK_CONTROL
            | K_CG_EVENT_FLAG_MASK_SHIFT)
        != 0;

    event_type == FLAGS_CHANGED_EVENT_TYPE
        && is_option_modifier_keycode(keycode)
        && (flags & K_CG_EVENT_FLAG_MASK_OPTION) != 0
        && !has_disallowed_modifiers
}

fn is_within_double_press_interval(last_time: u64, now: u64, interval: u64) -> bool {
    last_time > 0 && now.saturating_sub(last_time) < interval
}

fn resolve_restored_window_size(
    monitor_logical_width: i32,
    current_logical_size: Option<(i32, i32)>,
    saved_width: Option<i32>,
) -> (i32, i32) {
    let width = saved_width
        .unwrap_or_else(|| crate::utils::monitor::calculate_adaptive_width(monitor_logical_width))
        .clamp(800, 1600);
    let height = current_logical_size
        .map(|(_, height)| height)
        .unwrap_or(280)
        .clamp(48, 760);

    (width, height)
}

fn get_current_logical_window_size(window: &tauri::WebviewWindow) -> Option<(i32, i32)> {
    let size = window.outer_size().ok()?;
    let scale = window.scale_factor().ok()?;
    if scale <= 0.0 {
        return None;
    }

    Some((
        (size.width as f64 / scale).round() as i32,
        (size.height as f64 / scale).round() as i32,
    ))
}

pub fn set_double_press_interval(interval_ms: u64) {
    let clamped = interval_ms.clamp(200, 1000);
    DOUBLE_PRESS_INTERVAL_MS.store(clamped, Ordering::SeqCst);
    log::info!("Updated hotkey double press interval to {}ms", clamped);
}

/// Show the window and set focus
/// Before showing, restore onto the last-used monitor when available.
/// If that monitor is no longer present after hotplug changes, fall back to
/// the cursor monitor and then the primary monitor.
pub fn show_window_at_position(window: &tauri::WebviewWindow) {
    use crate::utils::monitor::{generate_monitor_key, sort_monitors_by_position};

    // Calculate and set position before showing
    if let Ok(monitors) = window.app_handle().available_monitors() {
        let sorted = sort_monitors_by_position(monitors.iter());
        let resolved_monitor =
            crate::commands::window::resolve_last_used_monitor(window.app_handle(), &sorted);

        if let Some((monitor_index, monitor)) = resolved_monitor {
            let mon_pos = monitor.position();
            let mon_size = monitor.size();
            let scale = monitor.scale_factor();

            let mon_logical_width = (mon_size.width as f64 / scale) as i32;
            let mon_logical_height = (mon_size.height as f64 / scale) as i32;
            let monitor_key = generate_monitor_key(mon_size.width, mon_size.height, scale);
            let current_logical_size = get_current_logical_window_size(window);
            let saved_width =
                window
                    .app_handle()
                    .try_state::<crate::AppState>()
                    .and_then(|state| {
                        tauri::async_runtime::block_on(async {
                            match state.db.get_monitor_window_width(&monitor_key).await {
                                Ok(width) => width,
                                Err(err) => {
                                    log::warn!(
                                        "Failed to load saved width for monitor {}: {}",
                                        monitor_key,
                                        err
                                    );
                                    None
                                }
                            }
                        })
                    });

            let (win_width, win_height) =
                resolve_restored_window_size(mon_logical_width, current_logical_size, saved_width);

            let x = mon_pos.x + (mon_logical_width - win_width) / 2;
            let y = mon_pos.y + mon_logical_height - win_height;

            if let Err(err) = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
                width: win_width as f64,
                height: win_height as f64,
            })) {
                log::warn!("Failed to set window size before showing: {}", err);
            }
            if let Err(err) =
                window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                    x: x as f64,
                    y: y as f64,
                }))
            {
                log::warn!("Failed to set window position before showing: {}", err);
            }

            log::info!(
                "Window positioned at ({}, {}), size {}x{} before showing (monitor_index={}, monitor_key={}, saved_width={:?}, current_size={:?})",
                x,
                y,
                win_width,
                win_height,
                monitor_index,
                monitor_key,
                saved_width,
                current_logical_size
            );
        }
    }

    if let Err(err) = window.show() {
        log::warn!("Failed to show window: {}", err);
    }
    if let Err(err) = window.set_focus() {
        log::warn!("Failed to focus window: {}", err);
    }

    log::info!("Window shown and focused");
}

#[cfg(test)]
mod tests {
    use super::{
        is_option_press_event, is_within_double_press_interval, resolve_restored_window_size,
        FLAGS_CHANGED_EVENT_TYPE, KEY_DOWN_EVENT_TYPE, KEY_LEFT_OPTION, KEY_RIGHT_OPTION,
        K_CG_EVENT_FLAG_MASK_COMMAND, K_CG_EVENT_FLAG_MASK_CONTROL, K_CG_EVENT_FLAG_MASK_OPTION,
        K_CG_EVENT_FLAG_MASK_SHIFT,
    };

    #[test]
    fn resolve_restored_window_size_prefers_saved_width_and_preserves_height() {
        let size = resolve_restored_window_size(1512, Some((1209, 428)), Some(1344));
        assert_eq!(size, (1344, 428));
    }

    #[test]
    fn resolve_restored_window_size_falls_back_to_adaptive_defaults() {
        let size = resolve_restored_window_size(1512, None, None);
        assert_eq!(size, (1209, 280));
    }

    #[test]
    fn resolve_restored_window_size_clamps_height_bounds() {
        assert_eq!(
            resolve_restored_window_size(1920, Some((1400, 12)), None),
            (1344, 48)
        );
        assert_eq!(
            resolve_restored_window_size(1920, Some((1400, 999)), None),
            (1344, 760)
        );
    }

    #[test]
    fn option_press_event_accepts_left_and_right_option_double_taps() {
        assert!(is_option_press_event(
            FLAGS_CHANGED_EVENT_TYPE,
            KEY_LEFT_OPTION,
            K_CG_EVENT_FLAG_MASK_OPTION
        ));
        assert!(is_option_press_event(
            FLAGS_CHANGED_EVENT_TYPE,
            KEY_RIGHT_OPTION,
            K_CG_EVENT_FLAG_MASK_OPTION
        ));
    }

    #[test]
    fn option_press_event_rejects_wrong_event_type_or_extra_modifiers() {
        assert!(!is_option_press_event(
            KEY_DOWN_EVENT_TYPE,
            KEY_LEFT_OPTION,
            K_CG_EVENT_FLAG_MASK_OPTION
        ));
        assert!(!is_option_press_event(
            FLAGS_CHANGED_EVENT_TYPE,
            KEY_LEFT_OPTION,
            K_CG_EVENT_FLAG_MASK_OPTION | K_CG_EVENT_FLAG_MASK_COMMAND
        ));
        assert!(!is_option_press_event(
            FLAGS_CHANGED_EVENT_TYPE,
            KEY_LEFT_OPTION,
            K_CG_EVENT_FLAG_MASK_OPTION | K_CG_EVENT_FLAG_MASK_CONTROL
        ));
        assert!(!is_option_press_event(
            FLAGS_CHANGED_EVENT_TYPE,
            KEY_LEFT_OPTION,
            K_CG_EVENT_FLAG_MASK_OPTION | K_CG_EVENT_FLAG_MASK_SHIFT
        ));
    }

    #[test]
    fn double_press_interval_requires_previous_press_within_threshold() {
        assert!(is_within_double_press_interval(1_000, 1_320, 400));
        assert!(!is_within_double_press_interval(0, 1_320, 400));
        assert!(!is_within_double_press_interval(1_000, 1_500, 400));
    }
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
            tap: u32,     // CGEventTapLocation
            place: u32,   // CGEventTapPlacement
            options: u32, // CGEventTapOptions
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
        fn CFRunLoopAddSource(run_loop: *mut c_void, source: *mut c_void, mode: *const c_void);
        fn CFRunLoopGetCurrent() -> *mut c_void;
        fn CFRunLoopRun();
        fn CFRunLoopStop(run_loop: *mut c_void);
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
        LAST_CMD_C_TIME.store(0, Ordering::SeqCst);
        LAST_CMD_E_TIME.store(0, Ordering::SeqCst);
        LAST_OPTION_TIME.store(0, Ordering::SeqCst);
        HOTKEY_ENABLED.store(true, Ordering::SeqCst);

        // Spawn a thread for the event tap
        thread::spawn(move || {
            run_event_tap();
        });

        log::info!("Hotkey event tap started (interval: {}ms)", interval_ms);
        Ok(())
    }

    pub fn stop_event_tap() {
        HOTKEY_ENABLED.store(false, Ordering::SeqCst);
        LAST_CMD_C_TIME.store(0, Ordering::SeqCst);
        LAST_CMD_E_TIME.store(0, Ordering::SeqCst);
        LAST_OPTION_TIME.store(0, Ordering::SeqCst);

        let tap_ptr = EVENT_TAP_PTR.swap(0, Ordering::SeqCst) as *mut c_void;
        if !tap_ptr.is_null() {
            unsafe {
                CGEventTapEnable(tap_ptr, false);
            }
        }

        let run_loop_ptr = EVENT_RUN_LOOP_PTR.swap(0, Ordering::SeqCst) as *mut c_void;
        if !run_loop_ptr.is_null() {
            unsafe {
                CFRunLoopStop(run_loop_ptr);
            }
        }
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

        if event_type != KEY_DOWN_EVENT_TYPE && event_type != FLAGS_CHANGED_EVENT_TYPE {
            return event;
        }

        unsafe {
            let keycode = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE);
            let flags = CGEventGetFlags(event);

            if is_option_press_event(event_type, keycode, flags) {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                let interval = DOUBLE_PRESS_INTERVAL_MS.load(Ordering::SeqCst);
                let last_time = LAST_OPTION_TIME.swap(now, Ordering::SeqCst);

                LAST_CMD_C_TIME.store(0, Ordering::SeqCst);
                LAST_CMD_E_TIME.store(0, Ordering::SeqCst);

                if is_within_double_press_interval(last_time, now, interval) {
                    log::info!("Double Option detected! Interval: {}ms", now - last_time);
                    LAST_OPTION_TIME.store(0, Ordering::SeqCst);

                    crate::commands::clipboard::save_frontmost_app();
                    thread::spawn(|| {
                        trigger_show_history();
                    });
                }

                return event;
            }

            if event_type == FLAGS_CHANGED_EVENT_TYPE {
                if !is_option_modifier_keycode(keycode) {
                    LAST_OPTION_TIME.store(0, Ordering::SeqCst);
                }
                return event;
            }

            let is_cmd_pressed = (flags & K_CG_EVENT_FLAG_MASK_COMMAND) != 0;
            let is_shift_pressed = (flags & K_CG_EVENT_FLAG_MASK_SHIFT) != 0;
            let is_c_key = keycode == KEY_C;
            let is_e_key = keycode == KEY_E;

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let interval = DOUBLE_PRESS_INTERVAL_MS.load(Ordering::SeqCst);

            LAST_OPTION_TIME.store(0, Ordering::SeqCst);

            // Cmd+C detected - check for double-press for Translation
            if is_cmd_pressed && !is_shift_pressed && is_c_key {
                let last_time = LAST_CMD_C_TIME.swap(now, Ordering::SeqCst);

                // Reset E timer to prevent cross-triggering
                LAST_CMD_E_TIME.store(0, Ordering::SeqCst);

                if is_within_double_press_interval(last_time, now, interval) {
                    log::info!("Double Cmd+C detected! Interval: {}ms", now - last_time);

                    // Reset the timer to prevent triple-press triggers
                    LAST_CMD_C_TIME.store(0, Ordering::SeqCst);

                    // Small delay to let the clipboard update from the copy action
                    // Save the frontmost app before our popup steals focus
                    crate::commands::clipboard::save_frontmost_app();
                    thread::spawn(|| {
                        thread::sleep(std::time::Duration::from_millis(100));
                        trigger_translation_or_history();
                    });
                }
            }

            // Cmd+E detected - check for double-press for Polish
            if is_cmd_pressed && !is_shift_pressed && is_e_key {
                let last_time = LAST_CMD_E_TIME.swap(now, Ordering::SeqCst);

                // Reset C timer to prevent cross-triggering
                LAST_CMD_C_TIME.store(0, Ordering::SeqCst);

                if is_within_double_press_interval(last_time, now, interval) {
                    log::info!("Double Cmd+E detected! Interval: {}ms", now - last_time);

                    // Reset the timer to prevent triple-press triggers
                    LAST_CMD_E_TIME.store(0, Ordering::SeqCst);

                    // Small delay then trigger polish
                    // Save the frontmost app before our popup steals focus
                    crate::commands::clipboard::save_frontmost_app();
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
            // Listen to KeyDown and FlagsChanged so we can detect Cmd/C/E keydowns
            // and double-taps of modifier-only keys such as Option.
            let event_mask: u64 = (1 << KEY_DOWN_EVENT_TYPE) | (1 << FLAGS_CHANGED_EVENT_TYPE);

            let tap = CGEventTapCreate(
                CGEventTapLocation::HID as u32,
                0, // kCGHeadInsertEventTap
                1, // kCGEventTapOptionListenOnly
                event_mask,
                event_callback,
                std::ptr::null_mut(),
            );

            if tap.is_null() {
                log::error!(
                    "Failed to create CGEventTap. Accessibility permission may be required."
                );
                HOTKEY_ENABLED.store(false, Ordering::SeqCst);
                return;
            }

            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            if source.is_null() {
                log::error!("Failed to create run loop source");
                CGEventTapEnable(tap, false);
                HOTKEY_ENABLED.store(false, Ordering::SeqCst);
                return;
            }

            let run_loop = CFRunLoopGetCurrent();
            EVENT_TAP_PTR.store(tap as usize, Ordering::SeqCst);
            EVENT_RUN_LOOP_PTR.store(run_loop as usize, Ordering::SeqCst);

            // kCFRunLoopCommonModes
            #[link(name = "CoreFoundation", kind = "framework")]
            extern "C" {
                static kCFRunLoopCommonModes: *const c_void;
            }

            CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);

            log::info!("CGEventTap is running");
            CFRunLoopRun();

            EVENT_TAP_PTR.store(0, Ordering::SeqCst);
            EVENT_RUN_LOOP_PTR.store(0, Ordering::SeqCst);
            HOTKEY_ENABLED.store(false, Ordering::SeqCst);
            log::info!("CGEventTap stopped");
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
                        if let Err(err) = window.hide() {
                            log::warn!("Failed to hide window during toggle: {}", err);
                        }
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
            // since Cmd+E doesn't copy anything
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
        .env("LANG", "en_US.UTF-8")
        .env("LC_ALL", "en_US.UTF-8")
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

#[cfg(target_os = "macos")]
pub fn stop_hotkey_monitor() {
    macos::stop_event_tap();
}

#[cfg(not(target_os = "macos"))]
pub fn stop_hotkey_monitor() {
    HOTKEY_ENABLED.store(false, Ordering::SeqCst);
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
