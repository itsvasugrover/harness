//! Approvals: every write/tool action outside the session scope
//! pauses for a human (or a pre-approved once-token). All decisions
//! are audit-logged; deny always wins over allow.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Pending,
    Approved,
    Denied,
    Once,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    pub id: String,
    pub tool: String,
    pub summary: String,
    pub decision: Decision,
}

impl Approval {
    pub fn request(id: &str, tool: &str, summary: &str) -> Self {
        Self {
            id: id.into(),
            tool: tool.into(),
            summary: summary.into(),
            decision: Decision::Pending,
        }
    }

    pub fn approve(&mut self) {
        if self.decision == Decision::Pending {
            self.decision = Decision::Approved;
        }
    }

    pub fn deny(&mut self) {
        self.decision = Decision::Denied;
    }

    pub fn settled(&self) -> bool {
        self.decision != Decision::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approve_and_deny() {
        let mut a = Approval::request("1", "bash", "run tests");
        assert!(!a.settled());
        a.approve();
        assert!(a.settled());
        a.deny();
        assert_eq!(a.decision, Decision::Denied);
    }
}
