use crate::database::{Database, GlossaryEntryRow, TranslationRow};
use crate::keychain::Keychain;
use serde::Serialize;

#[allow(dead_code)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResult {
    pub success: bool,
    pub translated_text: Option<String>,
    pub detected_language: Option<String>,
    pub from_cache: bool,
    pub glossary_applied: Vec<String>,
    pub token_usage: Option<TokenUsage>,
    pub error: Option<TranslationError>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct TranslationError {
    pub code: String,
    pub message: String,
}

#[allow(dead_code)]
pub struct TranslationService {
    client: reqwest::Client,
}

impl TranslationService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Translate text using Claude API
    pub async fn translate(
        &self,
        db: &Database,
        text: &str,
        source_language: Option<&str>,
        target_language: Option<&str>,
        model: &str,
        cache_days: i32,
    ) -> TranslationResult {
        // Validation
        if text.is_empty() {
            return TranslationResult {
                success: false,
                translated_text: None,
                detected_language: None,
                from_cache: false,
                glossary_applied: vec![],
                token_usage: None,
                error: Some(TranslationError {
                    code: "EMPTY_TEXT".to_string(),
                    message: "Text cannot be empty".to_string(),
                }),
            };
        }

        if text.len() > 10000 {
            return TranslationResult {
                success: false,
                translated_text: None,
                detected_language: None,
                from_cache: false,
                glossary_applied: vec![],
                token_usage: None,
                error: Some(TranslationError {
                    code: "TEXT_TOO_LONG".to_string(),
                    message: "Text exceeds maximum length of 10000 characters".to_string(),
                }),
            };
        }

        // Get API key
        let api_key = match Keychain::get() {
            Ok(key) => key,
            Err(_) => {
                return TranslationResult {
                    success: false,
                    translated_text: None,
                    detected_language: None,
                    from_cache: false,
                    glossary_applied: vec![],
                    token_usage: None,
                    error: Some(TranslationError {
                        code: "INVALID_API_KEY".to_string(),
                        message: "API key not configured".to_string(),
                    }),
                };
            }
        };

        // Detect or use provided language
        let src_lang = source_language
            .map(String::from)
            .unwrap_or_else(|| detect_language(text));
        let tgt_lang = target_language.map(String::from).unwrap_or_else(|| {
            if src_lang == "ko" {
                "en".to_string()
            } else {
                "ko".to_string()
            }
        });

        // Check cache first
        if let Ok(Some(cached)) = db
            .find_cached_translation(text, &src_lang, &tgt_lang, cache_days)
            .await
        {
            return TranslationResult {
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
            };
        }

        // Find matching glossary entries (language-agnostic)
        let glossary_matches = db
            .find_glossary_matches(text)
            .await
            .unwrap_or_default();

        // Call Claude API
        match self
            .call_claude_api(&api_key, text, &src_lang, &tgt_lang, model, &glossary_matches)
            .await
        {
            Ok((translated_text, input_tokens, output_tokens)) => {
                let glossary_ids: Vec<String> =
                    glossary_matches.iter().map(|g| g.id.clone()).collect();

                // Cache the translation
                let translation = TranslationRow {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_text: text.to_string(),
                    translated_text: translated_text.clone(),
                    source_language: src_lang.clone(),
                    target_language: tgt_lang,
                    model: model.to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    glossary_used: Some(serde_json::to_string(&glossary_ids).unwrap()),
                    input_tokens: Some(input_tokens),
                    output_tokens: Some(output_tokens),
                };

                let _ = db.insert_translation(&translation).await;

                // Update glossary usage counts
                if !glossary_ids.is_empty() {
                    let _ = db.increment_glossary_usage(&glossary_ids).await;
                }

                TranslationResult {
                    success: true,
                    translated_text: Some(translated_text),
                    detected_language: Some(src_lang),
                    from_cache: false,
                    glossary_applied: glossary_ids,
                    token_usage: Some(TokenUsage {
                        input_tokens,
                        output_tokens,
                    }),
                    error: None,
                }
            }
            Err(e) => TranslationResult {
                success: false,
                translated_text: None,
                detected_language: Some(src_lang),
                from_cache: false,
                glossary_applied: vec![],
                token_usage: None,
                error: Some(e),
            },
        }
    }

    async fn call_claude_api(
        &self,
        api_key: &str,
        text: &str,
        source_language: &str,
        target_language: &str,
        model: &str,
        glossary_matches: &[GlossaryEntryRow],
    ) -> Result<(String, i32, i32), TranslationError> {
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

        let prompt = format!(
            "Translate the following text from {} to {}. Return only the translated text without any explanation or additional formatting.{}\n\nText to translate:\n{}",
            language_name(source_language),
            language_name(target_language),
            glossary_context,
            text
        );

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "max_tokens": 4096,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send()
            .await
            .map_err(|e| TranslationError {
                code: "NETWORK_ERROR".to_string(),
                message: format!("Network error: {}", e),
            })?;

        let status = response.status();

        if status.is_success() {
            let body: serde_json::Value = response.json().await.map_err(|e| TranslationError {
                code: "API_ERROR".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

            let translated_text = body["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string();

            let input_tokens = body["usage"]["input_tokens"].as_i64().unwrap_or(0) as i32;
            let output_tokens = body["usage"]["output_tokens"].as_i64().unwrap_or(0) as i32;

            Ok((translated_text, input_tokens, output_tokens))
        } else if status.as_u16() == 401 {
            Err(TranslationError {
                code: "INVALID_API_KEY".to_string(),
                message: "Invalid API key".to_string(),
            })
        } else if status.as_u16() == 429 {
            Err(TranslationError {
                code: "API_ERROR".to_string(),
                message: "Rate limit exceeded. Please try again later.".to_string(),
            })
        } else {
            Err(TranslationError {
                code: "API_ERROR".to_string(),
                message: format!("API error: {}", status),
            })
        }
    }
}

/// Detect language based on character analysis
#[allow(dead_code)]
pub fn detect_language(text: &str) -> String {
    let has_korean = text.chars().any(|c| {
        let code = c as u32;
        // Korean Unicode ranges
        (0xAC00..=0xD7AF).contains(&code) || // Hangul Syllables
        (0x1100..=0x11FF).contains(&code) || // Hangul Jamo
        (0x3130..=0x318F).contains(&code)    // Hangul Compatibility Jamo
    });

    if has_korean {
        "ko".to_string()
    } else {
        "en".to_string()
    }
}

#[allow(dead_code)]
fn language_name(code: &str) -> &str {
    match code {
        "ko" => "Korean",
        "en" => "English",
        _ => code,
    }
}

impl Default for TranslationService {
    fn default() -> Self {
        Self::new()
    }
}
