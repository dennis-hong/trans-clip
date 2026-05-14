use futures_util::StreamExt;
use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ChatStreamEvent};
use genai::resolver::{AuthData, Endpoint};
use genai::{Client, ModelIden, ServiceTarget};
use serde::{Deserialize, Serialize};

pub const ANTHROPIC_PROVIDER_ID: &str = "anthropic";
pub const OPENAI_PROVIDER_ID: &str = "openai";
pub const GOOGLE_PROVIDER_ID: &str = "google";
pub const CUSTOM_ENDPOINT_API_KEY_ACCOUNT: &str = "custom-endpoint";

pub const DEFAULT_ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com/v1";
pub const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_GOOGLE_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderKind {
    Anthropic,
    OpenAi,
    Google,
}

impl ProviderKind {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
            Self::Google => "google",
        }
    }

    pub fn from_db_value(value: &str) -> Option<Self> {
        match value {
            "anthropic" => Some(Self::Anthropic),
            "openai" => Some(Self::OpenAi),
            "google" => Some(Self::Google),
            _ => None,
        }
    }

    pub fn default_base_url(self) -> &'static str {
        match self {
            Self::Anthropic => DEFAULT_ANTHROPIC_BASE_URL,
            Self::OpenAi => DEFAULT_OPENAI_BASE_URL,
            Self::Google => DEFAULT_GOOGLE_BASE_URL,
        }
    }

    fn adapter_kind(self) -> AdapterKind {
        match self {
            Self::Anthropic => AdapterKind::Anthropic,
            Self::OpenAi => AdapterKind::OpenAIResp,
            Self::Google => AdapterKind::Gemini,
        }
    }

    pub fn default_api_interface(self) -> ApiInterface {
        match self {
            Self::Anthropic => ApiInterface::AnthropicMessages,
            Self::OpenAi => ApiInterface::OpenAiResponses,
            Self::Google => ApiInterface::GeminiGenerateContent,
        }
    }

    pub fn default_auth_scheme(self) -> AuthScheme {
        match self {
            Self::Anthropic => AuthScheme::XApiKey,
            Self::OpenAi => AuthScheme::Bearer,
            Self::Google => AuthScheme::XGoogApiKey,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EndpointMode {
    Public,
    Custom,
}

impl EndpointMode {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Custom => "custom",
        }
    }

    pub fn from_db_value(value: &str) -> Option<Self> {
        match value {
            "public" => Some(Self::Public),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApiInterface {
    AnthropicMessages,
    OpenAiResponses,
    GeminiGenerateContent,
}

impl ApiInterface {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::AnthropicMessages => "anthropic_messages",
            Self::OpenAiResponses => "openai_responses",
            Self::GeminiGenerateContent => "gemini_generate_content",
        }
    }

    pub fn from_db_value(value: &str) -> Option<Self> {
        match value {
            "anthropic_messages" => Some(Self::AnthropicMessages),
            "openai_responses" => Some(Self::OpenAiResponses),
            "gemini_generate_content" => Some(Self::GeminiGenerateContent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthScheme {
    Bearer,
    XApiKey,
    XGoogApiKey,
}

impl AuthScheme {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::Bearer => "bearer",
            Self::XApiKey => "x_api_key",
            Self::XGoogApiKey => "x_goog_api_key",
        }
    }

    pub fn from_db_value(value: &str) -> Option<Self> {
        match value {
            "bearer" => Some(Self::Bearer),
            "x_api_key" => Some(Self::XApiKey),
            "x_goog_api_key" => Some(Self::XGoogApiKey),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProviderConfig {
    pub id: String,
    pub display_name: String,
    pub provider_kind: ProviderKind,
    pub endpoint_mode: EndpointMode,
    pub base_url: String,
    pub auth_scheme: AuthScheme,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ModelProfile {
    pub id: String,
    pub provider_config_id: String,
    pub display_name: String,
    pub model_id: String,
    pub api_interface: ApiInterface,
    pub supports_streaming: bool,
    pub max_output_tokens: i32,
    pub sort_order: i32,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AiTextRequest {
    pub model_profile_id: Option<String>,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub max_output_tokens: i32,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AiTextResponse {
    pub text: String,
    pub provider_config_id: String,
    pub model_profile_id: String,
    pub model_id: String,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
}

#[derive(Debug, Clone)]
pub enum AiStreamEvent {
    Delta { text: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AiErrorCode {
    MissingApiKey,
    InvalidApiKey,
    InvalidEndpoint,
    UnsupportedModel,
    StreamParseError,
    NetworkError,
    ProviderError,
}

#[derive(Debug, Clone)]
pub struct AiError {
    pub code: AiErrorCode,
    pub message: String,
}

impl AiError {
    pub fn new(code: AiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub struct AiResolvedRequest {
    pub provider_config: ProviderConfig,
    pub model_profile: ModelProfile,
    pub api_key: String,
    pub request: AiTextRequest,
}

impl TryFrom<crate::database::AiProviderConfigRow> for ProviderConfig {
    type Error = String;

    fn try_from(row: crate::database::AiProviderConfigRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            display_name: row.display_name,
            provider_kind: ProviderKind::from_db_value(&row.provider_kind)
                .ok_or_else(|| "Invalid provider kind".to_string())?,
            endpoint_mode: EndpointMode::from_db_value(&row.endpoint_mode)
                .ok_or_else(|| "Invalid endpoint mode".to_string())?,
            base_url: row.base_url,
            auth_scheme: AuthScheme::from_db_value(&row.auth_scheme)
                .ok_or_else(|| "Invalid auth scheme".to_string())?,
            enabled: row.enabled != 0,
        })
    }
}

impl TryFrom<crate::database::AiModelProfileRow> for ModelProfile {
    type Error = String;

    fn try_from(row: crate::database::AiModelProfileRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            provider_config_id: row.provider_config_id,
            display_name: row.display_name,
            model_id: row.model_id,
            api_interface: ApiInterface::from_db_value(&row.api_interface)
                .ok_or_else(|| "Invalid API interface".to_string())?,
            supports_streaming: row.supports_streaming != 0,
            max_output_tokens: row.max_output_tokens,
            sort_order: row.sort_order,
        })
    }
}

pub struct AiRuntime {
    client: Client,
}

impl Default for AiRuntime {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .with_reqwest(crate::utils::streaming::anthropic_http_client().clone())
                .build(),
        }
    }
}

impl AiRuntime {
    pub async fn complete(&self, resolved: AiResolvedRequest) -> Result<AiTextResponse, AiError> {
        let options = chat_options(&resolved.request);
        let target = service_target(&resolved)?;
        let chat_req = chat_request(&resolved.request);
        let response = self
            .client
            .exec_chat(target, chat_req, Some(&options))
            .await
            .map_err(map_genai_error)?;

        let input_tokens = response.usage.prompt_tokens;
        let output_tokens = response.usage.completion_tokens;
        let text = response
            .first_text()
            .map(ToString::to_string)
            .ok_or_else(|| AiError::new(AiErrorCode::ProviderError, "missing response text"))?;

        Ok(AiTextResponse {
            text,
            provider_config_id: resolved.provider_config.id,
            model_profile_id: resolved.model_profile.id,
            model_id: resolved.model_profile.model_id,
            input_tokens,
            output_tokens,
        })
    }

    pub async fn stream(
        &self,
        resolved: AiResolvedRequest,
        mut on_event: impl FnMut(AiStreamEvent),
    ) -> Result<AiTextResponse, AiError> {
        if !resolved.model_profile.supports_streaming {
            return Err(AiError::new(
                AiErrorCode::UnsupportedModel,
                "selected model profile does not support streaming",
            ));
        }

        let options = chat_options(&resolved.request)
            .with_capture_content(true)
            .with_capture_usage(true);
        let target = service_target(&resolved)?;
        let chat_req = chat_request(&resolved.request);
        let response = self
            .client
            .exec_chat_stream(target, chat_req, Some(&options))
            .await
            .map_err(map_genai_error)?;

        let mut stream = response.stream;
        let mut full_text = String::new();
        let mut input_tokens = None;
        let mut output_tokens = None;

        while let Some(event) = stream.next().await {
            match event.map_err(map_genai_error)? {
                ChatStreamEvent::Chunk(chunk) => {
                    full_text.push_str(&chunk.content);
                    on_event(AiStreamEvent::Delta {
                        text: chunk.content,
                    });
                }
                ChatStreamEvent::End(end) => {
                    if let Some(usage) = &end.captured_usage {
                        input_tokens = usage.prompt_tokens;
                        output_tokens = usage.completion_tokens;
                    }
                    if full_text.is_empty() {
                        if let Some(text) = end.captured_first_text() {
                            full_text = text.to_string();
                        }
                    }
                }
                ChatStreamEvent::Start
                | ChatStreamEvent::ReasoningChunk(_)
                | ChatStreamEvent::ThoughtSignatureChunk(_)
                | ChatStreamEvent::ToolCallChunk(_) => {}
            }
        }

        if full_text.is_empty() {
            return Err(AiError::new(
                AiErrorCode::StreamParseError,
                "stream completed without text",
            ));
        }

        Ok(AiTextResponse {
            text: full_text,
            provider_config_id: resolved.provider_config.id,
            model_profile_id: resolved.model_profile.id,
            model_id: resolved.model_profile.model_id,
            input_tokens,
            output_tokens,
        })
    }

    #[allow(dead_code)]
    pub async fn validate_key(
        &self,
        provider_config: ProviderConfig,
        model_profile: ModelProfile,
        api_key: String,
    ) -> Result<bool, AiError> {
        let request = AiTextRequest {
            model_profile_id: Some(model_profile.id.clone()),
            system_prompt: None,
            user_prompt: "Reply with ok.".to_string(),
            max_output_tokens: 1,
            temperature: None,
        };

        let resolved = AiResolvedRequest {
            provider_config,
            model_profile,
            api_key,
            request,
        };

        match self.complete(resolved).await {
            Ok(_) => Ok(true),
            Err(err) if err.code == AiErrorCode::InvalidApiKey => Ok(false),
            Err(err) => Err(err),
        }
    }
}

fn chat_request(request: &AiTextRequest) -> ChatRequest {
    let mut chat_req = ChatRequest::new(vec![ChatMessage::user(request.user_prompt.clone())]);
    if let Some(system_prompt) = &request.system_prompt {
        chat_req = chat_req.with_system(system_prompt.clone());
    }
    chat_req
}

fn chat_options(request: &AiTextRequest) -> ChatOptions {
    let mut options =
        ChatOptions::default().with_max_tokens(request.max_output_tokens.max(1) as u32);
    if let Some(temperature) = request.temperature {
        options = options.with_temperature(temperature as f64);
    }
    options
}

fn service_target(resolved: &AiResolvedRequest) -> Result<ServiceTarget, AiError> {
    let base_url = genai_base_url(
        resolved.provider_config.provider_kind,
        &resolved.provider_config.base_url,
    )?;
    Ok(ServiceTarget {
        endpoint: Endpoint::from_owned(base_url),
        auth: AuthData::from_single(resolved.api_key.clone()),
        model: ModelIden::new(
            resolved.provider_config.provider_kind.adapter_kind(),
            resolved.model_profile.model_id.clone(),
        ),
    })
}

pub fn api_key_account_for_provider(provider_config: &ProviderConfig) -> String {
    if provider_config.endpoint_mode == EndpointMode::Custom {
        CUSTOM_ENDPOINT_API_KEY_ACCOUNT.to_string()
    } else {
        format!("provider:{}", provider_config.id)
    }
}

pub fn normalize_provider_base_url(
    provider_kind: ProviderKind,
    base_url: &str,
) -> Result<String, String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    let base = if trimmed.is_empty() {
        provider_kind.default_base_url().to_string()
    } else {
        let mut url =
            reqwest::Url::parse(trimmed).map_err(|_| "Endpoint URL must be valid".to_string())?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err("Endpoint URL must start with http:// or https://".to_string());
        }
        url.set_query(None);
        url.set_fragment(None);
        url.as_str().trim_end_matches('/').to_string()
    };

    match provider_kind {
        ProviderKind::Anthropic => normalize_versioned_base(
            &base,
            provider_kind.default_base_url(),
            &["/v1/messages"],
            "/v1",
        ),
        ProviderKind::OpenAi => normalize_versioned_base(
            &base,
            provider_kind.default_base_url(),
            &["/v1/responses", "/v1/chat/completions"],
            "/v1",
        ),
        ProviderKind::Google => normalize_google_base(&base),
    }
}

pub fn genai_base_url(provider_kind: ProviderKind, base_url: &str) -> Result<String, AiError> {
    let normalized = normalize_provider_base_url(provider_kind, base_url)
        .map_err(|err| AiError::new(AiErrorCode::InvalidEndpoint, err))?;
    Ok(format!("{}/", normalized.trim_end_matches('/')))
}

fn normalize_versioned_base(
    base: &str,
    default_base: &str,
    full_endpoint_suffixes: &[&str],
    version_suffix: &str,
) -> Result<String, String> {
    for suffix in full_endpoint_suffixes {
        if base.ends_with(suffix) {
            return Ok(base.trim_end_matches(suffix).to_string() + version_suffix);
        }
    }

    if base.ends_with(version_suffix) {
        return Ok(base.to_string());
    }

    if base == default_base.trim_end_matches(version_suffix) {
        return Ok(default_base.to_string());
    }

    Ok(format!("{base}{version_suffix}"))
}

fn normalize_google_base(base: &str) -> Result<String, String> {
    if let Some((prefix, _)) = base.split_once("/v1beta/models/") {
        return Ok(format!("{prefix}/v1beta"));
    }
    if base.ends_with("/v1beta") {
        return Ok(base.to_string());
    }
    if base == DEFAULT_GOOGLE_BASE_URL.trim_end_matches("/v1beta") {
        return Ok(DEFAULT_GOOGLE_BASE_URL.to_string());
    }
    Ok(format!("{base}/v1beta"))
}

fn map_genai_error(err: genai::Error) -> AiError {
    let message = err.to_string();
    let lower = message.to_ascii_lowercase();
    let code = if lower.contains("401")
        || lower.contains("unauthorized")
        || lower.contains("api key")
    {
        AiErrorCode::InvalidApiKey
    } else if lower.contains("endpoint") || lower.contains("url") {
        AiErrorCode::InvalidEndpoint
    } else if lower.contains("network") || lower.contains("connection") || lower.contains("timeout")
    {
        AiErrorCode::NetworkError
    } else {
        AiErrorCode::ProviderError
    };

    AiError::new(code, message)
}

#[cfg(test)]
mod tests {
    use super::{normalize_provider_base_url, ProviderKind};

    #[test]
    fn normalizes_provider_base_urls() {
        assert_eq!(
            normalize_provider_base_url(ProviderKind::Anthropic, "https://api.anthropic.com")
                .expect("anthropic root"),
            "https://api.anthropic.com/v1"
        );
        assert_eq!(
            normalize_provider_base_url(ProviderKind::Anthropic, "https://gw.example/v1/messages")
                .expect("anthropic full endpoint"),
            "https://gw.example/v1"
        );
        assert_eq!(
            normalize_provider_base_url(ProviderKind::OpenAi, "https://api.openai.com")
                .expect("openai root"),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            normalize_provider_base_url(
                ProviderKind::Google,
                "https://generativelanguage.googleapis.com"
            )
            .expect("google root"),
            "https://generativelanguage.googleapis.com/v1beta"
        );
        assert_eq!(
            normalize_provider_base_url(
                ProviderKind::Google,
                "https://gw.example/v1beta/models/gemini-2.5-flash:generateContent"
            )
            .expect("google full endpoint"),
            "https://gw.example/v1beta"
        );
    }
}
