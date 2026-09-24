//! Public surface of recall-ledger.
//!
//! Layer-aware notes: personal habits go global, repo rules go local.
//! The skill proposes; a human approves; the daemon writes.
use serde::{Deserialize, Serialize};

/// Where a failure-note belongs. Never inferred silently — proposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteTarget {
    Global,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteProposal {
    pub title: String,
    pub what_failed: String,
    pub correction: String,
    pub target: NoteTarget,
    pub approved: bool,
}

impl NoteProposal {
    /// Filesystem home for the note once approved.
    pub fn path(&self, global_dir: &str, local_dir: &str) -> String {
        let base = match self.target {
            NoteTarget::Global => global_dir,
            NoteTarget::Local => local_dir,
        };
        format!("{base}/notes/{}.md", slug(&self.title))
    }
}

fn slug(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
