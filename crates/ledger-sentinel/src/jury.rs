//! Bet 5 — Sentinel jury (stub; full build v1).
//! High-risk merges need two cheap-model votes; split vote = block.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JuryVote {
    pub approve: bool,
    pub reason: String,
}

/// Unanimous approve passes. Anything else blocks with reasons kept.
pub fn tally(votes: &[JuryVote]) -> bool {
    !votes.is_empty() && votes.iter().all(|v| v.approve)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vote(approve: bool) -> JuryVote {
        JuryVote {
            approve,
            reason: "r".into(),
        }
    }

    #[test]
    fn split_vote_blocks() {
        assert!(!tally(&[vote(true), vote(false)]));
        assert!(tally(&[vote(true), vote(true)]));
    }
}
