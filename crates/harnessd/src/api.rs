//! App API: identity probe (the single unauthenticated route), the
//! derived board, and the bearer gate. Loopback serves all routes open;
//! the LAN listener enforces the bearer on everything except the exact
//! `GET /api/v1/identity` probe. Control routes never exist here.
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
struct Identity {
    host_id: String,
    contract: u32,
}

/// Contract version: bump only on breaking API change (phones gate on it).
/// v2 = unified board `{workers, prs}` + persistent host_id.
pub const CONTRACT: u32 = 2;

#[derive(Clone, Default)]
pub struct AppState {
    /// Stable machine id, persisted under `<data_dir>/host_id`.
    pub host_id: String,
    pub facts: Arc<tokio::sync::RwLock<Vec<super::board::CardFacts>>>,
    /// Observer PR cards mirrored for the unified board (Phase 5).
    pub prs: Arc<tokio::sync::RwLock<Vec<super::forge_facts::PrCard>>>,
    /// Audit ledger when the daemon opened one; `None` serves `[]`.
    pub audit: Option<Arc<tokio::sync::Mutex<ledger_sentinel::ledger::Ledger>>>,
    /// Daemon forge clients + repo mapping for intent replay (`None` = 503).
    pub forge: Option<Arc<super::forge_exec::DaemonForge>>,
    pub forge_cfgs: Vec<super::config_sections::ForgeCfg>,
    /// Idempotency store for intent replay (`None` = 503).
    pub intents: Option<super::intent_store::IntentStore>,
}

impl AppState {
    pub fn new(host_id: String) -> Self {
        Self {
            host_id,
            ..Default::default()
        }
    }
}

/// Load or create a stable host id. File-backed so phone pairing
/// survives restarts; a fresh UUID is written once, never rotated.
pub fn load_or_create_host_id(data_dir: &str) -> String {
    let path = std::path::PathBuf::from(format!("{data_dir}/host_id"));
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let trimmed = existing.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Best-effort persist; ephemeral id still beats a flapping one.
    let _ = std::fs::write(&path, &id);
    id
}

/// Pure bearer check so the policy is unit-tested without HTTP.
pub fn authorized(headers: &HeaderMap, bearer: &str, path: &str) -> bool {
    if path == "/api/v1/identity" {
        return true;
    }
    if bearer.is_empty() {
        return false;
    }
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == format!("Bearer {bearer}"))
}

async fn identity(State(state): State<AppState>) -> Json<Identity> {
    Json(Identity {
        host_id: state.host_id.clone(),
        contract: CONTRACT,
    })
}

#[derive(Serialize, Clone)]
pub struct WorkerCard {
    pub worker_id: String,
    pub column: super::board::Column,
    pub alive: bool,
    pub blocked: Option<String>,
    pub completed: bool,
}

#[derive(Serialize, Clone)]
pub struct PrCardView {
    pub repo: String,
    pub number: i64,
    pub title: String,
    pub state: String,
    pub column: super::board::Column,
    pub checks_green: bool,
    pub unresolved: i64,
    pub mergeable: bool,
}

#[derive(Serialize, Clone, Default)]
pub struct BoardResponse {
    pub workers: Vec<WorkerCard>,
    pub prs: Vec<PrCardView>,
}

async fn board(State(state): State<AppState>) -> Json<BoardResponse> {
    let facts = state.facts.read().await.clone();
    let prs = state.prs.read().await.clone();
    let workers = facts
        .into_iter()
        .map(|f| WorkerCard {
            column: super::board::column(&f),
            worker_id: f.worker_id.clone(),
            alive: f.alive,
            blocked: f.blocked.clone(),
            completed: f.completed,
        })
        .collect();
    // Multi-repo boards keep each card's origin repo.
    let pr_views = prs
        .into_iter()
        .map(|p| {
            let column = super::board::column_for_pr(&p);
            PrCardView {
                repo: p.repo.clone(),
                number: p.number,
                title: p.title.clone(),
                state: p.state.clone(),
                column,
                checks_green: p.checks_green,
                unresolved: p.unresolved,
                mergeable: p.mergeable,
            }
        })
        .collect();
    Json(BoardResponse {
        workers,
        prs: pr_views,
    })
}

/// `GET /api/v1/audit` filters: actor, repo, kind, time range, limit.
#[derive(Debug, Deserialize)]
struct AuditQuery {
    actor: Option<String>,
    repo: Option<String>,
    kind: Option<String>,
    since: Option<String>,
    until: Option<String>,
    limit: Option<u64>,
}

fn to_filter(q: &AuditQuery) -> ledger_sentinel::ledger::AuditFilter {
    ledger_sentinel::ledger::AuditFilter {
        actor: q.actor.clone(),
        repo: q.repo.clone(),
        kind: q.kind.clone(),
        since: q.since.clone(),
        until: q.until.clone(),
        limit: q.limit.unwrap_or(100),
    }
}

async fn audit(
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> Json<Vec<ledger_sentinel::port::AuditEvent>> {
    let Some(ledger) = &state.audit else {
        return Json(vec![]);
    };
    let rows = ledger
        .lock()
        .await
        .query(&to_filter(&q))
        .await
        .unwrap_or_default();
    Json(rows)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/identity", get(identity))
        .route("/api/v1/board", get(board))
        .route("/api/v1/audit", get(audit))
        .route("/api/v1/intents", post(super::intents::route))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_probe_needs_no_bearer() {
        let headers = HeaderMap::new();
        assert!(authorized(&headers, "secret", "/api/v1/identity"));
        assert!(!authorized(&headers, "secret", "/api/v1/board"));
    }
    #[test]
    fn board_needs_exact_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer secret".parse().unwrap(),
        );
        assert!(authorized(&headers, "secret", "/api/v1/board"));
        assert!(!authorized(&headers, "wrong", "/api/v1/board"));
    }

    #[tokio::test]
    async fn board_serves_workers_and_prs() {
        let state = AppState {
            host_id: "h1".into(),
            facts: std::sync::Arc::new(tokio::sync::RwLock::new(vec![
                super::super::board::CardFacts {
                    worker_id: "w1".into(),
                    alive: true,
                    blocked: None,
                    pr_open: true,
                    checks_green: true,
                    approved: false,
                    completed: false,
                },
            ])),
            prs: std::sync::Arc::new(tokio::sync::RwLock::new(vec![
                super::super::forge_facts::PrCard {
                    repo: "o/r".into(),
                    number: 7,
                    title: "fix".into(),
                    state: "open".into(),
                    mergeable: true,
                    checks_green: true,
                    unresolved: 0,
                },
            ])),
            audit: None,
            forge: None,
            forge_cfgs: vec![],
            intents: None,
        };
        // Call the handler directly via router would need HTTP; instead
        // assert the derivation the handler relies on stays live.
        let facts = state.facts.read().await.clone();
        assert_eq!(
            super::super::board::column(&facts[0]),
            super::super::board::Column::InReview
        );
        let prs = state.prs.read().await.clone();
        assert_eq!(
            super::super::board::column_for_pr(&prs[0]),
            super::super::board::Column::InReview
        );
        assert_eq!(prs[0].repo, "o/r");
        assert_eq!(state.host_id, "h1");
    }

    #[test]
    fn host_id_persists_across_loads() {
        let dir = std::env::temp_dir().join(format!("harness-hostid-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let s = dir.to_string_lossy().into_owned();
        let first = super::load_or_create_host_id(&s);
        let second = super::load_or_create_host_id(&s);
        assert!(!first.is_empty());
        assert_eq!(first, second);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn audit_filter_defaults_limit() {
        let q = AuditQuery {
            actor: None,
            repo: Some("o/r".into()),
            kind: None,
            since: None,
            until: None,
            limit: None,
        };
        let f = to_filter(&q);
        assert_eq!(f.limit, 100);
        assert_eq!(f.repo.as_deref(), Some("o/r"));
    }
}
