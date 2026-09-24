//! Gitea private helpers: commit-status mapping, review threads,
//! label ids. Split from gitea.rs per the 300-line rule; the Forge
//! trait impl stays whole in gitea.rs.
use super::gitea::Gitea;
use super::parse;
use super::port::{Check, ReviewThread};
use anyhow::{Context, Result};

impl Gitea {
    /// Combined commit status mapped to checks (`state` is status and conclusion).
    pub(crate) async fn status_checks(&self, repo: &str, sha: &str) -> Result<Vec<Check>> {
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

    /// No thread object: each review is one thread (see `parse::parse_gitea_review`).
    pub(crate) async fn review_threads(
        &self,
        repo: &str,
        number: u64,
    ) -> Result<Vec<ReviewThread>> {
        let doc = self
            .get_json(&format!("/repos/{repo}/pulls/{number}/reviews"))
            .await?;
        Ok(doc
            .as_array()
            .context("reviews array")?
            .iter()
            .map(parse::parse_gitea_review)
            .collect())
    }

    /// Gitea creates issues with label *ids*: resolve names first.
    pub(crate) async fn label_ids(&self, repo: &str, names: &[String]) -> Result<Vec<i64>> {
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
