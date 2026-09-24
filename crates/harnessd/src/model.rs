//! Async drivers: the model decides turns over the network.
//! `FakeModel` proves the loop shape in tests; the OpenAI driver goes
//! live against real keys (env-configured, never logged).
use anyhow::Result;
use async_trait::async_trait;
use model_switchboard::adapters::openai::{self, ChatEvent, ChatMessage};
use work_engine::processor::StreamEvent;

#[async_trait]
pub trait AsyncDriver: Send {
    async fn next_turn(&mut self, steps_used: u32) -> Result<super::run::Turn>;
    fn observe(&mut self, _title: &str, _output: &str) {}
    fn take_usage(&mut self) -> (i64, i64) {
        (0, 0)
    }
}

/// Scripted events for tests: no network, deterministic usage.
#[allow(dead_code)] // exercised in run.rs tests; serve demo wires it next.
pub struct FakeModel {
    turns: Vec<Vec<ChatEvent>>,
    pos: usize,
    usage: (i64, i64),
}

impl FakeModel {
    #[allow(dead_code)] // tests construct it; serve demo next.
    pub fn new(turns: Vec<Vec<ChatEvent>>) -> Self {
        Self {
            turns,
            pos: 0,
            usage: (0, 0),
        }
    }
}

#[async_trait]
impl AsyncDriver for FakeModel {
    async fn next_turn(&mut self, _steps: u32) -> Result<super::run::Turn> {
        if self.pos >= self.turns.len() {
            return Ok(None);
        }
        self.usage.0 += 10;
        self.usage.1 += 5;
        let mut tool = None;
        for e in &self.turns[self.pos] {
            if let ChatEvent::ToolCall { name, input } = e {
                tool = Some((name.clone(), input.clone()));
                break;
            }
        }
        self.pos += 1;
        Ok(tool)
    }

    fn take_usage(&mut self) -> (i64, i64) {
        std::mem::take(&mut self.usage)
    }
}

/// Live OpenAI-compatible driver. History is plain chat messages;
/// tool outputs append as user notes (full role mapping is Phase 4).
#[allow(dead_code)] // constructed by serve once key config lands.
pub struct OpenAiDriver {
    pub base_url: String,
    pub key: String,
    pub model: String,
    history: Vec<ChatMessage>,
    pending_usage: (i64, i64),
}

impl OpenAiDriver {
    #[allow(dead_code)] // constructed by serve once key config lands.
    pub fn new(base_url: &str, key: &str, model: &str, system: &str) -> Self {
        Self {
            base_url: base_url.into(),
            key: key.into(),
            model: model.into(),
            history: vec![ChatMessage {
                role: "system".into(),
                content: system.into(),
            }],
            pending_usage: (0, 0),
        }
    }

    fn to_processor(events: &[ChatEvent]) -> Vec<StreamEvent> {
        events
            .iter()
            .map(|e| match e {
                ChatEvent::Text(t) => StreamEvent::Text(t.clone()),
                ChatEvent::ToolCall { name, input } => StreamEvent::ToolCall {
                    name: name.clone(),
                    input: input.clone(),
                },
            })
            .collect()
    }
}

#[async_trait]
impl AsyncDriver for OpenAiDriver {
    async fn next_turn(&mut self, _steps: u32) -> Result<super::run::Turn> {
        let (events, input_tokens, output_tokens) =
            openai::stream_events(&self.base_url, &self.key, &self.model, &self.history).await?;
        self.pending_usage.0 += input_tokens;
        self.pending_usage.1 += output_tokens;
        let folded = work_engine::processor::process(&Self::to_processor(&events));
        let text = folded.text;
        if !text.is_empty() {
            self.history.push(ChatMessage {
                role: "assistant".into(),
                content: text,
            });
        }
        Ok(folded.tool_calls.into_iter().next())
    }

    fn observe(&mut self, title: &str, output: &str) {
        self.history.push(ChatMessage {
            role: "user".into(),
            content: format!("tool {title} result:\n{output}"),
        });
    }

    fn take_usage(&mut self) -> (i64, i64) {
        std::mem::take(&mut self.pending_usage)
    }
}
