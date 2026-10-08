//! Direct Anthropic Messages API client for the latest Claude models.
//!
//! `genai` does not know how to tune thinking/effort for the Claude 5.5 generation
//! (it would fall back to the legacy `thinking.enabled` mode, which those models
//! reject). Without tuning, Sonnet 5.5 thinks up front at `high` effort and Opus 5.5
//! always thinks, which delays the first translated token by seconds. Requests for
//! these models therefore go through this module instead, with latency-friendly
//! `thinking` / `output_config.effort` values.

use futures_util::StreamExt;
use serde_json::{json, Value};

use super::{
    normalize_provider_base_url, stop_reason_error, AiError, AiErrorCode, AiResolvedRequest,
    AiStreamEvent, AiTextResponse, ProviderKind,
};

const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Per-model request tuning that keeps short text tasks (translate/polish) responsive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatencyTuning {
    /// Value for `thinking.type`, or `None` to omit the field (thinking cannot be disabled).
    pub thinking: Option<&'static str>,
    /// Value for `output_config.effort`.
    pub effort: &'static str,
}

/// Returns tuning for models that must use the direct client, `None` for everything else.
pub fn latency_tuning(provider_kind: ProviderKind, model_id: &str) -> Option<LatencyTuning> {
    if provider_kind != ProviderKind::Anthropic {
        return None;
    }
    if model_id.starts_with("claude-sonnet-5-5") {
        // `between_tools` turns off up-front thinking (`disabled` is rejected on 5.5).
        Some(LatencyTuning {
            thinking: Some("between_tools"),
            effort: "medium",
        })
    } else if model_id.starts_with("claude-haiku-5-5") {
        // Haiku 5.5 is the first Haiku with effort control; keep it at the lowest level.
        Some(LatencyTuning {
            thinking: None,
            effort: "low",
        })
    } else if model_id.starts_with("claude-opus-5-5") {
        // Adaptive thinking is always on for Opus 5.5; effort is the only control.
        Some(LatencyTuning {
            thinking: None,
            effort: "low",
        })
    } else {
        None
    }
}

fn messages_url(base_url: &str) -> Result<String, AiError> {
    let normalized = normalize_provider_base_url(ProviderKind::Anthropic, base_url)
        .map_err(|err| AiError::new(AiErrorCode::InvalidEndpoint, err))?;
    Ok(format!("{}/messages", normalized.trim_end_matches('/')))
}

pub fn request_body(resolved: &AiResolvedRequest, tuning: LatencyTuning, stream: bool) -> Value {
    let request = &resolved.request;
    let mut body = json!({
        "model": resolved.model_profile.model_id,
        "max_tokens": request.max_output_tokens.max(1),
        "stream": stream,
        "messages": [{ "role": "user", "content": request.user_prompt }],
        "output_config": { "effort": tuning.effort },
    });
    if let Some(system) = &request.system_prompt {
        body["system"] = json!(system);
    }
    if let Some(thinking) = tuning.thinking {
        body["thinking"] = json!({ "type": thinking });
    }
    body
}

fn map_reqwest_error(err: reqwest::Error) -> AiError {
    AiError::new(AiErrorCode::NetworkError, format!("Network error: {err}"))
}

fn map_http_error(status: u16, body: &str) -> AiError {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value["error"]["message"].as_str().map(ToString::to_string))
        .unwrap_or_else(|| body.chars().take(500).collect());
    let code = if matches!(status, 401 | 403) {
        AiErrorCode::InvalidApiKey
    } else {
        AiErrorCode::ProviderError
    };
    AiError::new(code, format!("Anthropic API error ({status}): {message}"))
}

async fn send(
    client: &reqwest::Client,
    resolved: &AiResolvedRequest,
    body: &Value,
) -> Result<reqwest::Response, AiError> {
    let url = messages_url(&resolved.provider_config.base_url)?;
    let response = client
        .post(url)
        .header("x-api-key", &resolved.api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(map_reqwest_error)?;

    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status().as_u16();
    let text = response.text().await.unwrap_or_default();
    Err(map_http_error(status, &text))
}

fn into_response(
    resolved: AiResolvedRequest,
    text: String,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    stop_reason: Option<&str>,
) -> Result<AiTextResponse, AiError> {
    if let Some(err) = stop_reason.and_then(stop_reason_error) {
        return Err(err);
    }
    if text.is_empty() {
        return Err(AiError::new(
            AiErrorCode::StreamParseError,
            "response completed without text",
        ));
    }
    Ok(AiTextResponse {
        text,
        provider_config_id: resolved.provider_config.id,
        model_profile_id: resolved.model_profile.id,
        model_id: resolved.model_profile.model_id,
        input_tokens,
        output_tokens,
    })
}

pub async fn complete(
    client: &reqwest::Client,
    resolved: AiResolvedRequest,
    tuning: LatencyTuning,
) -> Result<AiTextResponse, AiError> {
    let body = request_body(&resolved, tuning, false);
    let response = send(client, &resolved, &body).await?;
    let value: Value = response.json().await.map_err(map_reqwest_error)?;

    let text: String = value["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter(|block| block["type"] == "text")
                .filter_map(|block| block["text"].as_str())
                .collect()
        })
        .unwrap_or_default();
    let input_tokens = value["usage"]["input_tokens"].as_i64().map(|v| v as i32);
    let output_tokens = value["usage"]["output_tokens"].as_i64().map(|v| v as i32);
    let stop_reason = value["stop_reason"].as_str();

    into_response(resolved, text, input_tokens, output_tokens, stop_reason)
}

#[derive(Default)]
struct StreamState {
    text: String,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    stop_reason: Option<String>,
}

/// Applies one SSE `data:` payload. Returns the text delta, if any.
fn apply_event(state: &mut StreamState, data: &str) -> Result<Option<String>, AiError> {
    let Ok(event) = serde_json::from_str::<Value>(data) else {
        return Ok(None);
    };

    match event["type"].as_str() {
        Some("content_block_delta") if event["delta"]["type"] == "text_delta" => {
            if let Some(delta) = event["delta"]["text"].as_str() {
                state.text.push_str(delta);
                return Ok(Some(delta.to_string()));
            }
        }
        Some("message_start") => {
            state.input_tokens = event["message"]["usage"]["input_tokens"]
                .as_i64()
                .map(|v| v as i32);
        }
        Some("message_delta") => {
            if let Some(reason) = event["delta"]["stop_reason"].as_str() {
                state.stop_reason = Some(reason.to_string());
            }
            if let Some(tokens) = event["usage"]["output_tokens"].as_i64() {
                state.output_tokens = Some(tokens as i32);
            }
        }
        Some("error") => {
            let message = event["error"]["message"]
                .as_str()
                .unwrap_or("unknown stream error");
            return Err(AiError::new(
                AiErrorCode::ProviderError,
                format!("Anthropic stream error: {message}"),
            ));
        }
        _ => {}
    }
    Ok(None)
}

/// Handles one raw SSE line (without regard to UTF-8 boundaries of the network chunks).
fn apply_line(
    state: &mut StreamState,
    raw_line: &[u8],
    on_event: &mut impl FnMut(AiStreamEvent),
) -> Result<(), AiError> {
    let line = String::from_utf8_lossy(raw_line);
    let Some(data) = line.trim().strip_prefix("data:") else {
        return Ok(());
    };
    if let Some(text) = apply_event(state, data.trim_start())? {
        on_event(AiStreamEvent::Delta { text });
    }
    Ok(())
}

pub async fn stream(
    client: &reqwest::Client,
    resolved: AiResolvedRequest,
    tuning: LatencyTuning,
    mut on_event: impl FnMut(AiStreamEvent),
) -> Result<AiTextResponse, AiError> {
    let body = request_body(&resolved, tuning, true);
    let response = send(client, &resolved, &body).await?;

    let mut state = StreamState::default();
    // Buffer raw bytes and only decode complete lines: a network chunk can end in the
    // middle of a multi-byte character (e.g. Korean text).
    let mut buffer: Vec<u8> = Vec::new();
    let mut bytes = response.bytes_stream();

    while let Some(chunk) = bytes.next().await {
        buffer.extend_from_slice(&chunk.map_err(map_reqwest_error)?);
        while let Some(pos) = buffer.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = buffer.drain(..=pos).collect();
            apply_line(&mut state, &line, &mut on_event)?;
        }
    }
    if !buffer.is_empty() {
        apply_line(&mut state, &buffer, &mut on_event)?;
    }

    let StreamState {
        text,
        input_tokens,
        output_tokens,
        stop_reason,
    } = state;
    into_response(
        resolved,
        text,
        input_tokens,
        output_tokens,
        stop_reason.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::test_support::{http_response, one_shot_server, resolved_request};

    fn resolved(model_id: &str, base_url: &str) -> AiResolvedRequest {
        resolved_request(ProviderKind::Anthropic, model_id, base_url)
    }

    #[test]
    fn tunes_only_claude_5_5_models() {
        let sonnet = latency_tuning(ProviderKind::Anthropic, "claude-sonnet-5-5").unwrap();
        assert_eq!(sonnet.thinking, Some("between_tools"));
        let opus = latency_tuning(ProviderKind::Anthropic, "claude-opus-5-5").unwrap();
        assert_eq!(opus.thinking, None);
        assert_eq!(opus.effort, "low");

        assert!(latency_tuning(ProviderKind::Anthropic, "claude-haiku-4-5-20251001").is_none());
        assert!(latency_tuning(ProviderKind::Anthropic, "claude-sonnet-5").is_none());
        assert!(latency_tuning(ProviderKind::OpenAi, "claude-sonnet-5-5").is_none());
    }

    #[test]
    fn builds_request_body_with_thinking_and_effort() {
        let request = resolved("claude-sonnet-5-5", "https://api.anthropic.com/v1");
        let tuning = latency_tuning(ProviderKind::Anthropic, "claude-sonnet-5-5").unwrap();
        let body = request_body(&request, tuning, true);

        assert_eq!(body["model"], "claude-sonnet-5-5");
        assert_eq!(body["max_tokens"], 16384);
        assert_eq!(body["stream"], true);
        assert_eq!(body["thinking"], json!({ "type": "between_tools" }));
        assert_eq!(body["output_config"], json!({ "effort": "medium" }));
        assert!(body.get("temperature").is_none());

        let opus = resolved("claude-opus-5-5", "https://api.anthropic.com/v1");
        let tuning = latency_tuning(ProviderKind::Anthropic, "claude-opus-5-5").unwrap();
        let body = request_body(&opus, tuning, false);
        assert!(body.get("thinking").is_none());
        assert_eq!(body["output_config"], json!({ "effort": "low" }));
    }

    #[test]
    fn builds_messages_url_from_any_endpoint_form() {
        for base in [
            "https://gw.example",
            "https://gw.example/v1",
            "https://gw.example/v1/messages",
        ] {
            assert_eq!(
                messages_url(base).unwrap(),
                "https://gw.example/v1/messages"
            );
        }
    }

    #[test]
    fn accumulates_text_deltas_and_usage() {
        let mut state = StreamState::default();
        let events = [
            r#"{"type":"message_start","message":{"usage":{"input_tokens":12}}}"#,
            r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"hm"}}"#,
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hel"}}"#,
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"lo"}}"#,
            r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":5}}"#,
        ];
        let deltas: Vec<String> = events
            .iter()
            .filter_map(|data| apply_event(&mut state, data).unwrap())
            .collect();

        assert_eq!(deltas, ["Hel", "lo"]);
        assert_eq!(state.text, "Hello");
        assert_eq!(state.input_tokens, Some(12));
        assert_eq!(state.output_tokens, Some(5));
        assert_eq!(state.stop_reason.as_deref(), Some("end_turn"));
    }

    #[test]
    fn surfaces_stream_error_events() {
        let mut state = StreamState::default();
        let err = apply_event(
            &mut state,
            r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#,
        )
        .expect_err("error event should fail");
        assert!(err.message.contains("Overloaded"));
    }

    #[test]
    fn maps_http_errors() {
        let auth = map_http_error(401, r#"{"error":{"message":"invalid x-api-key"}}"#);
        assert_eq!(auth.code, AiErrorCode::InvalidApiKey);
        assert!(auth.message.contains("invalid x-api-key"));
        assert_eq!(
            map_http_error(529, "overloaded").code,
            AiErrorCode::ProviderError
        );
    }

    #[tokio::test]
    async fn streams_korean_text_end_to_end_against_local_server() {
        let sse = [
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":7}}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"안녕\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"하세요\"}}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":4}}\n\n",
        ]
        .concat();
        let (base_url, server) = one_shot_server(http_response("text/event-stream", &sse)).await;

        let request = resolved("claude-sonnet-5-5", &base_url);
        let tuning = latency_tuning(ProviderKind::Anthropic, "claude-sonnet-5-5").unwrap();
        let client = reqwest::Client::new();
        let mut deltas = Vec::new();
        let response = stream(&client, request, tuning, |event| {
            let AiStreamEvent::Delta { text } = event;
            deltas.push(text);
        })
        .await
        .expect("stream should succeed");

        assert_eq!(deltas, ["안녕", "하세요"]);
        assert_eq!(response.text, "안녕하세요");
        assert_eq!(response.input_tokens, Some(7));
        assert_eq!(response.output_tokens, Some(4));

        let received = server.await.unwrap();
        assert!(received.starts_with("POST /v1/messages "), "{received}");
        assert!(received
            .to_ascii_lowercase()
            .contains("x-api-key: test-key"));
        assert!(received.contains("\"between_tools\""));
        assert!(received.contains("\"effort\":\"medium\""));
    }

    #[tokio::test]
    async fn reports_truncation_when_max_tokens_reached() {
        let sse = "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"partial\"}}\n\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"max_tokens\"},\"usage\":{\"output_tokens\":9}}\n\n";
        let (base_url, _server) = one_shot_server(http_response("text/event-stream", sse)).await;

        let request = resolved("claude-opus-5-5", &base_url);
        let tuning = latency_tuning(ProviderKind::Anthropic, "claude-opus-5-5").unwrap();
        let err = stream(&reqwest::Client::new(), request, tuning, |_| {})
            .await
            .expect_err("truncated output should be an error");
        assert_eq!(err.code, AiErrorCode::ProviderError);
        assert!(err.message.contains("최대"));
    }

    #[tokio::test]
    async fn non_streaming_complete_skips_thinking_blocks() {
        let body = r#"{"content":[{"type":"thinking","thinking":"..."},{"type":"text","text":"Hello"}],"stop_reason":"end_turn","usage":{"input_tokens":3,"output_tokens":2}}"#;
        let (base_url, _server) = one_shot_server(http_response("application/json", body)).await;

        let request = resolved("claude-opus-5-5", &base_url);
        let tuning = latency_tuning(ProviderKind::Anthropic, "claude-opus-5-5").unwrap();
        let response = complete(&reqwest::Client::new(), request, tuning)
            .await
            .expect("complete should succeed");
        assert_eq!(response.text, "Hello");
        assert_eq!(response.input_tokens, Some(3));
    }

    #[tokio::test]
    async fn maps_unauthorized_response_to_invalid_api_key() {
        let body = r#"{"error":{"message":"invalid x-api-key","type":"x"}}"#;
        let response = format!(
            "HTTP/1.1 401 Unauthorized\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        let (base_url, _server) = one_shot_server(response).await;

        let request = resolved("claude-sonnet-5-5", &base_url);
        let tuning = latency_tuning(ProviderKind::Anthropic, "claude-sonnet-5-5").unwrap();
        let err = complete(&reqwest::Client::new(), request, tuning)
            .await
            .expect_err("401 should fail");
        assert_eq!(err.code, AiErrorCode::InvalidApiKey);
    }

    #[tokio::test]
    async fn delivers_each_delta_as_it_arrives_without_buffering() {
        use std::time::{Duration, Instant};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 8192];
            let _ = socket.read(&mut buf).await.unwrap();

            let event = |text: &str| {
                format!(
                    "data: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"text_delta\",\"text\":\"{text}\"}}}}\n\n"
                )
            };
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n")
                .await
                .unwrap();
            socket.write_all(event("first").as_bytes()).await.unwrap();
            socket.flush().await.unwrap();
            tokio::time::sleep(Duration::from_millis(400)).await;
            socket.write_all(event("second").as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        });

        let request = resolved("claude-sonnet-5-5", &base_url);
        let tuning = latency_tuning(ProviderKind::Anthropic, "claude-sonnet-5-5").unwrap();
        let started = Instant::now();
        let mut arrivals: Vec<(String, Duration)> = Vec::new();
        stream(&reqwest::Client::new(), request, tuning, |event| {
            let AiStreamEvent::Delta { text } = event;
            arrivals.push((text, started.elapsed()));
        })
        .await
        .expect("stream should succeed");
        server.await.unwrap();

        assert_eq!(arrivals.len(), 2);
        // The first token is forwarded immediately, not held back until the stream ends.
        assert!(arrivals[0].1 < Duration::from_millis(250), "{arrivals:?}");
        assert!(arrivals[1].1 >= Duration::from_millis(380), "{arrivals:?}");
    }
}
