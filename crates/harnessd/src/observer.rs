//! Forge observer: poll watched repos, mirror PR/check/review facts.
//! One daemon task per watched repo at the configured cadence. Remote
//! text is secret-masked before it lands in reasons or logs; tokens
//! never leave the call site that resolved them.
use super::forge_facts::ForgeFacts;
use anyhow::Result;
use forge_bridge::mask::mask_secrets;
use forge_bridge::port::{Forge, Search};
use tokio::sync::Semaphore;

/// Structured follow-up for the owning worker: failed CI or open
/// review threads, with the exact blocker named. Logs get trimmed
/// downstream; full bodies stay recallable on the forge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowUp {
    pub repo: String,
    pub number: u64,
    pub reason: String,
}

/// Max concurrent PR detail fetches per poll (bounds forge burst).
pub const DETAIL_CONCURRENCY: usize = 4;

/// One poll of a single repo: discover open PRs, refresh their facts,
/// and report what needs a human or worker. Pure against any `Forge`.
/// Concurrent repo polls are bounded by DETAIL_CONCURRENCY; per-PR detail
/// stays sequential inside a poll for forge rate limits.
pub async fn poll_once(
    forge: &(dyn Forge + Sync),
    repo: &str,
    facts: &ForgeFacts,
) -> Result<Vec<FollowUp>> {
    let mut follow_ups = vec![];
    let mut cursor: Option<String> = None;
    loop {
        let page = forge
            .pulls(
                repo,
                &Search {
                    query: String::new(),
                    per_page: 50,
                    cursor: cursor.clone(),
                },
            )
            .await?;
        for summary in &page.items {
            facts
                .upsert_pr(
                    repo,
                    summary.number as i64,
                    &summary.title,
                    "open",
                    &summary.head_sha,
                    false,
                )
                .await?;
            let detail = forge.pull(repo, summary.number).await?;
            facts
                .upsert_pr(
                    repo,
                    detail.number as i64,
                    &detail.title,
                    &detail.state,
                    &detail.head_sha,
                    detail.mergeable,
                )
                .await?;
            let checks: Vec<(String, String, String)> = detail
                .checks
                .iter()
                .map(|c| (c.name.clone(), c.status.clone(), c.conclusion.clone()))
                .collect();
            facts
                .replace_checks(repo, &detail.head_sha, &checks)
                .await?;
            let threads: Vec<(String, bool)> = detail
                .threads
                .iter()
                .map(|t| (t.id.clone(), t.resolved))
                .collect();
            facts
                .replace_threads(repo, detail.number as i64, &threads)
                .await?;
            let failed = facts.failing_checks(repo, &detail.head_sha).await?;
            if !failed.is_empty() {
                follow_ups.push(FollowUp {
                    repo: repo.into(),
                    number: detail.number,
                    reason: mask_secrets(&format!("failed checks: {}", failed.join(", "))),
                });
            }
            let open = facts.unresolved_threads(repo, detail.number as i64).await?;
            if open > 0 {
                follow_ups.push(FollowUp {
                    repo: repo.into(),
                    number: detail.number,
                    reason: format!("{open} unresolved review threads"),
                });
            }
        }
        cursor = page.next;
        if cursor.is_none() {
            break;
        }
    }
    Ok(follow_ups)
}

/// Spawn one background task per watched repo. A missing token or
/// unknown kind skips loudly but never fails boot; empty config idles.
pub async fn spawn_forges(cfgs: &[super::config::ForgeCfg], data_dir: &str) {
    let Ok(facts) = super::forge_facts::ForgeFacts::open(&format!(
        "sqlite://{data_dir}/db/harness.db?mode=rwc"
    ))
    .await
    else {
        return;
    };
    let poll_slots = std::sync::Arc::new(Semaphore::new(DETAIL_CONCURRENCY));
    for forge_cfg in cfgs {
        let env_refs: Vec<&str> = forge_cfg.env.iter().map(String::as_str).collect();
        let account = format!("forge-{}", forge_cfg.kind);
        let Some(token) = super::keys::resolve_all(None, &account, &env_refs) else {
            eprintln!("observer: no token for {}", forge_cfg.kind);
            continue;
        };
        let client: Option<Box<dyn forge_bridge::port::Forge + Send + Sync>> =
            match forge_cfg.kind.as_str() {
                "github" => {
                    let base = if forge_cfg.base_url.is_empty() {
                        "https://api.github.com"
                    } else {
                        &forge_cfg.base_url
                    };
                    forge_bridge::github::GitHub::new(base, &token)
                        .ok()
                        .map(|c| Box::new(c) as _)
                }
                "gitea" => {
                    if forge_cfg.base_url.is_empty() {
                        eprintln!("observer: gitea needs base_url");
                        continue;
                    }
                    forge_bridge::gitea::Gitea::new(&forge_cfg.base_url, &token)
                        .ok()
                        .map(|c| Box::new(c) as _)
                }
                other => {
                    eprintln!("observer: unknown forge kind {other}");
                    continue;
                }
            };
        let Some(client) = client else { continue };
        let client = std::sync::Arc::new(client);
        for repo in forge_cfg.repos.clone() {
            let facts = facts.clone();
            let task_client = client.clone();
            let interval = forge_cfg.interval();
            let poll_slots = poll_slots.clone();
            let mask = forge_bridge::mask::mask_secrets;
            tokio::spawn(async move {
                loop {
                    let _permit = poll_slots
                        .clone()
                        .acquire_owned()
                        .await
                        .expect("semaphore closed");
                    match poll_once(&**task_client, &repo, &facts).await {
                        Ok(items) => {
                            for f in items {
                                eprintln!("observer: {}#{} {}", f.repo, f.number, mask(&f.reason));
                            }
                        }
                        Err(e) => {
                            eprintln!("observer: {repo} poll failed: {}", mask(&e.to_string()))
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use forge_bridge::port::*;

    struct FakeForge;
    #[async_trait]
    impl Forge for FakeForge {
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
            Ok(Comment::default())
        }
        async fn pulls(&self, _r: &str, _q: &Search) -> anyhow::Result<Page<PullSummary>> {
            Ok(Page {
                items: vec![PullSummary {
                    number: 7,
                    title: "fix".into(),
                    state: "open".into(),
                    head_sha: "abc".into(),
                }],
                next: None,
            })
        }
        async fn pull(&self, _r: &str, _n: u64) -> anyhow::Result<PullFull> {
            Ok(PullFull {
                number: 7,
                title: "fix".into(),
                state: "open".into(),
                head_sha: "abc".into(),
                mergeable: true,
                checks: vec![Check {
                    name: "ci".into(),
                    status: "completed".into(),
                    conclusion: "failure".into(),
                }],
                threads: vec![ReviewThread {
                    id: "t1".into(),
                    resolved: false,
                    comments: vec![],
                }],
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
        async fn approve(&self, _r: &str, n: u64, _b: &str) -> anyhow::Result<Review> {
            Ok(Review {
                id: format!("approved-{n}"),
                state: "APPROVED".into(),
                ..Default::default()
            })
        }
    }

    #[tokio::test]
    async fn poll_routes_failures_and_threads() {
        let facts = ForgeFacts::open("sqlite::memory:").await.unwrap();
        let follow_ups = poll_once(&FakeForge, "o/r", &facts).await.unwrap();
        assert_eq!(follow_ups.len(), 2);
        assert!(follow_ups[0].reason.starts_with("failed checks: ci"));
        assert_eq!(facts.unresolved_threads("o/r", 7).await.unwrap(), 1);
    }
}
