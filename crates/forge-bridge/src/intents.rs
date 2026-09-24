//! Bet 11 — offline intent queue. Phone actions queue with
//! idempotency ids and replay on reconnect; `replay` decides whether
//! the world moved first and surfaces an explicit conflict instead of
//! a silent overwrite.
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

    /// Replay against the world's current facts (built daemon-side from
    /// the observer store, session liveness, and config fingerprints).
    /// Returns the settled intent plus a conflict when the operator
    /// must choose: rebase the intent, drop it, or escalate.
    pub fn replay(mut self, world: &WorldState) -> (Self, Option<Conflict>) {
        if world.config_changed {
            self.state = IntentState::Conflicted;
            let conflict = Conflict {
                intent_id: self.id,
                choices: vec!["rebase".into(), "drop".into(), "escalate".into()],
                reason: "repo config changed since queueing".into(),
            };
            return (self, Some(conflict));
        }
        let stale = match self.kind.as_str() {
            "approve" | "merge" => world.pr_merged,
            "retry" => world.session_closed,
            _ => false,
        };
        if stale {
            self.state = IntentState::Conflicted;
            let conflict = Conflict {
                intent_id: self.id,
                choices: vec!["drop".into(), "escalate".into()],
                reason: format!("{} target already settled", self.kind),
            };
            return (self, Some(conflict));
        }
        self.state = IntentState::Applied;
        (self, None)
    }
}

/// Facts the daemon assembles before replaying a queued intent.
#[derive(Debug, Clone, Default)]
pub struct WorldState {
    pub pr_merged: bool,
    pub session_closed: bool,
    pub config_changed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queues_with_id() {
        let i = Intent::queue("approve", "{}");
        assert_eq!(i.state, IntentState::Queued);
    }

    #[test]
    fn replay_conflicts_on_settled_world() {
        let world = WorldState {
            pr_merged: true,
            ..Default::default()
        };
        let (intent, conflict) = Intent::queue("approve", "{}").replay(&world);
        assert_eq!(intent.state, IntentState::Conflicted);
        assert_eq!(conflict.unwrap().choices.len(), 2);
        let fresh = WorldState::default();
        let (intent, conflict) = Intent::queue("comment", "{}").replay(&fresh);
        assert_eq!(intent.state, IntentState::Applied);
        assert!(conflict.is_none());
    }

    #[test]
    fn replay_escalates_on_config_change() {
        let world = WorldState {
            config_changed: true,
            ..Default::default()
        };
        let (_, conflict) = Intent::queue("retry", "{}").replay(&world);
        let conflict = conflict.unwrap();
        assert!(conflict.choices.contains(&"escalate".to_string()));
    }
}
