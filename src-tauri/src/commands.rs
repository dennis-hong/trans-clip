use crate::database::{ClipboardItemRow, GlossaryEntryRow, TranslationRow, UserSettingsRow};
use crate::keychain;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

// ============================================
// Response Types
// ============================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateResponse {
    pub success: bool,
    pub translated_text: Option<String>,
    pub detected_language: Option<String>,
    pub from_cache: bool,
    pub glossary_applied: Vec<String>,
    pub token_usage: Option<TokenUsage>,
    pub error: Option<TranslateError>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

#[derive(Debug, Serialize)]
pub struct TranslateError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardHistoryResponse {
    pub items: Vec<ClipboardItemResponse>,
    pub total: i64,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItemResponse {
    pub id: String,
    pub content: String,
    pub content_preview: String,
    pub copied_at: String,
    pub source_app: Option<String>,
    pub is_pinned: bool,
    pub metadata: Option<ClipboardMetadata>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardMetadata {
    pub character_count: i32,
    pub word_count: i32,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinResponse {
    pub success: bool,
    pub is_pinned: bool,
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryListResponse {
    pub entries: Vec<GlossaryEntryResponse>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryEntryResponse {
    pub id: String,
    pub keyword: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub usage_count: i32,
}

#[derive(Debug, Serialize)]
pub struct ImportGlossaryResponse {
    pub imported: i32,
    pub skipped: i32,
    pub errors: Vec<ImportError>,
}

#[derive(Debug, Serialize)]
pub struct ImportError {
    pub line: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportGlossaryResponse {
    pub success: bool,
    pub exported_count: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSettingsResponse {
    pub max_history_count: i32,
    pub preferred_model: String,
    pub auto_detect_language: bool,
    pub double_press_interval: i32,
    pub translation_cache_days: i32,
    pub show_source_app: bool,
    pub popup_position: String,
    pub launch_at_login: bool,
    pub paste_delay_ms: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyStatus {
    pub exists: bool,
    pub is_valid: Option<bool>,
    pub last_validated: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetApiKeyResponse {
    pub success: bool,
    pub is_valid: bool,
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Serialize)]
pub struct PermissionStatus {
    pub granted: bool,
}

#[derive(Debug, Serialize)]
pub struct PasteResponse {
    pub success: bool,
    pub error: Option<ErrorDetail>,
}

// ============================================
// Request Types
// ============================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsRequest {
    pub max_history_count: Option<i32>,
    pub preferred_model: Option<String>,
    pub auto_detect_language: Option<bool>,
    pub double_press_interval: Option<i32>,
    pub translation_cache_days: Option<i32>,
    pub show_source_app: Option<bool>,
    pub popup_position: Option<String>,
    pub launch_at_login: Option<bool>,
    pub paste_delay_ms: Option<i32>,
}

// ============================================
// Helper functions
// ============================================

impl From<ClipboardItemRow> for ClipboardItemResponse {
    fn from(row: ClipboardItemRow) -> Self {
        Self {
            id: row.id,
            content: row.content,
            content_preview: row.content_preview,
            copied_at: row.copied_at,
            source_app: row.source_app,
            is_pinned: row.is_pinned != 0,
            metadata: match (row.character_count, row.word_count) {
                (Some(cc), Some(wc)) => Some(ClipboardMetadata {
                    character_count: cc,
                    word_count: wc,
                }),
                _ => None,
            },
        }
    }
}

impl From<GlossaryEntryRow> for GlossaryEntryResponse {
    fn from(row: GlossaryEntryRow) -> Self {
        Self {
            id: row.id,
            keyword: row.keyword,
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
            usage_count: row.usage_count,
        }
    }
}

impl From<UserSettingsRow> for UserSettingsResponse {
    fn from(row: UserSettingsRow) -> Self {
        Self {
            max_history_count: row.max_history_count,
            preferred_model: row.preferred_model,
            auto_detect_language: row.auto_detect_language != 0,
            double_press_interval: row.double_press_interval,
            translation_cache_days: row.translation_cache_days,
            show_source_app: row.show_source_app != 0,
            popup_position: row.popup_position,
            launch_at_login: row.launch_at_login != 0,
            paste_delay_ms: row.paste_delay_ms,
        }
    }
}

// ============================================
// Translation Commands
// ============================================

#[tauri::command]
pub async fn translate(
    state: State<'_, AppState>,
    text: String,
    source_language: Option<String>,
    target_language: Option<String>,
) -> Result<TranslateResponse, String> {
    // Validation
    if text.is_empty() {
        return Ok(TranslateResponse {
            success: false,
            translated_text: None,
            detected_language: None,
            from_cache: false,
            glossary_applied: vec![],
            token_usage: None,
            error: Some(TranslateError {
                code: "EMPTY_TEXT".to_string(),
                message: "Text cannot be empty".to_string(),
            }),
        });
    }

    if text.len() > 10000 {
        return Ok(TranslateResponse {
            success: false,
            translated_text: None,
            detected_language: None,
            from_cache: false,
            glossary_applied: vec![],
            token_usage: None,
            error: Some(TranslateError {
                code: "TEXT_TOO_LONG".to_string(),
                message: "Text exceeds maximum length of 10000 characters".to_string(),
            }),
        });
    }

    let db = state.db.lock().await;
    let settings = db.get_settings().await.map_err(|e| e.to_string())?;

    // Get API key from database
    let api_key = match &settings.api_key {
        Some(key) if !key.is_empty() => key.clone(),
        _ => {
            return Ok(TranslateResponse {
                success: false,
                translated_text: None,
                detected_language: None,
                from_cache: false,
                glossary_applied: vec![],
                token_usage: None,
                error: Some(TranslateError {
                    code: "INVALID_API_KEY".to_string(),
                    message: "API key not configured. Please set your Claude API key in Settings.".to_string(),
                }),
            });
        }
    };

    // Detect or use provided language
    let src_lang = source_language.unwrap_or_else(|| detect_language(&text));
    let tgt_lang = target_language.unwrap_or_else(|| {
        if src_lang == "ko" {
            "en".to_string()
        } else {
            "ko".to_string()
        }
    });

    // Check cache first
    if let Ok(Some(cached)) = db
        .find_cached_translation(&text, &src_lang, &tgt_lang, settings.translation_cache_days)
        .await
    {
        return Ok(TranslateResponse {
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
        });
    }

    // Find matching glossary entries (language-agnostic)
    let glossary_matches = db
        .find_glossary_matches(&text)
        .await
        .unwrap_or_default();

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

    // Call Claude API
    let client = reqwest::Client::new();
    let prompt = format!(
        "Translate the following text from {} to {}. Return only the translated text without any explanation.{}\n\nText to translate:\n{}",
        if src_lang == "ko" { "Korean" } else { "English" },
        if tgt_lang == "ko" { "Korean" } else { "English" },
        glossary_context,
        text
    );

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": settings.preferred_model,
            "max_tokens": 4096,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().is_success() {
                let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

                let translated_text = body["content"][0]["text"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let input_tokens = body["usage"]["input_tokens"].as_i64().map(|v| v as i32);
                let output_tokens = body["usage"]["output_tokens"].as_i64().map(|v| v as i32);

                let glossary_ids: Vec<String> =
                    glossary_matches.iter().map(|g| g.id.clone()).collect();

                // Cache the translation
                let translation = TranslationRow {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_text: text,
                    translated_text: translated_text.clone(),
                    source_language: src_lang.clone(),
                    target_language: tgt_lang,
                    model: settings.preferred_model,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    glossary_used: Some(serde_json::to_string(&glossary_ids).unwrap()),
                    input_tokens,
                    output_tokens,
                };

                let _ = db.insert_translation(&translation).await;

                // Update glossary usage counts
                if !glossary_ids.is_empty() {
                    let _ = db.increment_glossary_usage(&glossary_ids).await;
                }

                Ok(TranslateResponse {
                    success: true,
                    translated_text: Some(translated_text),
                    detected_language: Some(src_lang),
                    from_cache: false,
                    glossary_applied: glossary_ids,
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
                Ok(TranslateResponse {
                    success: false,
                    translated_text: None,
                    detected_language: None,
                    from_cache: false,
                    glossary_applied: vec![],
                    token_usage: None,
                    error: Some(TranslateError {
                        code: "INVALID_API_KEY".to_string(),
                        message: "Invalid API key".to_string(),
                    }),
                })
            } else {
                Ok(TranslateResponse {
                    success: false,
                    translated_text: None,
                    detected_language: None,
                    from_cache: false,
                    glossary_applied: vec![],
                    token_usage: None,
                    error: Some(TranslateError {
                        code: "API_ERROR".to_string(),
                        message: format!("API error: {}", res.status()),
                    }),
                })
            }
        }
        Err(e) => Ok(TranslateResponse {
            success: false,
            translated_text: None,
            detected_language: None,
            from_cache: false,
            glossary_applied: vec![],
            token_usage: None,
            error: Some(TranslateError {
                code: "NETWORK_ERROR".to_string(),
                message: format!("Network error: {}", e),
            }),
        }),
    }
}

fn detect_language(text: &str) -> String {
    // Detect based on Korean character ratio (>= 30% = Korean)
    let (korean, total) = text.chars().fold((0, 0), |(ko, tot), c| {
        let code = c as u32;
        let is_korean = (0xAC00..=0xD7AF).contains(&code)
            || (0x1100..=0x11FF).contains(&code)
            || (0x3130..=0x318F).contains(&code);

        if is_korean {
            (ko + 1, tot + 1)
        } else if c.is_alphabetic() {
            (ko, tot + 1)
        } else {
            (ko, tot)
        }
    });

    if total > 0 && (korean as f64 / total as f64) >= 0.3 {
        "ko".to_string()
    } else {
        "en".to_string()
    }
}

// ============================================
// Polish (Text Refinement) Commands
// ============================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolishResponse {
    pub success: bool,
    pub polished_text: Option<String>,
    pub detected_language: Option<String>,
    pub token_usage: Option<TokenUsage>,
    pub error: Option<TranslateError>,
}

#[tauri::command]
pub async fn polish(
    state: State<'_, AppState>,
    text: String,
    context: String,
    channel: String,
    options: Vec<String>,
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

    // Get API key
    let api_key = match &settings.api_key {
        Some(key) if !key.is_empty() => key.clone(),
        _ => {
            return Ok(PolishResponse {
                success: false,
                polished_text: None,
                detected_language: None,
                token_usage: None,
                error: Some(TranslateError {
                    code: "INVALID_API_KEY".to_string(),
                    message: "API key not configured. Please set your Claude API key in Settings.".to_string(),
                }),
            });
        }
    };

    // Detect language
    let detected_lang = detect_language(&text);

    // Build context description
    let context_desc = match context.as_str() {
        "report-to-superior" => "상사나 임원에게 보고하는 상황입니다. 존댓말을 사용하고, 핵심을 먼저 말하며, 결론-근거 순서로 명확하게 작성해주세요.",
        "team-announcement" => "팀원들에게 전달하는 공지 상황입니다. 친근하면서도 명확하게, 불릿포인트로 정리하고 행동 요청을 명시해주세요.",
        "peer-discussion" => "동료와 논의하는 상황입니다. 편하게 작성하되 논의 포인트를 정리하고 질문을 명확히 해주세요.",
        "external-formal" => "파트너사나 고객사 등 외부와 소통하는 상황입니다. 격식체로 정중하게, 배경-목적-요청 구조로 작성해주세요.",
        "documentation" => "기술 문서나 가이드를 작성하는 상황입니다. 객관적이고 3인칭으로, 단계별로 명료하게 설명해주세요.",
        _ => "일반적인 업무 커뮤니케이션 상황입니다.",
    };

    // Build channel description
    let channel_desc = match channel.as_str() {
        "slack-message" => "슬랙 메시지입니다. 짧고 간결하게, 한눈에 파악할 수 있게 작성해주세요. 적절한 이모지 사용 가능합니다.",
        "slack-thread" => "슬랙 스레드 답글입니다. 컨텍스트를 유지하면서 약간 더 상세하게 작성해주세요.",
        "confluence-wiki" => "컨플루언스 위키 문서입니다. 헤딩과 불릿으로 구조화하고, 완전한 문장으로 작성해주세요.",
        "jira-comment" => "Jira 이슈 코멘트입니다. 간결하게, 결론과 액션 중심으로 작성해주세요.",
        "jira-description" => "Jira 이슈 설명입니다. 배경-목표-상세-AC(수락 기준) 구조로 작성해주세요.",
        "email" => "업무 이메일입니다. 인사-본문-마무리 구조로, 요청사항을 명확히 해주세요.",
        "pr-description" => "GitHub/GitLab PR 설명입니다. What-Why-How 구조로 변경사항을 요약해주세요.",
        "code-review" => "코드 리뷰 코멘트입니다. 건설적으로, 구체적인 제안을 포함해주세요.",
        _ => "일반적인 텍스트 형식입니다.",
    };

    // Build options description
    let mut options_desc = String::new();
    for opt in &options {
        let opt_text = match opt.as_str() {
            "shorter" => "더 짧게: 핵심만 남기고 불필요한 부분을 제거해주세요.",
            "longer" => "더 자세하게: 부연 설명과 맥락을 추가해주세요.",
            "bullet" => "불릿으로 정리: 나열된 내용을 불릿포인트로 구조화해주세요.",
            "formal" => "더 격식있게: 톤을 높여 공식적으로 작성해주세요.",
            "casual" => "더 캐주얼하게: 톤을 낮춰 편하게 작성해주세요.",
            "action-clear" => "액션 명확히: 요청사항이나 다음 단계를 명확하게 표현해주세요.",
            _ => "",
        };
        if !opt_text.is_empty() {
            options_desc.push_str("\n- ");
            options_desc.push_str(opt_text);
        }
    }

    let lang_instruction = if detected_lang == "ko" {
        "한국어로 다듬어주세요."
    } else {
        "Refine in English."
    };

    let prompt = format!(
        r#"당신은 전문 에디터입니다. 사용자가 빠르게 작성한 러프한 초안을 깔끔하고 명료하게 정돈해주세요.

**중요**: 내용을 바꾸거나 새로운 정보를 추가하지 말고, 원본의 의미를 유지하면서 전달력이 좋게 다듬어주세요.

## 상황
{context_desc}

## 채널/매체
{channel_desc}
{options_section}
## 원문
{text}

## 지시사항
{lang_instruction}
다듬어진 결과만 출력하세요. 설명이나 추가 코멘트는 포함하지 마세요."#,
        context_desc = context_desc,
        channel_desc = channel_desc,
        options_section = if options_desc.is_empty() { String::new() } else { format!("\n## 추가 요청사항{}", options_desc) },
        text = text,
        lang_instruction = lang_instruction,
    );

    // Call Claude API
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": settings.preferred_model,
            "max_tokens": 4096,
            "messages": [{"role": "user", "content": prompt}]
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

// ============================================
// Clipboard Commands
// ============================================

#[tauri::command]
pub async fn get_clipboard_history(
    state: State<'_, AppState>,
    limit: Option<i32>,
    offset: Option<i32>,
    search_query: Option<String>,
) -> Result<ClipboardHistoryResponse, String> {
    let db = state.db.lock().await;
    let limit = limit.unwrap_or(50).min(200);
    let offset = offset.unwrap_or(0);

    let (items, total) = db
        .get_clipboard_history(limit, offset, search_query.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(ClipboardHistoryResponse {
        items: items.into_iter().map(ClipboardItemResponse::from).collect(),
        total,
        has_more: (offset + limit) < total as i32,
    })
}

#[tauri::command]
pub async fn delete_clipboard_item(
    state: State<'_, AppState>,
    id: String,
) -> Result<DeleteResponse, String> {
    let db = state.db.lock().await;
    let deleted = db
        .delete_clipboard_item(&id)
        .await
        .map_err(|e| e.to_string())?;

    if deleted {
        Ok(DeleteResponse {
            success: true,
            error: None,
        })
    } else {
        Ok(DeleteResponse {
            success: false,
            error: Some(ErrorDetail {
                code: "NOT_FOUND".to_string(),
                message: "Clipboard item not found".to_string(),
            }),
        })
    }
}

#[tauri::command]
pub async fn toggle_pin_clipboard_item(
    state: State<'_, AppState>,
    id: String,
) -> Result<PinResponse, String> {
    let db = state.db.lock().await;
    let result = db
        .toggle_pin_clipboard_item(&id)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(is_pinned) = result {
        Ok(PinResponse {
            success: true,
            is_pinned,
            error: None,
        })
    } else {
        Ok(PinResponse {
            success: false,
            is_pinned: false,
            error: Some(ErrorDetail {
                code: "NOT_FOUND".to_string(),
                message: "Clipboard item not found".to_string(),
            }),
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearHistoryResponse {
    pub success: bool,
    pub deleted_count: i64,
    pub error: Option<ErrorDetail>,
}

#[tauri::command]
pub async fn clear_clipboard_history(
    state: State<'_, AppState>,
) -> Result<ClearHistoryResponse, String> {
    let db = state.db.lock().await;
    let deleted_count = db
        .clear_all_clipboard_items()
        .await
        .map_err(|e| e.to_string())?;

    Ok(ClearHistoryResponse {
        success: true,
        deleted_count,
        error: None,
    })
}

#[tauri::command]
pub async fn set_clipboard(text: String) -> Result<(), String> {
    // Use macOS pasteboard
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let mut child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }

        child.wait().map_err(|e| e.to_string())?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("Clipboard operations only supported on macOS".to_string())
    }
}

#[tauri::command]
pub async fn paste_text(
    state: State<'_, AppState>,
    text: String,
) -> Result<PasteResponse, String> {
    log::info!("paste_text: Setting clipboard with text: '{}'", &text[..text.len().min(50)]);
    
    // Get paste delay from settings
    let paste_delay_ms = {
        let db = state.db.lock().await;
        db.get_settings().await.map(|s| s.paste_delay_ms).unwrap_or(150)
    };
    let delay_seconds = paste_delay_ms as f64 / 1000.0;
    log::info!("paste_text: Using paste delay: {}ms ({:.3}s)", paste_delay_ms, delay_seconds);
    
    // First set the clipboard
    set_clipboard(text.clone()).await?;
    log::info!("paste_text: Clipboard set successfully");

    // Verify clipboard content
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("pbpaste").output() {
            let clipboard_content = String::from_utf8_lossy(&output.stdout);
            log::info!("paste_text: Clipboard verification: '{}'", &clipboard_content[..clipboard_content.len().min(50)]);
            if clipboard_content != text {
                log::warn!("paste_text: Clipboard content mismatch!");
            }
        }
    }

    // Longer delay to ensure clipboard is ready
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Then switch back to previous app and paste
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        log::info!("paste_text: Executing AppleScript to activate previous app and paste");
        
        // Use AppleScript to:
        // 1. Use Cmd+Tab to switch to previous app
        // 2. Wait for the app to be activated
        // 3. Paste using key code (more reliable than keystroke)
        let script = format!(r#"
            tell application "System Events"
                -- Switch to previous app using Cmd+Tab
                key code 48 using command down
                
                -- Wait for app switch to complete
                delay {:.3}
                
                -- Send Cmd+V using key code 9 (v key) - more reliable than keystroke
                key code 9 using command down
            end tell
        "#, delay_seconds);
        
        let result = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output();

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                log::info!("paste_text: AppleScript exit status: {}", output.status);
                if !stdout.is_empty() {
                    log::info!("paste_text: stdout: {}", stdout);
                }
                if !stderr.is_empty() {
                    log::warn!("paste_text: stderr: {}", stderr);
                }
                
                if output.status.success() {
                    log::info!("paste_text: Success!");
                    Ok(PasteResponse {
                        success: true,
                        error: None,
                    })
                } else {
                    log::error!("paste_text: AppleScript failed with status {}", output.status);
                    Ok(PasteResponse {
                        success: false,
                        error: Some(ErrorDetail {
                            code: "ACCESSIBILITY_DENIED".to_string(),
                            message: format!("Accessibility permission required for paste. stderr: {}", stderr),
                        }),
                    })
                }
            }
            Err(e) => {
                log::error!("paste_text: Failed to execute osascript: {}", e);
                Ok(PasteResponse {
                    success: false,
                    error: Some(ErrorDetail {
                        code: "PASTE_FAILED".to_string(),
                        message: format!("Failed to execute paste: {}", e),
                    }),
                })
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(PasteResponse {
            success: false,
            error: Some(ErrorDetail {
                code: "PASTE_FAILED".to_string(),
                message: "Paste operations only supported on macOS".to_string(),
            }),
        })
    }
}

// ============================================
// Glossary Commands
// ============================================

#[tauri::command]
pub async fn get_glossary_entries(
    state: State<'_, AppState>,
    search_query: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
) -> Result<GlossaryListResponse, String> {
    let db = state.db.lock().await;
    let sort_by = sort_by.unwrap_or_else(|| "keyword".to_string());
    let sort_order = sort_order.unwrap_or_else(|| "asc".to_string());

    let entries = db
        .get_glossary_entries(
            search_query.as_deref(),
            &sort_by,
            &sort_order,
        )
        .await
        .map_err(|e| e.to_string())?;

    let total = entries.len() as i64;

    Ok(GlossaryListResponse {
        entries: entries
            .into_iter()
            .map(GlossaryEntryResponse::from)
            .collect(),
        total,
    })
}

#[tauri::command]
pub async fn add_glossary_entry(
    state: State<'_, AppState>,
    keyword: String,
    description: String,
) -> Result<GlossaryEntryResponse, String> {
    // Validation
    if keyword.is_empty() || keyword.len() > 100 {
        return Err("Keyword must be 1-100 characters".to_string());
    }

    if description.is_empty() || description.len() > 500 {
        return Err("Description must be 1-500 characters".to_string());
    }

    let now = chrono::Utc::now().to_rfc3339();
    let entry = GlossaryEntryRow {
        id: uuid::Uuid::new_v4().to_string(),
        keyword,
        description,
        created_at: now.clone(),
        updated_at: now,
        usage_count: 0,
    };

    let db = state.db.lock().await;
    db.insert_glossary_entry(&entry)
        .await
        .map_err(|e| e.to_string())?;

    Ok(GlossaryEntryResponse::from(entry))
}

#[tauri::command]
pub async fn update_glossary_entry(
    state: State<'_, AppState>,
    id: String,
    keyword: Option<String>,
    description: Option<String>,
) -> Result<GlossaryEntryResponse, String> {
    // Validation
    if let Some(ref kw) = keyword {
        if kw.is_empty() || kw.len() > 100 {
            return Err("Keyword must be 1-100 characters".to_string());
        }
    }
    if let Some(ref desc) = description {
        if desc.is_empty() || desc.len() > 500 {
            return Err("Description must be 1-500 characters".to_string());
        }
    }

    let db = state.db.lock().await;
    let entry = db
        .update_glossary_entry(&id, keyword.as_deref(), description.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    entry
        .map(GlossaryEntryResponse::from)
        .ok_or_else(|| "Glossary entry not found".to_string())
}

#[tauri::command]
pub async fn delete_glossary_entry(
    state: State<'_, AppState>,
    id: String,
) -> Result<DeleteResponse, String> {
    let db = state.db.lock().await;
    let deleted = db
        .delete_glossary_entry(&id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(DeleteResponse {
        success: deleted,
        error: if deleted {
            None
        } else {
            Some(ErrorDetail {
                code: "NOT_FOUND".to_string(),
                message: "Glossary entry not found".to_string(),
            })
        },
    })
}

#[tauri::command]
pub async fn import_glossary(
    _state: State<'_, AppState>,
    _file_path: String,
    _format: String,
    _overwrite: bool,
) -> Result<ImportGlossaryResponse, String> {
    // TODO: Implement CSV/JSON import
    Ok(ImportGlossaryResponse {
        imported: 0,
        skipped: 0,
        errors: vec![ImportError {
            line: 0,
            message: "Import not yet implemented".to_string(),
        }],
    })
}

#[tauri::command]
pub async fn export_glossary(
    _state: State<'_, AppState>,
    _file_path: String,
    _format: String,
) -> Result<ExportGlossaryResponse, String> {
    // TODO: Implement CSV/JSON export
    Ok(ExportGlossaryResponse {
        success: false,
        exported_count: 0,
    })
}

// ============================================
// Settings Commands
// ============================================

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

// ============================================
// System Commands
// ============================================

#[tauri::command]
pub async fn check_accessibility_permission() -> Result<PermissionStatus, String> {
    let granted = crate::hotkey::check_accessibility_permission();
    Ok(PermissionStatus { granted })
}

#[tauri::command]
pub async fn request_accessibility_permission() -> Result<(), String> {
    crate::hotkey::request_accessibility_permission();
    Ok(())
}

#[tauri::command]
pub async fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map_err(|e| format!("Failed to open settings: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn show_translation_popup(
    app: tauri::AppHandle,
    _text: String,
    _position: Option<Position>,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        crate::hotkey::show_window_at_position(&window);
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_translation_popup(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

// ============================================
// Window Management Commands
// ============================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub name: Option<String>,
    pub position_x: i32,
    pub position_y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub is_primary: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub async fn get_monitors(app: tauri::AppHandle) -> Result<Vec<MonitorInfo>, String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let primary = app.primary_monitor().map_err(|e| e.to_string())?;
    
    let mut result = Vec::new();
    for monitor in monitors.iter() {
        let pos = monitor.position();
        let size = monitor.size();
        let is_primary = primary.as_ref().map_or(false, |p| {
            p.position() == monitor.position() && p.size() == monitor.size()
        });
        
        result.push(MonitorInfo {
            name: monitor.name().map(|s| s.to_string()),
            position_x: pos.x,
            position_y: pos.y,
            width: size.width,
            height: size.height,
            scale_factor: monitor.scale_factor(),
            is_primary,
        });
    }
    
    // Sort monitors by physical position (left to right)
    // This ensures button 1 = leftmost monitor, button 2 = next monitor, etc.
    result.sort_by(|a, b| {
        // Primary sort by x position (left to right)
        // Secondary sort by y position (top to bottom) for vertically stacked monitors
        match a.position_x.cmp(&b.position_x) {
            std::cmp::Ordering::Equal => a.position_y.cmp(&b.position_y),
            other => other,
        }
    });
    
    Ok(result)
}

#[tauri::command]
pub async fn get_window_position(app: tauri::AppHandle) -> Result<WindowPosition, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    
    Ok(WindowPosition {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
    })
}

#[tauri::command]
pub async fn set_window_position(
    app: tauri::AppHandle,
    x: i32,
    y: i32,
) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_window_size(
    app: tauri::AppHandle,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    window.set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn move_to_monitor(
    app: tauri::AppHandle,
    monitor_index: usize,
    anchor: String,
) -> Result<(), String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    
    log::info!("move_to_monitor: requested index={}, anchor={}", monitor_index, anchor);
    
    // Sort monitors by physical position (left to right) to match UI buttons
    let mut sorted_monitors: Vec<_> = monitors.iter().collect();
    sorted_monitors.sort_by(|a, b| {
        let pos_a = a.position();
        let pos_b = b.position();
        match pos_a.x.cmp(&pos_b.x) {
            std::cmp::Ordering::Equal => pos_a.y.cmp(&pos_b.y),
            other => other,
        }
    });
    
    if monitor_index >= sorted_monitors.len() {
        log::error!("Invalid monitor index: {} >= {}", monitor_index, sorted_monitors.len());
        return Err("Invalid monitor index".to_string());
    }
    
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    
    // Get target monitor info
    let target_monitor = sorted_monitors[monitor_index];
    let mon_pos = target_monitor.position();
    let mon_size = target_monitor.size();
    let target_scale = target_monitor.scale_factor();
    
    // Convert target monitor size to logical
    let mon_logical_width = (mon_size.width as f64 / target_scale) as i32;
    let mon_logical_height = (mon_size.height as f64 / target_scale) as i32;
    
    // Get current window scale
    let current_scale = window.scale_factor().map_err(|e| e.to_string())?;
    
    // Check if we're moving between monitors with different scale factors
    let scale_differs = (current_scale - target_scale).abs() > 0.01;
    
    if scale_differs {
        // Two-phase move: first move to target monitor center to update scale factor
        let temp_x = mon_pos.x + mon_logical_width / 2;
        let temp_y = mon_pos.y + mon_logical_height / 2;
        
        log::info!("Scale differs ({} -> {}), moving to target monitor first", current_scale, target_scale);
        
        window.set_position(tauri::Position::Logical(tauri::LogicalPosition { 
            x: temp_x as f64, 
            y: temp_y as f64 
        })).map_err(|e| e.to_string())?;
        
        // Small delay to let the window update its scale factor
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    
    // Now get the accurate window size (after scale factor update if needed)
    let win_size = window.outer_size().map_err(|e| e.to_string())?;
    let win_scale = window.scale_factor().map_err(|e| e.to_string())?;
    let win_logical_width = (win_size.width as f64 / win_scale) as i32;
    let win_logical_height = (win_size.height as f64 / win_scale) as i32;
    
    log::info!("Window scale: {}, logical size: {}x{}", win_scale, win_logical_width, win_logical_height);
    log::info!("Target monitor[{}]: pos=({}, {}), logical size={}x{}, scale={}", 
        monitor_index, mon_pos.x, mon_pos.y, mon_logical_width, mon_logical_height, target_scale);
    
    // Calculate final position based on anchor using logical coordinates
    let (x, y) = match anchor.as_str() {
        "bottom" => {
            let x = mon_pos.x + (mon_logical_width - win_logical_width) / 2;
            let y = mon_pos.y + mon_logical_height - win_logical_height;
            (x, y)
        }
        "top" => {
            let x = mon_pos.x + (mon_logical_width - win_logical_width) / 2;
            let y = mon_pos.y;
            (x, y)
        }
        "center" => {
            let x = mon_pos.x + (mon_logical_width - win_logical_width) / 2;
            let y = mon_pos.y + (mon_logical_height - win_logical_height) / 2;
            (x, y)
        }
        _ => {
            let x = mon_pos.x + (mon_logical_width - win_logical_width) / 2;
            let y = mon_pos.y + mon_logical_height - win_logical_height;
            (x, y)
        }
    };
    
    log::info!("Setting final window position (logical) to: ({}, {})", x, y);
    
    window.set_position(tauri::Position::Logical(tauri::LogicalPosition { 
        x: x as f64, 
        y: y as f64 
    })).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn get_current_monitor_index(app: tauri::AppHandle) -> Result<usize, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;
    
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    
    // Sort monitors by physical position (left to right) to match UI buttons
    let mut sorted_monitors: Vec<_> = monitors.iter().enumerate().collect();
    sorted_monitors.sort_by(|(_, a), (_, b)| {
        let pos_a = a.position();
        let pos_b = b.position();
        match pos_a.x.cmp(&pos_b.x) {
            std::cmp::Ordering::Equal => pos_a.y.cmp(&pos_b.y),
            other => other,
        }
    });
    
    // Find which monitor the window center is on
    let win_center_x = win_pos.x + win_size.width as i32 / 2;
    let win_center_y = win_pos.y + win_size.height as i32 / 2;
    
    for (sorted_index, (_, monitor)) in sorted_monitors.iter().enumerate() {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        
        if win_center_x >= mon_pos.x && win_center_x < mon_pos.x + mon_size.width as i32 &&
           win_center_y >= mon_pos.y && win_center_y < mon_pos.y + mon_size.height as i32 {
            return Ok(sorted_index);
        }
    }
    
    // Default to first monitor if not found
    Ok(0)
}

#[tauri::command]
pub async fn toggle_always_on_top(app: tauri::AppHandle) -> Result<bool, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let current = window.is_always_on_top().map_err(|e| e.to_string())?;
    window.set_always_on_top(!current).map_err(|e| e.to_string())?;
    Ok(!current)
}

#[tauri::command]
pub async fn snap_to_bottom(app: tauri::AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;
    
    // Find which monitor the window is on
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let mut target_monitor = monitors.first().ok_or("No monitors found")?;
    
    for monitor in monitors.iter() {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        let scale = monitor.scale_factor();
        
        // Convert to logical coordinates for comparison
        let mon_logical_width = (mon_size.width as f64 / scale) as i32;
        let mon_logical_height = (mon_size.height as f64 / scale) as i32;
        
        // Check if window center is within this monitor (using logical coordinates)
        // win_pos is already in physical pixels, convert to logical
        let win_logical_x = (win_pos.x as f64 / scale) as i32;
        let win_logical_y = (win_pos.y as f64 / scale) as i32;
        let win_logical_width = (win_size.width as f64 / scale) as i32;
        let win_logical_height = (win_size.height as f64 / scale) as i32;
        
        let win_center_x = win_logical_x + win_logical_width / 2;
        let win_center_y = win_logical_y + win_logical_height / 2;
        
        if win_center_x >= mon_pos.x && win_center_x < mon_pos.x + mon_logical_width &&
           win_center_y >= mon_pos.y && win_center_y < mon_pos.y + mon_logical_height {
            target_monitor = monitor;
            break;
        }
    }
    
    let mon_pos = target_monitor.position();
    let mon_size = target_monitor.size();
    let scale = target_monitor.scale_factor();
    
    // Convert to logical coordinates
    let mon_logical_height = (mon_size.height as f64 / scale) as i32;
    let win_logical_x = (win_pos.x as f64 / scale) as i32;
    let win_logical_height = (win_size.height as f64 / scale) as i32;
    
    // Keep x position, snap y to bottom (using logical coordinates)
    let new_y = mon_pos.y + mon_logical_height - win_logical_height;
    
    window.set_position(tauri::Position::Logical(tauri::LogicalPosition { 
        x: win_logical_x as f64, 
        y: new_y as f64 
    })).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn set_drawer_collapsed(
    app: tauri::AppHandle,
    collapsed: bool,
) -> Result<(), String> {
    // Legacy wrapper for backwards compatibility
    let mode = if collapsed { "collapsed" } else { "expanded" };
    set_drawer_mode(app, mode.to_string()).await
}

#[tauri::command]
pub async fn set_drawer_mode(
    app: tauri::AppHandle,
    mode: String,
) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;

    // Find which monitor the window is on using physical coordinates
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let mut target_monitor = monitors.first().ok_or("No monitors found")?;

    let win_center_x = win_pos.x + win_size.width as i32 / 2;
    let win_center_y = win_pos.y + win_size.height as i32 / 2;

    for monitor in monitors.iter() {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();

        // Check if window center is within this monitor (physical coordinates)
        if win_center_x >= mon_pos.x && win_center_x < mon_pos.x + mon_size.width as i32 &&
           win_center_y >= mon_pos.y && win_center_y < mon_pos.y + mon_size.height as i32 {
            target_monitor = monitor;
            break;
        }
    }

    let mon_pos = target_monitor.position();
    let mon_size = target_monitor.size();
    let scale = target_monitor.scale_factor();

    // Convert monitor size to logical coordinates
    let mon_logical_width = (mon_size.width as f64 / scale) as i32;
    let mon_logical_height = (mon_size.height as f64 / scale) as i32;

    // Set new size based on mode (in logical pixels)
    // Width: 1200 for all modes (default app width)
    // Height: varies by mode
    let (new_logical_width, new_logical_height) = match mode.as_str() {
        "collapsed" => (1200, 48),    // Just header
        "expanded" => (1200, 200),    // History view
        "full" => (1200, 450),        // Settings/Glossary view
        "popup" => (1200, 350),       // Translation/Polish popup view
        _ => (1200, 200),
    };

    // Set new size using logical coordinates
    window.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: new_logical_width as f64,
        height: new_logical_height as f64
    })).map_err(|e| e.to_string())?;

    // Position at bottom center of the current monitor
    let new_x = mon_pos.x + (mon_logical_width - new_logical_width) / 2;
    let new_y = mon_pos.y + mon_logical_height - new_logical_height;

    window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
        x: new_x as f64,
        y: new_y as f64
    })).map_err(|e| e.to_string())?;

    log::info!("set_drawer_mode: mode={}, size={}x{}, pos=({}, {})", 
        mode, new_logical_width, new_logical_height, new_x, new_y);

    Ok(())
}
