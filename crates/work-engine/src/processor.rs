//! Stream processor: transport-agnostic event fold.
//! Live SSE parsing lands with provider adapters (Phase 2); the loop
//! reuses this outcome shape unchanged.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    Text(String),
    ToolCall {
        name: String,
        input: String,
    },
    Finish {
        input_tokens: i64,
        output_tokens: i64,
    },
}

#[derive(Debug, Default)]
pub struct TurnOutcome {
    pub text: String,
    pub tool_calls: Vec<(String, String)>,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// Fold one turn's events. Doom-loop detection lives in the loop (Phase 2).
pub fn process(events: &[StreamEvent]) -> TurnOutcome {
    let mut out = TurnOutcome::default();
    for e in events {
        match e {
            StreamEvent::Text(t) => out.text.push_str(t),
            StreamEvent::ToolCall { name, input } => {
                out.tool_calls.push((name.clone(), input.clone()))
            }
            StreamEvent::Finish {
                input_tokens,
                output_tokens,
            } => {
                out.input_tokens += input_tokens;
                out.output_tokens += output_tokens;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_turn() {
        let o = process(&[
            StreamEvent::Text("hi ".into()),
            StreamEvent::ToolCall {
                name: "read".into(),
                input: "a".into(),
            },
            StreamEvent::Finish {
                input_tokens: 10,
                output_tokens: 5,
            },
        ]);
        assert_eq!(o.text, "hi ");
        assert_eq!(o.tool_calls.len(), 1);
        assert_eq!((o.input_tokens, o.output_tokens), (10, 5));
    }
}
