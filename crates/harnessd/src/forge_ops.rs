//! Forge op dispatch: one JSON op in, compact masked text out.
//! Client lifecycle (build/add) lives in `forge_exec.rs`; this file
//! only translates ops to trait calls and shapes results for the model.
use forge_bridge::mask::mask_secrets;
use forge_bridge::port::{CapabilityLease, Forge, NewIssue, NewPull, Search};
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) async fn run_op(
    clients: &HashMap<String, Arc<dyn Forge + Send + Sync>>,
    lease: &CapabilityLease,
    op: &str,
    input: &str,
) -> anyhow::Result<String> {
    let v: serde_json::Value = serde_json::from_str(input)?;
    let repo = v.get("repo").and_then(|x| x.as_str()).unwrap_or("");
    let client = clients
        .get(&lease.forge)
        .ok_or_else(|| anyhow::anyhow!("forge: no client for '{}'", lease.forge))?;
    let out = match op {
        "issues" => {
            let page = client.issues(repo, &Search::default()).await?;
            page.items
                .iter()
                .map(|i| format!("#{} {} [{}]", i.number, i.title, i.state))
                .collect::<Vec<_>>()
                .join("\n")
        }
        "issue" => {
            let n = v.get("number").and_then(|x| x.as_u64()).unwrap_or(0);
            let full = client.issue_detail(repo, n).await?;
            format!(
                "#{} {}\n{} comments",
                full.issue.number,
                full.issue.title,
                full.comments.len()
            )
        }
        "open_issue" => {
            let issue = NewIssue {
                title: str_of(&v, "title"),
                body: str_of(&v, "body"),
                labels: vec![],
            };
            let opened = client.open_issue(repo, &issue).await?;
            format!("opened #{} {}", opened.number, opened.title)
        }
        "comment" => {
            let n = v.get("number").and_then(|x| x.as_u64()).unwrap_or(0);
            let c = client.comment(repo, n, &str_of(&v, "body")).await?;
            format!("comment {}", c.id)
        }
        "pulls" => {
            let page = client.pulls(repo, &Search::default()).await?;
            page.items
                .iter()
                .map(|p| format!("#{} {} [{}]", p.number, p.title, p.state))
                .collect::<Vec<_>>()
                .join("\n")
        }
        "pull" => {
            let n = v.get("number").and_then(|x| x.as_u64()).unwrap_or(0);
            let p = client.pull(repo, n).await?;
            format!(
                "#{} {} [{}] {} mergeable={} checks={} threads={}",
                p.number,
                p.title,
                p.state,
                p.head_sha,
                p.mergeable,
                p.checks.len(),
                p.threads.len()
            )
        }
        "open_pull" => {
            let pull = NewPull {
                title: str_of(&v, "title"),
                head: str_of(&v, "head"),
                base: str_of(&v, "base"),
                body: str_of(&v, "body"),
            };
            let opened = client.open_pull(repo, &pull).await?;
            format!("opened #{} {}", opened.number, opened.title)
        }
        "merge" => {
            let n = v.get("number").and_then(|x| x.as_u64()).unwrap_or(0);
            let r = client
                .merge(
                    repo,
                    n,
                    v.get("method").and_then(|x| x.as_str()).unwrap_or("merge"),
                )
                .await?;
            format!("merged={} sha={}", r.merged, r.sha.unwrap_or_default())
        }
        "checks" => {
            let sha = match v.get("sha").and_then(|x| x.as_str()) {
                Some(s) => s.to_string(),
                None => {
                    let n = v.get("number").and_then(|x| x.as_u64()).unwrap_or(0);
                    client.pull(repo, n).await?.head_sha
                }
            };
            client
                .checks(repo, &sha)
                .await?
                .iter()
                .map(|c| format!("{}: {}/{}", c.name, c.status, c.conclusion))
                .collect::<Vec<_>>()
                .join("\n")
        }
        "review" => {
            let n = v.get("number").and_then(|x| x.as_u64()).unwrap_or(0);
            let reviewers: Vec<String> = v
                .get("reviewers")
                .and_then(|x| x.as_array())
                .map(|xs| {
                    xs.iter()
                        .filter_map(|r| r.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let r = client.request_review(repo, n, &reviewers).await?;
            format!("review {} {}", r.id, r.state)
        }
        other => anyhow::bail!("forge: unknown op '{other}'"),
    };
    Ok(mask_secrets(&out))
}

fn str_of(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use forge_bridge::port::*;

    struct FakeClient;
    #[async_trait]
    impl Forge for FakeClient {
        fn name(&self) -> &'static str {
            "fake"
        }
        async fn repos(&self, _q: &Search) -> anyhow::Result<Page<Repo>> {
            Ok(Page::default())
        }
        async fn issues(&self, _r: &str, _q: &Search) -> anyhow::Result<Page<Issue>> {
            Ok(Page {
                items: vec![Issue {
                    number: 1,
                    title: "t".into(),
                    state: "open".into(),
                    ..Default::default()
                }],
                next: None,
            })
        }
        async fn issue_detail(&self, _r: &str, n: u64) -> anyhow::Result<IssueFull> {
            Ok(IssueFull {
                issue: Issue {
                    number: n,
                    ..Default::default()
                },
                comments: vec![],
            })
        }
        async fn open_issue(&self, _r: &str, i: &NewIssue) -> anyhow::Result<Issue> {
            Ok(Issue {
                number: 9,
                title: i.title.clone(),
                ..Default::default()
            })
        }
        async fn comment(&self, _r: &str, _n: u64, _b: &str) -> anyhow::Result<Comment> {
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
                head_sha: "abc".into(),
                ..Default::default()
            })
        }
        async fn open_pull(&self, _r: &str, p: &NewPull) -> anyhow::Result<PullFull> {
            Ok(PullFull {
                number: 4,
                title: p.title.clone(),
                ..Default::default()
            })
        }
        async fn merge(&self, _r: &str, _n: u64, _m: &str) -> anyhow::Result<MergeReport> {
            Ok(MergeReport {
                merged: true,
                sha: Some("abc".into()),
                message: String::new(),
            })
        }
        async fn checks(&self, _r: &str, _s: &str) -> anyhow::Result<Vec<Check>> {
            Ok(vec![])
        }
        async fn request_review(&self, _r: &str, _n: u64, _v: &[String]) -> anyhow::Result<Review> {
            Ok(Review::default())
        }
    }

    fn clients() -> HashMap<String, Arc<dyn Forge + Send + Sync>> {
        let mut map = HashMap::new();
        map.insert(
            "fake".into(),
            Arc::new(FakeClient) as Arc<dyn Forge + Send + Sync>,
        );
        map
    }

    fn lease() -> CapabilityLease {
        CapabilityLease::mint("fake", "o/r", vec!["forge.read".into()])
    }

    #[tokio::test]
    async fn routes_ops_and_rejects_unknown() {
        let clients = clients();
        let out = run_op(
            &clients,
            &lease(),
            "issues",
            r#"{"op":"issues","repo":"o/r"}"#,
        )
        .await
        .unwrap();
        assert!(out.contains("#1 t"));
        assert!(
            run_op(&clients, &lease(), "nope", r#"{"op":"nope","repo":"o/r"}"#)
                .await
                .is_err()
        );
    }
}
