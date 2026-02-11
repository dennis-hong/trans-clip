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

/// Get the saved bundle identifier of the previous frontmost app.
#[cfg(target_os = "macos")]
fn get_previous_app_bundle_id() -> Option<String> {
    match PREVIOUS_APP_BUNDLE_ID.lock() {
        Ok(guard) => guard.clone(),
        Err(_) => None,
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

    // Activate previous app and paste in a SINGLE AppleScript execution.
    // Combining activate + delay + Cmd+V in one script ensures proper sequencing
    // (separating them into Rust activate + osascript Cmd+V caused timing issues).
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let bundle_id = get_previous_app_bundle_id();

        let script = if let Some(ref bid) = bundle_id {
            log::info!("paste_text: Using bundle ID '{}' for activation", bid);
            format!(
                r#"
                tell application id "{}" to activate
                delay {:.3}
                tell application "System Events"
                    key code 9 using command down
                end tell
            "#,
                bid, delay_seconds
            )
        } else {
            log::warn!("paste_text: No saved bundle ID, falling back to Cmd+Tab");
            format!(
                r#"
                tell application "System Events"
                    key code 48 using command down
                    delay {:.3}
                    key code 9 using command down
                end tell
            "#,
                delay_seconds
            )
        };

        let result = Command::new("osascript").arg("-e").arg(&script).output();

        match result {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    log::info!("paste_text: Success!");
                    Ok(PasteResponse {
                        success: true,
                        error: None,
                    })
                } else {
                    log::error!("paste_text: AppleScript failed: {}", stderr);
                    Ok(PasteResponse {
                        success: false,
                        error: Some(ErrorDetail {
                            code: "PASTE_FAILED".to_string(),
                            message: format!("AppleScript failed: {}", stderr),
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
