//! Gitea adapter: OpenAPI-driven REST against a self-host URL. Same
//! [`Forge`](super::port::Forge) trait as GitHub; the token travels as
//! `Authorization: token …` (Gitea's scheme).
use super::http;
use super::parse;
use super::port::{
    Check, Comment, Forge, Issue, IssueFull, MergeReport, NewIssue, NewPull, Page, PullFull, Repo,
    Review, ReviewThread, Search,
};
use anyhow::{Context, Result};
use async_trait::async_trait;

pub struct Gitea {
    base: String,
    token: String,
    http_client: reqwest::Client,
}

impl Gitea {
    pub fn new(base_url: &str, token: &str) -> Result<Self> {
        Ok(Self {
            base: format!("{}/api/v1", base_url.trim_end_matches('/')),
            token: token.into(),
            http_client: http::client()?,
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    async fn send(&self, req: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let auth = format!("token {}", self.token);
        http::send(&auth, req).await
    }

    fn link_of(resp: &reqwest::Response) -> Option<String> {
        http::next_cursor(resp.headers().get("link")?.to_str().ok())
    }

    async fn get_json(&self, path: &str) -> Result<serde_json::Value> {
        let resp = self.send(self.http_client.get(self.url(path))).await?;
        http::ok_json(resp, "gitea").await
    }

    /// Combined commit status mapped to checks (`state` is status and conclusion).
    async fn status_checks(&self, repo: &str, sha: &str) -> Result<Vec<Check>> {
        let doc = self
            .get_json(&format!("/repos/{repo}/commits/{sha}/status"))
            .await?;
        Ok(doc
            .get("statuses")
            .and_then(|v| v.as_array())
            .context("status envelope")?
            .iter()
            .map(|s| Check {
                name: s
                    .get("context")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .into(),
                status: s
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .into(),
                conclusion: s
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .into(),
            })
            .collect())
    }

    /// No thread object: each pull review is one thread, resolved on approve.
    async fn review_threads(&self, repo: &str, number: u64) -> Result<Vec<ReviewThread>> {
        let doc = self
            .get_json(&format!("/repos/{repo}/pulls/{number}/reviews"))
            .await?;
        Ok(doc
            .as_array()
            .context("reviews array")?
            .iter()
            .map(|r| {
                let state = r.get("state").and_then(|v| v.as_str()).unwrap_or("");
                ReviewThread {
                    id: r.get("id").map(|x| x.to_string()).unwrap_or_default(),
                    resolved: state == "APPROVED",
                    comments: vec![Comment {
                        id: r.get("id").map(|x| x.to_string()).unwrap_or_default(),
                        author: r
                            .pointer("/user/login")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .into(),
                        body: r.get("body").and_then(|v| v.as_str()).unwrap_or("").into(),
                    }],
                }
            })
            .collect())
    }

    /// Gitea creates issues with label *ids*: resolve names first.
    async fn label_ids(&self, repo: &str, names: &[String]) -> Result<Vec<i64>> {
        if names.is_empty() {
            return Ok(vec![]);
        }
        let doc = self.get_json(&format!("/repos/{repo}/labels")).await?;
        let all = doc.as_array().context("labels array")?;
        Ok(all
            .iter()
            .filter(|l| {
                l.get("name")
                    .and_then(|v| v.as_str())
                    .is_some_and(|n| names.iter().any(|w| w == n))
            })
            .filter_map(|l| l.get("id").and_then(|v| v.as_i64()))
            .collect())
    }
}

#[async_trait]
impl Forge for Gitea {
    fn name(&self) -> &'static str {
        "gitea"
    }

    async fn repos(&self, q: &Search) -> Result<Page<Repo>> {
        let per = q.per_page.clamp(1, 50);
        let path = format!("/repos/search?q={}&limit={per}", http::url_encode(&q.query));
        let resp = self.send(self.http_client.get(self.url(&path))).await?;
        let next = Self::link_of(&resp);
        let doc = http::ok_json(resp, "gitea").await?;
        let items = doc
            .get("data")
            .and_then(|v| v.as_array())
            .context("search envelope")?
            .iter()
            .map(parse::parse_repo)
            .collect();
        Ok(Page { items, next })
    }

    async fn issues(&self, repo: &str, q: &Search) -> Result<Page<Issue>> {
        let per = q.per_page.clamp(1, 50);
        let page: u64 = q.cursor.as_deref().unwrap_or("1").parse().unwrap_or(1);
        let path = format!("/repos/{repo}/issues?state=all&limit={per}&page={page}");
        let resp = self.send(self.http_client.get(self.url(&path))).await?;
        let next = Self::link_of(&resp).or(Some((page + 1).to_string()));
        let doc = http::ok_json(resp, "gitea").await?;
        let items = doc
            .as_array()
            .context("issues array")?
            .iter()
            .map(parse::parse_issue)
            .collect();
        Ok(Page { items, next })
    }

    async fn issue_detail(&self, repo: &str, number: u64) -> Result<IssueFull> {
        let issue = parse::parse_issue(
            &self
                .get_json(&format!("/repos/{repo}/issues/{number}"))
                .await?,
        );
        let doc = self
            .get_json(&format!("/repos/{repo}/issues/{number}/comments"))
            .await?;
        let comments = doc
            .as_array()
            .context("comments array")?
            .iter()
            .map(parse::parse_comment)
            .collect();
        Ok(IssueFull { issue, comments })
    }

    async fn open_issue(&self, repo: &str, issue: &NewIssue) -> Result<Issue> {
        let ids = self.label_ids(repo, &issue.labels).await?;
        let payload = serde_json::json!({"title": issue.title, "body": issue.body, "labels": ids});
        let resp = self
            .send(
                self.http_client
                    .post(self.url(&format!("/repos/{repo}/issues")))
                    .json(&payload),
            )
            .await?;
        Ok(parse::parse_issue(&http::ok_json(resp, "gitea").await?))
    }

    async fn comment(&self, repo: &str, number: u64, body: &str) -> Result<Comment> {
        let payload = serde_json::json!({"body": body});
        let resp = self
            .send(
                self.http_client
                    .post(self.url(&format!("/repos/{repo}/issues/{number}/comments")))
                    .json(&payload),
            )
            .await?;
        Ok(parse::parse_comment(&http::ok_json(resp, "gitea").await?))
    }

    async fn pull(&self, repo: &str, number: u64) -> Result<PullFull> {
        let doc = self
            .get_json(&format!("/repos/{repo}/pulls/{number}"))
            .await?;
        let mut pull = parse::parse_pull(&doc);
        pull.checks = self
            .status_checks(repo, &pull.head_sha)
            .await
            .unwrap_or_default();
        pull.threads = self.review_threads(repo, number).await.unwrap_or_default();
        Ok(pull)
    }

    async fn open_pull(&self, repo: &str, pull: &NewPull) -> Result<PullFull> {
        let resp = self
            .send(
                self.http_client
                    .post(self.url(&format!("/repos/{repo}/pulls")))
                    .json(pull),
            )
            .await?;
        Ok(parse::parse_pull(&http::ok_json(resp, "gitea").await?))
    }

    async fn merge(&self, repo: &str, number: u64, method: &str) -> Result<MergeReport> {
        let op = match method {
            "squash" => "squash",
            "rebase" => "rebase",
            _ => "merge",
        };
        let payload = serde_json::json!({"Do": op});
        let resp = self
            .send(
                self.http_client
                    .post(self.url(&format!("/repos/{repo}/pulls/{number}/merge")))
                    .json(&payload),
            )
            .await?;
        let status = resp.status();
        let doc = http::ok_json(resp, "gitea").await?;
        Ok(MergeReport {
            merged: status.is_success(),
            sha: doc
                .get("merge_commit_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            message: String::new(),
        })
    }

    async fn checks(&self, repo: &str, sha: &str) -> Result<Vec<Check>> {
        self.status_checks(repo, sha).await
    }

    async fn request_review(
        &self,
        repo: &str,
        number: u64,
        reviewers: &[String],
    ) -> Result<Review> {
        let payload = serde_json::json!({"reviewers": reviewers});
        let resp = self
            .send(
                self.http_client
                    .post(self.url(&format!("/repos/{repo}/pulls/{number}/requested_reviewers")))
                    .json(&payload),
            )
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("gitea {}: review request failed", resp.status());
        }
        Ok(Review {
            id: format!("{repo}#{number}"),
            author: reviewers.first().cloned().unwrap_or_default(),
            state: "requested".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_builds_and_names() {
        let g = Gitea::new("https://git.example.com", "t").unwrap();
        assert_eq!(g.name(), "gitea");
        assert!(g.url("/x").starts_with("https://git.example.com/api/v1"));
    }
}
