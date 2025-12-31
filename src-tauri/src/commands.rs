use crate::database::{ClipboardItemRow, GlossaryEntryRow, TranslationRow, UserSettingsRow};
use crate::keychain;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

// ============================================
// Response Types
// ============================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateResponse {
    pub success: bool,
    pub translated_text: Option<String>,
    pub detected_language: Option<String>,
    pub from_cache: bool,
    pub glossary_applied: Vec<String>,
    pub token_usage: Option<TokenUsage>,
    pub error: Option<TranslateError>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

#[derive(Debug, Serialize)]
pub struct TranslateError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardHistoryResponse {
    pub items: Vec<ClipboardItemResponse>,
    pub total: i64,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItemResponse {
    pub id: String,
    pub content: String,
    pub content_preview: String,
    pub copied_at: String,
    pub source_app: Option<String>,
    pub is_pinned: bool,
    pub metadata: Option<ClipboardMetadata>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardMetadata {
    pub character_count: i32,
    pub word_count: i32,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinResponse {
    pub success: bool,
    pub is_pinned: bool,
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryListResponse {
    pub entries: Vec<GlossaryEntryResponse>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryEntryResponse {
    pub id: String,
    pub keyword: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub usage_count: i32,
}

#[derive(Debug, Serialize)]
pub struct ImportGlossaryResponse {
    pub imported: i32,
    pub skipped: i32,
    pub errors: Vec<ImportError>,
}

#[derive(Debug, Serialize)]
pub struct ImportError {
    pub line: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportGlossaryResponse {
    pub success: bool,
    pub exported_count: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSettingsResponse {
    pub max_history_count: i32,
    pub preferred_model: String,
    pub auto_detect_language: bool,
    pub double_press_interval: i32,
    pub translation_cache_days: i32,
    pub show_source_app: bool,
    pub popup_position: String,
    pub launch_at_login: bool,
    pub paste_delay_ms: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyStatus {
    pub exists: bool,
    pub is_valid: Option<bool>,
    pub last_validated: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetApiKeyResponse {
    pub success: bool,
    pub is_valid: bool,
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Serialize)]
pub struct PermissionStatus {
    pub granted: bool,
}

#[derive(Debug, Serialize)]
pub struct PasteResponse {
    pub success: bool,
    pub error: Option<ErrorDetail>,
}

// ============================================
// Request Types
// ============================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsRequest {
    pub max_history_count: Option<i32>,
    pub preferred_model: Option<String>,
    pub auto_detect_language: Option<bool>,
    pub double_press_interval: Option<i32>,
    pub translation_cache_days: Option<i32>,
    pub show_source_app: Option<bool>,
    pub popup_position: Option<String>,
    pub launch_at_login: Option<bool>,
    pub paste_delay_ms: Option<i32>,
}

// ============================================
// Helper functions
// ============================================

impl From<ClipboardItemRow> for ClipboardItemResponse {
    fn from(row: ClipboardItemRow) -> Self {
        Self {
            id: row.id,
            content: row.content,
            content_preview: row.content_preview,
            copied_at: row.copied_at,
            source_app: row.source_app,
            is_pinned: row.is_pinned != 0,
            metadata: match (row.character_count, row.word_count) {
                (Some(cc), Some(wc)) => Some(ClipboardMetadata {
                    character_count: cc,
                    word_count: wc,
                }),
                _ => None,
            },
        }
    }
}

impl From<GlossaryEntryRow> for GlossaryEntryResponse {
    fn from(row: GlossaryEntryRow) -> Self {
        Self {
            id: row.id,
            keyword: row.keyword,
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
            usage_count: row.usage_count,
        }
    }
}

impl From<UserSettingsRow> for UserSettingsResponse {
    fn from(row: UserSettingsRow) -> Self {
        Self {
            max_history_count: row.max_history_count,
            preferred_model: row.preferred_model,
            auto_detect_language: row.auto_detect_language != 0,
            double_press_interval: row.double_press_interval,
            translation_cache_days: row.translation_cache_days,
            show_source_app: row.show_source_app != 0,
            popup_position: row.popup_position,
            launch_at_login: row.launch_at_login != 0,
            paste_delay_ms: row.paste_delay_ms,
        }
    }
}

// ============================================
// Translation Commands
// ============================================

#[tauri::command]
pub async fn translate(
    state: State<'_, AppState>,
    text: String,
    source_language: Option<String>,
    target_language: Option<String>,
) -> Result<TranslateResponse, String> {
    // Validation
    if text.is_empty() {
        return Ok(TranslateResponse {
            success: false,
            translated_text: None,
            detected_language: None,
            from_cache: false,
            glossary_applied: vec![],
            token_usage: None,
            error: Some(TranslateError {
                code: "EMPTY_TEXT".to_string(),
                message: "Text cannot be empty".to_string(),
            }),
        });
    }

    if text.len() > 10000 {
        return Ok(TranslateResponse {
            success: false,
            translated_text: None,
            detected_language: None,
            from_cache: false,
            glossary_applied: vec![],
            token_usage: None,
            error: Some(TranslateError {
                code: "TEXT_TOO_LONG".to_string(),
                message: "Text exceeds maximum length of 10000 characters".to_string(),
            }),
        });
    }

    let db = state.db.lock().await;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;

    // Get API key from database
    let api_key = match &settings.api_key {
        Some(key) if !key.is_empty() => key.clone(),
        _ => {
            return Ok(TranslateResponse {
                success: false,
                translated_text: None,
                detected_language: None,
                from_cache: false,
                glossary_applied: vec![],
                token_usage: None,
                error: Some(TranslateError {
                    code: "INVALID_API_KEY".to_string(),
                    message: "API key not configured. Please set your Claude API key in Settings.".to_string(),
                }),
            });
        }
    };

    // Detect or use provided language
    let src_lang = source_language.unwrap_or_else(|| detect_language(&text));
    let tgt_lang = target_language.unwrap_or_else(|| {
        if src_lang == "ko" {
            "en".to_string()
        } else {
            "ko".to_string()
        }
    });

    // Check cache first
    if let Ok(Some(cached)) = db
        .find_cached_translation(&text, &src_lang, &tgt_lang, settings.translation_cache_days)
        .await
    {
        return Ok(TranslateResponse {
            success: true,
            translated_text: Some(cached.translated_text),
            detected_language: Some(src_lang),
            from_cache: true,
            glossary_applied: cached
                .glossary_used
                .map(|g| serde_json::from_str(&g).unwrap_or_default())
                .unwrap_or_default(),
            token_usage: match (cached.input_tokens, cached.output_tokens) {
                (Some(i), Some(o)) => Some(TokenUsage {
                    input_tokens: i,
                    output_tokens: o,
                }),
                _ => None,
            },
            error: None,
        });
    }

    // Find matching glossary entries (language-agnostic)
    let glossary_matches = db
        .find_glossary_matches(&text)
        .await
        .unwrap_or_default();

    // Build glossary context for prompt
    let glossary_context = if glossary_matches.is_empty() {
        String::new()
    } else {
        let terms: Vec<String> = glossary_matches
            .iter()
            .map(|g| format!("- {}: {}", g.keyword, g.description))
            .collect();
        format!(
            "\n\nIMPORTANT: When you encounter the following terms, use the provided descriptions as context for translation:\n{}",
            terms.join("\n")
        )
    };

    // Call Claude API
    let client = reqwest::Client::new();
    let prompt = format!(
        "Translate the following text from {} to {}. Return only the translated text without any explanation.{}\n\nText to translate:\n{}",
        if src_lang == "ko" { "Korean" } else { "English" },
        if tgt_lang == "ko" { "Korean" } else { "English" },
        glossary_context,
        text
    );

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": settings.preferred_model,
            "max_tokens": 4096,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().is_success() {
                let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

                let translated_text = body["content"][0]["text"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let input_tokens = body["usage"]["input_tokens"].as_i64().map(|v| v as i32);
                let output_tokens = body["usage"]["output_tokens"].as_i64().map(|v| v as i32);

                let glossary_ids: Vec<String> =
                    glossary_matches.iter().map(|g| g.id.clone()).collect();

                // Cache the translation
                let translation = TranslationRow {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_text: text,
                    translated_text: translated_text.clone(),
                    source_language: src_lang.clone(),
                    target_language: tgt_lang,
                    model: settings.preferred_model,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    glossary_used: Some(serde_json::to_string(&glossary_ids).unwrap()),
                    input_tokens,
                    output_tokens,
                };

                let _ = db.insert_translation(&translation).await;

                // Update glossary usage counts
                if !glossary_ids.is_empty() {
                    let _ = db.increment_glossary_usage(&glossary_ids).await;
                }

                Ok(TranslateResponse {
                    success: true,
                    translated_text: Some(translated_text),
                    detected_language: Some(src_lang),
                    from_cache: false,
                    glossary_applied: glossary_ids,
                    token_usage: match (input_tokens, output_tokens) {
                        (Some(i), Some(o)) => Some(TokenUsage {
                            input_tokens: i,
                            output_tokens: o,
                        }),
                        _ => None,
                    },
                    error: None,
                })
            } else if res.status().as_u16() == 401 {
                Ok(TranslateResponse {
                    success: false,
                    translated_text: None,
                    detected_language: None,
                    from_cache: false,
                    glossary_applied: vec![],
                    token_usage: None,
                    error: Some(TranslateError {
                        code: "INVALID_API_KEY".to_string(),
                        message: "Invalid API key".to_string(),
                    }),
                })
            } else {
                Ok(TranslateResponse {
                    success: false,
                    translated_text: None,
                    detected_language: None,
                    from_cache: false,
                    glossary_applied: vec![],
                    token_usage: None,
                    error: Some(TranslateError {
                        code: "API_ERROR".to_string(),
                        message: format!("API error: {}", res.status()),
                    }),
                })
            }
        }
        Err(e) => Ok(TranslateResponse {
            success: false,
            translated_text: None,
            detected_language: None,
            from_cache: false,
            glossary_applied: vec![],
            token_usage: None,
            error: Some(TranslateError {
                code: "NETWORK_ERROR".to_string(),
                message: format!("Network error: {}", e),
            }),
        }),
    }
}

fn detect_language(text: &str) -> String {
    // Simple heuristic: if text contains Korean characters, it's Korean
    let has_korean = text.chars().any(|c| {
        let code = c as u32;
        // Korean Unicode ranges
        (0xAC00..=0xD7AF).contains(&code) || // Hangul Syllables
        (0x1100..=0x11FF).contains(&code) || // Hangul Jamo
        (0x3130..=0x318F).contains(&code)    // Hangul Compatibility Jamo
    });

    if has_korean {
        "ko".to_string()
    } else {
        "en".to_string()
    }
}

// ============================================
// Clipboard Commands
// ============================================

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearHistoryResponse {
    pub success: bool,
    pub deleted_count: i64,
    pub error: Option<ErrorDetail>,
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

#[tauri::command]
pub async fn set_clipboard(text: String) -> Result<(), String> {
    // Use macOS pasteboard
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let mut child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }

        child.wait().map_err(|e| e.to_string())?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("Clipboard operations only supported on macOS".to_string())
    }
}

#[tauri::command]
pub async fn paste_text(
    state: State<'_, AppState>,
    text: String,
) -> Result<PasteResponse, String> {
    log::info!("paste_text: Setting clipboard with text: '{}'", &text[..text.len().min(50)]);
    
    // Get paste delay from settings
    let paste_delay_ms = {
        let db = state.db.lock().await;
        db.get_settings().await.map(|s| s.paste_delay_ms).unwrap_or(150)
    };
    let delay_seconds = paste_delay_ms as f64 / 1000.0;
    log::info!("paste_text: Using paste delay: {}ms ({:.3}s)", paste_delay_ms, delay_seconds);
    
    // First set the clipboard
    set_clipboard(text.clone()).await?;
    log::info!("paste_text: Clipboard set successfully");

    // Verify clipboard content
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("pbpaste").output() {
            let clipboard_content = String::from_utf8_lossy(&output.stdout);
            log::info!("paste_text: Clipboard verification: '{}'", &clipboard_content[..clipboard_content.len().min(50)]);
            if clipboard_content != text {
                log::warn!("paste_text: Clipboard content mismatch!");
            }
        }
    }

    // Longer delay to ensure clipboard is ready
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Then simulate Cmd+Tab to switch back to previous app, then Cmd+V
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        log::info!("paste_text: Executing AppleScript for Cmd+Tab then Cmd+V");
        
        // Use AppleScript to switch to previous app and paste
        // The delay is configurable via settings
        let script = format!(r#"
            tell application "System Events"
                -- Switch to previous app using Cmd+Tab
                keystroke tab using command down
                delay {:.3}
                -- Now paste
                keystroke "v" using command down
            end tell
        "#, delay_seconds);
        
        let result = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output();

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
                    log::error!("paste_text: AppleScript failed with status {}", output.status);
                    Ok(PasteResponse {
                        success: false,
                        error: Some(ErrorDetail {
                            code: "ACCESSIBILITY_DENIED".to_string(),
                            message: format!("Accessibility permission required for paste. stderr: {}", stderr),
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

// ============================================
// Glossary Commands
// ============================================

#[tauri::command]
pub async fn get_glossary_entries(
    state: State<'_, AppState>,
    search_query: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
) -> Result<GlossaryListResponse, String> {
    let db = state.db.lock().await;
    let sort_by = sort_by.unwrap_or_else(|| "keyword".to_string());
    let sort_order = sort_order.unwrap_or_else(|| "asc".to_string());

    let entries = db
        .get_glossary_entries(
            search_query.as_deref(),
            &sort_by,
            &sort_order,
        )
        .await
        .map_err(|e| e.to_string())?;

    let total = entries.len() as i64;

    Ok(GlossaryListResponse {
        entries: entries
            .into_iter()
            .map(GlossaryEntryResponse::from)
            .collect(),
        total,
    })
}

#[tauri::command]
pub async fn add_glossary_entry(
    state: State<'_, AppState>,
    keyword: String,
    description: String,
) -> Result<GlossaryEntryResponse, String> {
    // Validation
    if keyword.is_empty() || keyword.len() > 100 {
        return Err("Keyword must be 1-100 characters".to_string());
    }

    if description.is_empty() || description.len() > 500 {
        return Err("Description must be 1-500 characters".to_string());
    }

    let now = chrono::Utc::now().to_rfc3339();
    let entry = GlossaryEntryRow {
        id: uuid::Uuid::new_v4().to_string(),
        keyword,
        description,
        created_at: now.clone(),
        updated_at: now,
        usage_count: 0,
    };

    let db = state.db.lock().await;
    db.insert_glossary_entry(&entry)
        .await
        .map_err(|e| e.to_string())?;

    Ok(GlossaryEntryResponse::from(entry))
}

#[tauri::command]
pub async fn update_glossary_entry(
    state: State<'_, AppState>,
    id: String,
    keyword: Option<String>,
    description: Option<String>,
) -> Result<GlossaryEntryResponse, String> {
    // Validation
    if let Some(ref kw) = keyword {
        if kw.is_empty() || kw.len() > 100 {
            return Err("Keyword must be 1-100 characters".to_string());
        }
    }
    if let Some(ref desc) = description {
        if desc.is_empty() || desc.len() > 500 {
            return Err("Description must be 1-500 characters".to_string());
        }
    }

    let db = state.db.lock().await;
    let entry = db
        .update_glossary_entry(&id, keyword.as_deref(), description.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    entry
        .map(GlossaryEntryResponse::from)
        .ok_or_else(|| "Glossary entry not found".to_string())
}

#[tauri::command]
pub async fn delete_glossary_entry(
    state: State<'_, AppState>,
    id: String,
) -> Result<DeleteResponse, String> {
    let db = state.db.lock().await;
    let deleted = db
        .delete_glossary_entry(&id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(DeleteResponse {
        success: deleted,
        error: if deleted {
            None
        } else {
            Some(ErrorDetail {
                code: "NOT_FOUND".to_string(),
                message: "Glossary entry not found".to_string(),
            })
        },
    })
}

#[tauri::command]
pub async fn import_glossary(
    _state: State<'_, AppState>,
    _file_path: String,
    _format: String,
    _overwrite: bool,
) -> Result<ImportGlossaryResponse, String> {
    // TODO: Implement CSV/JSON import
    Ok(ImportGlossaryResponse {
        imported: 0,
        skipped: 0,
        errors: vec![ImportError {
            line: 0,
            message: "Import not yet implemented".to_string(),
        }],
    })
}

#[tauri::command]
pub async fn export_glossary(
    _state: State<'_, AppState>,
    _file_path: String,
    _format: String,
) -> Result<ExportGlossaryResponse, String> {
    // TODO: Implement CSV/JSON export
    Ok(ExportGlossaryResponse {
        success: false,
        exported_count: 0,
    })
}

// ============================================
// Settings Commands
// ============================================

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<UserSettingsResponse, String> {
    let db = state.db.lock().await;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;
    Ok(UserSettingsResponse::from(settings))
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    settings: UpdateSettingsRequest,
) -> Result<UserSettingsResponse, String> {
    let db = state.db.lock().await;
    let mut current = db.get_settings().await.map_err(|e| e.to_string())?;

    // Apply updates
    if let Some(v) = settings.max_history_count {
        current.max_history_count = v.clamp(10, 200);
    }
    if let Some(v) = settings.preferred_model {
        current.preferred_model = v;
    }
    if let Some(v) = settings.auto_detect_language {
        current.auto_detect_language = if v { 1 } else { 0 };
    }
    if let Some(v) = settings.double_press_interval {
        current.double_press_interval = v.clamp(200, 1000);
    }
    if let Some(v) = settings.translation_cache_days {
        current.translation_cache_days = v.clamp(1, 30);
    }
    if let Some(v) = settings.show_source_app {
        current.show_source_app = if v { 1 } else { 0 };
    }
    if let Some(v) = settings.popup_position {
        current.popup_position = v.clone();
        // Update the static popup position
        crate::hotkey::set_popup_position(&v);
    }
    if let Some(v) = settings.launch_at_login {
        current.launch_at_login = if v { 1 } else { 0 };
    }
    if let Some(v) = settings.paste_delay_ms {
        current.paste_delay_ms = v.clamp(50, 500);
    }

    db.update_settings(&current)
        .await
        .map_err(|e| e.to_string())?;

    Ok(UserSettingsResponse::from(current))
}

#[tauri::command]
pub async fn get_api_key(state: State<'_, AppState>) -> Result<ApiKeyStatus, String> {
    let db = state.db.lock().await;
    let api_key = db.get_api_key().await.map_err(|e| e.to_string())?;
    let exists = api_key.is_some();
    log::info!("get_api_key called, exists: {}", exists);
    Ok(ApiKeyStatus {
        exists,
        is_valid: None,
        last_validated: None,
    })
}

#[tauri::command]
pub async fn set_api_key(
    state: State<'_, AppState>,
    api_key: String,
) -> Result<SetApiKeyResponse, String> {
    // Basic format validation
    if !crate::database::Database::validate_api_key_format(&api_key) {
        return Ok(SetApiKeyResponse {
            success: false,
            is_valid: false,
            error: Some(ErrorDetail {
                code: "INVALID_KEY".to_string(),
                message: "Invalid API key format. Key should start with 'sk-ant-'".to_string(),
            }),
        });
    }

    // Validate with API (but save anyway if validation fails due to network issues)
    let is_valid = match keychain::validate_api_key(&api_key).await {
        Ok(valid) => valid,
        Err(e) => {
            log::warn!("API key validation error (saving anyway): {}", e);
            true // Assume valid if there's a network error
        }
    };

    if is_valid {
        let db = state.db.lock().await;
        match db.set_api_key(&api_key).await {
            Ok(_) => {
                log::info!("API key saved successfully to database");
                Ok(SetApiKeyResponse {
                    success: true,
                    is_valid: true,
                    error: None,
                })
            }
            Err(e) => {
                log::error!("Failed to save API key: {}", e);
                Err(e.to_string())
            }
        }
    } else {
        // Key is invalid (401 from API)
        Ok(SetApiKeyResponse {
            success: false,
            is_valid: false,
            error: Some(ErrorDetail {
                code: "INVALID_KEY".to_string(),
                message: "API key validation failed. Please check your key.".to_string(),
            }),
        })
    }
}

#[tauri::command]
pub async fn delete_api_key(state: State<'_, AppState>) -> Result<DeleteResponse, String> {
    let db = state.db.lock().await;
    match db.delete_api_key().await {
        Ok(_) => Ok(DeleteResponse {
            success: true,
            error: None,
        }),
        Err(e) => Ok(DeleteResponse {
            success: false,
            error: Some(ErrorDetail {
                code: "DATABASE_ERROR".to_string(),
                message: e.to_string(),
            }),
        }),
    }
}

// ============================================
// System Commands
// ============================================

#[tauri::command]
pub async fn check_accessibility_permission() -> Result<PermissionStatus, String> {
    let granted = crate::hotkey::check_accessibility_permission();
    Ok(PermissionStatus { granted })
}

#[tauri::command]
pub async fn request_accessibility_permission() -> Result<(), String> {
    crate::hotkey::request_accessibility_permission();
    Ok(())
}

#[tauri::command]
pub async fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map_err(|e| format!("Failed to open settings: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn show_translation_popup(
    app: tauri::AppHandle,
    _text: String,
    _position: Option<Position>,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        crate::hotkey::show_window_at_position(&window);
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_translation_popup(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}
