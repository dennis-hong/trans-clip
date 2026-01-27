use crate::keychain;
use crate::AppState;
use tauri::State;

use super::types::{
    ApiKeyStatus, DeleteResponse, ErrorDetail, SetApiKeyResponse, UpdateSettingsRequest,
    UserSettingsResponse,
};

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
