use crate::AppState;
use tauri::State;

use super::types::{
    ClearHistoryResponse, ClipboardHistoryResponse, ClipboardItemResponse, DeleteResponse,
    ErrorDetail, PasteResponse, PinResponse,
};

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

    // Generate content preview (first 100 chars)
    let content_preview = if content.len() > 100 {
        format!("{}...", &content[..100])
    } else {
        content.clone()
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

    // Generate content preview (first 100 chars)
    let content_preview = if content.len() > 100 {
        format!("{}...", &content[..100])
    } else {
        content.clone()
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
    // Use macOS pasteboard
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::process::Command;

        let mut child = Command::new("pbcopy")
            .env("LANG", "en_US.UTF-8")
            .env("LC_ALL", "en_US.UTF-8")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }

        child.wait().map_err(|e| e.to_string())?;
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
    log::info!(
        "paste_text: Setting clipboard with text: '{}'",
        &text[..text.len().min(50)]
    );

    // Get paste delay from settings
    let paste_delay_ms = {
        let db = state.db.lock().await;
        db.get_settings()
            .await
            .map(|s| s.paste_delay_ms)
            .unwrap_or(150)
    };
    let delay_seconds = paste_delay_ms as f64 / 1000.0;
    log::info!(
        "paste_text: Using paste delay: {}ms ({:.3}s)",
        paste_delay_ms,
        delay_seconds
    );

    // First set the clipboard
    set_clipboard(text.clone()).await?;
    log::info!("paste_text: Clipboard set successfully");

    // Verify clipboard content
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("pbpaste").output() {
            let clipboard_content = String::from_utf8_lossy(&output.stdout);
            log::info!(
                "paste_text: Clipboard verification: '{}'",
                &clipboard_content[..clipboard_content.len().min(50)]
            );
            if clipboard_content != text {
                log::warn!("paste_text: Clipboard content mismatch!");
            }
        }
    }

    // Longer delay to ensure clipboard is ready
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Then switch back to previous app and paste
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        log::info!("paste_text: Executing AppleScript to activate previous app and paste");

        let script = format!(
            r#"
            tell application "System Events"
                -- Switch to previous app using Cmd+Tab
                key code 48 using command down

                -- Wait for app switch to complete
                delay {:.3}

                -- Send Cmd+V using key code 9 (v key) - more reliable than keystroke
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
                    log::info!("paste_text: Success!");
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
