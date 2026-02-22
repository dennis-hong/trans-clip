use crate::database::TranslationRow;
use crate::keychain;
use crate::utils::streaming::{anthropic_http_client, stream_anthropic_sse};
use crate::AppState;
use tauri::ipc::Channel;
use tauri::State;

use super::types::{TokenUsage, TranslateError, TranslateResponse, TranslateStreamEvent};

#[tauri::command]
pub async fn get_cached_translation(
    state: State<'_, AppState>,
    text: String,
    source_language: Option<String>,
    target_language: Option<String>,
    model: Option<String>,
) -> Result<Option<TranslateResponse>, String> {
    // Validation
    if text.is_empty() {
        return Ok(Some(TranslateResponse {
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
        }));
    }

    if text.len() > 10000 {
        return Ok(Some(TranslateResponse {
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
        }));
    }

    // If explicit model is provided, caller intends to bypass cache.
    if model.is_some() {
        return Ok(None);
    }

    let db = &state.db;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;

    // Detect or use provided language
    let src_lang = source_language.unwrap_or_else(|| detect_language(&text));
    let tgt_lang = target_language.unwrap_or_else(|| {
        if src_lang == "ko" {
            "en".to_string()
        } else {
            "ko".to_string()
        }
    });

    if let Ok(Some(cached)) = db
        .find_cached_translation(&text, &src_lang, &tgt_lang, settings.translation_cache_days)
        .await
    {
        return Ok(Some(TranslateResponse {
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
        }));
    }

    Ok(None)
}

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

    let db = &state.db;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;

    // Get API key from Keychain (with SQLite fallback)
    let api_key = match keychain::resolve_api_key(&settings.api_key) {
        Some(key) => key,
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
                    message: "API key not configured. Please set your Claude API key in Settings."
                        .to_string(),
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

    // Use provided model or fall back to settings
    let use_model = model.unwrap_or_else(|| settings.preferred_model.clone());

    // Call Claude API
    let client = anthropic_http_client();
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
                let glossary_used = match serde_json::to_string(&glossary_ids) {
                    Ok(value) => Some(value),
                    Err(err) => {
                        log::warn!(
                            "Failed to serialize glossary IDs for cache entry (non-streaming): {}",
                            err
                        );
                        None
                    }
                };

                // Cache the translation
                let translation = TranslationRow {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_text: text,
                    translated_text: translated_text.clone(),
                    source_language: src_lang.clone(),
                    target_language: tgt_lang,
                    model: use_model,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    glossary_used,
                    input_tokens,
                    output_tokens,
                };

                let db = &state.db;
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

    let db = &state.db;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;

    // Get API key from Keychain (with SQLite fallback)
    let api_key = match keychain::resolve_api_key(&settings.api_key) {
        Some(key) => key,
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

            let cached_text = cached.translated_text;

            let _ = on_event.send(TranslateStreamEvent::Started {
                detected_language: Some(src_lang.clone()),
                from_cache: true,
                glossary_applied: glossary_applied.clone(),
            });

            let _ = on_event.send(TranslateStreamEvent::Completed {
                full_text: cached_text.clone(),
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

    // Call Claude API with streaming
    let client = anthropic_http_client();
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

            let stream_result = stream_anthropic_sse(res, |delta| {
                let _ = on_event.send(TranslateStreamEvent::Delta {
                    text: delta.to_string(),
                });
            })
            .await;

            let (full_text, input_tokens, output_tokens) = match stream_result {
                Ok(result) => (result.full_text, result.input_tokens, result.output_tokens),
                Err(err) => {
                    let _ = on_event.send(TranslateStreamEvent::Error {
                        code: "STREAM_ERROR".to_string(),
                        message: err,
                    });
                    return Ok(());
                }
            };
            let glossary_used = match serde_json::to_string(&glossary_ids) {
                Ok(value) => Some(value),
                Err(err) => {
                    log::warn!(
                        "Failed to serialize glossary IDs for cache entry (streaming): {}",
                        err
                    );
                    None
                }
            };

            // Cache the translation
            let db = &state.db;
            let translation = TranslationRow {
                id: uuid::Uuid::new_v4().to_string(),
                source_text: text,
                translated_text: full_text.clone(),
                source_language: src_lang,
                target_language: tgt_lang,
                model: use_model,
                created_at: chrono::Utc::now().to_rfc3339(),
                glossary_used,
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
                full_text: full_text.clone(),
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

pub fn detect_language(text: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::detect_language;

    #[test]
    fn detects_korean_when_ratio_is_high_enough() {
        let result = detect_language("안녕하세요 hello");
        assert_eq!(result, "ko");
    }

    #[test]
    fn detects_english_when_ratio_is_low() {
        let result = detect_language("hello world 123");
        assert_eq!(result, "en");
    }

    #[test]
    fn defaults_to_english_when_no_alphabetic_chars() {
        let result = detect_language("12345 !@#$%");
        assert_eq!(result, "en");
    }
}
