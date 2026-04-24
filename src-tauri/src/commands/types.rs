use crate::database::{ClipboardItemRow, GlossaryEntryRow, UserSettingsRow};
use serde::{Deserialize, Serialize};

// ============================================
// Common Types
// ============================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub error: Option<ErrorDetail>,
}

// ============================================
// Translation Types
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
pub struct TranslateError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum TranslateStreamEvent {
    Started {
        detected_language: Option<String>,
        from_cache: bool,
        glossary_applied: Vec<String>,
    },
    Delta {
        text: String,
    },
    Completed {
        full_text: String,
        token_usage: Option<TokenUsage>,
    },
    Error {
        code: String,
        message: String,
    },
}

// ============================================
// Polish Types
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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum PolishStreamEvent {
    Started {
        detected_language: Option<String>,
    },
    Delta {
        text: String,
    },
    Completed {
        full_text: String,
        token_usage: Option<TokenUsage>,
    },
    Error {
        code: String,
        message: String,
    },
}

// ============================================
// Clipboard Types
// ============================================

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
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardMetadata {
    pub character_count: i32,
    pub word_count: i32,
}

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
            updated_at: row.updated_at,
        }
    }
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
pub struct ClearHistoryResponse {
    pub success: bool,
    pub deleted_count: i64,
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Serialize)]
pub struct PasteResponse {
    pub success: bool,
    pub error: Option<ErrorDetail>,
}

// ============================================
// Glossary Types
// ============================================

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

// ============================================
// Settings Types
// ============================================

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
    pub anthropic_base_url: String,
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
            anthropic_base_url: row.anthropic_base_url,
        }
    }
}

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
    pub anthropic_base_url: Option<String>,
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

// ============================================
// System Types
// ============================================

#[derive(Debug, Serialize)]
pub struct PermissionStatus {
    pub granted: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

// ============================================
// Window Types
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

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SnapEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapResult {
    pub snapped: bool,
    pub edges: Vec<SnapEdge>,
    pub position: WindowPosition,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentMonitorInfo {
    pub monitor_key: String,
    pub monitor_index: usize,
    pub monitor_width: i32,
    pub saved_window_width: Option<i32>,
}
