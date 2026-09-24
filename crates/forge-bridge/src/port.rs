//! Unified forge surface. Agent + decks program to this, never SDKs.
//!
//! Workers never hold raw tokens: the daemon mints a CapabilityLease
//! (scoped, expiring, revocable) per session instead.
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One page of results. `next` is an opaque cursor; `None` ends paging.
/// Writes stay idempotent via caller-supplied ids where supported.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Page<T> {
    #[serde(default)]
    pub items: Vec<T>,
    #[serde(default)]
    pub next: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Search {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub per_page: u32,
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Repo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub default_branch: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Issue {
    #[serde(default)]
    pub number: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub author: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Comment {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub body: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IssueFull {
    #[serde(default)]
    pub issue: Issue,
    #[serde(default)]
    pub comments: Vec<Comment>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewIssue {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Check {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub conclusion: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewThread {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub resolved: bool,
    #[serde(default)]
    pub comments: Vec<Comment>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PullFull {
    #[serde(default)]
    pub number: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub head_sha: String,
    #[serde(default)]
    pub mergeable: bool,
    #[serde(default)]
    pub checks: Vec<Check>,
    #[serde(default)]
    pub threads: Vec<ReviewThread>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewPull {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub head: String,
    #[serde(default)]
    pub base: String,
    #[serde(default)]
    pub body: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MergeReport {
    #[serde(default)]
    pub merged: bool,
    #[serde(default)]
    pub sha: Option<String>,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Review {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub state: String,
}

/// Lightweight PR row for observer listing. Detail (checks,
/// threads) comes from [`Forge::pull`] only on demand.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PullSummary {
    #[serde(default)]
    pub number: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub head_sha: String,
}

/// Unified forge surface. Agent + decks program to this, never SDKs.
/// Token handling stays daemon-side: adapters receive a resolved token
/// per call site, workers only ever present a [`CapabilityLease`].
#[async_trait]
pub trait Forge: Send + Sync {
    fn name(&self) -> &'static str;
    async fn repos(&self, q: &Search) -> Result<Page<Repo>>;
    async fn issues(&self, repo: &str, q: &Search) -> Result<Page<Issue>>;
    async fn issue_detail(&self, repo: &str, number: u64) -> Result<IssueFull>;
    async fn open_issue(&self, repo: &str, issue: &NewIssue) -> Result<Issue>;
    async fn comment(&self, repo: &str, number: u64, body: &str) -> Result<Comment>;
    async fn pulls(&self, repo: &str, q: &Search) -> Result<Page<PullSummary>>;
    async fn pull(&self, repo: &str, number: u64) -> Result<PullFull>;
    async fn open_pull(&self, repo: &str, pull: &NewPull) -> Result<PullFull>;
    async fn merge(&self, repo: &str, number: u64, method: &str) -> Result<MergeReport>;
    async fn checks(&self, repo: &str, sha: &str) -> Result<Vec<Check>>;
    async fn request_review(&self, repo: &str, number: u64, reviewers: &[String])
        -> Result<Review>;
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
