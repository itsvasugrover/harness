//! Bet 3 — deterministic session replay (stub; full build v1).
//! Every turn journals prompt, tool I/O, model deltas for offline replay.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JournalEvent {
    Prompt {
        text: String,
    },
    ToolIo {
        tool: String,
        input: String,
        output: String,
    },
    ModelDelta {
        text: String,
    },
}

#[derive(Debug, Default)]
pub struct Journal {
    events: Vec<JournalEvent>,
}

impl Journal {
    pub fn push(&mut self, e: JournalEvent) {
        self.events.push(e);
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn events(&self) -> &[JournalEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journals_turns() {
        let mut j = Journal::default();
        j.push(JournalEvent::Prompt { text: "hi".into() });
        assert_eq!(j.len(), 1);
    }
}
