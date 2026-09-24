//! Unified forge surface. Agent + decks program to this, never SDKs.
//!
//! Workers never hold raw tokens: the daemon mints a CapabilityLease
//! (scoped, expiring, revocable) per session instead.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub trait Forge {
    fn name(&self) -> &'static str;
}

/// Scoped handle handed to a worker. No token material inside —
/// the daemon resolves it to credentials server-side per call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityLease {
    pub lease_id: Uuid,
    pub forge: String,
    pub repo: String,
    /// e.g. ["issue.read", "pr.comment", "pr.open"] — never raw scopes.
    pub capabilities: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
}

impl CapabilityLease {
    pub fn mint(forge: &str, repo: &str, capabilities: Vec<String>) -> Self {
        Self {
            lease_id: Uuid::new_v4(),
            forge: forge.into(),
            repo: repo.into(),
            capabilities,
            expires_at: Utc::now() + chrono::Duration::hours(4),
            revoked: false,
        }
    }

    pub fn allows(&self, cap: &str) -> bool {
        !self.revoked && Utc::now() < self.expires_at && self.capabilities.contains(&cap.into())
    }
}
