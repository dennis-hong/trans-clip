use crate::ai::{
    normalize_provider_base_url, EndpointMode, ProviderKind, CUSTOM_ENDPOINT_API_KEY_ACCOUNT,
};
use crate::database::AiModelProfileRow;
use crate::keychain;
use crate::utils::streaming::normalize_anthropic_base_url;
use crate::AppState;
use tauri::State;

use super::types::{
    AddAiModelProfileRequest, AiApiKeyRequest, ApiKeyStatus, DeleteResponse, ErrorDetail,
    SetApiKeyResponse, UpdateAiModelProfileRequest, UpdateAiProviderConfigRequest,
    UpdateSettingsRequest, UserSettingsResponse,
};

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<UserSettingsResponse, String> {
    load_settings_response(&state).await
}

async fn load_settings_response(
    state: &State<'_, AppState>,
) -> Result<UserSettingsResponse, String> {
    let db = &state.db;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;
    let providers = db
        .get_ai_provider_configs()
        .await
        .map_err(|e| e.to_string())?;
    let models = db
        .get_ai_model_profiles()
        .await
        .map_err(|e| e.to_string())?;
    Ok(UserSettingsResponse::from_parts(
        settings, providers, models,
    ))
}

#[tauri::command]
pub async fn update_settings(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: UpdateSettingsRequest,
) -> Result<UserSettingsResponse, String> {
    let db = &state.db;
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
        let interval = v.clamp(200, 1000);
        current.double_press_interval = interval;
        crate::hotkey::set_double_press_interval(interval as u64);
    }
    if let Some(v) = settings.translation_cache_days {
        current.translation_cache_days = v.clamp(1, 30);
    }
    if let Some(v) = settings.show_source_app {
        current.show_source_app = if v { 1 } else { 0 };
    }
    if let Some(v) = settings.popup_position {
        current.popup_position = v.clone();
    }
    if let Some(v) = settings.launch_at_login {
        #[cfg(desktop)]
        {
            use tauri_plugin_autostart::ManagerExt;

            let autolaunch = app.autolaunch();
            let result = if v {
                autolaunch.enable()
            } else {
                autolaunch.disable()
            };

            if let Err(err) = result {
                return Err(format!("Failed to apply launch-at-login setting: {}", err));
            }
        }

        current.launch_at_login = if v { 1 } else { 0 };
    }
    if let Some(v) = settings.paste_delay_ms {
        current.paste_delay_ms = v.clamp(50, 500);
    }
    if let Some(v) = settings.anthropic_base_url {
        current.anthropic_base_url = normalize_anthropic_base_url(&v)?;
    }
    if let Some(v) = settings.preferred_model_profile_id {
        current.preferred_model_profile_id = Some(v);
    }

    db.update_settings(&current)
        .await
        .map_err(|e| e.to_string())?;

    load_settings_response(&state).await
}

#[tauri::command]
pub async fn update_ai_provider_config(
    state: State<'_, AppState>,
    request: UpdateAiProviderConfigRequest,
) -> Result<UserSettingsResponse, String> {
    let db = &state.db;
    let provider = db
        .get_ai_provider_config(&request.id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Provider config not found".to_string())?;
    let provider_kind = ProviderKind::from_db_value(&provider.provider_kind)
        .ok_or_else(|| "Invalid provider kind".to_string())?;
    let endpoint_mode = EndpointMode::from_db_value(&request.endpoint_mode)
        .ok_or_else(|| "Invalid endpoint mode".to_string())?;
    let base_url = if endpoint_mode == EndpointMode::Public {
        provider_kind.default_base_url().to_string()
    } else {
        normalize_provider_base_url(provider_kind, &request.base_url)?
    };

    db.update_ai_provider_config(
        &request.id,
        endpoint_mode.as_db_value(),
        &base_url,
        request.enabled,
    )
    .await
    .map_err(|e| e.to_string())?;

    load_settings_response(&state).await
}

#[tauri::command]
pub async fn add_ai_model_profile(
    state: State<'_, AppState>,
    request: AddAiModelProfileRequest,
) -> Result<UserSettingsResponse, String> {
    let db = &state.db;
    let provider = db
        .get_ai_provider_config(&request.provider_config_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Provider config not found".to_string())?;
    let provider_kind = ProviderKind::from_db_value(&provider.provider_kind)
        .ok_or_else(|| "Invalid provider kind".to_string())?;
    let display_name = request.display_name.trim();
    let model_id = request.model_id.trim();
    if display_name.is_empty() || model_id.is_empty() {
        return Err("Model display name and model id are required".to_string());
    }

    let profile = AiModelProfileRow {
        id: uuid::Uuid::new_v4().to_string(),
        provider_config_id: request.provider_config_id,
        display_name: display_name.to_string(),
        model_id: model_id.to_string(),
        api_interface: provider_kind
            .default_api_interface()
            .as_db_value()
            .to_string(),
        supports_streaming: 1,
        max_output_tokens: 4096,
        sort_order: 1000,
        created_at: String::new(),
        updated_at: String::new(),
    };

    db.insert_ai_model_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    load_settings_response(&state).await
}

#[tauri::command]
pub async fn update_ai_model_profile(
    state: State<'_, AppState>,
    request: UpdateAiModelProfileRequest,
) -> Result<UserSettingsResponse, String> {
    let db = &state.db;
    let provider = db
        .get_ai_provider_config(&request.provider_config_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Provider config not found".to_string())?;
    let provider_kind = ProviderKind::from_db_value(&provider.provider_kind)
        .ok_or_else(|| "Invalid provider kind".to_string())?;
    let existing = db
        .get_ai_model_profile(&request.id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Model profile not found".to_string())?;

    let profile = AiModelProfileRow {
        id: request.id,
        provider_config_id: request.provider_config_id,
        display_name: request.display_name.trim().to_string(),
        model_id: request.model_id.trim().to_string(),
        api_interface: provider_kind
            .default_api_interface()
            .as_db_value()
            .to_string(),
        supports_streaming: if request.supports_streaming { 1 } else { 0 },
        max_output_tokens: request.max_output_tokens.clamp(1, 32768),
        sort_order: existing.sort_order,
        created_at: existing.created_at,
        updated_at: existing.updated_at,
    };

    if profile.display_name.is_empty() || profile.model_id.is_empty() {
        return Err("Model display name and model id are required".to_string());
    }

    db.update_ai_model_profile(&profile)
        .await
        .map_err(|e| e.to_string())?;

    load_settings_response(&state).await
}

#[tauri::command]
pub async fn delete_ai_model_profile(
    state: State<'_, AppState>,
    id: String,
) -> Result<UserSettingsResponse, String> {
    let db = &state.db;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;
    if settings.preferred_model_profile_id.as_deref() == Some(id.as_str()) {
        return Err("Cannot delete the preferred model profile".to_string());
    }

    db.delete_ai_model_profile(&id)
        .await
        .map_err(|e| e.to_string())?;

    load_settings_response(&state).await
}

#[tauri::command]
pub async fn get_ai_api_key(request: AiApiKeyRequest) -> Result<ApiKeyStatus, String> {
    let exists = keychain::get_ai_api_key(&request.account)?.is_some();
    Ok(ApiKeyStatus {
        exists,
        is_valid: None,
        last_validated: None,
    })
}

#[tauri::command]
pub async fn set_ai_api_key(
    request: AiApiKeyRequest,
    api_key: String,
) -> Result<SetApiKeyResponse, String> {
    let api_key = api_key.trim().to_string();
    if !crate::database::Database::validate_api_key_format(&api_key) {
        return Ok(SetApiKeyResponse {
            success: false,
            is_valid: false,
            error: Some(ErrorDetail {
                code: "INVALID_KEY".to_string(),
                message: "Invalid API key format.".to_string(),
            }),
        });
    }

    keychain::store_ai_api_key(&request.account, &api_key)?;
    Ok(SetApiKeyResponse {
        success: true,
        is_valid: true,
        error: None,
    })
}

#[tauri::command]
pub async fn delete_ai_api_key(request: AiApiKeyRequest) -> Result<DeleteResponse, String> {
    if request.account == CUSTOM_ENDPOINT_API_KEY_ACCOUNT {
        log::info!("Deleting shared custom endpoint API key");
    }
    keychain::delete_ai_api_key(&request.account)?;
    Ok(DeleteResponse {
        success: true,
        error: None,
    })
}

#[tauri::command]
pub async fn get_api_key(state: State<'_, AppState>) -> Result<ApiKeyStatus, String> {
    // Try Keychain first
    match keychain::get_api_key_from_keychain() {
        Ok(Some(_)) => {
            log::info!("get_api_key: found in Keychain");
            return Ok(ApiKeyStatus {
                exists: true,
                is_valid: None,
                last_validated: None,
            });
        }
        Ok(None) => {}
        Err(e) => {
            log::warn!("Keychain read failed, falling back to SQLite: {}", e);
        }
    }

    // Fallback to SQLite (backward compat) and migrate if found
    let db = &state.db;
    let api_key = db.get_api_key().await.map_err(|e| e.to_string())?;
    let exists = api_key.is_some();

    if exists {
        // Trigger migration in the background
        keychain::migrate_api_key_from_db(db).await;
    }

    log::info!("get_api_key: exists={} (from SQLite fallback)", exists);
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
    let api_key = api_key.trim().to_string();

    // Basic format validation
    if !crate::database::Database::validate_api_key_format(&api_key) {
        return Ok(SetApiKeyResponse {
            success: false,
            is_valid: false,
            error: Some(ErrorDetail {
                code: "INVALID_KEY".to_string(),
                message: "Invalid API key format. Key should start with 'sk-'".to_string(),
            }),
        });
    }

    let settings = state.db.get_settings().await.map_err(|e| e.to_string())?;

    // Validate with API (but save anyway if validation fails due to network issues)
    let is_valid = match keychain::validate_api_key(&api_key, &settings.anthropic_base_url).await {
        Ok(valid) => valid,
        Err(e) => {
            log::warn!("API key validation error (saving anyway): {}", e);
            true // Assume valid if there's a network error
        }
    };

    if is_valid {
        // Store in Keychain
        match keychain::store_api_key(&api_key) {
            Ok(()) => {
                // Clear from SQLite if it was stored there (backward compat cleanup)
                let db = &state.db;
                let _ = db.delete_api_key().await;

                log::info!("API key saved to Keychain");
                Ok(SetApiKeyResponse {
                    success: true,
                    is_valid: true,
                    error: None,
                })
            }
            Err(e) => {
                log::error!("Failed to save API key to Keychain: {}", e);
                // Fallback: save to SQLite if Keychain fails
                let db = &state.db;
                db.set_api_key(&api_key).await.map_err(|e| e.to_string())?;
                log::warn!("API key saved to SQLite as Keychain fallback");
                Ok(SetApiKeyResponse {
                    success: true,
                    is_valid: true,
                    error: None,
                })
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
    // Delete from Keychain
    if let Err(e) = keychain::delete_api_key_from_keychain() {
        log::warn!("Failed to delete from Keychain: {}", e);
    }

    // Also clear from SQLite (backward compat cleanup)
    let db = &state.db;
    if let Err(e) = db.delete_api_key().await {
        log::warn!("Failed to clear API key from SQLite: {}", e);
    }

    Ok(DeleteResponse {
        success: true,
        error: None,
    })
}
