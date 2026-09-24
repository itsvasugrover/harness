//! Audit HTTP surface: filtered reads over the Sentinel ledger.
//! Split from api.rs per the 300-line rule; the ledger itself lives in
//! ledger-sentinel, this file only shapes the endpoint.
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

/// `GET /api/v1/audit` filters: actor, repo, kind, time range, limit.
#[derive(Debug, Deserialize)]
pub(crate) struct AuditQuery {
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

pub(crate) async fn audit(
    State(state): State<super::api::AppState>,
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

#[cfg(test)]
mod tests {
    use super::*;
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
