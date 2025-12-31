use keyring::Entry;

const SERVICE_NAME: &str = "com.transclip.app";
const API_KEY_ACCOUNT: &str = "claude-api-key";

#[derive(Debug)]
#[allow(dead_code)]
pub enum KeychainError {
    NotFound,
    AccessDenied,
    Other(String),
}

impl std::fmt::Display for KeychainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeychainError::NotFound => write!(f, "API key not found in keychain"),
            KeychainError::AccessDenied => write!(f, "Access to keychain denied"),
            KeychainError::Other(msg) => write!(f, "Keychain error: {}", msg),
        }
    }
}

impl std::error::Error for KeychainError {}

impl From<keyring::Error> for KeychainError {
    fn from(err: keyring::Error) -> Self {
        match err {
            keyring::Error::NoEntry => KeychainError::NotFound,
            keyring::Error::Ambiguous(_) => KeychainError::Other("Ambiguous entry".to_string()),
            keyring::Error::NoStorageAccess(_) => KeychainError::AccessDenied,
            _ => KeychainError::Other(err.to_string()),
        }
    }
}

pub struct Keychain;

impl Keychain {
    fn get_entry() -> Result<Entry, KeychainError> {
        Entry::new(SERVICE_NAME, API_KEY_ACCOUNT).map_err(|e| KeychainError::Other(e.to_string()))
    }

    /// Check if an API key exists in the keychain
    pub fn exists() -> bool {
        match Self::get() {
            Ok(_) => {
                log::info!("Keychain: API key exists");
                true
            }
            Err(KeychainError::NotFound) => {
                log::info!("Keychain: API key not found");
                false
            }
            Err(e) => {
                // NOTE: in some environments keychain access can be temporarily unavailable
                // (locked keychain, permission issues, etc.). Treat this as "exists unknown"
                // and don't falsely report "missing".
                log::warn!("Keychain: failed to check API key existence: {}", e);
                true
            }
        }
    }

    /// Get the API key from the keychain
    pub fn get() -> Result<String, KeychainError> {
        let entry = Self::get_entry()?;
        match entry.get_password() {
            Ok(pw) => Ok(pw),
            Err(keyring::Error::Ambiguous(credentials)) => {
                log::warn!(
                    "Keychain: ambiguous credential match ({} items). Using the first match.",
                    credentials.len()
                );
                let first = credentials
                    .first()
                    .ok_or_else(|| KeychainError::Other("Ambiguous credential list is empty".to_string()))?;
                first.get_password().map_err(KeychainError::from)
            }
            Err(e) => Err(KeychainError::from(e)),
        }
    }

    /// Set the API key in the keychain
    pub fn set(api_key: &str) -> Result<(), KeychainError> {
        let entry = Self::get_entry()?;
        match entry.set_password(api_key) {
            Ok(_) => Ok(()),
            Err(keyring::Error::Ambiguous(credentials)) => {
                // If duplicates exist, clean them up and retry.
                log::warn!(
                    "Keychain: ambiguous credential match on set ({} items). Deleting duplicates and retrying.",
                    credentials.len()
                );
                for cred in credentials {
                    if let Err(e) = cred.delete_credential() {
                        log::warn!("Keychain: failed to delete duplicate credential: {}", e);
                    }
                }
                entry.set_password(api_key).map_err(KeychainError::from)
            }
            Err(e) => Err(KeychainError::from(e)),
        }
    }

    /// Delete the API key from the keychain
    pub fn delete() -> Result<(), KeychainError> {
        let entry = Self::get_entry()?;
        match entry.delete_credential() {
            Ok(_) => Ok(()),
            Err(keyring::Error::Ambiguous(credentials)) => {
                // Delete all matching credentials to ensure cleanup.
                log::warn!(
                    "Keychain: ambiguous credential match on delete ({} items). Deleting all matches.",
                    credentials.len()
                );
                for cred in credentials {
                    if let Err(e) = cred.delete_credential() {
                        log::warn!("Keychain: failed to delete duplicate credential: {}", e);
                    }
                }
                Ok(())
            }
            Err(e) => Err(KeychainError::from(e)),
        }
    }

    /// Validate the API key format (basic validation)
    pub fn validate_format(api_key: &str) -> bool {
        // Claude API keys typically start with "sk-ant-" and have a specific length
        api_key.starts_with("sk-ant-") && api_key.len() > 20
    }
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
