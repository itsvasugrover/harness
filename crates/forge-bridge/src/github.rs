//! GitHub adapter: REST + GraphQL hybrid (search via REST, review
//! threads via GraphQL). The PAT arrives resolved from the daemon; this
//! client never reads keychains or env itself.
use super::http;
use super::parse;
use super::port::{
    Check, Comment, Forge, Issue, IssueFull, MergeReport, NewIssue, NewPull, Page, PullFull,
    PullSummary, Repo, Review, ReviewThread, Search,
};
use anyhow::{Context, Result};
use async_trait::async_trait;

pub struct GitHub {
    base: String,
    token: String,
    http_client: reqwest::Client,
}

impl GitHub {
    pub fn new(base_url: &str, token: &str) -> Result<Self> {
        Ok(Self {
            base: base_url.trim_end_matches('/').into(),
            token: token.into(),
            http_client: http::client()?,
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    async fn send(&self, req: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let auth = format!("Bearer {}", self.token);
        http::send(&auth, req).await
    }

    fn link_of(resp: &reqwest::Response) -> Option<String> {
        http::next_cursor(resp.headers().get("link")?.to_str().ok())
    }

    async fn get_json(&self, path: &str) -> Result<serde_json::Value> {
        let resp = self.send(self.http_client.get(self.url(path))).await?;
        http::ok_json(resp, "github").await
    }

    /// Review threads live only in GraphQL: one query, first page.
    pub async fn review_threads(&self, repo: &str, number: u64) -> Result<Vec<ReviewThread>> {
        let (owner, name) = http::split_repo(repo)?;
        let query = "query($o:String!,$n:String!,$p:Int!){repository(owner:$o,name:$n){pullRequest(number:$p){reviewThreads(first:20){nodes{id,isResolved,comments(first:20){nodes{id,author{login},body}}}}}}}";
        let body =
            serde_json::json!({"query": query, "variables": {"o": owner, "n": name, "p": number}});
        let resp = self
            .send(self.http_client.post(self.url("/graphql")).json(&body))
            .await?;
        let doc = http::ok_json(resp, "github").await?;
        let nodes = doc
            .pointer("/data/repository/pullRequest/reviewThreads/nodes")
            .and_then(|v| v.as_array())
            .context("graphql threads shape")?;
        Ok(nodes.iter().map(parse::parse_thread).collect())
    }
}

#[async_trait]
impl Forge for GitHub {
    fn name(&self) -> &'static str {
        "github"
    }

    async fn repos(&self, q: &Search) -> Result<Page<Repo>> {
        let per = q.per_page.clamp(1, 100);
        let path = format!(
            "/search/repositories?q={}&per_page={per}",
            http::url_encode(&q.query)
        );
        let resp = self.send(self.http_client.get(self.url(&path))).await?;
        let next = Self::link_of(&resp);
        let doc = http::ok_json(resp, "github").await?;
        let items = doc
            .get("items")
            .and_then(|v| v.as_array())
            .context("search envelope")?
            .iter()
            .map(parse::parse_repo)
            .collect();
        Ok(Page { items, next })
    }

    async fn issues(&self, repo: &str, q: &Search) -> Result<Page<Issue>> {
        let per = q.per_page.clamp(1, 100);
        let page: u64 = q.cursor.as_deref().unwrap_or("1").parse().unwrap_or(1);
        let path = format!("/repos/{repo}/issues?state=all&per_page={per}&page={page}");
        let resp = self.send(self.http_client.get(self.url(&path))).await?;
        let next = Self::link_of(&resp).or(Some((page + 1).to_string()));
        let doc = http::ok_json(resp, "github").await?;
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
            .get_json(&format!(
                "/repos/{repo}/issues/{number}/comments?per_page=100"
            ))
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
        let resp = self
            .send(
                self.http_client
                    .post(self.url(&format!("/repos/{repo}/issues")))
                    .json(issue),
            )
            .await?;
        Ok(parse::parse_issue(&http::ok_json(resp, "github").await?))
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
        Ok(parse::parse_comment(&http::ok_json(resp, "github").await?))
    }

    async fn pulls(&self, repo: &str, q: &Search) -> Result<Page<PullSummary>> {
        let per = q.per_page.clamp(1, 100);
        let page: u64 = q.cursor.as_deref().unwrap_or("1").parse().unwrap_or(1);
        let path = format!("/repos/{repo}/pulls?state=open&per_page={per}&page={page}");
        let resp = self.send(self.http_client.get(self.url(&path))).await?;
        let next = Self::link_of(&resp).or(Some((page + 1).to_string()));
        let doc = http::ok_json(resp, "github").await?;
        let items = doc
            .as_array()
            .context("pulls array")?
            .iter()
            .map(parse::parse_summary)
            .collect();
        Ok(Page { items, next })
    }

    async fn pull(&self, repo: &str, number: u64) -> Result<PullFull> {
        let doc = self
            .get_json(&format!("/repos/{repo}/pulls/{number}"))
            .await?;
        let mut pull = parse::parse_pull(&doc);
        pull.checks = self.checks(repo, &pull.head_sha).await.unwrap_or_default();
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
        Ok(parse::parse_pull(&http::ok_json(resp, "github").await?))
    }

    async fn merge(&self, repo: &str, number: u64, method: &str) -> Result<MergeReport> {
        let payload = serde_json::json!({"merge_method": method});
        let resp = self
            .send(
                self.http_client
                    .put(self.url(&format!("/repos/{repo}/pulls/{number}/merge")))
                    .json(&payload),
            )
            .await?;
        let doc = http::ok_json(resp, "github").await?;
        Ok(MergeReport {
            merged: doc.get("merged").and_then(|v| v.as_bool()).unwrap_or(false),
            sha: doc.get("sha").and_then(|v| v.as_str()).map(String::from),
            message: doc
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .into(),
        })
    }

    async fn checks(&self, repo: &str, sha: &str) -> Result<Vec<Check>> {
        let doc = self
            .get_json(&format!("/repos/{repo}/commits/{sha}/check-runs"))
            .await?;
        Ok(doc
            .get("check_runs")
            .and_then(|v| v.as_array())
            .context("check-runs envelope")?
            .iter()
            .map(parse::parse_check)
            .collect())
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
        http::ok_json(resp, "github").await?;
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
        let gh = GitHub::new("https://api.github.com", "t").unwrap();
        assert_eq!(gh.name(), "github");
    }
}
