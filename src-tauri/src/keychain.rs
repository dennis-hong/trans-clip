use log;

const KEYCHAIN_SERVICE: &str = "com.transclip.app";
const KEYCHAIN_ACCOUNT: &str = "api_key";

/// Store API key in macOS Keychain
pub fn store_api_key(api_key: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use security_framework::passwords::set_generic_password;

        // delete first to avoid "duplicate item" errors on update
        let _ = delete_api_key_from_keychain();

        set_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, api_key.as_bytes())
            .map_err(|e| format!("Failed to store API key in Keychain: {}", e))
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = api_key;
        Err("Keychain is only supported on macOS".to_string())
    }
}

/// Retrieve API key from macOS Keychain
pub fn get_api_key_from_keychain() -> Result<Option<String>, String> {
    #[cfg(target_os = "macos")]
    {
        use security_framework::passwords::get_generic_password;

        match get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
            Ok(bytes) => {
                let key = String::from_utf8(bytes.to_vec())
                    .map_err(|e| format!("Invalid UTF-8 in Keychain: {}", e))?;
                Ok(Some(key))
            }
            Err(e) => {
                // errSecItemNotFound = -25300
                if e.code() == -25300 {
                    Ok(None)
                } else {
                    Err(format!("Failed to read from Keychain: {}", e))
                }
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(None)
    }
}

/// Delete API key from macOS Keychain
pub fn delete_api_key_from_keychain() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use security_framework::passwords::delete_generic_password;

        match delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Ignore "item not found" errors
                if e.code() == -25300 {
                    Ok(())
                } else {
                    Err(format!("Failed to delete from Keychain: {}", e))
                }
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

/// Migrate API key from SQLite to Keychain (for backward compatibility)
/// Returns true if migration occurred
pub async fn migrate_api_key_from_db(db: &crate::database::Database) -> bool {
    // Important: check legacy SQLite value first.
    // If no legacy key exists, skip touching Keychain to avoid unnecessary auth prompts on startup.
    let legacy_key = match db.get_api_key().await {
        Ok(Some(key)) if !key.is_empty() => key,
        Ok(_) => return false,
        Err(e) => {
            log::warn!("Failed to read API key from SQLite during migration: {}", e);
            return false;
        }
    };

    // Check if key exists in Keychain already
    match get_api_key_from_keychain() {
        Ok(Some(_)) => return false, // Already in Keychain, no migration needed
        Ok(None) => {}
        Err(e) => {
            log::warn!("Keychain read failed during migration check: {}", e);
            return false;
        }
    }

    // Migrate to Keychain
    match store_api_key(&legacy_key) {
        Ok(()) => {
            // Clear from SQLite after successful migration
            if let Err(e) = db.delete_api_key().await {
                log::warn!("Failed to clear API key from SQLite after migration: {}", e);
            }
            log::info!("API key migrated from SQLite to Keychain");
            true
        }
        Err(e) => {
            log::warn!("Failed to migrate API key to Keychain: {}", e);
            false
        }
    }
}

/// Resolve API key: try Keychain first, then fall back to settings (SQLite)
pub fn resolve_api_key(settings_api_key: &Option<String>) -> Option<String> {
    // Try Keychain first
    match get_api_key_from_keychain() {
        Ok(Some(key)) => return Some(key),
        Ok(None) => {}
        Err(e) => {
            log::warn!("Keychain read failed, falling back to settings: {}", e);
        }
    }

    // Fallback to settings (SQLite)
    settings_api_key.as_ref().filter(|k| !k.is_empty()).cloned()
}

/// Validate the API key by making a test request to the Claude API
pub async fn validate_api_key(api_key: &str) -> Result<bool, String> {
    let client = reqwest::Client::new();

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "test"}]
        }))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    // 200 = valid, 401 = invalid key, 429 = rate limited (but key is valid)
    // 400 = bad request (but key might still be valid)
    match response.status().as_u16() {
        200 | 429 | 400 => Ok(true),
        401 => Ok(false),
        status => {
            log::warn!("API validation returned unexpected status: {}", status);
            Ok(true) // Assume valid if we get an unexpected status
        }
    }
}
