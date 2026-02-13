use crate::keychain;
use crate::prompts;
use crate::AppState;
use futures_util::StreamExt;
use tauri::ipc::Channel;
use tauri::State;

use super::translate::detect_language;
use super::types::{PolishResponse, PolishStreamEvent, TokenUsage, TranslateError};

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

    // Release DB lock before external API call to avoid blocking unrelated DB operations.
    drop(db);

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

    // Get API key from Keychain (with SQLite fallback)
    let api_key = match keychain::resolve_api_key(&settings.api_key) {
        Some(key) => key,
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

    let user_prompt =
        prompts::polish::build_user_prompt(&text, &context, &channel, &options, &detected_lang);

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

                            if let Some(json_str) = line.strip_prefix("data: ") {
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
                                            if let Some(usage) =
                                                event["message"]["usage"].as_object()
                                            {
                                                input_tokens = usage["input_tokens"]
                                                    .as_i64()
                                                    .map(|v| v as i32);
                                            }
                                        }
                                        "message_delta" => {
                                            if let Some(usage) = event["usage"].as_object() {
                                                output_tokens = usage["output_tokens"]
                                                    .as_i64()
                                                    .map(|v| v as i32);
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
