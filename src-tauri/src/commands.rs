use crate::database::{ClipboardItemRow, GlossaryEntryRow, TranslationRow, UserSettingsRow};
use crate::keychain;
use crate::prompts;
use crate::AppState;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::ipc::Channel;
use tauri::{Manager, State};

// Store the last valid monitor index to preserve position across hide/show cycles
static LAST_MONITOR_INDEX: AtomicUsize = AtomicUsize::new(0);

/// Update LAST_MONITOR_INDEX based on current mouse cursor position
/// This is called before showing the window to ensure it appears on the correct monitor
pub fn update_monitor_from_cursor(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use core_graphics::event::CGEvent;
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        // Get current mouse position
        let source = match CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
            Ok(s) => s,
            Err(_) => {
                log::warn!("Failed to create event source for cursor position");
                return;
            }
        };
        let event = match CGEvent::new(source) {
            Ok(e) => e,
            Err(_) => {
                log::warn!("Failed to create event for cursor position");
                return;
            }
        };
        let cursor_pos = event.location();
        let cursor_x = cursor_pos.x as i32;
        let cursor_y = cursor_pos.y as i32;

        log::info!("Cursor position: ({}, {})", cursor_x, cursor_y);

        // Get monitors and sort by position
        let monitors = match app.available_monitors() {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to get monitors: {}", e);
                return;
            }
        };

        let mut sorted_monitors: Vec<_> = monitors.iter().collect();
        sorted_monitors.sort_by(|a, b| {
            let pos_a = a.position();
            let pos_b = b.position();
            match pos_a.x.cmp(&pos_b.x) {
                std::cmp::Ordering::Equal => pos_a.y.cmp(&pos_b.y),
                other => other,
            }
        });

        // Find which monitor contains the cursor
        for (idx, monitor) in sorted_monitors.iter().enumerate() {
            let mon_pos = monitor.position();
            let mon_size = monitor.size();
            let scale = monitor.scale_factor();

            // Monitor position is in physical pixels, cursor is in logical
            // Convert cursor to physical for comparison
            let cursor_physical_x = (cursor_x as f64 * scale) as i32;
            let cursor_physical_y = (cursor_y as f64 * scale) as i32;

            if cursor_physical_x >= mon_pos.x
                && cursor_physical_x < mon_pos.x + mon_size.width as i32
                && cursor_physical_y >= mon_pos.y
                && cursor_physical_y < mon_pos.y + mon_size.height as i32
            {
                log::info!("Cursor is on monitor {} (sorted index)", idx);
                LAST_MONITOR_INDEX.store(idx, Ordering::SeqCst);
                return;
            }
        }

        // Fallback: try with logical coordinates directly (for single-scale setups)
        for (idx, monitor) in sorted_monitors.iter().enumerate() {
            let mon_pos = monitor.position();
            let mon_size = monitor.size();
            let scale = monitor.scale_factor();

            let mon_logical_width = (mon_size.width as f64 / scale) as i32;
            let mon_logical_height = (mon_size.height as f64 / scale) as i32;

            if cursor_x >= mon_pos.x
                && cursor_x < mon_pos.x + mon_logical_width
                && cursor_y >= mon_pos.y
                && cursor_y < mon_pos.y + mon_logical_height
            {
                log::info!("Cursor is on monitor {} (logical fallback)", idx);
                LAST_MONITOR_INDEX.store(idx, Ordering::SeqCst);
                return;
            }
        }

        log::warn!("Could not determine monitor from cursor position");
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        log::info!("Cursor-based monitor detection not implemented for this platform");
    }
}

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

#[derive(Debug, Clone, Serialize)]
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
    pub updated_at: Option<String>,
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
// Streaming Event Types
// ============================================

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum TranslateStreamEvent {
    Started {
        detected_language: Option<String>,
        from_cache: bool,
        glossary_applied: Vec<String>,
    },
    Delta {
        text: String,
    },
    Completed {
        full_text: String,
        token_usage: Option<TokenUsage>,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum PolishStreamEvent {
    Started {
        detected_language: Option<String>,
    },
    Delta {
        text: String,
    },
    Completed {
        full_text: String,
        token_usage: Option<TokenUsage>,
    },
    Error {
        code: String,
        message: String,
    },
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
            updated_at: row.updated_at,
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
    model: Option<String>,
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

    // Check if explicit model is provided (user wants a specific model, skip cache)
    let explicit_model = model.is_some();

    // Check cache first (skip cache if explicit model is provided)
    if !explicit_model {
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

    // Use provided model or fall back to settings
    let use_model = model.unwrap_or_else(|| settings.preferred_model.clone());

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
            "model": use_model,
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
                    model: use_model,
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

#[tauri::command]
pub async fn translate_stream(
    state: State<'_, AppState>,
    text: String,
    source_language: Option<String>,
    target_language: Option<String>,
    model: Option<String>,
    on_event: Channel<TranslateStreamEvent>,
) -> Result<(), String> {
    // Validation
    if text.is_empty() {
        let _ = on_event.send(TranslateStreamEvent::Error {
            code: "EMPTY_TEXT".to_string(),
            message: "Text cannot be empty".to_string(),
        });
        return Ok(());
    }

    if text.len() > 10000 {
        let _ = on_event.send(TranslateStreamEvent::Error {
            code: "TEXT_TOO_LONG".to_string(),
            message: "Text exceeds maximum length of 10000 characters".to_string(),
        });
        return Ok(());
    }

    let db = state.db.lock().await;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;

    // Get API key from database
    let api_key = match &settings.api_key {
        Some(key) if !key.is_empty() => key.clone(),
        _ => {
            let _ = on_event.send(TranslateStreamEvent::Error {
                code: "INVALID_API_KEY".to_string(),
                message: "API key not configured. Please set your Claude API key in Settings."
                    .to_string(),
            });
            return Ok(());
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

    // Check if explicit model is provided (user wants a specific model, skip cache)
    let explicit_model = model.is_some();

    // Check cache first (skip cache if explicit model is provided)
    if !explicit_model {
        if let Ok(Some(cached)) = db
            .find_cached_translation(&text, &src_lang, &tgt_lang, settings.translation_cache_days)
            .await
        {
            let glossary_applied: Vec<String> = cached
                .glossary_used
                .map(|g| serde_json::from_str(&g).unwrap_or_default())
                .unwrap_or_default();

            let _ = on_event.send(TranslateStreamEvent::Started {
                detected_language: Some(src_lang.clone()),
                from_cache: true,
                glossary_applied: glossary_applied.clone(),
            });

            let _ = on_event.send(TranslateStreamEvent::Completed {
                full_text: cached.translated_text,
                token_usage: match (cached.input_tokens, cached.output_tokens) {
                    (Some(i), Some(o)) => Some(TokenUsage {
                        input_tokens: i,
                        output_tokens: o,
                    }),
                    _ => None,
                },
            });
            return Ok(());
        }
    }

    // Find matching glossary entries (language-agnostic)
    let glossary_matches = db.find_glossary_matches(&text).await.unwrap_or_default();

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

    let glossary_ids: Vec<String> = glossary_matches.iter().map(|g| g.id.clone()).collect();

    // Send Started event
    let _ = on_event.send(TranslateStreamEvent::Started {
        detected_language: Some(src_lang.clone()),
        from_cache: false,
        glossary_applied: glossary_ids.clone(),
    });

    // Use provided model or fall back to settings
    let use_model = model.unwrap_or_else(|| settings.preferred_model.clone());

    // Build prompt
    let prompt = format!(
        "Translate the following text from {} to {}. Return only the translated text without any explanation.{}\n\nText to translate:\n{}",
        if src_lang == "ko" { "Korean" } else { "English" },
        if tgt_lang == "ko" { "Korean" } else { "English" },
        glossary_context,
        text
    );

    // Release the db lock before making the API call
    drop(db);

    // Call Claude API with streaming
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": use_model,
            "max_tokens": 4096,
            "stream": true,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().as_u16() == 401 {
                let _ = on_event.send(TranslateStreamEvent::Error {
                    code: "INVALID_API_KEY".to_string(),
                    message: "Invalid API key".to_string(),
                });
                return Ok(());
            }

            if !res.status().is_success() {
                let _ = on_event.send(TranslateStreamEvent::Error {
                    code: "API_ERROR".to_string(),
                    message: format!("API error: {}", res.status()),
                });
                return Ok(());
            }

            // Process SSE stream
            let mut full_text = String::new();
            let mut input_tokens: Option<i32> = None;
            let mut output_tokens: Option<i32> = None;
            let mut stream = res.bytes_stream();

            let mut buffer = String::new();

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let chunk_str = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&chunk_str);

                        // Process complete lines
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].trim().to_string();
                            buffer = buffer[pos + 1..].to_string();

                            if line.starts_with("data: ") {
                                let json_str = &line[6..];
                                if json_str == "[DONE]" {
                                    continue;
                                }

                                if let Ok(event) =
                                    serde_json::from_str::<serde_json::Value>(json_str)
                                {
                                    let event_type = event["type"].as_str().unwrap_or("");

                                    match event_type {
                                        "content_block_delta" => {
                                            if let Some(delta) = event["delta"]["text"].as_str() {
                                                full_text.push_str(delta);
                                                let _ = on_event.send(TranslateStreamEvent::Delta {
                                                    text: delta.to_string(),
                                                });
                                            }
                                        }
                                        "message_start" => {
                                            if let Some(usage) = event["message"]["usage"].as_object()
                                            {
                                                input_tokens =
                                                    usage["input_tokens"].as_i64().map(|v| v as i32);
                                            }
                                        }
                                        "message_delta" => {
                                            if let Some(usage) = event["usage"].as_object() {
                                                output_tokens =
                                                    usage["output_tokens"].as_i64().map(|v| v as i32);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = on_event.send(TranslateStreamEvent::Error {
                            code: "STREAM_ERROR".to_string(),
                            message: format!("Stream error: {}", e),
                        });
                        return Ok(());
                    }
                }
            }

            // Cache the translation
            let db = state.db.lock().await;
            let translation = TranslationRow {
                id: uuid::Uuid::new_v4().to_string(),
                source_text: text,
                translated_text: full_text.clone(),
                source_language: src_lang,
                target_language: tgt_lang,
                model: use_model,
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

            // Send Completed event
            let _ = on_event.send(TranslateStreamEvent::Completed {
                full_text,
                token_usage: match (input_tokens, output_tokens) {
                    (Some(i), Some(o)) => Some(TokenUsage {
                        input_tokens: i,
                        output_tokens: o,
                    }),
                    _ => None,
                },
            });
        }
        Err(e) => {
            let _ = on_event.send(TranslateStreamEvent::Error {
                code: "NETWORK_ERROR".to_string(),
                message: format!("Network error: {}", e),
            });
        }
    }

    Ok(())
}

fn detect_language(text: &str) -> String {
    // Detect based on Korean character ratio (>= 30% = Korean)
    let (korean, total) = text.chars().fold((0, 0), |(ko, tot), c| {
        let code = c as u32;
        let is_korean = (0xAC00..=0xD7AF).contains(&code)
            || (0x1100..=0x11FF).contains(&code)
            || (0x3130..=0x318F).contains(&code);

        if is_korean {
            (ko + 1, tot + 1)
        } else if c.is_alphabetic() {
            (ko, tot + 1)
        } else {
            (ko, tot)
        }
    });

    if total > 0 && (korean as f64 / total as f64) >= 0.3 {
        "ko".to_string()
    } else {
        "en".to_string()
    }
}

// ============================================
// Polish (Text Refinement) Commands
// ============================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolishResponse {
    pub success: bool,
    pub polished_text: Option<String>,
    pub detected_language: Option<String>,
    pub token_usage: Option<TokenUsage>,
    pub error: Option<TranslateError>,
}

#[tauri::command]
pub async fn polish(
    state: State<'_, AppState>,
    text: String,
    context: String,
    channel: String,
    options: Vec<String>,
    model: Option<String>,
) -> Result<PolishResponse, String> {
    // Validation
    if text.is_empty() {
        return Ok(PolishResponse {
            success: false,
            polished_text: None,
            detected_language: None,
            token_usage: None,
            error: Some(TranslateError {
                code: "EMPTY_TEXT".to_string(),
                message: "Text cannot be empty".to_string(),
            }),
        });
    }

    if text.len() > 10000 {
        return Ok(PolishResponse {
            success: false,
            polished_text: None,
            detected_language: None,
            token_usage: None,
            error: Some(TranslateError {
                code: "TEXT_TOO_LONG".to_string(),
                message: "Text exceeds maximum length of 10000 characters".to_string(),
            }),
        });
    }

    let db = state.db.lock().await;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;

    // Get API key
    let api_key = match &settings.api_key {
        Some(key) if !key.is_empty() => key.clone(),
        _ => {
            return Ok(PolishResponse {
                success: false,
                polished_text: None,
                detected_language: None,
                token_usage: None,
                error: Some(TranslateError {
                    code: "INVALID_API_KEY".to_string(),
                    message: "API key not configured. Please set your Claude API key in Settings.".to_string(),
                }),
            });
        }
    };

    // Detect language
    let detected_lang = detect_language(&text);

    // Use provided model or fall back to settings
    let use_model = model.unwrap_or_else(|| settings.preferred_model.clone());

    // Build prompts using the prompts module
    let system_prompt = if detected_lang == "ko" {
        prompts::polish::build_system_prompt()
    } else {
        prompts::polish::build_system_prompt_english()
    };

    let user_prompt = prompts::polish::build_user_prompt(
        &text,
        &context,
        &channel,
        &options,
        &detected_lang,
    );

    // Call Claude API
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": use_model,
            "max_tokens": 4096,
            "system": system_prompt,
            "messages": [{"role": "user", "content": user_prompt}]
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().is_success() {
                let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

                let polished_text = body["content"][0]["text"]
                    .as_str()
                    .unwrap_or("")
                    .trim()
                    .to_string();

                let input_tokens = body["usage"]["input_tokens"].as_i64().map(|v| v as i32);
                let output_tokens = body["usage"]["output_tokens"].as_i64().map(|v| v as i32);

                Ok(PolishResponse {
                    success: true,
                    polished_text: Some(polished_text),
                    detected_language: Some(detected_lang),
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
                Ok(PolishResponse {
                    success: false,
                    polished_text: None,
                    detected_language: None,
                    token_usage: None,
                    error: Some(TranslateError {
                        code: "INVALID_API_KEY".to_string(),
                        message: "Invalid API key".to_string(),
                    }),
                })
            } else {
                Ok(PolishResponse {
                    success: false,
                    polished_text: None,
                    detected_language: None,
                    token_usage: None,
                    error: Some(TranslateError {
                        code: "API_ERROR".to_string(),
                        message: format!("API error: {}", res.status()),
                    }),
                })
            }
        }
        Err(e) => Ok(PolishResponse {
            success: false,
            polished_text: None,
            detected_language: None,
            token_usage: None,
            error: Some(TranslateError {
                code: "NETWORK_ERROR".to_string(),
                message: format!("Network error: {}", e),
            }),
        }),
    }
}

#[tauri::command]
pub async fn polish_stream(
    state: State<'_, AppState>,
    text: String,
    context: String,
    channel: String,
    options: Vec<String>,
    model: Option<String>,
    on_event: Channel<PolishStreamEvent>,
) -> Result<(), String> {
    // Validation
    if text.is_empty() {
        let _ = on_event.send(PolishStreamEvent::Error {
            code: "EMPTY_TEXT".to_string(),
            message: "Text cannot be empty".to_string(),
        });
        return Ok(());
    }

    if text.len() > 10000 {
        let _ = on_event.send(PolishStreamEvent::Error {
            code: "TEXT_TOO_LONG".to_string(),
            message: "Text exceeds maximum length of 10000 characters".to_string(),
        });
        return Ok(());
    }

    let db = state.db.lock().await;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;

    // Get API key
    let api_key = match &settings.api_key {
        Some(key) if !key.is_empty() => key.clone(),
        _ => {
            let _ = on_event.send(PolishStreamEvent::Error {
                code: "INVALID_API_KEY".to_string(),
                message: "API key not configured. Please set your Claude API key in Settings."
                    .to_string(),
            });
            return Ok(());
        }
    };

    // Detect language
    let detected_lang = detect_language(&text);

    // Send Started event
    let _ = on_event.send(PolishStreamEvent::Started {
        detected_language: Some(detected_lang.clone()),
    });

    // Use provided model or fall back to settings
    let use_model = model.unwrap_or_else(|| settings.preferred_model.clone());

    // Build prompts using the prompts module
    let system_prompt = if detected_lang == "ko" {
        prompts::polish::build_system_prompt()
    } else {
        prompts::polish::build_system_prompt_english()
    };

    let user_prompt = prompts::polish::build_user_prompt(
        &text,
        &context,
        &channel,
        &options,
        &detected_lang,
    );

    // Release the db lock before making the API call
    drop(db);

    // Call Claude API with streaming
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": use_model,
            "max_tokens": 4096,
            "stream": true,
            "system": system_prompt,
            "messages": [{"role": "user", "content": user_prompt}]
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().as_u16() == 401 {
                let _ = on_event.send(PolishStreamEvent::Error {
                    code: "INVALID_API_KEY".to_string(),
                    message: "Invalid API key".to_string(),
                });
                return Ok(());
            }

            if !res.status().is_success() {
                let _ = on_event.send(PolishStreamEvent::Error {
                    code: "API_ERROR".to_string(),
                    message: format!("API error: {}", res.status()),
                });
                return Ok(());
            }

            // Process SSE stream
            let mut full_text = String::new();
            let mut input_tokens: Option<i32> = None;
            let mut output_tokens: Option<i32> = None;
            let mut stream = res.bytes_stream();

            let mut buffer = String::new();

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let chunk_str = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&chunk_str);

                        // Process complete lines
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].trim().to_string();
                            buffer = buffer[pos + 1..].to_string();

                            if line.starts_with("data: ") {
                                let json_str = &line[6..];
                                if json_str == "[DONE]" {
                                    continue;
                                }

                                if let Ok(event) =
                                    serde_json::from_str::<serde_json::Value>(json_str)
                                {
                                    let event_type = event["type"].as_str().unwrap_or("");

                                    match event_type {
                                        "content_block_delta" => {
                                            if let Some(delta) = event["delta"]["text"].as_str() {
                                                full_text.push_str(delta);
                                                let _ = on_event.send(PolishStreamEvent::Delta {
                                                    text: delta.to_string(),
                                                });
                                            }
                                        }
                                        "message_start" => {
                                            if let Some(usage) = event["message"]["usage"].as_object()
                                            {
                                                input_tokens =
                                                    usage["input_tokens"].as_i64().map(|v| v as i32);
                                            }
                                        }
                                        "message_delta" => {
                                            if let Some(usage) = event["usage"].as_object() {
                                                output_tokens =
                                                    usage["output_tokens"].as_i64().map(|v| v as i32);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = on_event.send(PolishStreamEvent::Error {
                            code: "STREAM_ERROR".to_string(),
                            message: format!("Stream error: {}", e),
                        });
                        return Ok(());
                    }
                }
            }

            // Send Completed event
            let _ = on_event.send(PolishStreamEvent::Completed {
                full_text: full_text.trim().to_string(),
                token_usage: match (input_tokens, output_tokens) {
                    (Some(i), Some(o)) => Some(TokenUsage {
                        input_tokens: i,
                        output_tokens: o,
                    }),
                    _ => None,
                },
            });
        }
        Err(e) => {
            let _ = on_event.send(PolishStreamEvent::Error {
                code: "NETWORK_ERROR".to_string(),
                message: format!("Network error: {}", e),
            });
        }
    }

    Ok(())
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

    // Then switch back to previous app and paste
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        log::info!("paste_text: Executing AppleScript to activate previous app and paste");
        
        // Use AppleScript to:
        // 1. Use Cmd+Tab to switch to previous app
        // 2. Wait for the app to be activated
        // 3. Paste using key code (more reliable than keystroke)
        let script = format!(r#"
            tell application "System Events"
                -- Switch to previous app using Cmd+Tab
                key code 48 using command down
                
                -- Wait for app switch to complete
                delay {:.3}
                
                -- Send Cmd+V using key code 9 (v key) - more reliable than keystroke
                key code 9 using command down
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

// ============================================
// Window Management Commands
// ============================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub name: Option<String>,
    pub position_x: i32,
    pub position_y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub is_primary: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SnapEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapResult {
    pub snapped: bool,
    pub edges: Vec<SnapEdge>,
    pub position: WindowPosition,
}

#[tauri::command]
pub async fn get_monitors(app: tauri::AppHandle) -> Result<Vec<MonitorInfo>, String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let primary = app.primary_monitor().map_err(|e| e.to_string())?;
    
    let mut result = Vec::new();
    for monitor in monitors.iter() {
        let pos = monitor.position();
        let size = monitor.size();
        let is_primary = primary.as_ref().map_or(false, |p| {
            p.position() == monitor.position() && p.size() == monitor.size()
        });
        
        result.push(MonitorInfo {
            name: monitor.name().map(|s| s.to_string()),
            position_x: pos.x,
            position_y: pos.y,
            width: size.width,
            height: size.height,
            scale_factor: monitor.scale_factor(),
            is_primary,
        });
    }
    
    // Sort monitors by physical position (left to right)
    // This ensures button 1 = leftmost monitor, button 2 = next monitor, etc.
    result.sort_by(|a, b| {
        // Primary sort by x position (left to right)
        // Secondary sort by y position (top to bottom) for vertically stacked monitors
        match a.position_x.cmp(&b.position_x) {
            std::cmp::Ordering::Equal => a.position_y.cmp(&b.position_y),
            other => other,
        }
    });
    
    Ok(result)
}

#[tauri::command]
pub async fn get_window_position(app: tauri::AppHandle) -> Result<WindowPosition, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    
    Ok(WindowPosition {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
    })
}

#[tauri::command]
pub async fn set_window_position(
    app: tauri::AppHandle,
    x: i32,
    y: i32,
) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_window_size(
    app: tauri::AppHandle,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    window.set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Generate a unique key for a monitor based on its resolution
fn generate_monitor_key(width: u32, height: u32, scale_factor: f64) -> String {
    format!("{}x{}@{:.2}", width, height, scale_factor)
}

/// Calculate adaptive window width based on monitor width
/// Returns a width that works well for the monitor size
fn calculate_adaptive_width(monitor_logical_width: i32) -> i32 {
    // For ultra-wide monitors (>= 2560 logical), use ~60% of width, max 1600
    // For wide monitors (1920-2560), use ~70% of width, max 1400
    // For standard monitors (< 1920), use ~80% of width, min 800
    
    let base_width = if monitor_logical_width >= 2560 {
        // Ultra-wide: 60%
        ((monitor_logical_width as f64) * 0.6) as i32
    } else if monitor_logical_width >= 1920 {
        // Wide: 70%
        ((monitor_logical_width as f64) * 0.7) as i32
    } else {
        // Standard: 80%
        ((monitor_logical_width as f64) * 0.8) as i32
    };
    
    // Clamp to reasonable range
    base_width.clamp(800, 1600)
}

#[tauri::command]
pub async fn move_to_monitor(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    monitor_index: usize,
    anchor: String,
) -> Result<(), String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    
    log::info!("move_to_monitor: requested index={}, anchor={}", monitor_index, anchor);
    
    // Sort monitors by physical position (left to right) to match UI buttons
    let mut sorted_monitors: Vec<_> = monitors.iter().collect();
    sorted_monitors.sort_by(|a, b| {
        let pos_a = a.position();
        let pos_b = b.position();
        match pos_a.x.cmp(&pos_b.x) {
            std::cmp::Ordering::Equal => pos_a.y.cmp(&pos_b.y),
            other => other,
        }
    });
    
    if monitor_index >= sorted_monitors.len() {
        log::error!("Invalid monitor index: {} >= {}", monitor_index, sorted_monitors.len());
        return Err("Invalid monitor index".to_string());
    }
    
    // Save the monitor index for later use (e.g., when set_drawer_mode is called)
    LAST_MONITOR_INDEX.store(monitor_index, Ordering::SeqCst);
    log::info!("move_to_monitor: saved LAST_MONITOR_INDEX={}", monitor_index);
    
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    
    // Get target monitor info
    let target_monitor = sorted_monitors[monitor_index];
    let mon_pos = target_monitor.position();
    let mon_size = target_monitor.size();
    let target_scale = target_monitor.scale_factor();
    
    // Convert target monitor size to logical
    let mon_logical_width = (mon_size.width as f64 / target_scale) as i32;
    let mon_logical_height = (mon_size.height as f64 / target_scale) as i32;
    
    // Generate monitor key for this monitor
    let monitor_key = generate_monitor_key(mon_size.width, mon_size.height, target_scale);
    log::info!("Target monitor key: {}", monitor_key);
    
    // Get saved width for this monitor or calculate adaptive width
    let db = state.db.lock().await;
    let target_width = match db.get_monitor_window_width(&monitor_key).await {
        Ok(Some(saved_width)) => {
            log::info!("Using saved width for monitor {}: {}", monitor_key, saved_width);
            saved_width
        }
        _ => {
            let adaptive_width = calculate_adaptive_width(mon_logical_width);
            log::info!("Using adaptive width for monitor {}: {} (monitor logical width: {})", 
                monitor_key, adaptive_width, mon_logical_width);
            adaptive_width
        }
    };
    drop(db);
    
    // Get current window scale
    let current_scale = window.scale_factor().map_err(|e| e.to_string())?;
    
    // Check if we're moving between monitors with different scale factors
    let scale_differs = (current_scale - target_scale).abs() > 0.01;
    
    if scale_differs {
        // Two-phase move: first move to target monitor center to update scale factor
        let temp_x = mon_pos.x + mon_logical_width / 2;
        let temp_y = mon_pos.y + mon_logical_height / 2;
        
        log::info!("Scale differs ({} -> {}), moving to target monitor first", current_scale, target_scale);
        
        window.set_position(tauri::Position::Logical(tauri::LogicalPosition { 
            x: temp_x as f64, 
            y: temp_y as f64 
        })).map_err(|e| e.to_string())?;
        
        // Small delay to let the window update its scale factor
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    
    // Get current window height (we only adapt width, height is determined by drawer mode)
    let win_size = window.outer_size().map_err(|e| e.to_string())?;
    let win_scale = window.scale_factor().map_err(|e| e.to_string())?;
    let win_logical_height = (win_size.height as f64 / win_scale) as i32;
    
    // Set the new width
    window.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: target_width as f64,
        height: win_logical_height as f64
    })).map_err(|e| e.to_string())?;
    
    log::info!("Window size set to: {}x{}", target_width, win_logical_height);
    log::info!("Target monitor[{}]: pos=({}, {}), logical size={}x{}, scale={}", 
        monitor_index, mon_pos.x, mon_pos.y, mon_logical_width, mon_logical_height, target_scale);
    
    // Calculate final position based on anchor using logical coordinates
    let (x, y) = match anchor.as_str() {
        "bottom" => {
            let x = mon_pos.x + (mon_logical_width - target_width) / 2;
            let y = mon_pos.y + mon_logical_height - win_logical_height;
            (x, y)
        }
        "top" => {
            let x = mon_pos.x + (mon_logical_width - target_width) / 2;
            let y = mon_pos.y;
            (x, y)
        }
        "center" => {
            let x = mon_pos.x + (mon_logical_width - target_width) / 2;
            let y = mon_pos.y + (mon_logical_height - win_logical_height) / 2;
            (x, y)
        }
        _ => {
            let x = mon_pos.x + (mon_logical_width - target_width) / 2;
            let y = mon_pos.y + mon_logical_height - win_logical_height;
            (x, y)
        }
    };
    
    log::info!("Setting final window position (logical) to: ({}, {})", x, y);
    
    window.set_position(tauri::Position::Logical(tauri::LogicalPosition { 
        x: x as f64, 
        y: y as f64 
    })).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn get_current_monitor_index(app: tauri::AppHandle) -> Result<usize, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;
    
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    
    // Sort monitors by physical position (left to right) to match UI buttons
    let mut sorted_monitors: Vec<_> = monitors.iter().enumerate().collect();
    sorted_monitors.sort_by(|(_, a), (_, b)| {
        let pos_a = a.position();
        let pos_b = b.position();
        match pos_a.x.cmp(&pos_b.x) {
            std::cmp::Ordering::Equal => pos_a.y.cmp(&pos_b.y),
            other => other,
        }
    });
    
    // Find which monitor the window center is on
    let win_center_x = win_pos.x + win_size.width as i32 / 2;
    let win_center_y = win_pos.y + win_size.height as i32 / 2;
    
    for (sorted_index, (_, monitor)) in sorted_monitors.iter().enumerate() {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        
        if win_center_x >= mon_pos.x && win_center_x < mon_pos.x + mon_size.width as i32 &&
           win_center_y >= mon_pos.y && win_center_y < mon_pos.y + mon_size.height as i32 {
            // Update the saved monitor index
            LAST_MONITOR_INDEX.store(sorted_index, Ordering::SeqCst);
            return Ok(sorted_index);
        }
    }
    
    // Default to saved index or first monitor if not found
    let saved_index = LAST_MONITOR_INDEX.load(Ordering::SeqCst);
    if saved_index < sorted_monitors.len() {
        Ok(saved_index)
    } else {
        Ok(0)
    }
}

#[tauri::command]
pub async fn toggle_always_on_top(app: tauri::AppHandle) -> Result<bool, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let current = window.is_always_on_top().map_err(|e| e.to_string())?;
    window.set_always_on_top(!current).map_err(|e| e.to_string())?;
    Ok(!current)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentMonitorInfo {
    pub monitor_key: String,
    pub monitor_index: usize,
    pub monitor_width: i32,
    pub saved_window_width: Option<i32>,
}

#[tauri::command]
pub async fn get_current_monitor_info(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<CurrentMonitorInfo, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;
    
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    
    // Sort monitors by physical position (left to right) to match UI buttons
    let mut sorted_monitors: Vec<_> = monitors.iter().enumerate().collect();
    sorted_monitors.sort_by(|(_, a), (_, b)| {
        let pos_a = a.position();
        let pos_b = b.position();
        match pos_a.x.cmp(&pos_b.x) {
            std::cmp::Ordering::Equal => pos_a.y.cmp(&pos_b.y),
            other => other,
        }
    });
    
    // Find which monitor the window center is on
    let win_center_x = win_pos.x + win_size.width as i32 / 2;
    let win_center_y = win_pos.y + win_size.height as i32 / 2;
    
    let mut found_monitor = None;
    for (sorted_index, (_, monitor)) in sorted_monitors.iter().enumerate() {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        
        if win_center_x >= mon_pos.x && win_center_x < mon_pos.x + mon_size.width as i32 &&
           win_center_y >= mon_pos.y && win_center_y < mon_pos.y + mon_size.height as i32 {
            found_monitor = Some((sorted_index, *monitor));
            break;
        }
    }
    
    let (monitor_index, monitor) = found_monitor.unwrap_or_else(|| {
        (0, sorted_monitors.first().map(|(_, m)| *m).unwrap())
    });
    
    let mon_size = monitor.size();
    let scale = monitor.scale_factor();
    let mon_logical_width = (mon_size.width as f64 / scale) as i32;
    let monitor_key = generate_monitor_key(mon_size.width, mon_size.height, scale);
    
    let db = state.db.lock().await;
    let saved_width = db.get_monitor_window_width(&monitor_key).await.unwrap_or(None);
    
    Ok(CurrentMonitorInfo {
        monitor_key,
        monitor_index,
        monitor_width: mon_logical_width,
        saved_window_width: saved_width,
    })
}

#[tauri::command]
pub async fn save_window_width_for_monitor(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    width: i32,
) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;
    
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    
    // Find which monitor the window center is on
    let win_center_x = win_pos.x + win_size.width as i32 / 2;
    let win_center_y = win_pos.y + win_size.height as i32 / 2;
    
    let mut found_monitor = None;
    for monitor in monitors.iter() {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        
        if win_center_x >= mon_pos.x && win_center_x < mon_pos.x + mon_size.width as i32 &&
           win_center_y >= mon_pos.y && win_center_y < mon_pos.y + mon_size.height as i32 {
            found_monitor = Some(monitor);
            break;
        }
    }
    
    let monitor = found_monitor.unwrap_or_else(|| monitors.first().unwrap());
    let mon_size = monitor.size();
    let scale = monitor.scale_factor();
    let monitor_key = generate_monitor_key(mon_size.width, mon_size.height, scale);
    
    log::info!("Saving window width {} for monitor {}", width, monitor_key);
    
    let db = state.db.lock().await;
    db.save_monitor_window_width(&monitor_key, width)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn snap_to_bottom(app: tauri::AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;
    
    // Find which monitor the window is on
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    
    // Sort monitors by position (left to right) for consistent indexing
    let mut sorted_monitors: Vec<_> = monitors.iter().enumerate().collect();
    sorted_monitors.sort_by(|(_, a), (_, b)| {
        let pos_a = a.position();
        let pos_b = b.position();
        match pos_a.x.cmp(&pos_b.x) {
            std::cmp::Ordering::Equal => pos_a.y.cmp(&pos_b.y),
            other => other,
        }
    });
    
    let mut target_monitor = sorted_monitors.first().map(|(_, m)| *m).ok_or("No monitors found")?;
    let mut found_index = 0usize;
    
    for (sorted_index, (_, monitor)) in sorted_monitors.iter().enumerate() {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        let scale = monitor.scale_factor();
        
        // Convert to logical coordinates for comparison
        let mon_logical_width = (mon_size.width as f64 / scale) as i32;
        let mon_logical_height = (mon_size.height as f64 / scale) as i32;
        
        // Check if window center is within this monitor (using logical coordinates)
        // win_pos is already in physical pixels, convert to logical
        let win_logical_x = (win_pos.x as f64 / scale) as i32;
        let win_logical_y = (win_pos.y as f64 / scale) as i32;
        let win_logical_width = (win_size.width as f64 / scale) as i32;
        let win_logical_height = (win_size.height as f64 / scale) as i32;
        
        let win_center_x = win_logical_x + win_logical_width / 2;
        let win_center_y = win_logical_y + win_logical_height / 2;
        
        if win_center_x >= mon_pos.x && win_center_x < mon_pos.x + mon_logical_width &&
           win_center_y >= mon_pos.y && win_center_y < mon_pos.y + mon_logical_height {
            target_monitor = *monitor;
            found_index = sorted_index;
            break;
        }
    }
    
    // Update the saved monitor index
    LAST_MONITOR_INDEX.store(found_index, Ordering::SeqCst);
    log::info!("snap_to_bottom: updated LAST_MONITOR_INDEX={}", found_index);
    
    let mon_pos = target_monitor.position();
    let mon_size = target_monitor.size();
    let scale = target_monitor.scale_factor();
    
    // Convert to logical coordinates
    let mon_logical_height = (mon_size.height as f64 / scale) as i32;
    let win_logical_x = (win_pos.x as f64 / scale) as i32;
    let win_logical_height = (win_size.height as f64 / scale) as i32;
    
    // Keep x position, snap y to bottom (using logical coordinates)
    let new_y = mon_pos.y + mon_logical_height - win_logical_height;
    
    window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
        x: win_logical_x as f64,
        y: new_y as f64
    })).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn snap_to_edge(app: tauri::AppHandle, threshold: i32) -> Result<SnapResult, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;

    // Find which monitor the window is on
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;

    // Sort monitors by position (left to right) for consistent indexing
    let mut sorted_monitors: Vec<_> = monitors.iter().enumerate().collect();
    sorted_monitors.sort_by(|(_, a), (_, b)| {
        let pos_a = a.position();
        let pos_b = b.position();
        match pos_a.x.cmp(&pos_b.x) {
            std::cmp::Ordering::Equal => pos_a.y.cmp(&pos_b.y),
            other => other,
        }
    });

    let mut target_monitor = sorted_monitors.first().map(|(_, m)| *m).ok_or("No monitors found")?;
    let mut found_index = 0usize;

    // Determine which monitor the window is on based on window center
    for (sorted_index, (_, monitor)) in sorted_monitors.iter().enumerate() {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        let scale = monitor.scale_factor();

        // Convert to logical coordinates for comparison
        let mon_logical_width = (mon_size.width as f64 / scale) as i32;
        let mon_logical_height = (mon_size.height as f64 / scale) as i32;

        // win_pos is in physical pixels, convert to logical
        let win_logical_x = (win_pos.x as f64 / scale) as i32;
        let win_logical_y = (win_pos.y as f64 / scale) as i32;
        let win_logical_width = (win_size.width as f64 / scale) as i32;
        let win_logical_height = (win_size.height as f64 / scale) as i32;

        let win_center_x = win_logical_x + win_logical_width / 2;
        let win_center_y = win_logical_y + win_logical_height / 2;

        if win_center_x >= mon_pos.x && win_center_x < mon_pos.x + mon_logical_width &&
           win_center_y >= mon_pos.y && win_center_y < mon_pos.y + mon_logical_height {
            target_monitor = *monitor;
            found_index = sorted_index;
            break;
        }
    }

    // Update the saved monitor index
    LAST_MONITOR_INDEX.store(found_index, Ordering::SeqCst);
    log::info!("snap_to_edge: updated LAST_MONITOR_INDEX={}", found_index);

    let mon_pos = target_monitor.position();
    let mon_size = target_monitor.size();
    let scale = target_monitor.scale_factor();

    // Convert all measurements to logical coordinates
    let mon_logical_width = (mon_size.width as f64 / scale) as i32;
    let mon_logical_height = (mon_size.height as f64 / scale) as i32;
    let win_logical_x = (win_pos.x as f64 / scale) as i32;
    let win_logical_y = (win_pos.y as f64 / scale) as i32;
    let win_logical_width = (win_size.width as f64 / scale) as i32;
    let win_logical_height = (win_size.height as f64 / scale) as i32;

    // Calculate work area (excluding menu bar and dock on macOS)
    // Menu bar is typically 25 pixels, dock is typically 70 pixels at bottom
    #[cfg(target_os = "macos")]
    let (work_top, work_bottom) = {
        let menu_bar_height = 25;
        let dock_height = 70; // Approximate dock height when visible at bottom
        (mon_pos.y + menu_bar_height, mon_pos.y + mon_logical_height - dock_height)
    };

    #[cfg(not(target_os = "macos"))]
    let (work_top, work_bottom) = (mon_pos.y, mon_pos.y + mon_logical_height);

    let work_left = mon_pos.x;
    let work_right = mon_pos.x + mon_logical_width;

    // Calculate distances to each edge
    let dist_to_top = win_logical_y - work_top;
    let dist_to_bottom = work_bottom - (win_logical_y + win_logical_height);
    let dist_to_left = win_logical_x - work_left;
    let dist_to_right = work_right - (win_logical_x + win_logical_width);

    let mut snapped_edges: Vec<SnapEdge> = Vec::new();
    let mut new_x = win_logical_x;
    let mut new_y = win_logical_y;

    // Check each edge and snap if within threshold
    if dist_to_top.abs() <= threshold {
        new_y = work_top;
        snapped_edges.push(SnapEdge::Top);
    } else if dist_to_bottom.abs() <= threshold {
        new_y = work_bottom - win_logical_height;
        snapped_edges.push(SnapEdge::Bottom);
    }

    if dist_to_left.abs() <= threshold {
        new_x = work_left;
        snapped_edges.push(SnapEdge::Left);
    } else if dist_to_right.abs() <= threshold {
        new_x = work_right - win_logical_width;
        snapped_edges.push(SnapEdge::Right);
    }

    let snapped = !snapped_edges.is_empty();

    // Apply new position if snapped
    if snapped {
        window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
            x: new_x as f64,
            y: new_y as f64
        })).map_err(|e| e.to_string())?;
        log::info!("snap_to_edge: snapped to {:?} at ({}, {})", snapped_edges, new_x, new_y);
    } else {
        log::info!("snap_to_edge: no snap (distances: top={}, bottom={}, left={}, right={})",
                   dist_to_top, dist_to_bottom, dist_to_left, dist_to_right);
    }

    Ok(SnapResult {
        snapped,
        edges: snapped_edges,
        position: WindowPosition {
            x: new_x,
            y: new_y,
            width: win_size.width,
            height: win_size.height,
        },
    })
}

#[tauri::command]
pub async fn set_drawer_collapsed(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    collapsed: bool,
) -> Result<(), String> {
    // Legacy wrapper for backwards compatibility
    let mode = if collapsed { "collapsed" } else { "expanded" };
    set_drawer_mode(app, state, mode.to_string()).await
}

#[tauri::command]
pub async fn set_drawer_mode(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    mode: String,
) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;

    // Get monitors and sort by position (left to right)
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let mut sorted_monitors: Vec<_> = monitors.iter().collect();
    sorted_monitors.sort_by(|a, b| {
        let pos_a = a.position();
        let pos_b = b.position();
        match pos_a.x.cmp(&pos_b.x) {
            std::cmp::Ordering::Equal => pos_a.y.cmp(&pos_b.y),
            other => other,
        }
    });

    if sorted_monitors.is_empty() {
        return Err("No monitors found".to_string());
    }

    // Check if window is visible
    let is_visible = window.is_visible().unwrap_or(false);

    // Get current window position (in logical pixels) if visible
    let current_logical_x = if is_visible {
        let pos = window.outer_position().map_err(|e| e.to_string())?;
        let win_scale = window.scale_factor().map_err(|e| e.to_string())?;
        Some((pos.x as f64 / win_scale) as i32)
    } else {
        None
    };

    // Find which monitor to use based on current X position or saved index
    let (monitor_index, target_monitor) = if let Some(current_x) = current_logical_x {
        // Window is visible - find monitor containing current X position
        let mut found_idx = 0;
        let mut found_monitor = sorted_monitors[0];

        for (idx, monitor) in sorted_monitors.iter().enumerate() {
            let mon_pos = monitor.position();
            let mon_size = monitor.size();
            let scale = monitor.scale_factor();
            let mon_logical_width = (mon_size.width as f64 / scale) as i32;

            // Check if current_x is within this monitor's horizontal bounds
            if current_x >= mon_pos.x && current_x < mon_pos.x + mon_logical_width {
                found_idx = idx;
                found_monitor = *monitor;
                break;
            }

            // If X is past this monitor, this monitor becomes the candidate
            // (handles case where X is between monitors or past last monitor)
            if current_x >= mon_pos.x {
                found_idx = idx;
                found_monitor = *monitor;
            }
        }

        log::info!("set_drawer_mode: window visible at x={}, found monitor index={}", current_x, found_idx);
        (found_idx, found_monitor)
    } else {
        // Window is not visible - use saved monitor index
        let saved_index = LAST_MONITOR_INDEX.load(Ordering::SeqCst);
        let idx = if saved_index < sorted_monitors.len() { saved_index } else { 0 };
        log::info!("set_drawer_mode: window not visible, using saved monitor index={}", idx);
        (idx, sorted_monitors[idx])
    };

    // Update LAST_MONITOR_INDEX
    LAST_MONITOR_INDEX.store(monitor_index, Ordering::SeqCst);

    let mon_pos = target_monitor.position();
    let mon_size = target_monitor.size();
    let scale = target_monitor.scale_factor();

    // Convert monitor size to logical coordinates
    let mon_logical_width = (mon_size.width as f64 / scale) as i32;
    let mon_logical_height = (mon_size.height as f64 / scale) as i32;

    // Generate monitor key for this monitor
    let monitor_key = generate_monitor_key(mon_size.width, mon_size.height, scale);

    // Get saved width for this monitor or calculate adaptive width
    let db = state.db.lock().await;
    let saved_width = match db.get_monitor_window_width(&monitor_key).await {
        Ok(Some(w)) => {
            log::info!("set_drawer_mode: Using saved width {} for monitor {}", w, monitor_key);
            w
        }
        _ => {
            let adaptive = calculate_adaptive_width(mon_logical_width);
            log::info!("set_drawer_mode: Using adaptive width {} for monitor {}", adaptive, monitor_key);
            adaptive
        }
    };
    drop(db);

    // Set new height based on mode
    let new_logical_height = match mode.as_str() {
        "collapsed" => 48,    // Just header
        "expanded" => 280,    // History view (header ~48px + padding ~24px + card 192px)
        "full" => 450,        // Settings/Glossary view
        "popup" => 350,       // Translation/Polish popup view
        _ => 280,
    };

    // Set new size using logical coordinates
    window.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: saved_width as f64,
        height: new_logical_height as f64
    })).map_err(|e| e.to_string())?;

    // Calculate position
    let new_x = if let Some(current_x) = current_logical_x {
        // Keep current X, but clamp to current monitor bounds
        let min_x = mon_pos.x;
        let max_x = mon_pos.x + mon_logical_width - saved_width;
        current_x.clamp(min_x, max_x)
    } else {
        // Center horizontally on the monitor
        mon_pos.x + (mon_logical_width - saved_width) / 2
    };
    let new_y = mon_pos.y + mon_logical_height - new_logical_height;

    window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
        x: new_x as f64,
        y: new_y as f64
    })).map_err(|e| e.to_string())?;

    log::info!("set_drawer_mode: mode={}, monitor={}, size={}x{}, pos=({}, {})",
        mode, monitor_index, saved_width, new_logical_height, new_x, new_y);

    Ok(())
}
