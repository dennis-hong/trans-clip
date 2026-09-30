//! Shared helpers for tests that talk to a local fake provider endpoint.

use super::{
    AiResolvedRequest, AiTextRequest, AuthScheme, EndpointMode, ModelProfile, ProviderConfig,
    ProviderKind,
};

pub(crate) fn resolved_request(
    provider_kind: ProviderKind,
    model_id: &str,
    base_url: &str,
) -> AiResolvedRequest {
    AiResolvedRequest {
        provider_config: ProviderConfig {
            id: provider_kind.as_db_value().to_string(),
            display_name: provider_kind.as_db_value().to_string(),
            provider_kind,
            endpoint_mode: EndpointMode::Custom,
            base_url: base_url.to_string(),
            auth_scheme: AuthScheme::XApiKey,
            enabled: true,
        },
        model_profile: ModelProfile {
            id: format!("{}:{model_id}", provider_kind.as_db_value()),
            provider_config_id: provider_kind.as_db_value().to_string(),
            display_name: model_id.to_string(),
            model_id: model_id.to_string(),
            api_interface: provider_kind.default_api_interface(),
            supports_streaming: true,
            max_output_tokens: 16384,
            sort_order: 1,
        },
        api_key: "test-key".to_string(),
        request: AiTextRequest {
            model_profile_id: None,
            system_prompt: None,
            user_prompt: "안녕".to_string(),
            max_output_tokens: 16384,
            temperature: None,
        },
    }
}

/// Starts a one-shot HTTP server that records the request and replies with `response`.
pub(crate) async fn one_shot_server(response: String) -> (String, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut received = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = socket.read(&mut buf).await.unwrap();
            received.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&received).to_string();
            if let Some(header_end) = text.find("\r\n\r\n") {
                let length = text
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if received.len() >= header_end + 4 + length {
                    break;
                }
            }
            if n == 0 {
                break;
            }
        }
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
        String::from_utf8_lossy(&received).to_string()
    });
    (format!("http://{addr}"), handle)
}

pub(crate) fn http_response(content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}
