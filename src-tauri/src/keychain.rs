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
