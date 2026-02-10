use crate::AppState;
use tauri::State;

use super::types::{
    ClearHistoryResponse, ClipboardHistoryResponse, ClipboardItemResponse, DeleteResponse,
    ErrorDetail, PasteResponse, PinResponse,
};

#[cfg(target_os = "macos")]
use std::sync::Mutex;

/// Stores the bundle identifier of the frontmost app before our popup was shown.
/// This allows us to activate that specific app when pasting, instead of using Cmd+Tab.
#[cfg(target_os = "macos")]
static PREVIOUS_APP_BUNDLE_ID: Mutex<Option<String>> = Mutex::new(None);

/// Save the currently frontmost application's bundle identifier.
/// Called from hotkey handlers before showing the TransClip popup.
#[cfg(target_os = "macos")]
pub fn save_frontmost_app() {
    use objc::{msg_send, sel, sel_impl, class};
    use objc::runtime::Object;
    use std::ffi::CStr;

    unsafe {
        let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
        let front_app: *mut Object = msg_send![workspace, frontmostApplication];
        if front_app.is_null() {
            log::warn!("save_frontmost_app: frontmostApplication is nil");
            return;
        }

        let bundle_id: *mut Object = msg_send![front_app, bundleIdentifier];
        if bundle_id.is_null() {
            log::warn!("save_frontmost_app: bundleIdentifier is nil");
            return;
        }

        let c_str: *const std::os::raw::c_char = msg_send![bundle_id, UTF8String];
        if c_str.is_null() {
            log::warn!("save_frontmost_app: UTF8String is nil");
            return;
        }

        let bundle_id_str = CStr::from_ptr(c_str).to_string_lossy().to_string();
        log::info!("save_frontmost_app: saved '{}'", bundle_id_str);

        if let Ok(mut prev) = PREVIOUS_APP_BUNDLE_ID.lock() {
            *prev = Some(bundle_id_str);
        }
    }
}

/// Activate a previously saved app by its bundle identifier.
/// Returns true if the app was successfully activated.
#[cfg(target_os = "macos")]
fn activate_previous_app() -> bool {
    let bundle_id = {
        match PREVIOUS_APP_BUNDLE_ID.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => None,
        }
    };

    let Some(bundle_id) = bundle_id else {
        log::warn!("activate_previous_app: no saved app bundle ID, falling back to Cmd+Tab");
        return false;
    };

    log::info!("activate_previous_app: activating '{}'", bundle_id);

    use objc::{msg_send, sel, sel_impl, class};
    use objc::runtime::Object;
    use cocoa::foundation::NSString;
    use cocoa::base::nil;

    unsafe {
        let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];

        // Create NSString from bundle ID
        let ns_bundle_id = NSString::alloc(nil).init_str(&bundle_id);

        // Get running applications
        let running_apps: *mut Object = msg_send![workspace, runningApplications];
        let count: usize = msg_send![running_apps, count];

        for i in 0..count {
            let app: *mut Object = msg_send![running_apps, objectAtIndex: i];
            let app_bundle: *mut Object = msg_send![app, bundleIdentifier];
            if app_bundle.is_null() {
                continue;
            }

            let is_equal: objc::runtime::BOOL = msg_send![app_bundle, isEqualToString: ns_bundle_id];
            if is_equal != objc::runtime::NO {
                // NSApplicationActivateAllWindows | NSApplicationActivateIgnoringOtherApps
                let _activated: objc::runtime::BOOL = msg_send![app, activateWithOptions: 3usize];
                log::info!("activate_previous_app: activated '{}'", bundle_id);
                return true;
            }
        }

        log::warn!("activate_previous_app: app '{}' not found in running apps", bundle_id);
        false
    }
}

#[tauri::command]
pub async fn get_clipboard_history(
    state: State<'_, AppState>,
    limit: Option<i32>,
    offset: Option<i32>,
    search_query: Option<String>,
) -> Result<ClipboardHistoryResponse, String> {
    let db = state.db.lock().await;
    let limit = limit.unwrap_or(50).min(200);
    let offset = offset.unwrap_or(0);

    let (items, total) = db
        .get_clipboard_history(limit, offset, search_query.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(ClipboardHistoryResponse {
        items: items.into_iter().map(ClipboardItemResponse::from).collect(),
        total,
        has_more: (offset + limit) < total as i32,
    })
}

#[tauri::command]
pub async fn delete_clipboard_item(
    state: State<'_, AppState>,
    id: String,
) -> Result<DeleteResponse, String> {
    let db = state.db.lock().await;
    let deleted = db
        .delete_clipboard_item(&id)
        .await
        .map_err(|e| e.to_string())?;

    if deleted {
        Ok(DeleteResponse {
            success: true,
            error: None,
        })
    } else {
        Ok(DeleteResponse {
            success: false,
            error: Some(ErrorDetail {
                code: "NOT_FOUND".to_string(),
                message: "Clipboard item not found".to_string(),
            }),
        })
    }
}

#[tauri::command]
pub async fn toggle_pin_clipboard_item(
    state: State<'_, AppState>,
    id: String,
) -> Result<PinResponse, String> {
    let db = state.db.lock().await;
    let result = db
        .toggle_pin_clipboard_item(&id)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(is_pinned) = result {
        Ok(PinResponse {
            success: true,
            is_pinned,
            error: None,
        })
    } else {
        Ok(PinResponse {
            success: false,
            is_pinned: false,
            error: Some(ErrorDetail {
                code: "NOT_FOUND".to_string(),
                message: "Clipboard item not found".to_string(),
            }),
        })
    }
}

#[tauri::command]
pub async fn clear_clipboard_history(
    state: State<'_, AppState>,
) -> Result<ClearHistoryResponse, String> {
    let db = state.db.lock().await;
    let deleted_count = db
        .clear_all_clipboard_items()
        .await
        .map_err(|e| e.to_string())?;

    Ok(ClearHistoryResponse {
        success: true,
        deleted_count,
        error: None,
    })
}

/// Create a new clipboard item manually (not from clipboard)
#[tauri::command]
pub async fn create_clipboard_item(
    state: State<'_, AppState>,
    content: String,
) -> Result<ClipboardItemResponse, String> {
    // Validation
    if content.is_empty() {
        return Err("Content cannot be empty".to_string());
    }

    if content.len() > 50000 {
        return Err("Content is too long (max 50000 characters)".to_string());
    }

    // Generate content preview (first 100 chars, UTF-8 safe)
    let content_preview = {
        let char_count = content.chars().count();
        if char_count > 100 {
            let preview: String = content.chars().take(100).collect();
            format!("{}...", preview)
        } else {
            content.clone()
        }
    };

    // Calculate metadata
    let character_count = content.chars().count() as i32;
    let word_count = content.split_whitespace().count() as i32;

    let id = uuid::Uuid::new_v4().to_string();

    let db = state.db.lock().await;
    let item = db
        .create_clipboard_item(&id, &content, &content_preview, character_count, word_count)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ClipboardItemResponse::from(item))
}

/// Update the content of an existing clipboard item
#[tauri::command]
pub async fn update_clipboard_item(
    state: State<'_, AppState>,
    id: String,
    content: String,
) -> Result<ClipboardItemResponse, String> {
    // Validation
    if content.is_empty() {
        return Err("Content cannot be empty".to_string());
    }

    if content.len() > 50000 {
        return Err("Content is too long (max 50000 characters)".to_string());
    }

    // Generate content preview (first 100 chars, UTF-8 safe)
    let content_preview = {
        let char_count = content.chars().count();
        if char_count > 100 {
            let preview: String = content.chars().take(100).collect();
            format!("{}...", preview)
        } else {
            content.clone()
        }
    };

    // Calculate metadata
    let character_count = content.chars().count() as i32;
    let word_count = content.split_whitespace().count() as i32;

    let db = state.db.lock().await;
    let item = db
        .update_clipboard_item_content(&id, &content, &content_preview, character_count, word_count)
        .await
        .map_err(|e| e.to_string())?;

    item.map(ClipboardItemResponse::from)
        .ok_or_else(|| "Clipboard item not found".to_string())
}

#[tauri::command]
pub async fn set_clipboard(text: String) -> Result<(), String> {
    // Use macOS NSPasteboard directly (faster and more reliable than pbcopy)
    #[cfg(target_os = "macos")]
    {
        use cocoa::base::nil;
        use cocoa::foundation::NSString;
        use objc::{msg_send, sel, sel_impl, class};
        use objc::runtime::Object;

        unsafe {
            let pasteboard: *mut Object = msg_send![class!(NSPasteboard), generalPasteboard];
            let _: i64 = msg_send![pasteboard, clearContents];

            let ns_string = NSString::alloc(nil).init_str(&text);

            // Use writeObjects: with an NSArray containing the NSString
            // NSString conforms to NSPasteboardWriting, so this sets the proper UTI types
            let objects: *mut Object = msg_send![class!(NSArray), arrayWithObject: ns_string];
            let result: objc::runtime::BOOL = msg_send![pasteboard, writeObjects: objects];

            if result == objc::runtime::NO {
                return Err("Failed to write to NSPasteboard".to_string());
            }

            log::info!("set_clipboard: Set via NSPasteboard (native)");
        }
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = text;
        Err("Clipboard operations only supported on macOS".to_string())
    }
}

#[tauri::command]
pub async fn paste_text(state: State<'_, AppState>, text: String) -> Result<PasteResponse, String> {
    let text_preview: String = text.chars().take(50).collect();
    log::info!(
        "paste_text: Setting clipboard with text: '{}'",
        text_preview
    );

    // Get paste delay from settings
    let paste_delay_ms = {
        let db = state.db.lock().await;
        db.get_settings()
            .await
            .map(|s| s.paste_delay_ms)
            .unwrap_or(200)
    };
    let delay_seconds = paste_delay_ms as f64 / 1000.0;
    log::info!(
        "paste_text: Using paste delay: {}ms ({:.3}s)",
        paste_delay_ms,
        delay_seconds
    );

    // First set the clipboard (native NSPasteboard)
    set_clipboard(text.clone()).await?;
    log::info!("paste_text: Clipboard set successfully");

    // Wait for clipboard to be fully committed
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Then activate previous app and paste
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Try to activate the previously saved app directly
        let activated = activate_previous_app();

        if activated {
            log::info!("paste_text: Activated previous app directly by bundle ID");

            // Wait for app activation to complete
            tokio::time::sleep(tokio::time::Duration::from_millis(paste_delay_ms as u64)).await;

            // Send Cmd+V
            let script = r#"
                tell application "System Events"
                    key code 9 using command down
                end tell
            "#;

            let result = Command::new("osascript").arg("-e").arg(script).output();

            match result {
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if output.status.success() {
                        log::info!("paste_text: Success (direct activate)!");
                        Ok(PasteResponse {
                            success: true,
                            error: None,
                        })
                    } else {
                        log::error!("paste_text: Cmd+V failed: {}", stderr);
                        Ok(PasteResponse {
                            success: false,
                            error: Some(ErrorDetail {
                                code: "PASTE_FAILED".to_string(),
                                message: format!("Failed to send Cmd+V: {}", stderr),
                            }),
                        })
                    }
                }
                Err(e) => {
                    log::error!("paste_text: Failed to execute osascript: {}", e);
                    Ok(PasteResponse {
                        success: false,
                        error: Some(ErrorDetail {
                            code: "PASTE_FAILED".to_string(),
                            message: format!("Failed to execute paste: {}", e),
                        }),
                    })
                }
            }
        } else {
            // Fallback: use Cmd+Tab (old behavior)
            log::warn!("paste_text: Falling back to Cmd+Tab");

            let script = format!(
                r#"
                tell application "System Events"
                    key code 48 using command down
                    delay {:.3}
                    key code 9 using command down
                end tell
            "#,
                delay_seconds
            );

            let result = Command::new("osascript").arg("-e").arg(&script).output();

            match result {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    log::info!("paste_text: AppleScript exit status: {}", output.status);
                    if !stdout.is_empty() {
                        log::info!("paste_text: stdout: {}", stdout);
                    }
                    if !stderr.is_empty() {
                        log::warn!("paste_text: stderr: {}", stderr);
                    }

                    if output.status.success() {
                        log::info!("paste_text: Success (Cmd+Tab fallback)!");
                        Ok(PasteResponse {
                            success: true,
                            error: None,
                        })
                    } else {
                        log::error!(
                            "paste_text: AppleScript failed with status {}",
                            output.status
                        );
                        Ok(PasteResponse {
                            success: false,
                            error: Some(ErrorDetail {
                                code: "ACCESSIBILITY_DENIED".to_string(),
                                message: format!(
                                    "Accessibility permission required for paste. stderr: {}",
                                    stderr
                                ),
                            }),
                        })
                    }
                }
                Err(e) => {
                    log::error!("paste_text: Failed to execute osascript: {}", e);
                    Ok(PasteResponse {
                        success: false,
                        error: Some(ErrorDetail {
                            code: "PASTE_FAILED".to_string(),
                            message: format!("Failed to execute paste: {}", e),
                        }),
                    })
                }
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(PasteResponse {
            success: false,
            error: Some(ErrorDetail {
                code: "PASTE_FAILED".to_string(),
                message: "Paste operations only supported on macOS".to_string(),
            }),
        })
    }
}
