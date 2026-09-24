//! Phone intent replay: idempotent approve/comment execution.
//! One JSON intent in, one settled result out. Replays dedup by the
//! phone's idempotency id (storms return the stored result); staleness
//! goes through bridge `replay()` so conflicts surface with explicit
//! choices instead of silent overwrites. Every settlement is audited.
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use forge_bridge::port::{CapabilityLease, Forge};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Phone intent body. `body` carries comment text (or the approval note).
#[derive(Debug, Clone, Deserialize)]
pub struct IntentRequest {
    pub idempotency_id: Option<String>,
    pub kind: Option<String>,
    pub repo: Option<String>,
    pub number: Option<u64>,
    pub body: Option<String>,
}

/// Settled replay: applied carries the forge summary, conflicts carry
/// the bridge choices (rebase/drop/escalate), unsupported names the
/// follow-up that unlocks the kind.
#[derive(Debug, Clone, Serialize)]
pub struct IntentOutcome {
    pub idempotency_id: String,
    pub applied: bool,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
}

/// Replay one intent: dedup → world check → lease-scoped execute → audit.
/// Pure against any `Forge` map; the HTTP layer only maps the outcome.
pub async fn dispatch(
    store: &super::intent_store::IntentStore,
    clients: &HashMap<String, Arc<dyn Forge + Send + Sync>>,
    forges: &[super::config_sections::ForgeCfg],
    audit_log: Option<Arc<tokio::sync::Mutex<ledger_sentinel::ledger::Ledger>>>,
    req: &IntentRequest,
) -> IntentOutcome {
    let id = req.idempotency_id.clone().unwrap_or_default();
    let kind = req.kind.clone().unwrap_or_default();
    let repo = req.repo.clone().unwrap_or_default();
    let number = req.number.unwrap_or(0);
    let body = req.body.clone().unwrap_or_default();
    if id.is_empty() || repo.is_empty() || number == 0 {
        return unsupported(&id, "intent needs {idempotency_id, repo, number}");
    }
    if let Ok(Some(prior)) = store.prior(&id).await {
        return applied(&id, format!("replay: {prior}"));
    }
    if kind != "approve_pr" && kind != "comment" && kind != "retry_worker" {
        return unsupported(&id, &format!("unknown kind '{kind}'"));
    }
    let Some(cfg) = forges.iter().find(|f| f.repos.iter().any(|r| r == &repo)) else {
        let out = conflict(&id, format!("repo {repo} is not watched by this daemon"));
        audit(&audit_log, &repo, &kind, &out.summary, "conflict", &id).await;
        return out;
    };
    let Some(client) = clients.get(&cfg.kind) else {
        let out = conflict(&id, format!("forge '{}' has no credentials here", cfg.kind));
        audit(&audit_log, &repo, &kind, &out.summary, "conflict", &id).await;
        return out;
    };
    // World check against the live PR: merged/deleted targets conflict.
    let pull = match client.pull(&repo, number).await {
        Ok(p) => p,
        Err(e) => {
            let reason = forge_bridge::mask::mask_secrets(&format!("target unreadable: {e:#}"));
            let out = conflict(&id, reason);
            audit(&audit_log, &repo, &kind, &out.summary, "conflict", &id).await;
            return out;
        }
    };
    let world = forge_bridge::intents::WorldState {
        pr_merged: pull.state != "open",
        ..Default::default()
    };
    let bridge_kind = if kind == "approve_pr" {
        "approve"
    } else {
        "comment"
    };
    let (settled, bridge_conflict) =
        forge_bridge::intents::Intent::queue(bridge_kind, &repo).replay(&world);
    if settled.state != forge_bridge::intents::IntentState::Applied {
        let reason = bridge_conflict.map(|c| c.reason).unwrap_or_default();
        let out = conflict(&id, reason);
        audit(&audit_log, &repo, &kind, &out.summary, "conflict", &id).await;
        return out;
    }
    // Retry is a recheck: fresh PR facts plus a merge-gate verdict plus
    // an audit write. Read-only against the forge, so no lease is needed
    // and replaying it can never double-apply.
    if kind == "retry_worker" {
        return recheck(store, &audit_log, &id, &repo, number, &pull).await;
    }

    // Lease-scoped execution: the daemon mints the exact caps the op
    // needs; the phone never sees credentials, only this result.
    let op = if kind == "approve_pr" {
        "approve"
    } else {
        "comment"
    };
    let cap = if kind == "approve_pr" {
        "pr.approve"
    } else {
        "pr.comment"
    };
    let lease = CapabilityLease::mint(&cfg.kind, &repo, vec![cap.into()]);
    let input = serde_json::json!({"op": op, "repo": repo, "number": number, "body": body});
    let exec_result = super::forge_ops::run_op(clients, &lease, op, &input.to_string()).await;
    let out = match exec_result {
        Ok(summary) => applied(&id, forge_bridge::mask::mask_secrets(&summary)),
        Err(e) => conflict(
            &id,
            forge_bridge::mask::mask_secrets(&format!(
                "forge refused (check the PR before retrying): {e:#}"
            )),
        ),
    };
    if out.applied {
        let _ = store
            .record(&id, &kind, &repo, number as i64, &out.summary)
            .await;
    }
    let verdict = if out.applied { "applied" } else { "conflict" };
    audit(&audit_log, &repo, &kind, &out.summary, verdict, &id).await;
    out
}

/// Recheck one PR: fresh facts, gate verdict, audit write, stored result.
async fn recheck(
    store: &super::intent_store::IntentStore,
    audit_log: &Option<Arc<tokio::sync::Mutex<ledger_sentinel::ledger::Ledger>>>,
    id: &str,
    repo: &str,
    number: u64,
    pull: &forge_bridge::port::PullFull,
) -> IntentOutcome {
    let input = super::forge_gate::merge_input_from_pull(pull, false);
    let (verdict, reasons) =
        ledger_sentinel::review_gate::evaluate(&ledger_sentinel::policy::Policy::default(), &input);
    let verdict_str = match verdict {
        ledger_sentinel::review_gate::Verdict::Pass => "pass",
        ledger_sentinel::review_gate::Verdict::Warn => "warn",
        ledger_sentinel::review_gate::Verdict::Block => "block",
    };
    let summary = if reasons.is_empty() {
        format!("recheck {repo}#{number}: {verdict_str}")
    } else {
        format!(
            "recheck {repo}#{number}: {verdict_str} — {}",
            reasons.join("; ")
        )
    };
    let out = applied(id, summary);
    let _ = store
        .record(id, "retry_worker", repo, number as i64, &out.summary)
        .await;
    audit(
        audit_log,
        repo,
        "retry_worker",
        &out.summary,
        verdict_str,
        id,
    )
    .await;
    out
}

fn applied(id: &str, summary: String) -> IntentOutcome {
    IntentOutcome {
        idempotency_id: id.into(),
        applied: true,
        summary,
        choices: vec![],
    }
}

fn conflict(id: &str, reason: String) -> IntentOutcome {
    IntentOutcome {
        idempotency_id: id.into(),
        applied: false,
        summary: reason,
        choices: vec!["drop".into(), "escalate".into()],
    }
}

fn unsupported(id: &str, reason: &str) -> IntentOutcome {
    IntentOutcome {
        idempotency_id: id.into(),
        applied: false,
        summary: reason.into(),
        choices: vec![],
    }
}

async fn audit(
    audit: &Option<Arc<tokio::sync::Mutex<ledger_sentinel::ledger::Ledger>>>,
    repo: &str,
    kind: &str,
    summary: &str,
    verdict: &str,
    id: &str,
) {
    let Some(ledger) = audit else { return };
    let attribution = ledger_sentinel::port::Attribution {
        actor: "phone".into(),
        agent: "field-deck".into(),
        skill: None,
        mcp_server: None,
    };
    let refs = vec![format!("intent:{id}")];
    let _ = ledger
        .lock()
        .await
        .append(attribution, repo, kind, summary, verdict, &refs)
        .await;
}

/// `POST /api/v1/intents`: phone approve/comment with idempotency.
/// 200 applied (or replayed), 409 conflict with choices, 422 for kinds
/// this daemon cannot execute yet, 503 when replay is unconfigured.
pub(crate) async fn route(
    State(state): State<super::api::AppState>,
    Json(body): Json<IntentRequest>,
) -> impl IntoResponse {
    let (Some(store), Some(forge)) = (&state.intents, &state.forge) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "intent replay unavailable"})),
        );
    };
    let mut clients = std::collections::HashMap::new();
    for cfg in &state.forge_cfgs {
        if let Some(client) = forge.client(&cfg.kind) {
            clients.insert(cfg.kind.clone(), client);
        }
    }
    let out = dispatch(
        store,
        &clients,
        &state.forge_cfgs,
        state.audit.clone(),
        &body,
    )
    .await;
    state.hub.publish("intent", &out.summary).await;
    let status = if out.applied {
        StatusCode::OK
    } else if out.choices.is_empty() {
        StatusCode::UNPROCESSABLE_ENTITY
    } else {
        StatusCode::CONFLICT
    };
    (status, Json(serde_json::json!(out)))
}
