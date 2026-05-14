use crate::ai::{
    AiError, AiErrorCode, AiResolvedRequest, AiRuntime, AiStreamEvent, AiTextRequest, ModelProfile,
    ProviderConfig,
};
use crate::database::{TranslationRow, UserSettingsRow};
use crate::keychain;
use crate::AppState;
use tauri::ipc::Channel;
use tauri::State;

use super::types::{TokenUsage, TranslateError, TranslateResponse, TranslateStreamEvent};

fn emit_translate_stream_event(
    on_event: &Channel<TranslateStreamEvent>,
    event: TranslateStreamEvent,
) {
    if let Err(err) = on_event.send(event) {
        log::warn!("Failed to send translate stream event: {}", err);
    }
}

fn translate_error(code: &str, message: impl Into<String>) -> TranslateError {
    TranslateError {
        code: code.to_string(),
        message: message.into(),
    }
}

fn map_ai_error(err: AiError) -> TranslateError {
    let code = match err.code {
        AiErrorCode::MissingApiKey | AiErrorCode::InvalidApiKey => "INVALID_API_KEY",
        AiErrorCode::InvalidEndpoint => "NETWORK_ERROR",
        AiErrorCode::UnsupportedModel | AiErrorCode::ProviderError => "API_ERROR",
        AiErrorCode::StreamParseError => "STREAM_ERROR",
        AiErrorCode::NetworkError => "NETWORK_ERROR",
    };
    translate_error(code, err.message)
}

fn response_token_usage(
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
) -> Option<TokenUsage> {
    match (input_tokens, output_tokens) {
        (Some(i), Some(o)) => Some(TokenUsage {
            input_tokens: i,
            output_tokens: o,
        }),
        _ => None,
    }
}

fn effective_model_profile_id(settings: &UserSettingsRow, model: Option<&String>) -> String {
    model
        .cloned()
        .or_else(|| settings.preferred_model_profile_id.clone())
        .unwrap_or_else(|| format!("anthropic:{}", settings.preferred_model))
}

async fn resolve_ai_request(
    db: &crate::database::Database,
    settings: &UserSettingsRow,
    model_profile_id: String,
    system_prompt: Option<String>,
    user_prompt: String,
) -> Result<AiResolvedRequest, TranslateError> {
    let profile_row = db
        .get_ai_model_profile(&model_profile_id)
        .await
        .map_err(|e| translate_error("API_ERROR", e.to_string()))?
        .ok_or_else(|| translate_error("API_ERROR", "Selected model profile was not found"))?;
    let provider_row = db
        .get_ai_provider_config(&profile_row.provider_config_id)
        .await
        .map_err(|e| translate_error("API_ERROR", e.to_string()))?
        .ok_or_else(|| translate_error("API_ERROR", "Selected provider was not found"))?;

    let provider_config =
        ProviderConfig::try_from(provider_row).map_err(|e| translate_error("API_ERROR", e))?;
    if !provider_config.enabled {
        return Err(translate_error(
            "API_ERROR",
            "Selected provider is disabled",
        ));
    }
    let model_profile =
        ModelProfile::try_from(profile_row).map_err(|e| translate_error("API_ERROR", e))?;
    let api_key =
        keychain::resolve_ai_api_key(&provider_config, &settings.api_key).ok_or_else(|| {
            translate_error(
                "INVALID_API_KEY",
                "API key is not configured for the selected provider.",
            )
        })?;

    let max_output_tokens = model_profile.max_output_tokens;
    Ok(AiResolvedRequest {
        provider_config,
        model_profile,
        api_key,
        request: AiTextRequest {
            model_profile_id: Some(model_profile_id),
            system_prompt,
            user_prompt,
            max_output_tokens,
            temperature: None,
        },
    })
}

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

    let model_profile_id = effective_model_profile_id(&settings, None);
    if let Ok(Some(cached)) = db
        .find_cached_translation(
            &text,
            &src_lang,
            &tgt_lang,
            Some(&model_profile_id),
            settings.translation_cache_days,
        )
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
        let model_profile_id = effective_model_profile_id(&settings, model.as_ref());
        if let Ok(Some(cached)) = db
            .find_cached_translation(
                &text,
                &src_lang,
                &tgt_lang,
                Some(&model_profile_id),
                settings.translation_cache_days,
            )
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

    let model_profile_id = effective_model_profile_id(&settings, model.as_ref());
    let prompt = format!(
        "Translate the following text from {} to {}. Return only the translated text without any explanation.{}\n\nText to translate:\n{}",
        if src_lang == "ko" { "Korean" } else { "English" },
        if tgt_lang == "ko" { "Korean" } else { "English" },
        glossary_context,
        text
    );

    let resolved = match resolve_ai_request(db, &settings, model_profile_id, None, prompt).await {
        Ok(resolved) => resolved,
        Err(error) => {
            return Ok(TranslateResponse {
                success: false,
                translated_text: None,
                detected_language: None,
                from_cache: false,
                glossary_applied: vec![],
                token_usage: None,
                error: Some(error),
            });
        }
    };

    let provider_kind = resolved
        .provider_config
        .provider_kind
        .as_db_value()
        .to_string();
    let endpoint_mode = resolved
        .provider_config
        .endpoint_mode
        .as_db_value()
        .to_string();
    let provider_config_id = resolved.provider_config.id.clone();
    let model_profile_id = resolved.model_profile.id.clone();
    let model_id = resolved.model_profile.model_id.clone();

    let response = match AiRuntime::default().complete(resolved).await {
        Ok(response) => response,
        Err(err) => {
            return Ok(TranslateResponse {
                success: false,
                translated_text: None,
                detected_language: None,
                from_cache: false,
                glossary_applied: vec![],
                token_usage: None,
                error: Some(map_ai_error(err)),
            });
        }
    };

    let glossary_ids: Vec<String> = glossary_matches.iter().map(|g| g.id.clone()).collect();
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

    let translation = TranslationRow {
        id: uuid::Uuid::new_v4().to_string(),
        source_text: text,
        translated_text: response.text.clone(),
        source_language: src_lang.clone(),
        target_language: tgt_lang,
        model: model_id,
        created_at: chrono::Utc::now().to_rfc3339(),
        glossary_used,
        input_tokens: response.input_tokens,
        output_tokens: response.output_tokens,
        model_profile_id: Some(model_profile_id),
        provider_kind: Some(provider_kind),
        provider_config_id: Some(provider_config_id),
        endpoint_mode: Some(endpoint_mode),
    };

    if let Err(err) = db.insert_translation(&translation).await {
        log::warn!("Failed to cache translation (non-streaming): {}", err);
    }

    if !glossary_ids.is_empty() {
        if let Err(err) = db.increment_glossary_usage(&glossary_ids).await {
            log::warn!("Failed to update glossary usage (non-streaming): {}", err);
        }
    }

    Ok(TranslateResponse {
        success: true,
        translated_text: Some(response.text),
        detected_language: Some(src_lang),
        from_cache: false,
        glossary_applied: glossary_ids,
        token_usage: response_token_usage(response.input_tokens, response.output_tokens),
        error: None,
    })
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
        emit_translate_stream_event(
            &on_event,
            TranslateStreamEvent::Error {
                code: "EMPTY_TEXT".to_string(),
                message: "Text cannot be empty".to_string(),
            },
        );
        return Ok(());
    }

    if text.len() > 10000 {
        emit_translate_stream_event(
            &on_event,
            TranslateStreamEvent::Error {
                code: "TEXT_TOO_LONG".to_string(),
                message: "Text exceeds maximum length of 10000 characters".to_string(),
            },
        );
        return Ok(());
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

    // Check if explicit model is provided (user wants a specific model, skip cache)
    let explicit_model = model.is_some();

    // Check cache first (skip cache if explicit model is provided)
    if !explicit_model {
        let model_profile_id = effective_model_profile_id(&settings, model.as_ref());
        if let Ok(Some(cached)) = db
            .find_cached_translation(
                &text,
                &src_lang,
                &tgt_lang,
                Some(&model_profile_id),
                settings.translation_cache_days,
            )
            .await
        {
            let glossary_applied: Vec<String> = cached
                .glossary_used
                .map(|g| serde_json::from_str(&g).unwrap_or_default())
                .unwrap_or_default();

            let cached_text = cached.translated_text;

            emit_translate_stream_event(
                &on_event,
                TranslateStreamEvent::Started {
                    detected_language: Some(src_lang.clone()),
                    from_cache: true,
                    glossary_applied: glossary_applied.clone(),
                },
            );

            emit_translate_stream_event(
                &on_event,
                TranslateStreamEvent::Completed {
                    full_text: cached_text.clone(),
                    token_usage: match (cached.input_tokens, cached.output_tokens) {
                        (Some(i), Some(o)) => Some(TokenUsage {
                            input_tokens: i,
                            output_tokens: o,
                        }),
                        _ => None,
                    },
                },
            );

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
    emit_translate_stream_event(
        &on_event,
        TranslateStreamEvent::Started {
            detected_language: Some(src_lang.clone()),
            from_cache: false,
            glossary_applied: glossary_ids.clone(),
        },
    );

    let model_profile_id = effective_model_profile_id(&settings, model.as_ref());
    let prompt = format!(
        "Translate the following text from {} to {}. Return only the translated text without any explanation.{}\n\nText to translate:\n{}",
        if src_lang == "ko" { "Korean" } else { "English" },
        if tgt_lang == "ko" { "Korean" } else { "English" },
        glossary_context,
        text
    );

    let resolved = match resolve_ai_request(db, &settings, model_profile_id, None, prompt).await {
        Ok(resolved) => resolved,
        Err(error) => {
            emit_translate_stream_event(
                &on_event,
                TranslateStreamEvent::Error {
                    code: error.code,
                    message: error.message,
                },
            );
            return Ok(());
        }
    };

    let provider_kind = resolved
        .provider_config
        .provider_kind
        .as_db_value()
        .to_string();
    let endpoint_mode = resolved
        .provider_config
        .endpoint_mode
        .as_db_value()
        .to_string();
    let provider_config_id = resolved.provider_config.id.clone();
    let model_profile_id = resolved.model_profile.id.clone();
    let model_id = resolved.model_profile.model_id.clone();

    let response = AiRuntime::default()
        .stream(resolved, |event| match event {
            AiStreamEvent::Delta { text } => {
                emit_translate_stream_event(&on_event, TranslateStreamEvent::Delta { text })
            }
        })
        .await;

    let response = match response {
        Ok(response) => response,
        Err(err) => {
            let error = map_ai_error(err);
            emit_translate_stream_event(
                &on_event,
                TranslateStreamEvent::Error {
                    code: error.code,
                    message: error.message,
                },
            );
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

    let translation = TranslationRow {
        id: uuid::Uuid::new_v4().to_string(),
        source_text: text,
        translated_text: response.text.clone(),
        source_language: src_lang,
        target_language: tgt_lang,
        model: model_id,
        created_at: chrono::Utc::now().to_rfc3339(),
        glossary_used,
        input_tokens: response.input_tokens,
        output_tokens: response.output_tokens,
        model_profile_id: Some(model_profile_id),
        provider_kind: Some(provider_kind),
        provider_config_id: Some(provider_config_id),
        endpoint_mode: Some(endpoint_mode),
    };

    if let Err(err) = db.insert_translation(&translation).await {
        log::warn!("Failed to cache translation (streaming): {}", err);
    }

    if !glossary_ids.is_empty() {
        if let Err(err) = db.increment_glossary_usage(&glossary_ids).await {
            log::warn!("Failed to update glossary usage (streaming): {}", err);
        }
    }

    emit_translate_stream_event(
        &on_event,
        TranslateStreamEvent::Completed {
            full_text: response.text,
            token_usage: response_token_usage(response.input_tokens, response.output_tokens),
        },
    );

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
