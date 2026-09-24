//! Intent replay tests: Fake forge behind dispatch (see intents.rs).
//! Compiled only for tests; main.rs gates this module with cfg(test).
use crate::intent_store::IntentStore;
use crate::intents::{dispatch, IntentRequest};
use async_trait::async_trait;
use forge_bridge::port::*;
use std::collections::HashMap;
use std::sync::Arc;

struct Fake {
    state: String,
    red: bool,
    approves: std::sync::Mutex<usize>,
    comments: std::sync::Mutex<usize>,
}

#[async_trait]
impl Forge for Fake {
    fn name(&self) -> &'static str {
        "fake"
    }
    async fn repos(&self, _q: &Search) -> anyhow::Result<Page<Repo>> {
        Ok(Page::default())
    }
    async fn issues(&self, _r: &str, _q: &Search) -> anyhow::Result<Page<Issue>> {
        Ok(Page::default())
    }
    async fn issue_detail(&self, _r: &str, _n: u64) -> anyhow::Result<IssueFull> {
        Ok(IssueFull::default())
    }
    async fn open_issue(&self, _r: &str, _i: &NewIssue) -> anyhow::Result<Issue> {
        Ok(Issue::default())
    }
    async fn comment(&self, _r: &str, _n: u64, _b: &str) -> anyhow::Result<Comment> {
        *self.comments.lock().unwrap() += 1;
        Ok(Comment {
            id: "c1".into(),
            ..Default::default()
        })
    }
    async fn pulls(&self, _r: &str, _q: &Search) -> anyhow::Result<Page<PullSummary>> {
        Ok(Page::default())
    }
    async fn pull(&self, _r: &str, n: u64) -> anyhow::Result<PullFull> {
        let checks = if self.red {
            vec![Check {
                name: "ci".into(),
                status: "completed".into(),
                conclusion: "failure".into(),
            }]
        } else {
            vec![]
        };
        Ok(PullFull {
            number: n,
            state: self.state.clone(),
            checks,
            ..Default::default()
        })
    }
    async fn open_pull(&self, _r: &str, _p: &NewPull) -> anyhow::Result<PullFull> {
        Ok(PullFull::default())
    }
    async fn merge(&self, _r: &str, _n: u64, _m: &str) -> anyhow::Result<MergeReport> {
        Ok(MergeReport::default())
    }
    async fn checks(&self, _r: &str, _s: &str) -> anyhow::Result<Vec<Check>> {
        Ok(vec![])
    }
    async fn request_review(&self, _r: &str, _n: u64, _v: &[String]) -> anyhow::Result<Review> {
        Ok(Review::default())
    }
    async fn approve(&self, _r: &str, _n: u64, _b: &str) -> anyhow::Result<Review> {
        *self.approves.lock().unwrap() += 1;
        Ok(Review {
            id: "r1".into(),
            state: "APPROVED".into(),
            ..Default::default()
        })
    }
}

async fn deps(
    state: &str,
    red: bool,
) -> (
    IntentStore,
    HashMap<String, Arc<dyn Forge + Send + Sync>>,
    Vec<crate::config_sections::ForgeCfg>,
    Arc<Fake>,
) {
    let store = IntentStore::open("sqlite::memory:").await.unwrap();
    let fake = Arc::new(Fake {
        state: state.into(),
        red,
        approves: std::sync::Mutex::new(0),
        comments: std::sync::Mutex::new(0),
    });
    let mut clients = HashMap::new();
    clients.insert("fake".into(), fake.clone() as Arc<dyn Forge + Send + Sync>);
    let forges = vec![crate::config_sections::ForgeCfg {
        kind: "fake".into(),
        base_url: String::new(),
        env: vec![],
        repos: vec!["o/r".into()],
        poll_secs: 0,
        writes: vec![],
    }];
    (store, clients, forges, fake)
}

fn req(id: &str, kind: &str) -> IntentRequest {
    IntentRequest {
        idempotency_id: Some(id.into()),
        kind: Some(kind.into()),
        repo: Some("o/r".into()),
        number: Some(7),
        body: Some("lgtm".into()),
    }
}

#[tokio::test]
async fn approve_applies_and_dedups_replays() {
    let (store, clients, forges, counts) = deps("open", false).await;
    let first = dispatch(&store, &clients, &forges, None, &req("a1", "approve_pr")).await;
    assert!(first.applied);
    let second = dispatch(&store, &clients, &forges, None, &req("a1", "approve_pr")).await;
    assert!(second.applied);
    assert_eq!(*counts.approves.lock().unwrap(), 1);
}

#[tokio::test]
async fn merged_target_conflicts_without_executing() {
    let (store, clients, forges, counts) = deps("merged", false).await;
    let out = dispatch(&store, &clients, &forges, None, &req("m1", "approve_pr")).await;
    assert!(!out.applied);
    assert_eq!(out.choices, vec!["drop", "escalate"]);
    assert_eq!(*counts.approves.lock().unwrap(), 0);
}

#[tokio::test]
async fn comment_applies_and_retry_rechecks() {
    let (store, clients, forges, counts) = deps("open", false).await;
    let out = dispatch(&store, &clients, &forges, None, &req("c1", "comment")).await;
    assert!(out.applied);
    assert_eq!(*counts.comments.lock().unwrap(), 1);
    let retry = dispatch(&store, &clients, &forges, None, &req("r1", "retry_worker")).await;
    assert!(retry.applied);
    assert!(retry.summary.contains("recheck"));
    let unknown = dispatch(&store, &clients, &forges, None, &req("u1", "nope")).await;
    assert!(!unknown.applied);
}

#[tokio::test]
async fn unwatched_repo_conflicts() {
    let (store, clients, _forges, counts) = deps("open", false).await;
    let mut bad = req("w1", "comment");
    bad.repo = Some("other/r".into());
    let out = dispatch(&store, &clients, &[], None, &bad).await;
    assert!(!out.applied);
    assert_eq!(*counts.comments.lock().unwrap(), 0);
}

#[tokio::test]
async fn conflicts_land_in_the_audit_ledger() {
    let dir = std::env::temp_dir().join(format!("hx-intent-audit-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(dir.join("db")).unwrap();
    let ledger = ledger_sentinel::ledger::Ledger::open(dir.to_str().unwrap())
        .await
        .unwrap();
    let ledger = Arc::new(tokio::sync::Mutex::new(ledger));
    let (store, clients, forges, _counts) = deps("merged", false).await;
    let out = dispatch(
        &store,
        &clients,
        &forges,
        Some(ledger.clone()),
        &req("m9", "approve_pr"),
    )
    .await;
    assert!(!out.applied);
    let rows = ledger
        .lock()
        .await
        .query(&ledger_sentinel::ledger::AuditFilter {
            actor: None,
            repo: None,
            kind: Some("approve_pr".into()),
            since: None,
            until: None,
            limit: 10,
        })
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].verdict, "conflict");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn recheck_reports_gate_verdict_and_dedups() {
    let (store, clients, forges, _counts) = deps("open", true).await;
    let first = dispatch(&store, &clients, &forges, None, &req("k1", "retry_worker")).await;
    assert!(first.applied);
    assert!(first.summary.contains("failing checks"));
    let second = dispatch(&store, &clients, &forges, None, &req("k1", "retry_worker")).await;
    assert!(second.applied);
    assert!(second.summary.contains("replay:"));
}
