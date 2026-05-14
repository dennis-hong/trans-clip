use crate::ai::{
    AiError, AiErrorCode, AiResolvedRequest, AiRuntime, AiStreamEvent, AiTextRequest, ModelProfile,
    ProviderConfig,
};
use crate::database::UserSettingsRow;
use crate::keychain;
use crate::prompts;
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

    // Detect language
    let detected_lang = detect_language(&text);

    // Build prompts using the prompts module
    let system_prompt = if detected_lang == "ko" {
        prompts::polish::build_system_prompt()
    } else {
        prompts::polish::build_system_prompt_english()
    };

    let user_prompt =
        prompts::polish::build_user_prompt(&text, &context, &channel, &options, &detected_lang);

    let model_profile_id = effective_model_profile_id(&settings, model.as_ref());
    let resolved = match resolve_ai_request(
        db,
        &settings,
        model_profile_id,
        Some(system_prompt),
        user_prompt,
    )
    .await
    {
        Ok(resolved) => resolved,
        Err(error) => {
            return Ok(PolishResponse {
                success: false,
                polished_text: None,
                detected_language: None,
                token_usage: None,
                error: Some(error),
            });
        }
    };

    let response = match AiRuntime::default().complete(resolved).await {
        Ok(response) => response,
        Err(err) => {
            return Ok(PolishResponse {
                success: false,
                polished_text: None,
                detected_language: None,
                token_usage: None,
                error: Some(map_ai_error(err)),
            });
        }
    };

    Ok(PolishResponse {
        success: true,
        polished_text: Some(response.text.trim().to_string()),
        detected_language: Some(detected_lang),
        token_usage: response_token_usage(response.input_tokens, response.output_tokens),
        error: None,
    })
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

    // Detect language
    let detected_lang = detect_language(&text);

    // Send Started event
    emit_polish_stream_event(
        &on_event,
        PolishStreamEvent::Started {
            detected_language: Some(detected_lang.clone()),
        },
    );

    // Build prompts using the prompts module
    let system_prompt = if detected_lang == "ko" {
        prompts::polish::build_system_prompt()
    } else {
        prompts::polish::build_system_prompt_english()
    };

    let user_prompt =
        prompts::polish::build_user_prompt(&text, &context, &channel, &options, &detected_lang);

    let model_profile_id = effective_model_profile_id(&settings, model.as_ref());
    let resolved = match resolve_ai_request(
        db,
        &settings,
        model_profile_id,
        Some(system_prompt),
        user_prompt,
    )
    .await
    {
        Ok(resolved) => resolved,
        Err(error) => {
            emit_polish_stream_event(
                &on_event,
                PolishStreamEvent::Error {
                    code: error.code,
                    message: error.message,
                },
            );
            return Ok(());
        }
    };

    let response = AiRuntime::default()
        .stream(resolved, |event| match event {
            AiStreamEvent::Delta { text } => {
                emit_polish_stream_event(&on_event, PolishStreamEvent::Delta { text })
            }
        })
        .await;

    match response {
        Ok(response) => {
            emit_polish_stream_event(
                &on_event,
                PolishStreamEvent::Completed {
                    full_text: response.text.trim().to_string(),
                    token_usage: response_token_usage(
                        response.input_tokens,
                        response.output_tokens,
                    ),
                },
            );
        }
        Err(err) => {
            let error = map_ai_error(err);
            emit_polish_stream_event(
                &on_event,
                PolishStreamEvent::Error {
                    code: error.code,
                    message: error.message,
                },
            );
        }
    }

    Ok(())
}
