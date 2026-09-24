//! Bet 11 — offline intent queue (stub; full build Phase 4).
//! Phone actions queue with idempotency ids; world-moved-first replays
//! surface explicit conflicts instead of silent overwrites.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentState {
    Queued,
    Applied,
    Conflicted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub id: Uuid,
    pub kind: String,
    pub payload: String,
    pub state: IntentState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub intent_id: Uuid,
    /// e.g. ["rebase", "drop", "escalate"].
    pub choices: Vec<String>,
    pub reason: String,
}

impl Intent {
    pub fn queue(kind: &str, payload: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind: kind.into(),
            payload: payload.into(),
            state: IntentState::Queued,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queues_with_id() {
        let i = Intent::queue("approve", "{}");
        assert_eq!(i.state, IntentState::Queued);
    }
}
