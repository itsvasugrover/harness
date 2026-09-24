//! Response → port-shape mappers shared by both adapters. GitHub and
//! Gitea agree on the JSON we read (`number/title/state`, `labels[].name`,
//! `user.login`, `head.sha`), so one parser set serves both; only the
//! request paths and envelopes differ per forge.
use super::port::{Check, Comment, Issue, PullFull, PullSummary, Repo, Review, ReviewThread};

pub fn str(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").into()
}

pub fn parse_repo(v: &serde_json::Value) -> Repo {
    Repo {
        name: str(v, "name"),
        full_name: str(v, "full_name"),
        default_branch: str(v, "default_branch"),
    }
}

pub fn parse_issue(v: &serde_json::Value) -> Issue {
    Issue {
        number: v.get("number").and_then(|x| x.as_u64()).unwrap_or(0),
        title: str(v, "title"),
        state: str(v, "state"),
        labels: v
            .get("labels")
            .and_then(|x| x.as_array())
            .map(|xs| {
                xs.iter()
                    .map(|l| l.get("name").and_then(|n| n.as_str()).unwrap_or("").into())
                    .collect()
            })
            .unwrap_or_default(),
        author: v
            .pointer("/user/login")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .into(),
    }
}

pub fn parse_comment(v: &serde_json::Value) -> Comment {
    Comment {
        id: v.get("id").map(|x| x.to_string()).unwrap_or_default(),
        author: v
            .pointer("/user/login")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .into(),
        body: str(v, "body"),
    }
}

pub fn parse_check(v: &serde_json::Value) -> Check {
    Check {
        name: str(v, "name"),
        status: str(v, "status"),
        conclusion: str(v, "conclusion"),
    }
}

pub fn parse_pull(v: &serde_json::Value) -> PullFull {
    PullFull {
        number: v.get("number").and_then(|x| x.as_u64()).unwrap_or(0),
        title: str(v, "title"),
        state: str(v, "state"),
        head_sha: v
            .pointer("/head/sha")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .into(),
        mergeable: v
            .get("mergeable")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        checks: vec![],
        threads: vec![],
    }
}

pub fn parse_summary(v: &serde_json::Value) -> PullSummary {
    PullSummary {
        number: v.get("number").and_then(|x| x.as_u64()).unwrap_or(0),
        title: str(v, "title"),
        state: str(v, "state"),
        head_sha: v
            .pointer("/head/sha")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .into(),
    }
}

/// Gitea has no thread object: each pull review is one thread,
/// resolved when the review approved.
pub fn parse_gitea_review(v: &serde_json::Value) -> ReviewThread {
    let state = v.get("state").and_then(|x| x.as_str()).unwrap_or("");
    ReviewThread {
        id: v.get("id").map(|x| x.to_string()).unwrap_or_default(),
        resolved: state == "APPROVED",
        comments: vec![Comment {
            id: v.get("id").map(|x| x.to_string()).unwrap_or_default(),
            author: v
                .pointer("/user/login")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .into(),
            body: str(v, "body"),
        }],
    }
}

/// Approve-review response into the uniform Review shape.
/// Covers the GitHub (`user.login`) and Gitea (`user.login` or
/// `reviewer.username`) envelopes; unknown shapes read APPROVED.
pub fn parse_approval(v: &serde_json::Value, repo: &str, number: u64) -> Review {
    Review {
        id: v
            .get("id")
            .map(|id| id.to_string())
            .unwrap_or_else(|| format!("{repo}#{number}")),
        author: v
            .pointer("/user/login")
            .or_else(|| v.pointer("/reviewer/username"))
            .and_then(|u| u.as_str())
            .unwrap_or_default()
            .into(),
        state: v
            .get("state")
            .and_then(|s| s.as_str())
            .unwrap_or("APPROVED")
            .into(),
    }
}

pub fn parse_thread(v: &serde_json::Value) -> ReviewThread {
    let comments = v
        .pointer("/comments/nodes")
        .and_then(|x| x.as_array())
        .map(|xs| {
            xs.iter()
                .map(|c| Comment {
                    id: c.get("id").map(|x| x.to_string()).unwrap_or_default(),
                    author: c
                        .pointer("/author/login")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .into(),
                    body: str(c, "body"),
                })
                .collect()
        })
        .unwrap_or_default();
    ReviewThread {
        id: v.get("id").map(|x| x.to_string()).unwrap_or_default(),
        resolved: v
            .get("isResolved")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        comments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_issue_fixture() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"number":7,"title":"panic on empty","state":"open","labels":[{"name":"bug"}],"user":{"login":"octo"}}"#,
        )
        .unwrap();
        let i = parse_issue(&v);
        assert_eq!((i.number, i.author.as_str()), (7, "octo"));
        assert_eq!(i.labels, vec!["bug"]);
    }

    #[test]
    fn parses_check_and_pull_fixtures() {
        let c: serde_json::Value =
            serde_json::from_str(r#"{"name":"ci","status":"completed","conclusion":"success"}"#)
                .unwrap();
        assert_eq!(parse_check(&c).conclusion, "success");
        let p: serde_json::Value = serde_json::from_str(
            r#"{"number":3,"title":"fix","state":"open","head":{"sha":"abc"},"mergeable":true}"#,
        )
        .unwrap();
        let pull = parse_pull(&p);
        assert_eq!(pull.head_sha, "abc");
        assert!(pull.mergeable);
    }

    #[test]
    fn parses_thread_fixture() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"id":"T1","isResolved":false,"comments":{"nodes":[{"id":"C1","author":{"login":"rev"},"body":"nit"}]}}"#,
        )
        .unwrap();
        let t = parse_thread(&v);
        assert!(!t.resolved && t.comments.len() == 1);
    }

    #[test]
    fn parses_summary_and_gitea_review() {
        let s: serde_json::Value =
            serde_json::from_str(r#"{"number":9,"title":"w","state":"open","head":{"sha":"d"}}"#)
                .unwrap();
        assert_eq!(parse_summary(&s).head_sha, "d");
        let r: serde_json::Value = serde_json::from_str(
            r#"{"id":2,"state":"APPROVED","body":"lgtm","user":{"login":"rev"}}"#,
        )
        .unwrap();
        let t = parse_gitea_review(&r);
        assert!(t.resolved && t.comments.len() == 1);
    }
}
