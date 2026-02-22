use futures_util::StreamExt;
use std::sync::OnceLock;
use std::time::Duration;

pub struct AnthropicStreamResult {
    pub full_text: String,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
}

pub fn extract_anthropic_message_text(body: &serde_json::Value) -> Result<String, String> {
    let content = body
        .get("content")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "missing content array".to_string())?;

    let first_block = content
        .first()
        .ok_or_else(|| "content array is empty".to_string())?;

    first_block
        .get("text")
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
        .ok_or_else(|| "missing content[0].text string".to_string())
}

pub fn anthropic_http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(90))
            .build()
            .unwrap_or_else(|err| {
                log::warn!(
                    "Failed to build configured HTTP client, falling back to default: {}",
                    err
                );
                reqwest::Client::new()
            })
    })
}

fn process_sse_line(
    line: &str,
    full_text: &mut String,
    input_tokens: &mut Option<i32>,
    output_tokens: &mut Option<i32>,
    on_delta: &mut impl FnMut(&str),
) {
    if let Some(json_str) = line.strip_prefix("data: ") {
        if json_str == "[DONE]" {
            return;
        }

        if let Ok(event) = serde_json::from_str::<serde_json::Value>(json_str) {
            match event["type"].as_str().unwrap_or("") {
                "content_block_delta" => {
                    if let Some(delta) = event["delta"]["text"].as_str() {
                        full_text.push_str(delta);
                        on_delta(delta);
                    }
                }
                "message_start" => {
                    if let Some(usage) = event["message"]["usage"].as_object() {
                        *input_tokens = usage["input_tokens"].as_i64().map(|v| v as i32);
                    }
                }
                "message_delta" => {
                    if let Some(usage) = event["usage"].as_object() {
                        *output_tokens = usage["output_tokens"].as_i64().map(|v| v as i32);
                    }
                }
                _ => {}
            }
        }
    }
}

pub async fn stream_anthropic_sse(
    res: reqwest::Response,
    mut on_delta: impl FnMut(&str),
) -> Result<AnthropicStreamResult, String> {
    let mut full_text = String::new();
    let mut input_tokens: Option<i32> = None;
    let mut output_tokens: Option<i32> = None;
    let mut stream = res.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                let chunk_str = String::from_utf8_lossy(&bytes);
                buffer.push_str(&chunk_str);

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();
                    process_sse_line(
                        &line,
                        &mut full_text,
                        &mut input_tokens,
                        &mut output_tokens,
                        &mut on_delta,
                    );
                }
            }
            Err(err) => {
                return Err(format!("Stream error: {}", err));
            }
        }
    }

    if !buffer.trim().is_empty() {
        process_sse_line(
            buffer.trim(),
            &mut full_text,
            &mut input_tokens,
            &mut output_tokens,
            &mut on_delta,
        );
    }

    Ok(AnthropicStreamResult {
        full_text,
        input_tokens,
        output_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::extract_anthropic_message_text;

    #[test]
    fn extracts_text_from_valid_message_body() {
        let body = serde_json::json!({
            "content": [
                { "type": "text", "text": "hello" }
            ]
        });

        let text = extract_anthropic_message_text(&body).expect("text should parse");
        assert_eq!(text, "hello");
    }

    #[test]
    fn returns_error_when_content_array_is_missing() {
        let body = serde_json::json!({ "foo": "bar" });
        let err = extract_anthropic_message_text(&body).expect_err("should fail");
        assert!(err.contains("missing content array"));
    }

    #[test]
    fn returns_error_when_text_is_missing() {
        let body = serde_json::json!({
            "content": [
                { "type": "text" }
            ]
        });
        let err = extract_anthropic_message_text(&body).expect_err("should fail");
        assert!(err.contains("missing content[0].text string"));
    }
}
