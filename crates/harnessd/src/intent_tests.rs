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
        Ok(PullFull {
            number: n,
            state: self.state.clone(),
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
) -> (
    IntentStore,
    HashMap<String, Arc<dyn Forge + Send + Sync>>,
    Vec<crate::config_sections::ForgeCfg>,
    Arc<Fake>,
) {
    let store = IntentStore::open("sqlite::memory:").await.unwrap();
    let fake = Arc::new(Fake {
        state: state.into(),
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
    let (store, clients, forges, counts) = deps("open").await;
    let first = dispatch(&store, &clients, &forges, None, &req("a1", "approve_pr")).await;
    assert!(first.applied);
    let second = dispatch(&store, &clients, &forges, None, &req("a1", "approve_pr")).await;
    assert!(second.applied);
    assert_eq!(*counts.approves.lock().unwrap(), 1);
}

#[tokio::test]
async fn merged_target_conflicts_without_executing() {
    let (store, clients, forges, counts) = deps("merged").await;
    let out = dispatch(&store, &clients, &forges, None, &req("m1", "approve_pr")).await;
    assert!(!out.applied);
    assert_eq!(out.choices, vec!["drop", "escalate"]);
    assert_eq!(*counts.approves.lock().unwrap(), 0);
}

#[tokio::test]
async fn comment_applies_and_retry_is_unsupported() {
    let (store, clients, forges, counts) = deps("open").await;
    let out = dispatch(&store, &clients, &forges, None, &req("c1", "comment")).await;
    assert!(out.applied);
    assert_eq!(*counts.comments.lock().unwrap(), 1);
    let retry = dispatch(&store, &clients, &forges, None, &req("r1", "retry_worker")).await;
    assert!(!retry.applied);
    assert!(retry.choices.is_empty());
    let unknown = dispatch(&store, &clients, &forges, None, &req("u1", "nope")).await;
    assert!(!unknown.applied);
}

#[tokio::test]
async fn unwatched_repo_conflicts() {
    let (store, clients, _forges, counts) = deps("open").await;
    let mut bad = req("w1", "comment");
    bad.repo = Some("other/r".into());
    let out = dispatch(&store, &clients, &[], None, &bad).await;
    assert!(!out.applied);
    assert_eq!(*counts.comments.lock().unwrap(), 0);
}
