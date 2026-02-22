use crate::keychain;
use crate::AppState;
use tauri::State;

use super::types::{
    ApiKeyStatus, DeleteResponse, ErrorDetail, SetApiKeyResponse, UpdateSettingsRequest,
    UserSettingsResponse,
};

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<UserSettingsResponse, String> {
    let db = &state.db;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;
    Ok(UserSettingsResponse::from(settings))
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

    db.update_settings(&current)
        .await
        .map_err(|e| e.to_string())?;

    Ok(UserSettingsResponse::from(current))
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
