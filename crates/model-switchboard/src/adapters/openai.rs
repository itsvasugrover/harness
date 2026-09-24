//! OpenAI-compatible chat adapter (`/v1/chat/completions`, SSE).
//! Keys resolve from explicit value -> listed env vars; never logged.
use anyhow::{Context, Result};
use futures::StreamExt as _;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Delta,
    /// Kept for protocol completeness; turn end derives from tool absence.
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCallDelta>,
}

#[derive(Debug, Deserialize)]
struct ToolCallDelta {
    #[serde(default)]
    function: FunctionDelta,
}

#[derive(Debug, Deserialize, Default)]
struct FunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: i64,
    #[serde(default)]
    completion_tokens: i64,
}

/// Fold one SSE `data:` payload into events. Returns true on `[DONE]`.
pub fn feed_event(payload: &str, texts: &mut Vec<ChatEvent>) -> bool {
    if payload.trim() == "[DONE]" {
        return true;
    }
    let Ok(chunk) = serde_json::from_str::<StreamChunk>(payload) else {
        return false;
    };
    for choice in chunk.choices {
        if let Some(text) = choice.delta.content {
            texts.push(ChatEvent::Text(text));
        }
        for tc in choice.delta.tool_calls {
            texts.push(ChatEvent::ToolCall {
                name: tc.function.name.unwrap_or_default(),
                input: tc.function.arguments.unwrap_or_default(),
            });
        }
    }
    false
}

/// Adapter-native event. The run loop maps these to
/// `work_engine::processor::StreamEvent` (adapter never imports engine).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEvent {
    Text(String),
    ToolCall { name: String, input: String },
}

/// Stream a chat completion into model-agnostic events + token usage.
pub async fn stream_events(
    base_url: &str,
    key: &str,
    model: &str,
    messages: &[ChatMessage],
) -> Result<(Vec<ChatEvent>, i64, i64)> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let body = ChatRequest {
        model: model.into(),
        messages: messages.into(),
        stream: true,
    };
    let mut stream = client
        .post(format!(
            "{}/chat/completions",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(key)
        .json(&body)
        .send()
        .await?
        .bytes_stream();
    let mut events = Vec::new();
    let (mut input_tokens, mut output_tokens) = (0, 0);
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        buf.push_str(&String::from_utf8_lossy(&chunk?));
        while let Some(pos) = buf.find("\n\n") {
            let block: String = buf.drain(..pos + 2).collect();
            for line in block.lines() {
                if let Some(payload) = line.strip_prefix("data:") {
                    if feed_event(payload.trim(), &mut events) {
                        return Ok::<_, anyhow::Error>((events, input_tokens, output_tokens));
                    }
                    if let Ok(usage_block) = serde_json::from_str::<Usage>(payload) {
                        input_tokens = usage_block.prompt_tokens;
                        output_tokens = usage_block.completion_tokens;
                    }
                }
            }
        }
    }
    Ok::<_, anyhow::Error>((events, input_tokens, output_tokens))
        .context("stream ended without [DONE]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_and_tool_deltas() {
        let mut events = Vec::new();
        assert!(!feed_event(
            r#"{"choices":[{"delta":{"content":"hi"},"finish_reason":null}]}"#,
            &mut events
        ));
        assert!(!feed_event(
            r#"{"choices":[{"delta":{"tool_calls":[{"function":{"name":"read","arguments":"{}"}}]},"finish_reason":null}]}"#,
            &mut events
        ));
        assert!(feed_event("[DONE]", &mut events));
        assert_eq!(events.len(), 2);
    }
}
