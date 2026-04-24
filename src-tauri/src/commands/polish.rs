use crate::keychain;
use crate::prompts;
use crate::utils::streaming::{
    anthropic_http_client, anthropic_messages_url, extract_anthropic_message_text,
    stream_anthropic_sse,
};
use crate::AppState;
use tauri::ipc::Channel;
use tauri::State;

use super::translate::detect_language;
use super::types::{PolishResponse, PolishStreamEvent, TokenUsage, TranslateError};

fn emit_polish_stream_event(on_event: &Channel<PolishStreamEvent>, event: PolishStreamEvent) {
    if let Err(err) = on_event.send(event) {
        log::warn!("Failed to send polish stream event: {}", err);
    }
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

    let db = &state.db;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;

    // Get API key from Keychain (with SQLite fallback)
    let api_key = match keychain::resolve_api_key(&settings.api_key) {
        Some(key) => key,
        _ => {
            return Ok(PolishResponse {
                success: false,
                polished_text: None,
                detected_language: None,
                token_usage: None,
                error: Some(TranslateError {
                    code: "INVALID_API_KEY".to_string(),
                    message: "API key not configured. Please set your Claude API key in Settings."
                        .to_string(),
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

    let user_prompt =
        prompts::polish::build_user_prompt(&text, &context, &channel, &options, &detected_lang);

    // Call Claude API
    let client = anthropic_http_client();
    let messages_url = anthropic_messages_url(&settings.anthropic_base_url)?;
    let response = client
        .post(messages_url)
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

                let polished_text = match extract_anthropic_message_text(&body) {
                    Ok(text) => text.trim().to_string(),
                    Err(parse_err) => {
                        return Ok(PolishResponse {
                            success: false,
                            polished_text: None,
                            detected_language: None,
                            token_usage: None,
                            error: Some(TranslateError {
                                code: "API_ERROR".to_string(),
                                message: format!("Invalid API response: {}", parse_err),
                            }),
                        });
                    }
                };

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
        emit_polish_stream_event(
            &on_event,
            PolishStreamEvent::Error {
                code: "EMPTY_TEXT".to_string(),
                message: "Text cannot be empty".to_string(),
            },
        );
        return Ok(());
    }

    if text.len() > 10000 {
        emit_polish_stream_event(
            &on_event,
            PolishStreamEvent::Error {
                code: "TEXT_TOO_LONG".to_string(),
                message: "Text exceeds maximum length of 10000 characters".to_string(),
            },
        );
        return Ok(());
    }

    let db = &state.db;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;

    // Get API key from Keychain (with SQLite fallback)
    let api_key = match keychain::resolve_api_key(&settings.api_key) {
        Some(key) => key,
        _ => {
            emit_polish_stream_event(
                &on_event,
                PolishStreamEvent::Error {
                    code: "INVALID_API_KEY".to_string(),
                    message: "API key not configured. Please set your Claude API key in Settings."
                        .to_string(),
                },
            );
            return Ok(());
        }
    };

    // Detect language
    let detected_lang = detect_language(&text);

    // Send Started event
    emit_polish_stream_event(
        &on_event,
        PolishStreamEvent::Started {
            detected_language: Some(detected_lang.clone()),
        },
    );

    // Use provided model or fall back to settings
    let use_model = model.unwrap_or_else(|| settings.preferred_model.clone());

    // Build prompts using the prompts module
    let system_prompt = if detected_lang == "ko" {
        prompts::polish::build_system_prompt()
    } else {
        prompts::polish::build_system_prompt_english()
    };

    let user_prompt =
        prompts::polish::build_user_prompt(&text, &context, &channel, &options, &detected_lang);

    // Call Claude API with streaming
    let client = anthropic_http_client();
    let messages_url = anthropic_messages_url(&settings.anthropic_base_url)?;
    let response = client
        .post(messages_url)
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
                emit_polish_stream_event(
                    &on_event,
                    PolishStreamEvent::Error {
                        code: "INVALID_API_KEY".to_string(),
                        message: "Invalid API key".to_string(),
                    },
                );
                return Ok(());
            }

            if !res.status().is_success() {
                emit_polish_stream_event(
                    &on_event,
                    PolishStreamEvent::Error {
                        code: "API_ERROR".to_string(),
                        message: format!("API error: {}", res.status()),
                    },
                );
                return Ok(());
            }

            let stream_result = stream_anthropic_sse(res, |delta| {
                emit_polish_stream_event(
                    &on_event,
                    PolishStreamEvent::Delta {
                        text: delta.to_string(),
                    },
                );
            })
            .await;

            let (full_text, input_tokens, output_tokens) = match stream_result {
                Ok(result) => (result.full_text, result.input_tokens, result.output_tokens),
                Err(err) => {
                    emit_polish_stream_event(
                        &on_event,
                        PolishStreamEvent::Error {
                            code: "STREAM_ERROR".to_string(),
                            message: err,
                        },
                    );
                    return Ok(());
                }
            };

            // Send Completed event
            emit_polish_stream_event(
                &on_event,
                PolishStreamEvent::Completed {
                    full_text: full_text.trim().to_string(),
                    token_usage: match (input_tokens, output_tokens) {
                        (Some(i), Some(o)) => Some(TokenUsage {
                            input_tokens: i,
                            output_tokens: o,
                        }),
                        _ => None,
                    },
                },
            );
        }
        Err(e) => {
            emit_polish_stream_event(
                &on_event,
                PolishStreamEvent::Error {
                    code: "NETWORK_ERROR".to_string(),
                    message: format!("Network error: {}", e),
                },
            );
        }
    }

    Ok(())
}
