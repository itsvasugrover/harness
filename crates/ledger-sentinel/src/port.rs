//! Ledger surface: audit events carry full attribution so bad
//! guidance (skill, MCP server, agent) is always traceable.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable `kind` strings for every auditable engine action.
pub mod kinds {
    pub const GATE_DECISION: &str = "gate.decision";
    pub const HANDOVER_EXPORT: &str = "handover.export";
    pub const SUBAGENT_SPAWN: &str = "subagent.spawn";
    pub const APPROVAL_DECISION: &str = "approval.decision";
    pub const TOOL_EXEC: &str = "tool.exec";
}

/// Who/what acted: every consequential call logs all three axes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribution {
    pub actor: String,
    pub agent: String,
    pub skill: Option<String>,
    pub mcp_server: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub seq: u64,
    pub id: Uuid,
    pub time: DateTime<Utc>,
    pub attribution: Attribution,
    pub repo: String,
    pub kind: String,
    pub summary: String,
    pub verdict: String,
    /// Refs like `pr:3`, `sha:abc`, `issue:7` for cross-linking.
    #[serde(default)]
    pub refs: Vec<String>,
    /// Hash of the previous event: tampering breaks the chain.
    pub hash_prev: String,
}

impl AuditEvent {
    pub fn new(
        seq: u64,
        attribution: Attribution,
        repo: &str,
        kind: &str,
        summary: &str,
        hash_prev: &str,
    ) -> Self {
        Self {
            seq,
            id: Uuid::new_v4(),
            time: Utc::now(),
            attribution,
            repo: repo.into(),
            kind: kind.into(),
            summary: summary.into(),
            verdict: "pending".into(),
            refs: vec![],
            hash_prev: hash_prev.into(),
        }
    }

    pub fn with_refs(mut self, refs: Vec<String>) -> Self {
        self.refs = refs;
        self
    }
}

/// Chain link: hex sha256 over the canonical fields plus `hash_prev`.
/// Lives here with the event shape so writers and verifiers share it.
pub fn chain_hash(event: &AuditEvent) -> String {
    use sha2::{Digest, Sha256};
    let canon = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        event.seq,
        event.time.to_rfc3339(),
        event.attribution.actor,
        event.attribution.agent,
        event.repo,
        event.kind,
        event.summary,
        event.hash_prev
    );
    Sha256::digest(canon.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
