//! App API: identity probe (the single unauthenticated route), the
//! derived board, and the bearer gate. Loopback serves all routes open;
//! the LAN listener enforces the bearer on everything except the exact
//! `GET /api/v1/identity` probe. Control routes never exist here.
use axum::{extract::State, http::HeaderMap, routing::get, Json, Router};
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Serialize)]
struct Identity {
    host_id: String,
    contract: u32,
}

/// Contract version: bump only on breaking API change (phones gate on it).
pub const CONTRACT: u32 = 1;

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub facts: Arc<Mutex<Vec<super::board::CardFacts>>>,
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

async fn identity() -> Json<Identity> {
    Json(Identity {
        host_id: uuid::Uuid::new_v4().to_string(),
        contract: CONTRACT,
    })
}

async fn board(State(state): State<AppState>) -> Json<Vec<(super::board::Column, String)>> {
    let facts = state.facts.lock().unwrap().clone();
    Json(
        facts
            .into_iter()
            .map(|f| (super::board::column(&f), f.worker_id))
            .collect(),
    )
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/identity", get(identity))
        .route("/api/v1/board", get(board))
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
}
